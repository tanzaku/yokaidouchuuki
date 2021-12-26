use std::io::Read;

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

fn to_roma(s: &str) -> Vec<String> {
    let table = vec![
        ("じゃ", vec!["JA"]),
        ("じゅ", vec!["JU"]),
        ("じぇ", vec!["JE"]),
        ("じょ", vec!["JO"]),
        ("きゃ", vec!["KYA"]),
        ("きゅ", vec!["KYU"]),
        ("きょ", vec!["KYO"]),
        ("ぎゃ", vec!["GYA"]),
        ("ぎゅ", vec!["GYU"]),
        ("ぎょ", vec!["GYO"]),
        ("しゃ", vec!["SYA"]),
        ("しゅ", vec!["SYU"]),
        ("しぇ", vec!["SYE"]),
        ("しょ", vec!["SYO"]),
        ("ひゃ", vec!["HYA"]),
        ("ひゅ", vec!["HYU"]),
        ("ひょ", vec!["HYO"]),
        ("ぴゃ", vec!["PYA"]),
        ("ぴゅ", vec!["PYU"]),
        ("ぴょ", vec!["PYO"]),
        ("びゃ", vec!["BYA"]),
        ("びゅ", vec!["BYU"]),
        ("びょ", vec!["BYO"]),
        ("ちゃ", vec!["CYA"]),
        ("ちぇ", vec!["CYE"]),
        ("ちゅ", vec!["CYU"]),
        ("ちょ", vec!["CYO"]),
        ("ぢゃ", vec!["DYA"]),
        ("ぢゅ", vec!["DYU"]),
        ("ぢょ", vec!["DYO"]),
        ("みゃ", vec!["MYA"]),
        ("みゅ", vec!["MYU"]),
        ("みょ", vec!["MYO"]),
        ("りゃ", vec!["RYA"]),
        ("りゅ", vec!["RYU"]),
        ("りょ", vec!["RYO"]),
        ("にゃ", vec!["NYA"]),
        ("にゅ", vec!["NYU"]),
        ("にょ", vec!["NYO"]),
        ("ふぁ", vec!["FA"]),
        ("ふぃ", vec!["FI"]),
        ("ふぇ", vec!["FE"]),
        ("ふぉ", vec!["FO"]),
        // ("って", vec!["TTE"]),
        // ("っしょく", vec!["TTE"]),
        ("げ", vec!["GE"]),
        ("ご", vec!["GO"]),
        ("あ", vec!["A"]),
        ("い", vec!["I"]),
        ("う", vec!["U"]),
        ("え", vec!["E"]),
        ("お", vec!["O"]),
        ("か", vec!["KA"]),
        ("き", vec!["KI"]),
        ("く", vec!["KU"]),
        ("け", vec!["KE"]),
        ("こ", vec!["KO"]),
        ("が", vec!["GA"]),
        ("ぎ", vec!["GI"]),
        ("ぐ", vec!["GU"]),
        ("げ", vec!["GE"]),
        ("ご", vec!["GO"]),
        ("さ", vec!["SA"]),
        ("し", vec!["SI", "SHI"]),
        ("す", vec!["SU"]),
        ("せ", vec!["SE"]),
        ("そ", vec!["SO"]),
        ("ざ", vec!["ZA"]),
        ("じ", vec!["JI", "ZI"]),
        ("ず", vec!["ZU"]),
        ("ぜ", vec!["ZE"]),
        ("ぞ", vec!["ZO"]),
        ("た", vec!["TA"]),
        ("ち", vec!["TI", "CHI"]),
        ("つ", vec!["TU", "TSU"]),
        ("て", vec!["TE"]),
        ("と", vec!["TO"]),
        ("だ", vec!["DA"]),
        ("ぢ", vec!["DI"]),
        ("づ", vec!["DU"]),
        ("で", vec!["DE"]),
        ("ど", vec!["DO"]),
        ("な", vec!["NA"]),
        ("に", vec!["NI"]),
        ("ぬ", vec!["NU"]),
        ("ね", vec!["NE"]),
        ("の", vec!["NO"]),
        ("は", vec!["HA"]),
        ("ひ", vec!["HI"]),
        ("ふ", vec!["HU", "FU"]),
        ("へ", vec!["HE"]),
        ("ほ", vec!["HO"]),
        ("ば", vec!["BA"]),
        ("び", vec!["BI"]),
        ("ぶ", vec!["BU"]),
        ("べ", vec!["BE"]),
        ("ぼ", vec!["BO"]),
        ("ぱ", vec!["PA"]),
        ("ぴ", vec!["PI"]),
        ("ぷ", vec!["PU"]),
        ("ぺ", vec!["PE"]),
        ("ぽ", vec!["PO"]),
        ("ま", vec!["MA"]),
        ("み", vec!["MI"]),
        ("む", vec!["MU"]),
        ("め", vec!["ME"]),
        ("も", vec!["MO"]),
        ("や", vec!["YA"]),
        ("ゆ", vec!["YU"]),
        ("よ", vec!["YO"]),
        ("ら", vec!["RA"]),
        ("り", vec!["RI"]),
        ("る", vec!["RU"]),
        ("れ", vec!["RE"]),
        ("ろ", vec!["RO"]),
        ("わ", vec!["WA"]),
        ("を", vec!["WO"]),
        ("ん", vec!["N"]),
    ];

    let s0 = s;
    let res = table.iter().fold(vec![s.to_owned()], |s, (k, v)| {
        let s: Vec<_> = s
            .iter()
            .flat_map(|s| {
                if s.contains(k) {
                    v.iter().map(|v| s.replace(k, v)).collect()
                } else {
                    vec![s.to_owned()]
                }
            })
            .collect();

        s
    });

    res.iter().for_each(|s| {
        if s.chars().any(|c| !c.is_ascii_alphabetic()) {
            eprintln!("{} {:?}", s0, s.chars());
            panic!();
        }
    });
    // eprintln!("{} {} {:?} {:?}", s0, k, v, &s);
    res
}

