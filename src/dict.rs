use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{BufReader, BufWriter, Read};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use chrono::Utc;
use once_cell::sync::Lazy;
use packed_simd_2::u8x32;
use rayon::iter::{
    IntoParallelIterator, IntoParallelRefIterator, ParallelBridge, ParallelIterator,
};
use rayon::slice::ParallelSlice;

use crate::bitset::BitSet256;
use crate::cpu::{forward_step_simd, forward_word, Memory};

use crate::domain::{to_charcode_index, to_string, CHAR_CODES, CODE2CHAR, CODE2INDEX};
use crate::opt::OPT;
use crate::pruning::{is_valid_password, satisfy_option_constraint};

#[derive(Hash)]
struct Dict {
    words: Vec<Vec<usize>>,
}

impl Dict {
    fn new() -> Self {
        let dict_file = "./dict.txt";
        let mut file = std::fs::File::open(dict_file).unwrap();
        let mut s = String::new();
        file.read_to_string(&mut s).unwrap();

        let mut set = HashSet::new();
        let mut words = Vec::new();
        for s in s.split_whitespace() {
            if s.is_empty() || s.contains(';') || !set.insert(s) {
                continue;
            }
            let mut v = Vec::new();
            for c in s.chars() {
                v.push({
                    let i = CODE2CHAR.iter().position(|&c2| c == c2).unwrap();
                    CHAR_CODES.iter().position(|&j| i == j as usize).unwrap()
                });
            }
            words.push(v);
        }

        Dict { words }
    }
}

static SPECIFIC_CHARS: Lazy<Vec<usize>> = Lazy::new(|| {
    ['C', 'F', 'L', 'Q', 'V', 'X']
        .iter()
        .map(|&c| to_charcode_index(c))
        .collect()
});

fn get_current_time() -> String {
    let tz = chrono_tz::Asia::Tokyo;
    let datetime = Utc::now().with_timezone(&tz);
    datetime.to_string()
}

const SIMD_LEN: usize = 1;

