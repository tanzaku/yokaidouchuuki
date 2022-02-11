use crate::{
    config::BACKWARD_LEN,
    domain::{CHAR2CODE, PASSWORD_CHARS},
};
use std::sync::{Arc, Mutex};

use once_cell::sync::Lazy;
use rayon::{
    iter::{IntoParallelIterator, ParallelIterator},
    slice::ParallelSliceMut,
};

use crate::cpu::{bit_reverse, forward_step, Memory};

static PREV_F4: Lazy<[u8; 0x100]> = Lazy::new(|| {
    let mut reverse = [0; 0x100];
    for j in 0..=0xFF {
        let mut memory = Memory {
            f5: j,
            ..Memory::default()
        };
        forward_step(&mut memory, 0);
        reverse[j as usize] = memory.f5;
        // dbg!(j, memory.f5);
    }
    reverse
});

static PREV_F5: Lazy<[u8; 0x100]> = Lazy::new(|| {
    let mut reverse = [0; 0x100];
    for j in 0..=0xFF {
        let mut memory = Memory {
            f5: j,
            ..Memory::default()
        };
        forward_step(&mut memory, 0);
        reverse[bit_reverse(memory.f4) as usize] = j;
    }
    reverse
});

fn calc_prev_f4(f5: u8, prev_f5: u8) -> u8 {
    PREV_F4[prev_f5 as usize] ^ f5
}

fn calc_prev_f5(f4: u8, c: u8) -> u8 {
    PREV_F5[(c ^ bit_reverse(f4)) as usize]
}

fn calc_prev_f7(f7: u8, f4: u8, c: u8) -> (u8, u8) {
    let prev_f7 = f7.wrapping_sub(c);
    if f4 >= 0xE5 {
        let prev_f7 = prev_f7.wrapping_sub(1);
        (prev_f7, if prev_f7 >= f7 { 1 } else { 0 })
    } else {
        (prev_f7, if prev_f7 > f7 { 1 } else { 0 })
    }
}

fn calc_prev_f8(f8: u8, f5: u8, carry_f7: u8) -> (u8, u8) {
    let prev_f8 = f8.wrapping_sub(f5);
    if carry_f7 == 1 {
        let prev_f8 = prev_f8.wrapping_sub(1);
        (prev_f8, if prev_f8 >= f8 { 1 } else { 0 })
    } else {
        (prev_f8, if prev_f8 > f8 { 1 } else { 0 })
    }
}

fn calc_prev_f9(f9: u8, c: u8) -> u8 {
    f9 ^ c
}

fn calc_prev_fa(fa: u8, c: u8, carry_f8: u8) -> Vec<(u8, u8)> {
    // fa = ror(prev_fa) + c
    let mut candidates = Vec::with_capacity(2);

    let ror_prev_fa = fa.wrapping_sub(c);
    if (ror_prev_fa >> 7) == carry_f8 {
        candidates.push((ror_prev_fa << 1, if ror_prev_fa > fa { 1 } else { 0 }))
    }

    let ror_prev_fa = fa.wrapping_sub(c).wrapping_sub(1);
    if (ror_prev_fa >> 7) == carry_f8 {
        candidates.push((ror_prev_fa << 1 | 1, if ror_prev_fa >= fa { 1 } else { 0 }))
    }

    candidates
}

fn calc_prev_fb(fb: u8, c: u8, carry_fa: u8) -> u8 {
    fb.wrapping_sub(carry_fa).wrapping_sub(c.count_ones() as u8)
}

#[derive(Default, Clone, PartialEq, Eq)]
pub struct Node {
    pub memory: Memory,
    pub password: [u8; BACKWARD_LEN],
}

