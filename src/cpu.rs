use once_cell::sync::Lazy;
use packed_simd_2::{u8x32, u8x64, FromBits};

use crate::domain::CHAR2CODE;

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
    pub f4: u8,
    pub f5: u8,
    pub f7: u8, // sum(a) + count(f4 >= 0xE5)
    pub f8: u8, // sum(f5) + sum(carry(f7))
    pub f9: u8, // xor(a)
    pub fa: u8, // a[D] + ror(a[C] + ror(a[B] + ...))), rorの際に先頭ビットに carry(f8) が来る。prev_faの最下位ビットが足される。
    pub fb: u8, // fb + carry(fa) + popcnt(a)
}

impl Memory {
    pub fn new() -> Self {
        Self {
            f4: 0,
            f5: 0,
            f7: 0,
            f8: 0,
            f9: 0,
            fa: 1,
            fb: 0,
        }
    }

    pub fn bit(&self) -> u8 {
        self.fb
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Memory {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{0:02X} {1:02X} {2:02X} {3:02X} {4:02X} {5:02X} {6:02X}",
            self.f4, self.f5, self.f7, self.f8, self.f9, self.fa, self.fb
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
    for (i, cache) in cache.iter_mut().enumerate() {
        let mut v = i;
        for j in 0..8 {
            cache[1] >>= 1;
            cache[1] |= cache[0] << 7;
            cache[0] >>= 1;
            if (v & 1) == 1 {
                cache[0] ^= 0x84;
                cache[1] ^= 0x08;
                v ^= 0x10;
            }
            if j == 7 {
                cache[2] = (v & 1) as u8;
            }
            v >>= 1;
        }
    }
    cache
});

pub fn bit_reverse(v: u8) -> u8 {
    let v = (v & 0x55) << 1 | (v >> 1 & 0x55);
    let v = (v & 0x33) << 2 | (v >> 2 & 0x33);
    (v & 0x0f) << 4 | (v >> 4 & 0x0f)
}

fn calc_f4_f5(cpu: &mut Cpu, memory: &mut Memory) {
    let v = bit_reverse(cpu.reg.a);
    let i = memory.f5 as usize;
    cpu.set_carry(CALC_CHECKDIGITS1_CACHE[i][2]);
    memory.f5 = memory.f4 ^ CALC_CHECKDIGITS1_CACHE[i][1];
    memory.f4 = v ^ CALC_CHECKDIGITS1_CACHE[i][0];
}

fn calc_f7_f8(cpu: &mut Cpu, memory: &mut Memory) {
    cpu.set_carry(if memory.f4 >= 0xE5 { 1 } else { 0 });

    // だいたい入力の和。memory.f4 >= 0xE5 の分だけずれる
    memory.f7 = cpu.adc(cpu.reg.a, memory.f7);
    memory.f8 = cpu.adc(memory.f8, memory.f5);
}

fn calc_f9(cpu: &mut Cpu, memory: &mut Memory) {
    memory.f9 ^= cpu.reg.a;
}

fn calc_fa(cpu: &mut Cpu, memory: &mut Memory) {
    let v = cpu.ror(memory.fa);
    memory.fa = cpu.adc(v, cpu.reg.a);
}

fn calc_fb(cpu: &mut Cpu, memory: &mut Memory) {
    // https://www.pagetable.com/c64ref/6502/?tab=2
    // PLA（pop）でもZフラグが変わることに注意
    memory.fb += cpu.get_carry() + (cpu.reg.a.count_ones() as u8);
}

pub fn forward_char(memory: &mut Memory, c: char) {
    forward_step(memory, CHAR2CODE[c as usize])
}

pub fn forward_step(memory: &mut Memory, a: u8) {
    let mut cpu = Cpu {
        reg: Register { a, c: 0 },
    };

    calc_f4_f5(&mut cpu, memory);
    calc_f7_f8(&mut cpu, memory);
    calc_f9(&mut cpu, memory);
    calc_fa(&mut cpu, memory);
    calc_fb(&mut cpu, memory);
}

#[allow(dead_code)]
pub fn forward_step_simd_u8x64(
    memory: &Memory,
    a: u8x64,
) -> (u8x64, u8x64, u8x64, u8x64, u8x64, u8x64, u8x64) {
    let mut f4 = u8x64::splat(memory.f4);
    let mut f5 = u8x64::splat(memory.f5);
    let mut f7 = u8x64::splat(memory.f7);
    let mut f8 = u8x64::splat(memory.f8);
    let mut f9 = u8x64::splat(memory.f9);
    let mut fa = u8x64::splat(memory.fa);
    let mut fb = u8x64::splat(memory.fb);

    {
        let mut a = a;

        // コンパイラがループアンローリングしてくれることを祈るループ
        for _ in 0..8 {
            let c = a & 0x80;
            a <<= 1;
            let c1 = f4 << 7;
            f4 = (f4 >> 1) | c;
            let c = f5 & 0x01;
            f5 = (f5 >> 1) | c1;

            f4 ^= c << 7 | c << 2;
            f5 ^= c << 3;
        }
    }

    #[inline]
    fn add(a: u8x64, b: u8x64, c: u8x64) -> (u8x64, u8x64) {
        let sum = a + b + c;
        let c = (a & b) | ((a | b) & !sum);
        (sum, c & 0x80)
    }

    {
        let c_0xe5 = u8x64::splat(0xE5);
        let c = u8x64::from_bits(f4.ge(c_0xe5)) >> 7;

        // dbg!(a, f7, f4, c);
        // dbg!(a, f7, c);

        let v = a + c; // ここでオーバーフローは発生し得ない
        f7 += v;
        let mut c = u8x64::from_bits(f7.lt(v)) >> 7;

        // dbg!(f8, f5, c, f7, v);

        (f8, c) = add(f8, f5, c);
        f9 ^= a;

        let c1 = fa & 0x01;
        fa = (fa >> 1) | c;
        (fa, c) = add(fa, a, c1);
        // dbg!(fb, a.count_ones(), c);
        fb += a.count_ones() + (c >> 7);
    }

    (f4, f5, f7, f8, f9, fa, fb)
}

#[allow(dead_code)]
pub fn forward_step_simd_u8x32_(
    memory: &Memory,
    a: u8x32,
) -> (u8x32, u8x32, u8x32, u8x32, u8x32, u8x32, u8x32) {
    let mut f4 = u8x32::splat(memory.f4);
    let mut f5 = u8x32::splat(memory.f5);
    let mut f7 = u8x32::splat(memory.f7);
    let mut f8 = u8x32::splat(memory.f8);
    let mut f9 = u8x32::splat(memory.f9);
    let mut fa = u8x32::splat(memory.fa);
    let mut fb = u8x32::splat(memory.fb);

    {
        let mut a = a;

        // コンパイラがループアンローリングしてくれることを祈るループ
        for _ in 0..8 {
            let c = a & 0x80;
            a <<= 1;
            let c1 = f4 << 7;
            f4 = (f4 >> 1) | c;
            let c = f5 & 0x01;
            f5 = (f5 >> 1) | c1;

            f4 ^= c << 7 | c << 2;
            f5 ^= c << 3;
        }
    }

    #[inline]
    fn add(a: u8x32, b: u8x32, c: u8x32) -> (u8x32, u8x32) {
        let sum = a + b + c;
        let c = (a & b) | ((a | b) & !sum);
        (sum, c & 0x80)
    }

    {
        let c_0xe5 = u8x32::splat(0xE5);
        let c = u8x32::from_bits(f4.ge(c_0xe5)) >> 7;

        // dbg!(a, f7, f4, c);
        // dbg!(a, f7, c);

        let v = a + c; // ここでオーバーフローは発生し得ない
        f7 += v;
        let mut c = u8x32::from_bits(f7.lt(v)) >> 7;

        // dbg!(f8, f5, c, f7, v);

        (f8, c) = add(f8, f5, c);
        f9 ^= a;

        let c1 = fa & 0x01;
        fa = (fa >> 1) | c;
        (fa, c) = add(fa, a, c1);
        // dbg!(fb, a.count_ones(), c);
        fb += a.count_ones() + (c >> 7);
    }

    (f4, f5, f7, f8, f9, fa, fb)
}

// pub fn forward_word(memory: &mut Memory, word: &[usize]) {
//     word.iter()
//         .map(|&c| CHAR_CODES[c])
//         .for_each(|a| forward_step(memory, a));
// }

// #[allow(dead_code)]
// pub fn satisfy(password: &[usize], expected_memory: &Memory) -> bool {
//     let mut memory = Memory::new();

//     for i in 0..password.len() {
//         let a = CHAR_CODES[password[i]];
//         forward_step(&mut memory, a);
//     }

//     expected_memory == &memory
// }

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
fn test_calc_f4_f5() {
    fn calc_f4_f5_naive(cpu: &mut Cpu, memory: &mut Memory) {
        for i in (0..8).rev() {
            cpu.set_carry(cpu.reg.a >> i & 1);

            memory.f4 = cpu.ror(memory.f4);
            memory.f5 = cpu.ror(memory.f5);
            if cpu.reg.c == 1 {
                memory.f4 ^= 0x84;
                memory.f5 ^= 0x08;
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
                    f4: j as u8,
                    f5: k as u8,
                    f7: 0x00,
                    f8: 0x00,
                    f9: 0x00,
                    fa: 0x00,
                    fb: 0x00,
                };

                let mut cpu1 = cpu.clone();
                let mut memory1 = memory.clone();
                let mut cpu2 = cpu.clone();
                let mut memory2 = memory.clone();
                calc_f4_f5(&mut cpu1, &mut memory1);
                calc_f4_f5_naive(&mut cpu2, &mut memory2);
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
        let a: u8x64 = u8x64::new(
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D,
            0x0E, 0x0F, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B,
            0x1C, 0x1D, 0x1E, 0x1F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        );

        let (f4, f5, f7, f8, f9, fa, fb) = forward_step_simd_u8x64(&memory, a);

        for i in 0..0x20 {
            if memory.f5 != 1 {
                continue;
            }
            let mut memory = memory.clone();
            forward_step(&mut memory, i as u8);
            // dbg!(f4, f5, f7, f8, f9, fa, fb, &memory, &memory0, i);
            assert_eq!(f4.extract(i), memory.f4);
            assert_eq!(f5.extract(i), memory.f5);
            assert_eq!(f7.extract(i), memory.f7);
            assert_eq!(f8.extract(i), memory.f8);
            assert_eq!(f9.extract(i), memory.f9);
            assert_eq!(fa.extract(i), memory.fa);
            assert_eq!(fb.extract(i), memory.fb);
        }
    }

    fn validate2(memory: &Memory) {
        let a: u8x64 = u8x64::new(
            0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D,
            0x2E, 0x2F, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x3B,
            0x3C, 0x3D, 0x3E, 0x3F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        );

        let (f4, f5, f7, f8, f9, fa, fb) = forward_step_simd_u8x64(&memory, a);

        for i in 0x20..0x40 {
            let mut memory = memory.clone();
            forward_step(&mut memory, i as u8);
            assert_eq!(f4.extract(i - 0x20), memory.f4);
            assert_eq!(f5.extract(i - 0x20), memory.f5);
            assert_eq!(f7.extract(i - 0x20), memory.f7);
            assert_eq!(f8.extract(i - 0x20), memory.f8);
            assert_eq!(f9.extract(i - 0x20), memory.f9);
            assert_eq!(fa.extract(i - 0x20), memory.fa);
            assert_eq!(fb.extract(i - 0x20), memory.fb);
        }
    }

    for j in 0..0x100 {
        for k in 0..0x100 {
            let memory = Memory {
                f4: j as u8,
                f5: k as u8,
                f7: 0xFF,
                f8: 0xFF,
                f9: 0xFF,
                fa: 0xFF,
                fb: 0x40,
            };
            validate1(&memory);
            validate2(&memory);

            let memory = Memory {
                f4: j as u8,
                f5: k as u8,
                f7: 0x00,
                f8: 0x00,
                f9: 0x00,
                fa: 0x00,
                fb: 0x00,
            };
            validate1(&memory);
            validate2(&memory);
        }
    }
}
