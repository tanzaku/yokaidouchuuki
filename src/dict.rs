use std::collections::{HashSet, VecDeque};
use std::io::Read;

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::cpu::{forward_step, forward_word, satisfy, Memory};

use crate::domain::{to_charcode_indices, CHAR_CODES, CODE2CHAR};

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

pub fn dict_search(expected_memory: &Memory) {
    let dict = Dict::new();

    // let mut words = vec![Vec::new(); 9];
    // for w in &dict.words {
    //     words[w.len()].push(w.clone());
    // }

    // for i in 1..=8 {
    //     for j in 1..=8.min(14 - i - 1) {
    //         let k = 14 - i - j;
    //         if k < 1 || k > 8 {
    //             continue;
    //         }

    //         for w1 in &words[i] {
    //             for w2 in &words[j] {
    //                 let mut w = w1.clone();
    //                 w.extend(w2);
    //                 words[k].par_iter().for_each(|w3| {
    //                     let mut w = w.clone();
    //                     w.extend(w3);

    //                     if satisfy(&w, expected_memory) {
    //                         eprintln!(
    //                             "find: {:?}, {}",
    //                             &w,
    //                             w.iter()
    //                                 .map(|&p| CODE2CHAR[CHAR_CODES[p] as usize])
    //                                 .collect::<String>()
    //                         );
    //                     }
    //                 });
    //             }
    //             // eprintln!(
    //             //     "{}",
    //             //     w.iter()
    //             //         .map(|&p| CODE2CHAR[CHAR_CODES[p] as usize])
    //             //         .collect::<String>()
    //             // );
    //         }
    //     }
    // }
    // panic!();

    fn build_pattern1(dict: &Dict, expected_memory: &Memory) -> Vec<Vec<Vec<Vec<bool>>>> {
        eprintln!("calc DP1");

        let bit = expected_memory.bit();
        let len = expected_memory.len();
        let sum = expected_memory.sum();
        let xor = expected_memory.xor();

        let mut pattern = vec![vec![vec![vec![false; 0x100]; 0x100]; bit + 1]; len + 1];

        pattern[len][bit][sum][xor] = true;

        // by dict
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

        if let Some(mut f) = std::fs::File::open("pattern2.bin").ok() {
            let mut pattern = Vec::new();
            f.read_to_end(&mut pattern).unwrap();
            return bincode::deserialize(&pattern[..]).unwrap();
        }

        let len = expected_memory.len();

        // pattern2 [cd2[0]][cd[1]][checkdigit5[1]][checkdigit5[3]]
        // memory.checkdigit5[1] = cpu.adc(memory.checkdigit5[1], memory.checkdigit2[1]);
        // let v = cpu.ror(memory.checkdigit5[3]); memory.checkdigit5[3] = cpu.adc(v, cpu.reg.a);

        let mut dp = vec![vec![vec![vec![false; 0x100]; 0x100]; 0x100]; len + 1];

        {
            let s0 = expected_memory.checkdigit2[0] as usize;
            let s1 = expected_memory.checkdigit2[1] as usize;
            let s2 = expected_memory.checkdigit5[0] as usize;
            dp[len][s0][s1][s2] = true;
        }

        // by dict
        loop {
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
                eprintln!("len={}", len);
                for s0 in 0..0x100 {
                    for s1 in 0..0x100 {
                        for word in &dict.words {
                            if len + word.len() >= visited.len() {
                                continue;
                            }

                            let mut memory = Memory {
                                checkdigit2: [s0 as u8, s1 as u8],
                                password_len: 0,
                                checkdigit5: [0, 0, 0, 0, 0],
                            };

                            forward_word(&mut memory, word);

                            for s2 in 0..0x100 {
                                if !visited[len][s0][s1][s2] {
                                    continue;
                                }

                                let next_len = len + word.len();
                                let next_s0 = memory.checkdigit2[0] as usize;
                                let next_s1 = memory.checkdigit2[1] as usize;
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
                std::fs::write("pattern2.bin", bincode::serialize(&dp).unwrap()).unwrap();
                break dp;
            }
        }
    }

    let pattern1 = build_pattern1(&dict, expected_memory);

    let pattern2 = build_pattern2(&dict, expected_memory);

    // fn contains_yasu(words: &Vec<usize>) -> bool {
    //     let yasu = [21, 0, 26, 38];
    //     if words.len() < yasu.len() {
    //         return false;
    //     }
    //     for i in 0..=words.len() - yasu.len() {
    //         if words[i..].starts_with(&yasu) {
    //             return true;
    //         }
    //     }
    //     false
    // }

    fn is_number(index: usize) -> bool {
        match index {
            29 => true, // '0'
            4 => true,  // '1'
            10 => true, // '2'
            16 => true, // '3'
            22 => true, // '4'
            28 => true, // '5'
            5 => true,  // '6'
            11 => true, // '7'
            17 => true, // '8'
            23 => true, // '9'
            _ => false, //
        }
    }

    fn is_symbol(index: usize) -> bool {
        match index {
            39 => true, // '-'
            33 => true, // '.'
            35 => true, // '!'
            _ => false, //
        }
    }

    fn is_vowel(index: usize) -> bool {
        match index {
            0 => true,
            7 => true,
            38 => true,
            24 => true,
            2 => true,
            _ => false, //
        }
    }

    fn is_alpha(index: usize) -> bool {
        match index {
            0 => true,
            6 => true,
            12 => true,
            18 => true,
            24 => true,
            30 => true,
            36 => true,
            1 => true,
            7 => true,
            13 => true,
            19 => true,
            25 => true,
            31 => true,
            37 => true,
            2 => true,
            8 => true,
            14 => true,
            20 => true,
            26 => true,
            32 => true,
            38 => true,
            3 => true,
            9 => true,
            15 => true,
            21 => true,
            27 => true,
            _ => false, //
        }
    }

    // HENTAIOSUGI
    fn bad_japanese(words: &Vec<usize>, word: &Vec<usize>) -> bool {
        if words.is_empty() {
            return false;
        }

        let c = words[words.len() - 1];
        if !is_alpha(c) {
            return false;
        }

        if is_number(word[0]) {
            if is_vowel(c) {
                return false;
            } else {
                return true;
            }
        }

        // if !is_vowel(c) && !is_vowel(word[0]) {
        //     return true;
        // }

        if c == word[0] {
            return true;
        }

        if words.len() >= 2 {
            let c2 = words[words.len() - 2];
            if !is_vowel(c2) && !is_vowel(c) && !is_vowel(word[0]) {
                // if words.len() >= 3 {
                //     let c3 = words[words.len() - 3];
                //     // KISSYのあとの子音で止まらないように
                //     if !(c3 == 26 && c2 == 26 && c == 21) {
                //         return true;
                //     }
                // }
                return true;
            }
        }

        if words.len() >= 3 {
            let c2 = words[words.len() - 2];
            let c3 = words[words.len() - 3];
            if is_vowel(c3) && is_vowel(c2) && is_vowel(c) && is_vowel(word[0]) {
                return true;
            }
        }

        return false;
    }

    fn suffix_consecutive_digits_length(words: &Vec<usize>) -> usize {
        (0..words.len())
            .rev()
            .take_while(|i| is_number(words[*i]))
            .count() as usize
    }

    fn next(
        word: &Vec<usize>,
        expected_memory: &Memory,
        memory: &Memory,
        words: &mut Vec<usize>,
    ) -> Option<Memory> {
        if words.len() + word.len() > memory.len() {
            return None;
        }

        // . or - の記号の連続はスキップ
        if is_symbol(word[0]) && is_symbol(*words.last().unwrap_or(&0)) {
            return None;
        }

        // 日本語的に不自然なのはスキップ
        if bad_japanese(words, word) {
            return None;
        }

        // if suffix_consecutive_digits_length(words) == 4 && is_number(word[0]) {
        //     return None;
        // }

        let mut memory = memory.clone();

        forward_word(&mut memory, word);

        // TODO bitの下限で枝刈り

        if memory.bit() > expected_memory.bit() {
            return None;
        }

        let suffix = [37, 2, 39, 4, 35];
        if words.len() + suffix.len() >= expected_memory.len() {
            let i = expected_memory.len() - words.len();

            if word.len() != 1 || word[0] != suffix[suffix.len() - i] {
                return None;
            }
        }

        // if expected_memory.bit() > memory.bit() + 5 * (memory.len() - words.len()) {
        //     return None;
        // }

        words.extend(word);

        Some(memory)
    }

    fn dfs_dict(
        dict: &Dict,
        pattern1: &Vec<Vec<Vec<Vec<bool>>>>,
        pattern2: &Vec<Vec<Vec<Vec<bool>>>>,
        expected_memory: &Memory,
        memory: &Memory,
        words: &mut Vec<usize>,
    ) {
        // if words != &to_charcode_indices(&"HENTAIOSUGI"[0..words.len()]) {
        //     return;
        // }

        // dbg!(&words);

        let len = words.len();
        let bit = memory.bit();
        let sum = memory.sum();
        let xor = memory.xor();
        // dbg!(len, bit, sum, xor);

        if !pattern1[len][bit][sum][xor] {
            return;
        }

        let s0 = memory.checkdigit2[0] as usize;
        let s1 = memory.checkdigit2[1] as usize;
        let s2 = memory.checkdigit5[0] as usize;
        if !pattern2[len][s0][s1][s2] {
            return;
        }

        // dbg!(words
        //     .iter()
        //     .map(|&p| CODE2CHAR[CHAR_CODES[p] as usize])
        //     .collect::<String>());
        // eprintln!(
        //     "checking: {}",
        //     words
        //         .iter()
        //         .map(|&p| CODE2CHAR[CHAR_CODES[p] as usize])
        //         .collect::<String>()
        // );
        if len == expected_memory.len() {
            // eprintln!(
            //     "checking: {}",
            //     words
            //         .iter()
            //         .map(|&p| CODE2CHAR[CHAR_CODES[p] as usize])
            //         .collect::<String>()
            // );
            // if satisfy(words, expected_memory) {
            if memory == expected_memory {
                eprintln!(
                    "find: {:?}, {}",
                    &words,
                    words
                        .iter()
                        .map(|&p| CODE2CHAR[CHAR_CODES[p] as usize])
                        .collect::<String>()
                );
                // panic!();
            }
            return;
        }

        // for word in &dict.words {
        //     if let Some((len, bit, sum, xor)) = next(word, len, bit, sum, xor, words) {
        //         dfs_dict(&expected_memory, &dict, &pattern, len, bit, sum, xor, words);
        //         for _ in 0..word.len() {
        //             words.pop();
        //         }
        //     }
        // }
        dict.words.par_iter().for_each(|word| {
            let mut words = words.clone();
            if let Some(memory) = next(word, expected_memory, &memory, &mut words) {
                dfs_dict(
                    &dict,
                    &pattern1,
                    &pattern2,
                    &expected_memory,
                    &memory,
                    &mut words,
                );
            }
        });
    }

    // {
    //     let mut memory = Memory::new(expected_memory.len() as u8);
    //     forward_word(&mut memory, &to_charcode_indices("HE"));
    //     let s0 = memory.checkdigit2[0] as usize;
    //     let s1 = memory.checkdigit2[1] as usize;
    //     let s2 = memory.checkdigit5[0] as usize;
    //     let len = 2;
    //     dbg!(len, s0, s1, s2, pattern2[len][s0][s1][s2]);
    //     // for i in 0..CHAR_CODES.len() {
    //     //     let mut w = to_charcode_indices("HE");
    //     //     w.push(CHAR_CODES[i] as usize);
    //     //     let mut memory = Memory::new(expected_memory.len() as u8);
    //     //     forward_word(&mut memory, &w);
    //     //     let s0 = memory.checkdigit2[0] as usize;
    //     //     let s1 = memory.checkdigit2[1] as usize;
    //     //     let s2 = memory.checkdigit5[0] as usize;
    //     //     dbg!(len, s0, s1, s2, pattern2[len][s0][s1][s2]);
    //     // }
    //     return;
    // }

    eprintln!("start search");
    dict.words.par_iter().for_each(|word| {
        if is_symbol(word[0]) {
            return;
        }

        let memory = Memory::new(expected_memory.len() as u8);
        let mut words = Vec::new();
        if let Some(memory) = next(word, expected_memory, &memory, &mut words) {
            dfs_dict(
                &dict,
                &pattern1,
                &pattern2,
                &expected_memory,
                &memory,
                &mut words,
            );
        }
    });
}
