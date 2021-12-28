use once_cell::sync::Lazy;
use packed_simd_2::{u8x32, FromBits};

use crate::domain::CHAR_CODES;

#[derive(Debug, PartialEq, Eq, Clone)]
struct Register {
    a: u8,
    c: u8,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct Cpu {
    reg: Register,
}

#[derive(PartialEq, Eq, Clone, Hash)]
pub struct Memory {
    pub checkdigit2: [u8; 2], // $31F4 ~ $31F5
    pub password_len: u8,     // $31F6
    pub checkdigit5: [u8; 5], // $31F7 ~ $31FB
}

impl Memory {
    pub fn new(len: u8) -> Self {
        Self {
            checkdigit2: [0; 2],
            password_len: len,
            checkdigit5: [0, 0, 0, 1, 0],
        }
    }

    pub fn sum(&self) -> usize {
        self.checkdigit5[0] as usize
    }

    pub fn bit(&self) -> usize {
        self.checkdigit5[4] as usize
    }

    pub fn xor(&self) -> usize {
        self.checkdigit5[2] as usize
    }

    pub fn len(&self) -> usize {
        self.password_len as usize
    }
}

impl std::fmt::Debug for Memory {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{0:02X} {1:02X} {2:02X} {3:02X} {4:02X} {5:02X} {6:02X}",
            self.checkdigit2[0],
            self.checkdigit2[1],
            self.checkdigit5[0],
            self.checkdigit5[1],
            self.checkdigit5[2],
            self.checkdigit5[3],
            self.checkdigit5[4]
        )
    }
}

impl Cpu {
    fn set_carry(&mut self, c: u8) {
        self.reg.c = c;
    }

    // rorはキャリーフラグ込みの9bitローテーション
    // https://taotao54321.hatenablog.com/entry/2017/04/09/151355
    fn ror(&mut self, v: u8) -> u8 {
        let c = self.reg.c;
        self.reg.c = v & 0x01;
        v >> 1 | c << 7
    }

    fn adc(&mut self, lhs: u8, rhs: u8) -> u8 {
        let lhs = lhs as u16;
        let rhs = rhs as u16;
        let c = self.reg.c as u16;
        let v = lhs + rhs + c;
        self.reg.c = if v > 0xFF { 1 } else { 0 };
        (v & 0xFF) as u8
    }

    fn get_carry(&self) -> u8 {
        self.reg.c
    }
}

static CALC_CHECKDIGITS1_CACHE: Lazy<[[u8; 3]; 0x100]> = Lazy::new(|| {
    let mut cache = [[0; 3]; 0x100];
    for i in 0..0x100 {
        let mut v = i;
        for j in 0..8 {
            cache[i][1] >>= 1;
            cache[i][1] |= cache[i][0] << 7;
            cache[i][0] >>= 1;
            if (v & 1) == 1 {
                cache[i][0] ^= 0x84;
                cache[i][1] ^= 0x08;
                v ^= 0x10;
            }
            if j == 7 {
                cache[i][2] = (v & 1) as u8;
            }
            v >>= 1;
        }
    }
    cache
});

fn bit_reverse(v: u8) -> u8 {
    let v = (v & 0x55) << 1 | (v >> 1 & 0x55);
    let v = (v & 0x33) << 2 | (v >> 2 & 0x33);
    let v = (v & 0x0f) << 4 | (v >> 4 & 0x0f);
    v
}

fn calc_checkdigit1(cpu: &mut Cpu, memory: &mut Memory) {
    let v = bit_reverse(cpu.reg.a);
    let i = memory.checkdigit2[1] as usize;
    cpu.set_carry(CALC_CHECKDIGITS1_CACHE[i][2]);
    memory.checkdigit2[1] = memory.checkdigit2[0] ^ CALC_CHECKDIGITS1_CACHE[i][1];
    memory.checkdigit2[0] = v ^ CALC_CHECKDIGITS1_CACHE[i][0];
}

fn calc_checkdigit2(cpu: &mut Cpu, memory: &mut Memory) {
    cpu.set_carry(if memory.checkdigit2[0] >= 0xE5 { 1 } else { 0 });

    // だいたい入力の和。memory.checkdigit2[0] >= 0xE5 の分だけずれる
    memory.checkdigit5[0] = cpu.adc(cpu.reg.a, memory.checkdigit5[0]);
    memory.checkdigit5[1] = cpu.adc(memory.checkdigit5[1], memory.checkdigit2[1]);
}