pub fn calc_prev_memories(memory: &Memory, c: char) -> Vec<Memory> {
    let c = CHAR2CODE[c as usize];
    let cur_f4 = memory.f4;
    let cur_f5 = memory.f5;
    let cur_f7 = memory.f7;
    let cur_f8 = memory.f8;
    let cur_f9 = memory.f9;
    let cur_fa = memory.fa;
    let cur_fb = memory.fb;

    let prev_f5 = calc_prev_f5(cur_f4, c);
    let prev_f4 = calc_prev_f4(cur_f5, prev_f5);

    let (prev_f7, carry_f7) = calc_prev_f7(cur_f7, cur_f4, c);
    let (prev_f8, carry_f8) = calc_prev_f8(cur_f8, cur_f5, carry_f7);
    let prev_f9 = calc_prev_f9(cur_f9, c);

    let prev_fa = calc_prev_fa(cur_fa, c, carry_f8);

    prev_fa
        .into_iter()
        .map(|(prev_fa, carry_fa)| {
            let prev_fb = calc_prev_fb(cur_fb, c, carry_fa);
            Memory {
                f4: prev_f4,
                f5: prev_f5,
                f7: prev_f7,
                f8: prev_f8,
                f9: prev_f9,
                fa: prev_fa,
                fb: prev_fb,
            }
        })
        .collect()
}

impl Node {
    fn calc_prev_nodes(&self, index: usize, c: char) -> Vec<Self> {
        calc_prev_memories(&self.memory, c)
            .into_iter()
            .map(|memory| {
                let mut password = self.password;
                password[index] = CHAR2CODE[c as usize];
                Self { memory, password }
            })
            .collect()
    }

    fn key(&self) -> u64 {
        self.memory.f4 as u64
            | (self.memory.f5 as u64) << 8
            | (self.memory.f7 as u64) << 16
            | (self.memory.f8 as u64) << 24
            | (self.memory.f9 as u64) << 32
            | (self.memory.fa as u64) << 40
            | (self.memory.fb as u64) << 48
    }
}

