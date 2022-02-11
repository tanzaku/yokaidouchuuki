#![allow(dead_code)]

use once_cell::sync::Lazy;

pub const PASSWORD_CHARS: [char; 42] = [
    'A', 'H', 'O', 'V', '1', '6', 'B', 'I', 'P', 'W', '2', '7', 'C', 'J', 'Q', 'X', '3', '8', 'D',
    'K', 'R', 'Y', '4', '9', 'E', 'L', 'S', 'Z', '5', '0', 'F', 'M', 'T', '-', 'n', '!', 'G', 'N',
    'U', '.', 'm', 'c',
];

pub const PASSWORD_CHAR_CODES: [u8; 42] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x10, 0x11, 0x12, 0x13,
    0x14, 0x15, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x28, 0x29,
    0x2A, 0x2B, 0x2C, 0x2D, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35,
];

pub static CHAR2CODE: Lazy<[u8; 0x100]> = Lazy::new(|| {
    let mut table = [!0; 0x100];
    PASSWORD_CHARS
        .iter()
        .zip(PASSWORD_CHAR_CODES)
        .for_each(|(ch, code)| table[*ch as usize] = code);
    table
});

pub static CODE2CHAR: Lazy<[char; 0x100]> = Lazy::new(|| {
    let mut table = ['?'; 0x100];
    PASSWORD_CHARS
        .iter()
        .zip(PASSWORD_CHAR_CODES)
        .for_each(|(ch, code)| table[code as usize] = *ch);
    table
});

pub fn to_string(password: &[u8]) -> String {
    password
        .iter()
        .map(|code| CODE2CHAR[*code as usize])
        .collect::<String>()
}
