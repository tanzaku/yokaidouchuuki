use std::collections::{HashSet, VecDeque};
use std::io::Read;

use once_cell::sync::Lazy;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::cpu::{forward_step, forward_word, satisfy, Memory};

use crate::domain::{
    is_alpha, is_number, is_symbol, is_vowel, to_charcode_indices, CHAR_CODES, CODE2CHAR,
};

type Validator = fn(&Vec<usize>, &Vec<usize>) -> bool;

const VALIDATORS: Lazy<Vec<Validator>> = Lazy::new(|| {
    vec![
        validate_first_char_is_symbol,
        validate_consecutive_symbols,
        validate_suffix_consecutive_digits_length,
        validate_natural_japanese,
    ]
});

/// 日本語として自然な言葉かどうかを検証する
fn validate_natural_japanese(password: &Vec<usize>, append_word: &Vec<usize>) -> bool {
    fn non_vowel_before_symbol(password_last_char: usize, append_word_first_char: usize) -> bool {
        return !is_vowel(password_last_char) && is_symbol(append_word_first_char);
    }

    fn non_vowel_before_number(password_last_char: usize, append_word_first_char: usize) -> bool {
        return !is_vowel(password_last_char) && is_number(append_word_first_char);
    }

    fn consecutive_same_char(password_last_char: usize, append_word_first_char: usize) -> bool {
        return password_last_char == append_word_first_char;
    }

    fn consecutive_vowel(password: &Vec<usize>, append_word: &Vec<usize>) -> bool {
        if password.len() < 3 {
            return false;
        }

        let c0 = password[password.len() - 3];
        let c1 = password[password.len() - 2];
        let c2 = password[password.len() - 1];
        let c3 = append_word[0];

        // 母音が4回以上連続するケースを除外
        is_vowel(c0) && is_vowel(c1) && is_vowel(c2) && is_vowel(c3)
    }

    fn consecutive_non_vowel(password: &Vec<usize>, append_word: &Vec<usize>) -> bool {
        if password.len() < 2 {
            return false;
        }

        let c0 = password[password.len() - 2];
        let c1 = password[password.len() - 1];
        let c2 = append_word[0];
        return !is_vowel(c0) && !is_vowel(c1) && !is_vowel(c2);
    }

    if password.is_empty() {
        return true;
    }

    let password_last_char = password[password.len() - 1];
    let append_word_first_char = append_word[0];

    if is_alpha(password_last_char) {
        if non_vowel_before_symbol(password_last_char, append_word_first_char) {
            return false;
        }

        if non_vowel_before_number(password_last_char, append_word_first_char) {
            return false;
        }
    }

    // アルファベットでなければ以下のチェックはしない
    if !is_alpha(append_word_first_char) {
        return true;
    }

    if consecutive_same_char(password_last_char, append_word_first_char) {
        return false;
    }

    if consecutive_non_vowel(password, append_word) {
        return false;
    }

    if consecutive_vowel(password, append_word) {
        return false;
    }

    true
}

// 5桁以上の数値はNG
fn validate_suffix_consecutive_digits_length(
    password: &Vec<usize>,
    append_word: &Vec<usize>,
) -> bool {
    if append_word.len() == 1 && is_number(append_word[0]) {
        let len = password.iter().rev().take_while(|&&c| is_number(c)).count() as usize;

        if len + append_word.len() > 4 {
            return false;
        }
    }

    true
}

// 記号の連続はNG
fn validate_consecutive_symbols(password: &Vec<usize>, append_word: &Vec<usize>) -> bool {
    if let Some(&c1) = password.last() {
        let c2 = append_word[0];
        return !is_symbol(c1) || !is_symbol(c2);
    }

    true
}

// 記号始まりはNG
fn validate_first_char_is_symbol(password: &Vec<usize>, append_word: &Vec<usize>) -> bool {
    if !password.is_empty() {
        return true;
    }

    !is_symbol(append_word[0])
}

pub fn is_valid_password(password: &Vec<usize>, append_word: &Vec<usize>) -> bool {
    VALIDATORS
        .iter()
        .all(|validator| validator(password, append_word))
}