impl From<&Memory> for Node {
    fn from(memory: &Memory) -> Self {
        Self {
            memory: memory.clone(),
            password: [0; BACKWARD_LEN],
        }
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.key().partial_cmp(&other.key())
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub struct Backward {
    pub nodes: Vec<Node>,
}

impl Backward {
    pub fn new(memory: &Memory) -> Self {
        let mut queue = vec![Node::from(memory)];
        for i in (0..BACKWARD_LEN).rev() {
            let next_queue = Arc::new(Mutex::new(Vec::with_capacity(
                queue.len() * PASSWORD_CHARS.len(),
            )));

            PASSWORD_CHARS.into_par_iter().for_each(|c| {
                queue.chunks(0x1000000).for_each(|chunk| {
                    let new_nodes: Vec<_> = chunk
                        .iter()
                        .flat_map(|node| node.calc_prev_nodes(i, c))
                        .collect();
                    next_queue.lock().unwrap().extend(new_nodes);
                });
            });

            let lock = Arc::try_unwrap(next_queue).unwrap_or_default();
            queue = lock.into_inner().unwrap();
            eprintln!("Backward {}: {}", i, queue.len());
        }

        // par_sort_unstableでスタックを食いつぶして死ぬことがある。
        // スタックサイズを増やせば回避できそうな気がするが、面倒&確率的に発生する事象なのでやり直して回避する
        queue.par_sort_unstable();
        Self { nodes: queue }
    }

    pub fn len(&self) -> usize {
        self.nodes[0].password.len()
    }

    pub fn for_each_password<F>(&self, memory: &Memory, mut f: F)
    where
        F: FnMut(&[u8; BACKWARD_LEN]),
    {
        let key = Node::from(memory).key();

        if let Ok(i) = self.nodes.binary_search_by_key(&key, |node| node.key()) {
            f(&self.nodes[i].password);
            for i in (0..i).rev().take_while(|&i| self.nodes[i].key() == key) {
                f(&self.nodes[i].password);
            }
            for i in (i + 1..self.nodes.len()).take_while(|&i| self.nodes[i].key() == key) {
                f(&self.nodes[i].password);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{backward::*, cpu::*};

    #[test]
    fn test() {
        use crate::cpu::forward_step;

        let mut reverse = [0; 0x100];
        for k in 0..=0xFF {
            let mut cnt = [0; 0x100];
            for j in 0..=0xFF {
                let mut memory = Memory {
                    f5: j,
                    ..Memory::default()
                };
                forward_step(&mut memory, k);
                cnt[memory.f4 as usize] += 1;
                if k == 0 {
                    reverse[(k ^ bit_reverse(memory.f4)) as usize] = j;
                } else {
                    assert_eq!(reverse[(k ^ bit_reverse(memory.f4)) as usize], j);
                }
            }
            for i in 0..=0xFF {
                assert_eq!(cnt[i], 1);
            }
        }
    }

    #[test]
    fn test_calc_prev_f4_f5() {
        for f4 in 0..=0xFF {
            for f5 in 0..=0xFF {
                for c in 0..=0xFF {
                    let mut memory = Memory {
                        f4,
                        f5,
                        ..Memory::default()
                    };

                    forward_step(&mut memory, c);

                    let cur_f4 = memory.f4;
                    let cur_f5 = memory.f5;

                    let prev_f5 = calc_prev_f5(cur_f4, c);
                    let prev_f4 = calc_prev_f4(cur_f5, prev_f5);

                    assert_eq!(prev_f5, f5);
                    assert_eq!(prev_f4, f4);
                }
            }
        }
    }

    #[test]
    fn test_calc_prev_f7() {
        for f5 in 0..=0xFF {
            for f7 in 0..=0xFF {
                for c in 0..=0xFF {
                    let mut memory = Memory {
                        f5,
                        f7,
                        ..Memory::default()
                    };

                    forward_step(&mut memory, c);

                    let cur_f4 = memory.f4;
                    let cur_f7 = memory.f7;

                    let prev_f7 = calc_prev_f7(cur_f7, cur_f4, c).0;

                    assert_eq!(prev_f7, f7);
                }
            }
        }
    }

    // cargo test --package decrypt --bin decrypt --release -- backward::test_calc_prev_mem_random --exact --nocapture
    #[test]
    fn test_calc_prev_mem_random() {
        use rand::Rng;
        // let mut rng = rand::thread_rng();
        let seed: [u8; 32] = [13; 32];
        let mut rng: rand::rngs::StdRng = rand::SeedableRng::from_seed(seed);

        for _ in 0..1000000000 {
            let mut memory = Memory {
                f4: rng.gen(),
                f5: rng.gen(),
                f7: rng.gen(),
                f8: rng.gen(),
                f9: rng.gen(),
                fa: rng.gen(),
                fb: rng.gen(),
            };

            let prev = memory.clone();

            let c = rng.gen();
            forward_step(&mut memory, c);

            let cur_f4 = memory.f4;
            let cur_f5 = memory.f5;
            let cur_f7 = memory.f7;
            let cur_f8 = memory.f8;
            let cur_f9 = memory.f9;
            let cur_fa = memory.fa;
            let cur_fb = memory.fb;

            let prev_f5 = calc_prev_f5(cur_f4, c);
            let prev_f4 = calc_prev_f4(cur_f5, prev_f5);

            let (prev_f7, carry_f7) = calc_prev_f7(cur_f7, cur_f4, c);
            let (prev_f8, carry_f8) = calc_prev_f8(cur_f8, cur_f5, carry_f7);
            let prev_f9 = calc_prev_f9(cur_f9, c);

            let prev_fa = calc_prev_fa(cur_fa, c, carry_f8);

            let prev_fb: Vec<_> = prev_fa
                .iter()
                .map(|&(_, carry_fa)| calc_prev_fb(cur_fb, c, carry_fa))
                .collect();

            // dbg!(&prev);
            // dbg!(&memory);
            // dbg!(prev_f8, carry_f8);
            assert_eq!(prev_f4, prev.f4);
            assert_eq!(prev_f5, prev.f5);
            assert_eq!(prev_f7, prev.f7);
            assert_eq!(prev_f8, prev.f8);
            assert_eq!(prev_f9, prev.f9);
            assert!(prev_fa.iter().any(|&(prev_fa, _)| prev_fa == prev.fa));
            // dbg!(&prev_fa);
            // dbg!(&prev_fb);
            assert!(prev_fb.iter().any(|&prev_fb| prev_fb == prev.fb));
        }
    }
}
