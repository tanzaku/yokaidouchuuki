use std::collections::HashSet;
use std::io::Read;

use crate::cpu::{satisfy, Memory};

use crate::domain::{CHAR_CODES, CODE2CHAR};

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
            if !set.insert(s) {
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
    let bit = expected_memory.bit();
    let len = expected_memory.len();
    let sum = expected_memory.sum();
    let xor = expected_memory.xor();

    let mut pattern = vec![vec![vec![vec![false; 0x100]; 0x100]; bit + 1]; len + 1];

    eprintln!("calc DP");
    pattern[0][0][0][0] = true;

    // by dict
    for len in 0..pattern.len() - 1 {
        for bit in 0..pattern[len].len() {
            for sum in 0..0x100 {
                for xor in 0..0x100 {
                    if pattern[len][bit][sum][xor] {
                        for word in &dict.words {
                            if len + word.len() >= pattern.len() {
                                continue;
                            }
                            let len = len + word.len();
                            let mut bit = bit;
                            let mut sum = sum;
                            let mut xor = xor;
                            for &i in word {
                                let c = CHAR_CODES[i] as usize;
                                bit += c.count_ones() as usize;
                                sum = (sum + c) & 0xFF;
                                xor ^= c;
                            }
                            if bit >= pattern[len].len() {
                                continue;
                            }
                            pattern[len][bit][sum][xor] = true;
                        }
                    }
                }
            }
        }
    }

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

    fn suffix_consecutive_digits_length(words: &Vec<usize>) -> usize {
        (0..words.len())
            .rev()
            .take_while(|i| is_number(words[*i]))
            .count() as usize
    }

    fn dfs_dict(
        expected_memory: &Memory,
        dict: &Dict,
        pattern: &Vec<Vec<Vec<Vec<bool>>>>,
        len: usize,
        bit: usize,
        sum: usize,
        xor: usize,
        words: &mut Vec<usize>,
    ) {
        if !pattern[len][bit][sum][xor] {
            return;
        }

        if len == 0 {
            // eprintln!(
            //     "checking: {}",
            //     words
            //         .iter()
            //         .map(|&p| CODE2CHAR[CHAR_CODES[p] as usize])
            //         .collect::<String>()
            // );
            if satisfy(words, expected_memory) {
                eprintln!(
                    "find: {:?}, {}",
                    &words,
                    words
                        .iter()
                        .map(|&p| CODE2CHAR[CHAR_CODES[p] as usize])
                        .collect::<String>()
                );
                panic!();
            }
            return;
        }

        for w in &dict.words {
            if w.len() > len {
                continue;
            }

            // . or - の記号の連続はスキップ
            if (w[0] == 39 || w[0] == 33)
                && (words.last() == Some(&39) || words.last() == Some(&33))
            {
                continue;
            }

            if suffix_consecutive_digits_length(words) == 4 && is_number(w[0]) {
                continue;
            }

            let mut len = len;
            let mut bit = bit as isize;
            let mut sum = sum;
            let mut xor = xor;

            for &i in w {
                let c = CHAR_CODES[i] as usize;

                len -= 1;
                sum = (sum - c) & 0xFF;
                bit = bit - c.count_ones() as isize;
                xor = xor ^ c;
            }

            if bit >= 0 {
                words.extend(w);
                dfs_dict(
                    expected_memory,
                    dict,
                    pattern,
                    len,
                    bit as usize,
                    sum,
                    xor,
                    words,
                );
                for _ in 0..w.len() {
                    words.pop();
                }
            }
        }
    }

    eprintln!("start search");
    for dbit in 0..=bit {
        for dsum in 0..=0x05 {
            let sum = (sum - dsum) & 0xFF;
            let bit = bit - dbit;
            eprintln!("check {} {}", dbit, dsum);
            if pattern[len][bit][sum][xor] {
                dfs_dict(
                    &expected_memory,
                    &dict,
                    &pattern,
                    len,
                    bit,
                    sum,
                    xor,
                    &mut Vec::new(),
                );
            }
        }
    }
}