fn calc_checkdigit3(cpu: &mut Cpu, memory: &mut Memory) {
    memory.checkdigit5[2] ^= cpu.reg.a;
}

fn calc_checkdigit4(cpu: &mut Cpu, memory: &mut Memory) {
    let v = cpu.ror(memory.checkdigit5[3]);
    memory.checkdigit5[3] = cpu.adc(v, cpu.reg.a);
}

fn calc_checkdigit5(cpu: &mut Cpu, memory: &mut Memory) {
    // https://www.pagetable.com/c64ref/6502/?tab=2
    // PLA（pop）でもZフラグが変わることに注意
    memory.checkdigit5[4] += cpu.get_carry() + (cpu.reg.a.count_ones() as u8);
}

pub fn forward_step(memory: &mut Memory, a: u8) {
    let mut cpu = Cpu {
        reg: Register { a, c: 0 },
    };

    calc_checkdigit1(&mut cpu, memory);
    calc_checkdigit2(&mut cpu, memory);
    calc_checkdigit3(&mut cpu, memory);
    calc_checkdigit4(&mut cpu, memory);
    calc_checkdigit5(&mut cpu, memory);
}

#[allow(non_snake_case)]
pub fn forward_step_simd(
    memory: &Memory,
    a: u8x32,
) -> (u8x32, u8x32, u8x32, u8x32, u8x32, u8x32, u8x32) {
    let mut a31F4 = u8x32::splat(memory.checkdigit2[0]);
    let mut a31F5 = u8x32::splat(memory.checkdigit2[1]);
    let mut a31F7 = u8x32::splat(memory.checkdigit5[0]);
    let mut a31F8 = u8x32::splat(memory.checkdigit5[1]);
    let mut a31F9 = u8x32::splat(memory.checkdigit5[2]);
    let mut a31FA = u8x32::splat(memory.checkdigit5[3]);
    let mut a31FB = u8x32::splat(memory.checkdigit5[4]);

    {
        let mut a = a.clone();

        // コンパイラがループアンローリングしてくれることを祈るループ
        for _ in 0..8 {
            let c = a & 0x80;
            a <<= 1;
            let c1 = a31F4 << 7;
            a31F4 = (a31F4 >> 1) | c;
            let c = a31F5 & 0x01;
            a31F5 = (a31F5 >> 1) | c1;

            a31F4 ^= c << 7 | c << 2;
            a31F5 ^= c << 3;
        }
    }

    #[inline]
    fn add(a: u8x32, b: u8x32, c: u8x32) -> (u8x32, u8x32) {
        let sum = a + b + c;
        let c = (a & b) | ((a | b) & !sum);
        (sum, c & 0x80)
    }

    {
        let c0xE5 = u8x32::splat(0xE5);
        let c = u8x32::from_bits(a31F4.ge(c0xE5)) >> 7;

        // dbg!(a, a31F7, a31F4, c);
        // dbg!(a, a31F7, c);

        let v = a + c; // ここでオーバーフローは発生し得ない
        a31F7 += v;
        let mut c = u8x32::from_bits(a31F7.lt(v)) >> 7;

        // dbg!(a31F8, a31F5, c, a31F7, v);

        (a31F8, c) = add(a31F8, a31F5, c);
        a31F9 ^= a;

        let c1 = a31FA & 0x01;
        a31FA = (a31FA >> 1) | c;
        (a31FA, c) = add(a31FA, a, c1);
        // dbg!(a31FB, a.count_ones(), c);
        a31FB += a.count_ones() + (c >> 7);
    }

    (a31F4, a31F5, a31F7, a31F8, a31F9, a31FA, a31FB)
}

pub fn forward_word(memory: &mut Memory, word: &[usize]) {
    word.iter()
        .map(|&c| CHAR_CODES[c])
        .for_each(|a| forward_step(memory, a));
}

#[allow(dead_code)]
pub fn satisfy(password: &[usize], expected_memory: &Memory) -> bool {
    let mut memory = Memory::new(password.len() as u8);

    for i in 0..password.len() {
        let a = CHAR_CODES[password[i]];
        forward_step(&mut memory, a);
    }

    expected_memory == &memory
}

#[test]
fn test_bit_reverse() {
    assert_eq!(bit_reverse(0x01), 0x80);
    assert_eq!(bit_reverse(0x02), 0x40);
    assert_eq!(bit_reverse(0x04), 0x20);
    assert_eq!(bit_reverse(0x08), 0x10);
    assert_eq!(bit_reverse(0x10), 0x08);
    assert_eq!(bit_reverse(0x20), 0x04);
    assert_eq!(bit_reverse(0x40), 0x02);
    assert_eq!(bit_reverse(0x80), 0x01);
}

