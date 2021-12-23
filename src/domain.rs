use once_cell::sync::Lazy;

use crate::cpu::Memory;

// 圧縮した文字集合
pub const CHAR_CODES: [u8; 42] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x10, 0x11, 0x12, 0x13,
    0x14, 0x15, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x28, 0x29,
    0x2A, 0x2B, 0x2C, 0x2D, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35,
];

pub static CODE2CHAR: Lazy<[char; 0x100]> = Lazy::new(|| {
    let mut table = ['?'; 0x100];
    table[0x00] = 'A';
    table[0x01] = 'H';
    table[0x02] = 'O';
    table[0x03] = 'V';
    table[0x04] = '1';
    table[0x05] = '6';
    table[0x08] = 'B';
    table[0x09] = 'I';
    table[0x0A] = 'P';
    table[0x0B] = 'W';
    table[0x0C] = '2';
    table[0x0D] = '7';
    table[0x10] = 'C';
    table[0x11] = 'J';
    table[0x12] = 'Q';
    table[0x13] = 'X';
    table[0x14] = '3';
    table[0x15] = '8';
    table[0x18] = 'D';
    table[0x19] = 'K';
    table[0x1A] = 'R';
    table[0x1B] = 'Y';
    table[0x1C] = '4';
    table[0x1D] = '9';
    table[0x20] = 'E';
    table[0x21] = 'L';
    table[0x22] = 'S';
    table[0x23] = 'Z';
    table[0x24] = '5';
    table[0x25] = '0';
    table[0x28] = 'F';
    table[0x29] = 'M';
    table[0x2A] = 'T';
    table[0x2B] = '-';
    table[0x2C] = 'n';
    table[0x2D] = '!';
    table[0x30] = 'G';
    table[0x31] = 'N';
    table[0x32] = 'U';
    table[0x33] = '.';
    table[0x34] = 'm';
    table[0x35] = 'c';
    table
});

// 未知のパスワード1(11桁)を解析する
// yokai03.exe 64 98 0B 15 91 18 B1 15
// 64: a[10]の逆順
// 98: a[9]の逆順
// 0B: 長さ
// 15: sum(a) + cnt(checkdigit2[0] >= 0xE5)
// 91: sum(checkdigit2[1])
// 18: xor が 0x18
// B1: a[10] + ror(a[9] + ror(a[8] + ...))
// 15: 0x15 bit

// KID
pub const EXPECTED_MEMORY_KID: Memory = Memory {
    checkdigit2: [0x00, 0x51],
    password_len: 0x03,
    checkdigit5: [0x3A, 0xE9, 0x08, 0x23, 0x07],
};

// 8文字 818-6104
pub const EXPECTED_MEMORY_8: Memory = Memory {
    checkdigit2: [0xDC, 0xD9],
    password_len: 0x08,
    checkdigit5: [0xA3, 0xE3, 0x17, 0x28, 0x15],
};

// 11文字
// yokai03.exe 64 98 0B 15 91 18 B1 15
pub const EXPECTED_MEMORY_11: Memory = Memory {
    checkdigit2: [0x64, 0x98],
    password_len: 0x0B,
    checkdigit5: [0x15, 0x91, 0x18, 0xB1, 0x15],
};

// 14文字
// yokai03.exe 65 94 0E AC E9 07 33 25
pub const EXPECTED_MEMORY_14: Memory = Memory {
    checkdigit2: [0x65, 0x94],
    password_len: 0x0E,
    checkdigit5: [0xAC, 0xE9, 0x07, 0x33, 0x25],
};

// 14文字(monitorのところにあるハッシュ値)
pub const EXPECTED_MEMORY_14_2: Memory = Memory {
    checkdigit2: [0x51, 0x62],
    password_len: 0x0E,
    checkdigit5: [0xFD, 0x39, 0x03, 0xCB, 0x26],
};

pub fn to_charcode_indices(password: &str) -> Vec<usize> {
    let mut result = Vec::new();
    for c in password.chars() {
        let c = CODE2CHAR.iter().position(|&x| x == c).unwrap();
        let i = CHAR_CODES.iter().position(|&x| x as usize == c).unwrap();
        result.push(i);
    }
    result
}

pub fn is_number(index: usize) -> bool {
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

pub fn is_symbol(index: usize) -> bool {
    match index {
        39 => true, // '-'
        33 => true, // '.'
        35 => true, // '!'
        _ => false, //
    }
}

pub fn is_vowel(index: usize) -> bool {
    match index {
        0 => true,
        7 => true,
        38 => true,
        24 => true,
        2 => true,
        _ => false, //
    }
}

pub fn is_alpha(index: usize) -> bool {
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

pub fn to_string(password: &Vec<usize>) -> String {
    password
        .iter()
        .map(|&p| CODE2CHAR[CHAR_CODES[p] as usize])
        .collect::<String>()
}