pub fn skk_dict_search() {
    let dict_file = "./skk/SKK-JISYO.L.unannotated";
    let mut file = std::fs::File::open(dict_file).unwrap();
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();

    let mut skkdict = Vec::new();
    for s in s.lines() {
        let s = s.split_ascii_whitespace().collect::<Vec<_>>()[0];
        if s.contains(';')
            || s.contains('>')
            || s.contains('#')
            || s.contains('!')
            || s.contains('"')
            || s.contains('$')
            || s.contains('%')
            || s.contains('&')
            || s.contains('\'')
            || s.contains('(')
            || s.contains(')')
            || s.contains('*')
            || s.contains('+')
            || s.contains(',')
            || s.contains('-')
            || s.contains('.')
            || s.contains('/')
            || s.contains('0')
            || s.contains('1')
            || s.contains('2')
            || s.contains('3')
            || s.contains('4')
            || s.contains('5')
            || s.contains('6')
            || s.contains('7')
            || s.contains('8')
            || s.contains('9')
            || s.contains(':')
            || s.contains('<')
            || s.contains('@')
            || s.contains('=')
            || s.contains('~')
            || s.contains('|')
            || s.contains('^')
            || s.contains('?')
            || s.contains('[')
            || s.contains(']')
            || s.contains('\\')
            || s.contains('_')
            || s.contains('`')
            || s.contains('{')
            || s.contains('}')
            || s.contains('ー')
            || s.contains('「')
            || s.contains('」')
            || s.contains('っ')
            || s.contains("いぇ")
            || s.contains("でぃ")
            || s.contains("うぃ")
            || s.contains("うぇ")
            || s.contains("ゑ")
            || s.contains("ゐ")
            || s.contains("う゛ぁ")
            || s.contains("つぁ")
            || s.contains("てぃ")
            || s.contains("、")
            || s.contains("。")
            || s.contains("でゅ")
        {
            continue;
        }
        // eprintln!("{}", s);
        if s.chars().any(|c| c.is_ascii_alphabetic()) {
            continue;
        }
        let romas = to_roma(s);
        for roma in romas {
            if roma.len() >= 6 && roma.len() <= 14 {
                skkdict.push(roma);
            }
        }
    }

    let dict_file = "./passwords.txt";
    let mut file = std::fs::File::open(dict_file).unwrap();
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();

    let passwords = s.lines().collect::<Vec<_>>();
    passwords.par_iter().for_each(|password| {
        if skkdict.iter().any(|s| password.contains(s)) {
            println!("{}", password);
            // skkdict.iter().for_each(|s| {
            //     if password.contains(s) {
            //         println!(" {}", s);
            //     }
            // });
        }
    });
}