#[test]
fn test_calc_checkdigit1() {
    fn calc_checkdigit1_naive(cpu: &mut Cpu, memory: &mut Memory) {
        for i in (0..8).rev() {
            cpu.set_carry(cpu.reg.a >> i & 1);

            memory.checkdigit2[0] = cpu.ror(memory.checkdigit2[0]);
            memory.checkdigit2[1] = cpu.ror(memory.checkdigit2[1]);
            if cpu.reg.c == 1 {
                memory.checkdigit2[0] ^= 0x84;
                memory.checkdigit2[1] ^= 0x08;
            }
        }
    }

    for i in 0..0x100 {
        for j in 0..0x100 {
            for k in 0..0x100 {
                let cpu = Cpu {
                    reg: Register { a: i as u8, c: 0 },
                };
                let memory = Memory {
                    checkdigit2: [j as u8, k as u8],
                    password_len: 0,
                    checkdigit5: [0, 0, 0, 0, 0],
                };

                let mut cpu1 = cpu.clone();
                let mut memory1 = memory.clone();
                let mut cpu2 = cpu.clone();
                let mut memory2 = memory.clone();
                calc_checkdigit1(&mut cpu1, &mut memory1);
                calc_checkdigit1_naive(&mut cpu2, &mut memory2);
                assert_eq!(cpu1, cpu2);
                assert_eq!(memory1, memory2);
            }
        }
    }
}

#[allow(non_snake_case)]
#[test]
fn test_calc_simd() {
    fn validate1(memory: &Memory) {
        let a: u8x32 = u8x32::new(
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D,
            0x0E, 0x0F, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B,
            0x1C, 0x1D, 0x1E, 0x1F,
        );

        let (a31F4, a31F5, a31F7, a31F8, a31F9, a31FA, a31FB) = forward_step_simd(&memory, a);

        for i in 0..0x20 {
            if memory.checkdigit2[1] != 1 {
                continue;
            }
            let mut memory = memory.clone();
            forward_step(&mut memory, i as u8);
            // dbg!(a31F4, a31F5, a31F7, a31F8, a31F9, a31FA, a31FB, &memory, &memory0, i);
            assert_eq!(a31F4.extract(i), memory.checkdigit2[0]);
            assert_eq!(a31F5.extract(i), memory.checkdigit2[1]);
            assert_eq!(a31F7.extract(i), memory.checkdigit5[0]);
            assert_eq!(a31F8.extract(i), memory.checkdigit5[1]);
            assert_eq!(a31F9.extract(i), memory.checkdigit5[2]);
            assert_eq!(a31FA.extract(i), memory.checkdigit5[3]);
            assert_eq!(a31FB.extract(i), memory.checkdigit5[4]);
        }
    }

    fn validate2(memory: &Memory) {
        let a: u8x32 = u8x32::new(
            0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D,
            0x2E, 0x2F, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x3B,
            0x3C, 0x3D, 0x3E, 0x3F,
        );

        let (a31F4, a31F5, a31F7, a31F8, a31F9, a31FA, a31FB) = forward_step_simd(&memory, a);

        for i in 0x20..0x40 {
            let mut memory = memory.clone();
            forward_step(&mut memory, i as u8);
            assert_eq!(a31F4.extract(i - 0x20), memory.checkdigit2[0]);
            assert_eq!(a31F5.extract(i - 0x20), memory.checkdigit2[1]);
            assert_eq!(a31F7.extract(i - 0x20), memory.checkdigit5[0]);
            assert_eq!(a31F8.extract(i - 0x20), memory.checkdigit5[1]);
            assert_eq!(a31F9.extract(i - 0x20), memory.checkdigit5[2]);
            assert_eq!(a31FA.extract(i - 0x20), memory.checkdigit5[3]);
            assert_eq!(a31FB.extract(i - 0x20), memory.checkdigit5[4]);
        }
    }

    for j in 0..0x100 {
        for k in 0..0x100 {
            let memory = Memory {
                checkdigit2: [j as u8, k as u8],
                password_len: 0,
                checkdigit5: [0xFF, 0xFF, 0xFF, 0xFF, 0x40],
            };
            validate1(&memory);
            validate2(&memory);

            let memory = Memory {
                checkdigit2: [j as u8, k as u8],
                password_len: 0,
                checkdigit5: [0x00, 0x00, 0x00, 0x00, 0x00],
            };
            validate1(&memory);
            validate2(&memory);
        }
    }
}
