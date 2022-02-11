use itertools::iproduct;
use rayon::iter::{IntoParallelIterator, ParallelBridge, ParallelIterator};
use spin::RwLock;

use crate::{
    backward::Backward,
    bitset::BitSet256,
    cpu::{forward_char, Memory},
    domain::PASSWORD_CHARS,
    target::Target,
    time::get_current_time,
};

pub struct Forward2 {
    dp: Vec<BitSet256>,
}

impl Forward2 {
    pub fn build(target: &Target, backward: &Backward) -> Self {
        eprintln!("calc DP2 ({})", get_current_time());

        Self {
            dp: build(target, backward),
        }
    }

    pub fn is_valid(&self, len: usize, memory: &Memory) -> bool {
        let f4 = memory.f4 as usize;
        let f5 = memory.f5 as usize;
        let f7 = memory.f7 as usize;
        let f8 = memory.f8 as usize;
        let i = calc_index(len, f4, f5, f7);
        self.dp[i].get(f8)
    }
}

#[inline]
fn calc_index(len: usize, f4: usize, f5: usize, f7: usize) -> usize {
    ((len * 0x100 + f4) * 0x100 + f5) * 0x100 + f7
}

fn build(target: &Target, backward: &Backward) -> Vec<BitSet256> {
    let max_len = target.len() - backward.len();

    let m = 0x100 * 0x100 * 0x100 * (max_len + 1);
    let can_visit_backward = (0..m)
        .into_par_iter()
        .map(|_| RwLock::new(BitSet256::default()))
        .collect::<Vec<_>>();

    {
        let can_visit_forward = (0..m)
            .into_par_iter()
            .map(|_| RwLock::new(BitSet256::default()))
            .collect::<Vec<_>>();

        backward.nodes.iter().for_each(|node| {
            let f4 = node.memory.f4 as usize;
            let f5 = node.memory.f5 as usize;
            let f7 = node.memory.f7 as usize;
            let f8 = node.memory.f8 as usize;
            can_visit_backward[calc_index(max_len, f4, f5, f7)]
                .write()
                .set(f8);
        });

        {
            let memory = Memory::new();
            let f4 = memory.f4 as usize;
            let f5 = memory.f5 as usize;
            let f7 = memory.f7 as usize;
            let f8 = memory.f8 as usize;
            can_visit_forward[calc_index(0, f4, f5, f7)].write().set(f8);
        }

        for len in 0..max_len {
            eprint!(".");
            iproduct!(0..0x100, 0..0x100, 0..0x100)
                .par_bridge()
                .map(|(f4, f5, f7)| (f4, f5, f7, calc_index(len, f4, f5, f7)))
                .filter(|&(_, _, _, i)| !can_visit_forward[i].read().is_zero())
                .for_each(|(f4, f5, f7, i)| {
                    for c in PASSWORD_CHARS {
                        let mut memory = Memory {
                            f4: f4 as u8,
                            f5: f5 as u8,
                            f7: f7 as u8,
                            f8: 0,
                            f9: 0,
                            fa: 0,
                            fb: 0,
                        };

                        forward_char(&mut memory, c);

                        let next_len = len + 1;
                        let next_f4 = memory.f4 as usize;
                        let next_f5 = memory.f5 as usize;
                        let next_f7 = memory.f7 as usize;
                        let offset = memory.f8 as usize;

                        let j = calc_index(next_len, next_f4, next_f5, next_f7);
                        *can_visit_forward[j].write() |=
                            can_visit_forward[i].read().rot_left(offset);
                    }
                });
        }

        eprintln!();

        for len in (0..max_len).rev() {
            eprint!(".");
            iproduct!(0..0x100, 0..0x100, 0..0x100)
                .par_bridge()
                .map(|(f4, f5, f7)| (f4, f5, f7, calc_index(len, f4, f5, f7)))
                .filter(|&(_, _, _, i)| !can_visit_forward[i].read().is_zero())
                .for_each(|(f4, f5, f7, i)| {
                    for c in PASSWORD_CHARS {
                        let mut memory = Memory {
                            f4: f4 as u8,
                            f5: f5 as u8,
                            f7: f7 as u8,
                            f8: 0,
                            f9: 0,
                            fa: 0,
                            fb: 0,
                        };

                        forward_char(&mut memory, c);

                        let next_len = len + 1;
                        let next_f4 = memory.f4 as usize;
                        let next_f5 = memory.f5 as usize;
                        let next_f7 = memory.f7 as usize;
                        let offset = memory.f8 as usize;

                        let j = calc_index(next_len, next_f4, next_f5, next_f7);
                        let rotated = can_visit_backward[j].read().rot_right(offset);
                        *can_visit_backward[i].write() |= &rotated & &can_visit_forward[i].read();
                    }
                });
        }
        eprintln!();

        // let mut f = BufWriter::new(File::create("f2.bin").unwrap());
        // can_visit_forward.iter().for_each(|b| {
        //     let s = bincode::serialize(&*b.read()).unwrap();
        //     f.write_all(&s).unwrap();
        // });

        // can_visit_backward.iter().for_each(|b| {
        //     let s = bincode::serialize(&*b.read()).unwrap();
        //     f.write_all(&s).unwrap();
        // });
    }

    can_visit_backward
        .into_par_iter()
        .map(RwLock::into_inner)
        .collect()
}
