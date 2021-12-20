// use crate::domain::CHAR_CODES;

// pub fn bruteforce() {
//     let mut pattern = vec![vec![vec![vec![false; 0x100]; 0x100]; bit + 1]; len + 1];

//     pattern[0][0][0][0] = true;

//     for len in 0..pattern.len() - 1 {
//         for bit in 0..pattern[len].len() {
//             for sum in 0..0x100 {
//                 for xor in 0..0x100 {
//                     if pattern[len][bit][sum][xor] {
//                         for &c in &CHAR_CODES {
//                             let c = c as usize;
//                             if bit + c.count_ones() as usize >= pattern[len].len() {
//                                 continue;
//                             }
//                             pattern[len + 1][bit + c.count_ones() as usize][(sum + c) & 0xFF]
//                                 [xor ^ c] = true;
//                         }
//                     }
//                 }
//             }
//         }
//     }

//     fn dfs(
//         dict: &Dict,
//         table: &[char],
//         pattern: &Vec<Vec<Vec<Vec<bool>>>>,
//         len: usize,
//         bit: usize,
//         sum: usize,
//         xor: usize,
//         last_val: usize,
//         cnts: &mut Vec<isize>,
//         candidates: &mut Vec<Vec<usize>>,
//     ) {
//         if !pattern[len][bit][sum][xor] {
//             return;
//         }

//         if len == 0 {
//             // dbg!(bit, sum, xor);
//             // println!("start {:?}", cnts);
//             candidates.extend(dict.matches(cnts, cnts.iter().sum()));
//             // println!("end {:?}", candidates.len());
//             return;
//         }

//         if len >= 10 {
//             eprintln!("dfs: {} {} {} {}", len, bit, sum, xor);
//         }

//         for (i, c) in CANDIDATES.iter().enumerate() {
//             let c = *c as usize;
//             if c < last_val || bit < c.count_ones() as usize {
//                 continue;
//             }

//             // if c == last_val {
//             //     continue;
//             // }

//             let sum = (sum - c) & 0xFF;
//             let bit = bit - c.count_ones() as usize;
//             let xor = xor ^ c;

//             cnts[i] += 1;
//             dfs(
//                 dict,
//                 table,
//                 pattern,
//                 len - 1,
//                 bit,
//                 sum,
//                 xor,
//                 c,
//                 cnts,
//                 candidates,
//             );
//             cnts[i] -= 1;
//         }
//     }

//     for dbit in 0..=bit {
//         for dsum in 0..=0x15 {
//             let sum = (sum - dsum) & 0xFF;
//             let bit = bit - dbit;
//             if pattern[len][bit][sum][xor] {
//                 let mut candidates = Vec::new();
//                 let mut cnts = vec![0; CANDIDATES.len()];
//                 dfs(
//                     &dict,
//                     &table,
//                     &pattern,
//                     len,
//                     bit,
//                     sum,
//                     xor,
//                     0,
//                     &mut cnts,
//                     &mut candidates,
//                 );

//                 eprintln!("check {} {}: {}", dbit, dsum, candidates.len());
//                 if let Some(password) = candidates
//                     .into_iter()
//                     .find(|c| satisfy(c, &expected_memory))
//                 {
//                     eprintln!(
//                         "find: {:?}, {}",
//                         &password,
//                         password.iter().map(|&p| table[p]).collect::<String>()
//                     );
//                     return;
//                 }
//             }
//         }
//     }
// }

// pub fn bruteforce2() {
//     loop {
//         let mut memory = Memory {
//             checkdigit2: [0; 2],
//             password_len: password.len() as u8,
//             checkdigit5: [0, 0, 0, 1, 0],
//         };

//         // eprintln!("==========");
//         for i in 0..password.len() {
//             let mut cpu = CPU {
//                 reg: Register {
//                     a: CANDIDATES[password[i]],
//                     c: 0,
//                 },
//             };

//             calc_checkdigit1_naive(&mut cpu, &mut memory);
//             calc_checkdigit2_naive(&mut cpu, &mut memory);
//             calc_checkdigit3_naive(&mut cpu, &mut memory);
//             calc_checkdigit4_naive(&mut cpu, &mut memory);
//             calc_checkdigit5_naive(&mut cpu, &mut memory);

//             // if CANDIDATES[password[0]] == 0x19
//             //     && CANDIDATES[password[1]] == 0x09
//             //     && CANDIDATES[password[2]] == 0x18
//             // {
//             //     dbg!(&memory);
//             // }
//         }

//         // dbg!(&expected_memory);
//         // dbg!(&memory);
//         if expected_memory == memory {
//             println!("{:?}", password);
//             break;
//         }

//         inc(&mut password);
//     }
// }
