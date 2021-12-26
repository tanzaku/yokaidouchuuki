use once_cell::sync::Lazy;

use crate::cpu::Memory;
use crate::opt::OPT;

use crate::domain::{
    is_alpha, is_dot, is_exclamation_mark, is_number, is_symbol, is_vowel, to_charcode_index,
    to_charcode_indices,
};

type Validator = fn(&Memory, &[usize], &[usize]) -> bool;

static CAN_TRANSITION: Lazy<Vec<Vec<bool>>> = Lazy::new(|| {
    let mut can_transition = vec![vec![false; 0x100]; 0x100];

    for c0 in "AIUEOc0123456789N".chars() {
        for c1 in "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.-!n".chars() {
            can_transition[to_charcode_index(c0)][to_charcode_index(c1)] = true;
        }
    }

    can_transition[to_charcode_index('n')][to_charcode_index('m')] = true;
    can_transition[to_charcode_index('m')][to_charcode_index('c')] = true;
    for c0 in ".-!".chars() {
        for c1 in "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789n".chars() {
            can_transition[to_charcode_index(c0)][to_charcode_index(c1)] = true;
        }
    }

    for c0 in "ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars() {
        // if c0 == 'F' || c0 == 'C' || c0 == 'L' || c0 == 'X' || c0 == 'V'  || c0 == 'Q' {
        //     continue;
        // }
        for c1 in "AIUEO".chars() {
            if c0 == 'W' && c1 != 'A' {
                continue;
            }
            if c0 == 'D' && c1 != 'A' && c1 != 'O' {
                continue;
            }
            can_transition[to_charcode_index(c0)][to_charcode_index(c1)] = true;
        }
    }

    // KISSY, TYAN, CHAN, 的な文字列を受理

    can_transition
});

static VALIDATORS: Lazy<Vec<Validator>> = Lazy::new(|| {
    let mut validators: Vec<Validator> = vec![
        // validate_option,
        // validate_transition,
        // validate_first_char_is_symbol,
        // validate_consecutive_symbols,
        // validate_suffix_consecutive_digits_length,
        // validate_atmost_one_dot,
    ];

    // if !OPT.disable_japanese_pruning {
    //     validators.push(validate_natural_japanese);
    // }
    validators
});

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

// 遷移テーブルによるvalidation
fn validate_transition(
    _expected_memory: &Memory,
    password: &[usize],
    append_word: &[usize],
) -> bool {
    if password.is_empty() {
        return true;
    }

    CAN_TRANSITION[password[password.len() - 1]][append_word[0]]
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

fn validate_atmost_one_dot(
    _expected_memory: &Memory,
    password: &[usize],
    append_word: &[usize],
) -> bool {
    if password.is_empty() {
        return true;
    }

    if is_dot(append_word[0]) && password.iter().any(|&c| is_dot(c)) {
        return false;
    }

    if is_exclamation_mark(append_word[0]) && password.iter().any(|&c| is_exclamation_mark(c)) {
        return false;
    }

    true
}

pub fn is_valid_password(
    expected_memory: &Memory,
    password: &[usize],
    append_word: &[usize],
) -> bool {
    VALIDATORS
        .iter()
        .all(|validator| validator(expected_memory, password, append_word))
}
