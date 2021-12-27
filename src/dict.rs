use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{BufReader, BufWriter, Read};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use chrono::Utc;
use once_cell::sync::Lazy;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rayon::slice::ParallelSlice;

use crate::bitset::BitSet256;
use crate::cpu::{forward_word, Memory};

use crate::domain::{to_charcode_index, to_string, CHAR_CODES, CODE2CHAR};
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

pub fn dict_search(expected_memory: &Memory) {
    let dict = Dict::new();

    // fn build_pattern1(dict: &Dict, expected_memory: &Memory) -> Vec<Vec<Vec<Vec<bool>>>> {
    //     eprintln!("calc DP1");

    //     let bit = expected_memory.bit();
    //     let len = expected_memory.len();
    //     let sum = expected_memory.sum();
    //     let xor = expected_memory.xor();

    //     let mut pattern = vec![vec![vec![vec![false; 0x100]; 0x100]; bit + 1]; len + 1];

    //     pattern[len][bit][sum][xor] = true;

    //     for len in (0..pattern.len()).rev() {
    //         for bit in 0..pattern[len].len() {
    //             for sum in 0..0x100 {
    //                 for xor in 0..0x100 {
    //                     if !pattern[len][bit][sum][xor] {
    //                         continue;
    //                     }

    //                     for word in &dict.words {
    //                         if len < word.len() {
    //                             continue;
    //                         }

    //                         let len = len - word.len();
    //                         if !satisfy_option_constraint(expected_memory, len, word) {
    //                             continue;
    //                         }

    //                         let mut bit = bit;
    //                         let mut sum = sum;
    //                         let mut xor = xor;

    //                         let mut dbit = 0;
    //                         for &i in word {
    //                             let c = CHAR_CODES[i] as usize;
    //                             dbit += c.count_ones() as usize;
    //                             sum = (sum - c) & 0xFF;
    //                             xor ^= c;
    //                         }
    //                         if bit < dbit {
    //                             continue;
    //                         }
    //                         bit -= dbit;
    //                         pattern[len][bit][sum][xor] = true;
    //                         if bit > 1 {
    //                             pattern[len][bit - 1][sum][xor] = true;
    //                             pattern[len][bit - 1][(sum - 1) & 0xFF][xor] = true;
    //                         }
    //                         pattern[len][bit][(sum - 1) & 0xFF][xor] = true;
    //                     }
    //                 }
    //             }
    //         }
    //     }

    //     pattern
    // }

    fn build_pattern1(dict: &Dict, expected_memory: &Memory) -> Vec<Vec<Vec<Vec<BitSet256>>>> {
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

        let mut dp = vec![vec![vec![vec![BitSet256::default(); 0x40]; 0x100]; 0x100]; len + 1];

        {
            let s0 = expected_memory.checkdigit2[0] as usize;
            let s1 = expected_memory.checkdigit2[1] as usize;
            let s2 = expected_memory.checkdigit5[0] as usize;
            let s3 = expected_memory.checkdigit5[2] as usize;
            dp[len][s0][s1][s3].flip(s2);
        }

        // by dict
        // グラフを作って最外ループを無くしたいが、自分の環境だとメモリが足りないため断念
        let mut n = len;
        loop {
            let mut updated = false;

            let mut visited = vec![vec![[[BitSet256::default(); 0x40]; 0x100]; 0x100]; len + 1];
            {
                let memory = Memory::new(expected_memory.len() as u8);
                let s0 = memory.checkdigit2[0] as usize;
                let s1 = memory.checkdigit2[1] as usize;
                let s2 = memory.checkdigit5[0] as usize;
                let s3 = memory.checkdigit5[2] as usize;
                visited[0][s0][s1][s3].flip(s2);
            }

            for len in 0..n {
                eprint!(".");
                for s0 in 0..0x100 {
                    for s1 in 0..0x100 {
                        for s3 in 0..0x40 {
                            if visited[len][s0][s1][s3].is_zero() {
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

                                let rotated = visited[len][s0][s1][s3].rot_left(offset);
                                visited[next_len][next_s0][next_s1][next_s3] |= rotated;

                                let rotated =
                                    dp[next_len][next_s0][next_s1][next_s3].rot_right(offset);
                                let prev = dp[len][s0][s1][s3].clone();
                                dp[len][s0][s1][s3] |= &rotated & &visited[len][s0][s1][s3];
                                updated |= prev != dp[len][s0][s1][s3];
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

    fn build_pattern2(dict: &Dict, expected_memory: &Memory) -> Vec<Vec<Vec<Vec<BitSet256>>>> {
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

        let mut dp = vec![vec![vec![vec![BitSet256::default(); 0x100]; 0x100]; 0x100]; len + 1];

        {
            let s0 = expected_memory.checkdigit2[0] as usize;
            let s1 = expected_memory.checkdigit2[1] as usize;
            let s2 = expected_memory.checkdigit5[0] as usize;
            let s3 = expected_memory.checkdigit5[1] as usize;
            dp[len][s0][s1][s2].flip(s3);
        }

        // by dict
        // グラフを作って最外ループを無くしたいが、自分の環境だとメモリが足りないため断念
        let mut n = len;
        loop {
            let mut updated = false;

            let mut visited = vec![vec![[[BitSet256::default(); 0x100]; 0x100]; 0x100]; len + 1];
            {
                let memory = Memory::new(expected_memory.len() as u8);
                let s0 = memory.checkdigit2[0] as usize;
                let s1 = memory.checkdigit2[1] as usize;
                let s2 = memory.checkdigit5[0] as usize;
                let s3 = memory.checkdigit5[1] as usize;
                visited[0][s0][s1][s2].flip(s3);
            }

            for len in 0..n {
                eprint!(".");
                for s0 in 0..0x100 {
                    for s1 in 0..0x100 {
                        for s2 in 0..0x100 {
                            if visited[len][s0][s1][s2].is_zero() {
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

                                let rotated = visited[len][s0][s1][s2].rot_left(offset);
                                visited[next_len][next_s0][next_s1][next_s2] |= rotated;

                                let rotated =
                                    dp[next_len][next_s0][next_s1][next_s2].rot_right(offset);
                                let prev = dp[len][s0][s1][s2].clone();
                                dp[len][s0][s1][s2] |= &rotated & &visited[len][s0][s1][s2];
                                updated |= prev != dp[len][s0][s1][s2];
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

    fn next(
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

        password.extend(append_word);

        Some(memory)
    }

    // {
    //     let mut password = Vec::new();
    //     dict.words.iter().take(5).for_each(|word| {
    //         password.extend(word.iter().map(|&c| c as u8));
    //         password.push(0xFF);
    //     });
    //     std::fs::write("progress.txt", &password).unwrap();
    // }

    fn dfs_dict(
        dict: &Dict,
        cache: &mut Vec<u8>,
        pattern1: &[Vec<Vec<Vec<BitSet256>>>],
        // pattern1: &[Vec<Vec<Vec<bool>>>],
        pattern2: &[Vec<Vec<Vec<BitSet256>>>],
        expected_memory: &Memory,
        memory: &Memory,
        password: &[usize],
        contains_specific_char: bool,
    ) {
        let len = password.len();

        {
            let s0 = memory.checkdigit2[0] as usize;
            let s1 = memory.checkdigit2[1] as usize;
            let s2 = memory.checkdigit5[0] as usize;

            let s3 = memory.checkdigit5[1] as usize;
            if !pattern2[len][s0][s1][s2].get(s3) {
                return;
            }

            let s3 = memory.checkdigit5[2] as usize;
            if !pattern1[len][s0][s1][s3].get(s2) {
                return;
            }

            // let bit = memory.bit();
            // let sum = memory.sum();
            // let xor = memory.xor();
            // if !pattern1[len][bit][sum][xor] {
            //     return;
            // }
        }

        if OPT.verbose {
            eprintln!(
                "checking: {}",
                password
                    .iter()
                    .map(|&p| CODE2CHAR[CHAR_CODES[p] as usize])
                    .collect::<String>()
            );
        }

        if len == expected_memory.len() {
            if contains_specific_char && memory == expected_memory {
                println!("{}", to_string(password));
            }

            return;
        }

        if password.len() <= 1 {
            dict.words.iter().for_each(|word| {
                let mut password = password.to_vec();
                if let Some(memory) = next(word, expected_memory, memory, &mut password) {
                    eprintln!(
                        "trying... {} ({})",
                        to_string(&password),
                        get_current_time()
                    );

                    let password_u8 = {
                        let mut password_u8: Vec<_> = password.iter().map(|&p| p as u8).collect();
                        password_u8.push(0xFF);
                        password_u8
                    };

                    if cache
                        .par_windows(password_u8.len())
                        .any(|cache| cache == password_u8)
                    {
                        eprintln!("skipped");
                        return;
                    }

                    dfs_dict(
                        dict,
                        cache,
                        pattern1,
                        pattern2,
                        expected_memory,
                        &memory,
                        &password,
                        contains_specific_char
                            || SPECIFIC_CHARS.par_iter().any(|c| word.contains(c)),
                    );

                    cache.extend(password_u8);
                    std::fs::write("progress.txt", &cache).unwrap();
                }
            });
        } else {
            dict.words.par_iter().for_each(|word| {
                let mut password = password.to_vec();
                if let Some(memory) = next(word, expected_memory, memory, &mut password) {
                    dfs_dict(
                        dict,
                        &mut Vec::with_capacity(0), // このルートはキャッシュしないので
                        pattern1,
                        pattern2,
                        expected_memory,
                        &memory,
                        &password,
                        contains_specific_char
                            || SPECIFIC_CHARS.par_iter().any(|c| word.contains(c)),
                    );
                }
            });
        }
    }

    eprintln!("start search");

    let memory = Memory::new(expected_memory.len() as u8);
    let password = Vec::new();
    let mut cache = std::fs::read("progress.txt").unwrap_or_default();
    dfs_dict(
        &dict,
        &mut cache,
        &pattern1,
        &pattern2,
        expected_memory,
        &memory,
        &password,
        false,
    );
}
