#![allow(dead_code)]

//! 探索対象の定義

use crate::cpu::Memory;

#[derive(Clone)]
pub struct Target {
    pub memory: Memory,
    len: u8,
}

impl Target {
    pub fn len(&self) -> usize {
        self.len as usize
    }
}

// 未知のパスワード1(11桁)を解析する
// yokai03.exe 64 98 0B 15 91 18 B1 15
// 64: a[10]の逆順
// 98: a[9]の逆順
// 0B: 長さ
// 15: sum(a) + cnt(f4 >= 0xE5)
// 91: sum(f5)
// 18: xor が 0x18
// B1: a[10] + ror(a[9] + ror(a[8] + ...))
// 15: 0x15 bit

// KID
pub const TARGET_KID: Target = Target {
    memory: Memory {
        f4: 0x00,
        f5: 0x51,
        f7: 0x3A,
        f8: 0xE9,
        f9: 0x08,
        fa: 0x23,
        fb: 0x07,
    },

    len: 0x03,
};

// 8文字 818-6104
pub const TARGET_TEL: Target = Target {
    memory: Memory {
        f4: 0xDC,
        f5: 0xD9,
        f7: 0xA3,
        f8: 0xE3,
        f9: 0x17,
        fa: 0x28,
        fb: 0x15,
    },

    len: 0x08,
};

// 11文字 HENTAIOSUGI
// yokai03.exe 64 98 0B 15 91 18 B1 15
pub const TARGET_HENTAIOSUGI: Target = Target {
    memory: Memory {
        f4: 0x64,
        f5: 0x98,
        f7: 0x15,
        f8: 0x91,
        f9: 0x18,
        fa: 0xB1,
        fb: 0x15,
    },

    len: 0x08,
};

// 14文字
// yokai03.exe 65 94 0E AC E9 07 33 25
pub const TARGET_MUTEKI: Target = Target {
    memory: Memory {
        f4: 0x65,
        f5: 0x94,
        f7: 0xAC,
        f8: 0xE9,
        f9: 0x07,
        fa: 0x33,
        fb: 0x25,
    },

    len: 0x0E,
};

// 14文字(monitorのところにあるハッシュ値)
pub const TARGET_MONITOR: Target = Target {
    memory: Memory {
        f4: 0x51,
        f5: 0x62,
        f7: 0xFD,
        f8: 0x39,
        f9: 0x03,
        fa: 0xCB,
        fb: 0x26,
    },

    len: 0x0E,
};