pub fn dict_search(expected_memory: &Memory) {
    let dict = Dict::new();

    #[inline]
    fn pattern1_index(len: usize, s0: usize, s1: usize, s3: usize) -> usize {
        ((len * 0x100 + s0) * 0x100 + s1) * 0x40 + s3
    }

    fn build_pattern1(dict: &Dict, expected_memory: &Memory) -> Vec<BitSet256> {
        eprintln!("calc DP1 ({})", get_current_time());

        std::fs::create_dir_all("cache").unwrap();
        let mut hasher = DefaultHasher::new();
        dict.hash(&mut hasher);
        OPT.prefix.hash(&mut hasher);
        OPT.suffix.hash(&mut hasher);
        let hash = hasher.finish();
        let cache_path = format!("cache/pattern1_{}.bin", hash);

        if !OPT.ignore_cache {
            if let Ok(f) = std::fs::File::open(&cache_path) {
                let mut f = BufReader::new(f);
                return bincode::deserialize_from(&mut f).unwrap();
            }
        }

        let len = expected_memory.len();

        let mut dp = vec![BitSet256::default(); 0x40 * 0x100 * 0x100 * (len + 1)];

        {
            let s0 = expected_memory.checkdigit2[0] as usize;
            let s1 = expected_memory.checkdigit2[1] as usize;
            let s2 = expected_memory.checkdigit5[0] as usize;
            let s3 = expected_memory.checkdigit5[2] as usize;
            let i = pattern1_index(len, s0, s1, s3);
            dp[i].flip(s2);
        }

        // by dict
        // グラフを作って最外ループを無くしたいが、自分の環境だとメモリが足りないため断念
        let mut n = len;
        loop {
            let mut updated = false;

            let mut visited = vec![BitSet256::default(); 0x40 * 0x100 * 0x100 * (len + 1)];
            {
                let memory = Memory::new(expected_memory.len() as u8);
                let s0 = memory.checkdigit2[0] as usize;
                let s1 = memory.checkdigit2[1] as usize;
                let s2 = memory.checkdigit5[0] as usize;
                let s3 = memory.checkdigit5[2] as usize;
                let i = pattern1_index(0, s0, s1, s3);
                visited[i].flip(s2);
            }

            for len in 0..n {
                eprint!(".");
                for s0 in 0..0x100 {
                    for s1 in 0..0x100 {
                        for s3 in 0..0x40 {
                            let i = pattern1_index(len, s0, s1, s3);
                            if visited[i].is_zero() {
                                continue;
                            }

                            for word in &dict.words {
                                if len + word.len() >= visited.len() {
                                    continue;
                                }

                                // if !satisfy_option_constraint(expected_memory, len, word) {
                                //     continue;
                                // }

                                let mut memory = Memory {
                                    checkdigit2: [s0 as u8, s1 as u8],
                                    password_len: 0,
                                    checkdigit5: [0 as u8, 0, s3 as u8, 0, 0],
                                };

                                forward_word(&mut memory, word);

                                let next_len = len + word.len();
                                let next_s0 = memory.checkdigit2[0] as usize;
                                let next_s1 = memory.checkdigit2[1] as usize;
                                let next_s3 = memory.checkdigit5[2] as usize;
                                let offset = memory.checkdigit5[0] as usize;

                                let i = pattern1_index(len, s0, s1, s3);
                                let j = pattern1_index(next_len, next_s0, next_s1, next_s3);
                                let rotated = visited[i].rot_left(offset);
                                visited[j] |= rotated;

                                let rotated = dp[j].rot_right(offset);
                                let prev = dp[i].clone();
                                dp[i] |= &rotated & &visited[i];
                                updated |= prev != dp[i];
                            }
                        }
                    }
                }
            }
            eprintln!();
            if !updated {
                let mut f = BufWriter::new(File::create(&cache_path).unwrap());
                bincode::serialize_into(&mut f, &dp).unwrap();
                break dp;
            }
            n -= 1;
        }
    }

    #[inline]
    fn pattern2_index(len: usize, s0: usize, s1: usize, s2: usize) -> usize {
        ((len * 0x100 + s0) * 0x100 + s1) * 0x100 + s2
    }

    fn build_pattern2(dict: &Dict, expected_memory: &Memory) -> Vec<BitSet256> {
        eprintln!("calc DP2 ({})", get_current_time());

        std::fs::create_dir_all("cache").unwrap();
        let mut hasher = DefaultHasher::new();
        dict.hash(&mut hasher);
        OPT.prefix.hash(&mut hasher);
        OPT.suffix.hash(&mut hasher);
        let hash = hasher.finish();
        let cache_path = format!("cache/pattern2_{}.bin", hash);

        if !OPT.ignore_cache {
            if let Ok(f) = std::fs::File::open(&cache_path) {
                let mut f = BufReader::new(f);
                return bincode::deserialize_from(&mut f).unwrap();
            }
        }

        let len = expected_memory.len();

        let mut dp = vec![BitSet256::default(); 0x100 * 0x100 * 0x100 * (len + 1)];

        {
            let s0 = expected_memory.checkdigit2[0] as usize;
            let s1 = expected_memory.checkdigit2[1] as usize;
            let s2 = expected_memory.checkdigit5[0] as usize;
            let s3 = expected_memory.checkdigit5[1] as usize;
            dp[pattern2_index(len, s0, s1, s2)].flip(s3);
        }

        // by dict
        // グラフを作って最外ループを無くしたいが、自分の環境だとメモリが足りないため断念
        let mut n = len;
        loop {
            let mut updated = false;

            let mut visited = vec![BitSet256::default(); 0x100 * 0x100 * 0x100 * (len + 1)];

            {
                let memory = Memory::new(expected_memory.len() as u8);
                let s0 = memory.checkdigit2[0] as usize;
                let s1 = memory.checkdigit2[1] as usize;
                let s2 = memory.checkdigit5[0] as usize;
                let s3 = memory.checkdigit5[1] as usize;
                visited[pattern2_index(0, s0, s1, s2)].flip(s3);
            }

            for len in 0..n {
                eprint!(".");
                for s0 in 0..0x100 {
                    for s1 in 0..0x100 {
                        for s2 in 0..0x100 {
                            let i = pattern2_index(len, s0, s1, s2);
                            if visited[i].is_zero() {
                                continue;
                            }

                            for word in &dict.words {
                                if len + word.len() >= visited.len() {
                                    continue;
                                }

                                // if !satisfy_option_constraint(expected_memory, len, word) {
                                //     continue;
                                // }

                                let mut memory = Memory {
                                    checkdigit2: [s0 as u8, s1 as u8],
                                    password_len: 0,
                                    checkdigit5: [s2 as u8, 0, 0, 0, 0],
                                };

                                forward_word(&mut memory, word);

                                let next_len = len + word.len();
                                let next_s0 = memory.checkdigit2[0] as usize;
                                let next_s1 = memory.checkdigit2[1] as usize;
                                let next_s2 = memory.checkdigit5[0] as usize;
                                let offset = memory.checkdigit5[1] as usize;

                                let rotated = visited[i].rot_left(offset);
                                let j = pattern2_index(next_len, next_s0, next_s1, next_s2);
                                visited[j] |= rotated;

                                let rotated = dp[j].rot_right(offset);
                                let prev = dp[i].clone();
                                dp[i] |= &rotated & &visited[i];
                                updated |= prev != dp[i];
                            }
                        }
                    }
                }
            }
            eprintln!();
            if !updated {
                let mut f = BufWriter::new(File::create(&cache_path).unwrap());
                bincode::serialize_into(&mut f, &dp).unwrap();
                break dp;
            }
            n -= 1;
        }
    }

    let pattern1 = build_pattern1(&dict, expected_memory);

    let pattern2 = build_pattern2(&dict, expected_memory);

    let dict3 = {
        let mut dict3s = Vec::new();
        fn init_dict3(dict: &Dict, cur_word: &mut Vec<usize>, dict3s: &mut Vec<Vec<usize>>) {
            if cur_word.len() == SIMD_LEN {
                dict3s.push(cur_word.clone());
                return;
            }

            for word in &dict.words {
                if cur_word.len() + word.len() > SIMD_LEN {
                    continue;
                }
                cur_word.extend(word);
                init_dict3(dict, cur_word, dict3s);
                cur_word.truncate(cur_word.len() - word.len());
            }
        }
        init_dict3(&dict, &mut Vec::new(), &mut dict3s);

        let mut dict3 = Vec::new();
        for d in dict3s.chunks(32) {
            let mut vec = Vec::new();
            for i in 0..SIMD_LEN {
                let v: Vec<_> = d
                    .iter()
                    .map(|w| CHAR_CODES[w[i]])
                    .chain(std::iter::repeat(0xFF))
                    .take(32)
                    .collect();
                vec.push(u8x32::from_slice_unaligned(&v));
            }

            dict3.push(vec);
        }
        dict3
    };

    fn is_valid_pattern(
        pattern1: &[BitSet256],
        pattern2: &[BitSet256],
        len: usize,
        memory: &Memory,
    ) -> bool {
        let s0 = memory.checkdigit2[0] as usize;
        let s1 = memory.checkdigit2[1] as usize;
        let s2 = memory.checkdigit5[0] as usize;

        let s3 = memory.checkdigit5[1] as usize;
        let i = pattern2_index(len, s0, s1, s2);
        if !pattern2[i].get(s3) {
            return false;
        }

        let s3 = memory.checkdigit5[2] as usize;
        let i = pattern1_index(len, s0, s1, s3);
        if !pattern1[i].get(s2) {
            return false;
        }

        true
    }

    fn next(
        pattern1: &[BitSet256],
        pattern2: &[BitSet256],
        append_word: &[usize],
        expected_memory: &Memory,
        memory: &Memory,
        password: &mut Vec<usize>,
    ) -> Option<Memory> {
        if password.len() + append_word.len() > expected_memory.len() {
            return None;
        }

        if !is_valid_password(expected_memory, password, append_word) {
            return None;
        }

        let mut memory = memory.clone();
        forward_word(&mut memory, append_word);

        if memory.bit() > expected_memory.bit() {
            return None;
        }

        let len = password.len() + append_word.len();
        if !is_valid_pattern(pattern1, pattern2, len, &memory) {
            return None;
        }

        password.extend(append_word);

        Some(memory)
    }

    #[allow(non_snake_case)]
    fn next_simd3(
        dict3: &Vec<u8x32>, // dict3[i][j]: i番目の組み合わせのj文字目
        pattern1: &[BitSet256],
        pattern2: &[BitSet256],
        expected_memory: &Memory,
        memory: &Memory,
        password: &[usize],
    ) -> Vec<(Memory, Vec<usize>)> {
        let (a31F4, a31F5, a31F7, a31F8, a31F9, a31FA, a31FB) = forward_step_simd(memory, dict3);

        (0..32)
            .take_while(|&i| dict3[0].extract(i) != 0xFF)
            .filter_map(move |i| {
                let memory = Memory {
                    checkdigit2: [a31F4.extract(i), a31F5.extract(i)],
                    password_len: memory.password_len,
                    checkdigit5: [
                        a31F7.extract(i),
                        a31F8.extract(i),
                        a31F9.extract(i),
                        a31FA.extract(i),
                        a31FB.extract(i),
                    ],
                };

                if memory.bit() > expected_memory.bit() {
                    return None;
                }

                if !is_valid_pattern(pattern1, pattern2, password.len() + SIMD_LEN, &memory) {
                    return None;
                }

                let mut password = password.to_vec();
                password.extend(dict3.iter().map(|&d| CODE2INDEX[d.extract(i) as usize]));
                Some((memory, password))
            })
            .collect()
    }

    fn contains_specific_char(word: &[usize]) -> bool {
        SPECIFIC_CHARS.par_iter().any(|c| word.contains(c))
    }

    fn dfs_dict(
        dict3: &Vec<Vec<u8x32>>,
        dict: &Dict,
        pattern1: &[BitSet256],
        pattern2: &[BitSet256],
        expected_memory: &Memory,
        memory: &Memory,
        password: &[usize],
    ) {
        let len = password.len();

        if OPT.verbose {
            eprintln!("checking: {}", to_string(password));
        }

        if len == expected_memory.len() {
            if memory == expected_memory && contains_specific_char(password) {
                println!("{}", to_string(password));
            }

            return;
        }

        dict3.iter().for_each(|dict3_0| {
            next_simd3(
                dict3_0,
                pattern1,
                pattern2,
                expected_memory,
                memory,
                password,
            )
            .iter()
            .for_each(|(memory, password)| {
                dfs_dict(
                    dict3,
                    dict,
                    pattern1,
                    pattern2,
                    expected_memory,
                    &memory,
                    &password,
                );
            });
        });

        // TODO nmcの探索
    }

    eprintln!("start search");

    let mut cache: Vec<_> = std::fs::read_to_string("progress.txt")
        .unwrap_or_default()
        .lines()
        .map(|s| s.to_owned())
        .collect();

    for w1 in &dict.words {
        let password_text = to_string(w1);

        eprintln!("trying... {} ({})", &password_text, get_current_time());

        if cache.contains(&password_text) {
            eprintln!("skipped");
            continue;
        }

        for w2 in &dict.words {
            let password: Vec<_> = w1.iter().chain(w2).cloned().collect();
            let password_text = to_string(&password);

            eprintln!("trying... {} ({})", &password_text, get_current_time());

            if cache.contains(&password_text) {
                eprintln!("skipped");
                continue;
            }

            let len = w1.len() + w2.len();
            let mut memory = Memory::new(expected_memory.len() as u8);
            forward_word(&mut memory, &password);

            if is_valid_pattern(&pattern1, &pattern2, len, &memory) {
                dict.words
                    .par_iter()
                    .flat_map(|w3| {
                        dict.words.par_iter().flat_map(|w4| {
                            dict.words.par_iter().flat_map(|w5| {
                                let mut memory = memory.clone();
                                let append_word: Vec<_> =
                                    w3.iter().chain(w4.iter()).chain(w5).cloned().collect();
                                let password: Vec<_> =
                                    password.iter().chain(&append_word).cloned().collect();

                                forward_word(&mut memory, &append_word);
                                if is_valid_pattern(&pattern1, &pattern2, password.len(), &memory) {
                                    Some((memory, password))
                                } else {
                                    None
                                }
                            })
                        })
                    })
                    .for_each(|(memory, password)| {
                        dfs_dict(
                            &dict3,
                            &dict,
                            &pattern1,
                            &pattern2,
                            expected_memory,
                            &memory,
                            &password,
                        );
                    });
            }

            cache.push(password_text);
            std::fs::write("progress.txt", cache.join("\n")).unwrap();
        }

        cache.push(password_text);
        std::fs::write("progress.txt", cache.join("\n")).unwrap();
    }
}
