use once_cell::sync::Lazy;

use crate::cpu::Memory;
use crate::opt::OPT;

use crate::domain::{is_alpha, is_number, is_symbol, is_vowel};

type Validator = fn(&Memory, &[usize], &[usize]) -> bool;

const VALIDATORS: Lazy<Vec<Validator>> = Lazy::new(|| {
    let mut validators: Vec<Validator> = vec![
        validate_option,
        validate_first_char_is_symbol,
        validate_consecutive_symbols,
        validate_suffix_consecutive_digits_length,
    ];
    if !OPT.disable_japanese_pruning {
        validators.push(validate_natural_japanese);
    }
    validators
});

static STATIC_VALIDATORS: Lazy<Vec<Validator>> = VALIDATORS;

// オプションによるvalidation
pub fn satisfy_option_constraint(expected_memory: &Memory, index: usize, word: &[usize]) -> bool {
    if let Some(prefix) = &OPT.prefix {
        if index < prefix.len() {
            let n = (prefix.len() - index).min(word.len());
            if prefix[index..index + n] != word[0..n] {
                return false;
            }
        }
    }

    if let Some(suffix) = &OPT.suffix {
        let i = index.max(expected_memory.len() - suffix.len());
        let j = index + word.len();
        if i < j {
            let o = expected_memory.len() - suffix.len();
            if suffix[i - o..j - o] != word[i - index..] {
                return false;
            }
        }
    }

    true
}

// オプションによるvalidation
fn validate_option(expected_memory: &Memory, password: &[usize], append_word: &[usize]) -> bool {
    satisfy_option_constraint(expected_memory, password.len(), append_word)
}

/// 日本語として自然な言葉かどうかを検証する
fn validate_natural_japanese(
    _expected_memory: &Memory,
    password: &[usize],
    append_word: &[usize],
) -> bool {
    fn non_vowel_before_symbol(password_last_char: usize, append_word_first_char: usize) -> bool {
        !is_vowel(password_last_char) && is_symbol(append_word_first_char)
    }

    fn non_vowel_before_number(password_last_char: usize, append_word_first_char: usize) -> bool {
        !is_vowel(password_last_char) && is_number(append_word_first_char)
    }

    fn consecutive_same_char(password_last_char: usize, append_word_first_char: usize) -> bool {
        password_last_char == append_word_first_char
    }

    fn consecutive_vowel(password: &[usize], append_word: &[usize]) -> bool {
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

    // TODO これはオフったほうがいい？
    fn consecutive_non_vowel(password: &[usize], append_word: &[usize]) -> bool {
        if password.len() < 2 {
            return false;
        }

        let c0 = password[password.len() - 2];
        let c1 = password[password.len() - 1];
        let c2 = append_word[0];
        !is_vowel(c0) && !is_vowel(c1) && !is_vowel(c2)
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
    _expected_memory: &Memory,
    password: &[usize],
    append_word: &[usize],
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
fn validate_consecutive_symbols(
    _expected_memory: &Memory,
    password: &[usize],
    append_word: &[usize],
) -> bool {
    if let Some(&c1) = password.last() {
        let c2 = append_word[0];
        return !is_symbol(c1) || !is_symbol(c2);
    }

    true
}

// 記号始まりはNG
fn validate_first_char_is_symbol(
    _expected_memory: &Memory,
    password: &[usize],
    append_word: &[usize],
) -> bool {
    if !password.is_empty() {
        return true;
    }

    !is_symbol(append_word[0])
}

pub fn is_valid_password(
    expected_memory: &Memory,
    password: &[usize],
    append_word: &[usize],
) -> bool {
    STATIC_VALIDATORS
        .iter()
        .all(|validator| validator(expected_memory, password, append_word))
}
