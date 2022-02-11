use itertools::Itertools;
use packed_simd_2::u8x64;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::backward::Backward;

use crate::cpu::{forward_step_simd_u8x64, Memory};

use crate::domain::{to_string, PASSWORD_CHAR_CODES};
use crate::forward1::Forward1;
use crate::forward2::Forward2;
use crate::target::Target;
use crate::time::get_current_time;

// 14文字固定
type Password = [u8; 0x0E];

// 探索中の不変の情報を保持
struct EnumerationContext {
    // 枝刈り用の情報
    forward1: Forward1,
    forward2: Forward2,
    backward: Backward,

    // 探索対象の情報
    target: Target,

    // AVX512で次のパスワード
    a: u8x64,
}

impl EnumerationContext {
    // 探索の終端どうか
    fn finish(&self, password_len: usize) -> bool {
        password_len == self.target.len() - self.backward.len()
    }

    // 見つかったパスワードをfound_passwordsにセット
    fn extend_passwords(
        &self,
        search_context: &mut SearchContext,
        found_passwords: &mut Vec<Password>,
    ) {
        self.backward
            .for_each_password(&search_context.memory, |password_suffix| {
                search_context.password[search_context.len..]
                    .iter_mut()
                    .zip(password_suffix)
                    .for_each(|(l, r)| *l = *r);
                found_passwords.push(search_context.password);
            });
    }

    // 枝刈り
    fn is_valid(&self, len: usize, memory: &Memory) -> bool {
        memory.bit() <= self.target.memory.bit()
            && self.forward1.is_valid(len, memory)
            && self.forward2.is_valid(len, memory)
    }
}

// 探索中の可変の情報を保持
#[derive(Debug, Clone)]
struct SearchContext {
    memory: Memory,
    password: Password,
    len: usize,
}

impl SearchContext {
    // 次の状態を列挙する
    fn extract<'a>(
        &'a self,
        enumeration_context: &'a EnumerationContext,
    ) -> impl Iterator<Item = Self> + 'a {
        let ai = enumeration_context.a;
        let (f4, f5, f7, f8, f9, fa, fb) = forward_step_simd_u8x64(&self.memory, ai);

        (0..PASSWORD_CHAR_CODES.len()).filter_map(move |i| {
            let memory = Memory {
                f4: f4.extract(i),
                f5: f5.extract(i),
                f7: f7.extract(i),
                f8: f8.extract(i),
                f9: f9.extract(i),
                fa: fa.extract(i),
                fb: fb.extract(i),
            };

            let len = self.len + 1;
            if !enumeration_context.is_valid(len, &memory) {
                return None;
            }

            let mut password = self.password;
            password[self.len] = PASSWORD_CHAR_CODES[i];
            Some(Self {
                memory,
                password,
                len,
            })
        })
    }
}

// パスワード列挙するdfs
fn dfs(
    search_context: &mut SearchContext,
    enumeration_context: &EnumerationContext,
    found_passwords: &mut Vec<Password>,
) {
    if enumeration_context.finish(search_context.len) {
        enumeration_context.extend_passwords(search_context, found_passwords);
        return;
    }

    let ai = enumeration_context.a;
    let (f4, f5, f7, f8, f9, fa, fb) = forward_step_simd_u8x64(&search_context.memory, ai);

    let j = search_context.len;
    search_context.len += 1;

    (0..PASSWORD_CHAR_CODES.len()).for_each(|i| {
        search_context.memory.f4 = f4.extract(i);
        search_context.memory.f5 = f5.extract(i);
        search_context.memory.f7 = f7.extract(i);
        search_context.memory.f8 = f8.extract(i);
        search_context.memory.f9 = f9.extract(i);
        search_context.memory.fa = fa.extract(i);
        search_context.memory.fb = fb.extract(i);

        if !enumeration_context.is_valid(j + 1, &search_context.memory) {
            return;
        }

        search_context.password[j] = PASSWORD_CHAR_CODES[i];
        dfs(search_context, enumeration_context, found_passwords);
    });

    search_context.len -= 1;
}

// パスワード列挙処理
pub fn enumeration(target: &Target) {
    // 情報構築
    let backward = Backward::new(&target.memory);
    let forward1 = Forward1::build(target, &backward);
    let forward2 = Forward2::build(target, &backward);
    let enumeration_context = EnumerationContext {
        forward1,
        forward2,
        backward,
        target: target.clone(),
        a: create_simd_value(),
    };

    let search_context = {
        let memory = Memory::new();
        let password = Password::default();

        SearchContext {
            memory,
            password,
            len: 0,
        }
    };

    // パスワードのprefix5文字を列挙
    let search_contexts = enumerate_prefix(5, &search_context, &enumeration_context);

    // 処理の進行状況を確認するために、prefix 2文字ごとにまとめてグループ化
    let groups: Vec<(String, Vec<SearchContext>)> = search_contexts
        .into_iter()
        .group_by(|s| to_string(&s.password[0..2].to_vec()))
        .into_iter()
        .map(|(key, group)| (key, group.collect_vec()))
        .sorted_by_cached_key(|(key, _)| {
            key.chars()
                .map(|c| {
                    "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-!.nmc"
                        .chars()
                        .position(|cc| c == cc)
                        .unwrap()
                })
                .collect::<Vec<usize>>()
        })
        .collect();

    // prefix 2文字のグループごとにパスワードを列挙していく
    for (prefix, search_contexts) in groups {
        eprintln!("{} ({})", &prefix, get_current_time());

        let passwords = search_contexts
            .into_par_iter()
            .flat_map(|mut search_context| {
                let mut found_passwords = Vec::new();

                dfs(
                    &mut search_context,
                    &enumeration_context,
                    &mut found_passwords,
                );

                found_passwords
            })
            .collect::<Vec<_>>();

        println!(
            "{}",
            passwords
                .into_iter()
                .map(|password| to_string(&password))
                .join("\n")
        );
    }

    // 並列化のために接頭辞5文字分のパスワードを列挙
    fn enumerate_prefix(
        len: usize,
        search_context: &SearchContext,
        enumeration_context: &EnumerationContext,
    ) -> Vec<SearchContext> {
        if len == 0 {
            return vec![search_context.clone()];
        }

        search_context
            .extract(enumeration_context)
            .flat_map(|search_context| {
                enumerate_prefix(len - 1, &search_context, enumeration_context)
            })
            .collect()
    }

    fn create_simd_value() -> u8x64 {
        let v: Vec<_> = PASSWORD_CHAR_CODES
            .into_iter()
            .chain(std::iter::repeat(0xFF))
            .take(64)
            .collect();

        u8x64::from_slice_unaligned(&v)
    }
}
