use std::collections::hash_map::DefaultHasher;
use std::collections::{HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::io::Read;

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::cpu::{forward_step, forward_word, satisfy, Memory};

use crate::domain::{
    is_alpha, is_number, is_symbol, is_vowel, to_charcode_indices, to_string, CHAR_CODES, CODE2CHAR,
};
use crate::opt::OPT;
use crate::pruning::is_valid_password;

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

fn satisfy_constraint(expected_memory: &Memory, index: usize, word: &Vec<usize>) -> bool {
    if let Some(prefix) = &OPT.prefix {
        if index < prefix.len() {
            let n = (prefix.len() - index).min(word.len());
            return prefix[index..index + n] == word[0..n];
        }
    }

    if let Some(suffix) = &OPT.suffix {
        let i = index.max(expected_memory.len() - suffix.len());
        let j = index + word.len();
        if i < j {
            let o = expected_memory.len() - suffix.len();
            return suffix[i - o..j - o] == word[i - index..];
        }
    }

    true
}

pub fn dict_search(expected_memory: &Memory) {
    let dict = Dict::new();

    fn build_pattern1(dict: &Dict, expected_memory: &Memory) -> Vec<Vec<Vec<Vec<bool>>>> {
        eprintln!("calc DP1");

        let bit = expected_memory.bit();
        let len = expected_memory.len();
        let sum = expected_memory.sum();
        let xor = expected_memory.xor();

        let mut pattern = vec![vec![vec![vec![false; 0x100]; 0x100]; bit + 1]; len + 1];

        pattern[len][bit][sum][xor] = true;

        for len in (0..pattern.len()).rev() {
            for bit in 0..pattern[len].len() {
                for sum in 0..0x100 {
                    for xor in 0..0x100 {
                        if !pattern[len][bit][sum][xor] {
                            continue;
                        }

                        for word in &dict.words {
                            if len < word.len() {
                                continue;
                            }

                            let len = len - word.len();
                            if !satisfy_constraint(expected_memory, len, word) {
                                continue;
                            }

                            let mut bit = bit;
                            let mut sum = sum;
                            let mut xor = xor;

                            let mut dbit = 0;
                            for &i in word {
                                let c = CHAR_CODES[i] as usize;
                                dbit += c.count_ones() as usize;
                                sum = (sum - c) & 0xFF;
                                xor ^= c;
                            }
                            if bit < dbit {
                                continue;
                            }
                            bit -= dbit;
                            pattern[len][bit][sum][xor] = true;
                            if bit > 1 {
                                pattern[len][bit - 1][sum][xor] = true;
                                pattern[len][bit - 1][(sum - 1) & 0xFF][xor] = true;
                            }
                            pattern[len][bit][(sum - 1) & 0xFF][xor] = true;
                        }
                    }
                }
            }
        }

        pattern
    }

    fn build_pattern2(dict: &Dict, expected_memory: &Memory) -> Vec<Vec<Vec<Vec<bool>>>> {
        eprintln!("calc DP2");

        std::fs::create_dir_all("cache").unwrap();
        let mut hasher = DefaultHasher::new();
        dict.hash(&mut hasher);
        OPT.prefix.hash(&mut hasher);
        OPT.suffix.hash(&mut hasher);
        let hash = hasher.finish();
        let cache_path = format!("cache/pattern2_{}.bin", hash);

        if !OPT.ignore_cache {
            if let Some(mut f) = std::fs::File::open(&cache_path).ok() {
                let mut pattern = Vec::new();
                f.read_to_end(&mut pattern).unwrap();
                return bincode::deserialize(&pattern[..]).unwrap();
            }
        }

        let len = expected_memory.len();

        let mut dp = vec![vec![vec![vec![false; 0x100]; 0x100]; 0x100]; len + 1];

        {
            let s0 = expected_memory.checkdigit2[0] as usize;
            let s1 = expected_memory.checkdigit2[1] as usize;
            let s2 = expected_memory.checkdigit5[0] as usize;
            dp[len][s0][s1][s2] = true;
        }

        // by dict
        loop {
            eprint!(".");
            let mut updated = false;

            let mut visited = vec![vec![vec![vec![false; 0x100]; 0x100]; 0x100]; len + 1];
            {
                let memory = Memory::new(expected_memory.len() as u8);
                let s0 = memory.checkdigit2[0] as usize;
                let s1 = memory.checkdigit2[1] as usize;
                let s2 = memory.checkdigit5[0] as usize;
                visited[0][s0][s1][s2] = true;
            }

            for len in 0..visited.len() {
                for s0 in 0..0x100 {
                    for s1 in 0..0x100 {
                        for word in &dict.words {
                            if len + word.len() >= visited.len() {
                                continue;
                            }

                            if !satisfy_constraint(expected_memory, len, word) {
                                continue;
                            }

                            let mut memory = Memory {
                                checkdigit2: [s0 as u8, s1 as u8],
                                password_len: 0,
                                checkdigit5: [0, 0, 0, 0, 0],
                            };

                            forward_word(&mut memory, word);

                            let next_len = len + word.len();
                            let next_s0 = memory.checkdigit2[0] as usize;
                            let next_s1 = memory.checkdigit2[1] as usize;

                            // TODO visited, dpをbitsetにして高速化したい
                            for s2 in 0..0x100 {
                                if !visited[len][s0][s1][s2] {
                                    continue;
                                }

                                let next_s2 = (s2 + memory.checkdigit5[0] as usize) & 0xFF;
                                visited[next_len][next_s0][next_s1][next_s2] = true;

                                if !dp[len][s0][s1][s2] && dp[next_len][next_s0][next_s1][next_s2] {
                                    dp[len][s0][s1][s2] = true;
                                    updated = true;
                                }
                            }
                        }
                    }
                }
            }
            if !updated {
                std::fs::write(&cache_path, bincode::serialize(&dp).unwrap()).unwrap();
                eprintln!("");
                break dp;
            }
        }
    }

    let pattern1 = build_pattern1(&dict, expected_memory);

    let pattern2 = build_pattern2(&dict, expected_memory);

    fn next(
        append_word: &Vec<usize>,
        expected_memory: &Memory,
        memory: &Memory,
        password: &mut Vec<usize>,
    ) -> Option<Memory> {
        if password.len() + append_word.len() > expected_memory.len() {
            return None;
        }

        if !is_valid_password(password, append_word) {
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

    fn dfs_dict(
        dict: &Dict,
        pattern1: &Vec<Vec<Vec<Vec<bool>>>>,
        pattern2: &Vec<Vec<Vec<Vec<bool>>>>,
        expected_memory: &Memory,
        memory: &Memory,
        password: &Vec<usize>,
    ) {
        let len = password.len();
        let bit = memory.bit();
        let sum = memory.sum();
        let xor = memory.xor();
        if !pattern1[len][bit][sum][xor] {
            return;
        }

        let s0 = memory.checkdigit2[0] as usize;
        let s1 = memory.checkdigit2[1] as usize;
        let s2 = memory.checkdigit5[0] as usize;
        if !pattern2[len][s0][s1][s2] {
            return;
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
            if memory == expected_memory {
                println!("find: {:?}, {}", &password, to_string(&password));
            }

            return;
        }

        dict.words.par_iter().for_each(|word| {
            let mut password = password.clone();
            if let Some(memory) = next(word, expected_memory, memory, &mut password) {
                dfs_dict(
                    &dict,
                    &pattern1,
                    &pattern2,
                    expected_memory,
                    &memory,
                    &password,
                );
            }
        });
    }

    eprintln!("start search");

    let memory = Memory::new(expected_memory.len() as u8);
    let mut password = Vec::new();
    dfs_dict(
        &dict,
        &pattern1,
        &pattern2,
        &expected_memory,
        &memory,
        &mut password,
    );
}
