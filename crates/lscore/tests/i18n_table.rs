//! 文案表（`i18n`）的**结构性**不变量。
//!
//! 为什么单列一个文件（2026-10-12）：`t(lang, key, args)` 对**未知 key** 的行为是
//! 「原样返回 key」，注释里写明这是"开发期立刻可见，不静默吞掉"。这个设计是好的——
//! 但它把一类缺陷变成了**静默的**：拼错一个 key、往 `ui_object` 的清单里漏写一个 key、
//! 或者给模板加了 `{0}` 却忘了在调用点传参数，用户看到的就不是句子，而是
//! `th_value` 或 `审阅失败：{0}` 这样的裸标识符/裸占位符。这与第五节的错误通道缺陷
//! 同属一类：**工具把一个内部标识符当成话说了出来**。
//!
//! 本文件因此**不跑任何固件**——它扫的是**表的源码**和**调用点的源码**。这样连
//! 不可达分支里的参数个数写错也能被抓住（F8 正是这种：分支不可达，但错在那里躺着）。
//!
//! ⚠️ 这是**仓库布局耦合**的测试：它用 `include_str!` 读兄弟 crate 的源码，因为文案表是
//! 全 workspace 的契约，只扫本 crate 会漏掉一半调用点。布局若变动，本文件会**编译失败**
//! ——这是有意的：响亮、立刻、无法忽略。

use std::collections::{BTreeMap, BTreeSet};
use lscore::i18n::{t, Lang};

/// 表本身。
const TABLE: &str = include_str!("../src/i18n.rs");

/// 所有可能出现 `t(...)` 调用点的源文件。名字只用于失败时定位。
const CALL_SITE_SOURCES: &[(&str, &str)] = &[
    ("crates/lscore/src/lib.rs", include_str!("../src/lib.rs")),
    ("crates/lscore/src/psbt.rs", include_str!("../src/psbt.rs")),
    ("crates/lscore/src/state.rs", include_str!("../src/state.rs")),
    ("crates/lscore/src/script.rs", include_str!("../src/script.rs")),
    ("crates/lscore/src/tx.rs", include_str!("../src/tx.rs")),
    ("crates/lswasm/src/serialize.rs", include_str!("../../lswasm/src/serialize.rs")),
    ("crates/lswasm/src/lib.rs", include_str!("../../lswasm/src/lib.rs")),
    ("crates/lscli/src/main.rs", include_str!("../../lscli/src/main.rs")),
];

// ---------------------------------------------------------------------------
// 源码解析（零依赖：表里每条都是严格单行的 `(Lang::X, "key") => "value",`）
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Row {
    en: Option<String>,
    zh: Option<String>,
}

/// 解析表的一行。不匹配（注释、空行、match 的其它分支）返回 None。
fn parse_table_line(line: &str) -> Option<(String, String, String)> {
    let s = line.trim_start();
    let rest = s.strip_prefix("(Lang::")?;
    let (lang, rest) = if let Some(r) = rest.strip_prefix("En, \"") {
        ("En", r)
    } else if let Some(r) = rest.strip_prefix("ZhCn, \"") {
        ("ZhCn", r)
    } else {
        return None;
    };
    let key_end = rest.find("\")")?;
    let key = &rest[..key_end];
    let after = rest[key_end..].strip_prefix("\") => \"")?;
    let value = &after[..find_unescaped_quote(after)?];
    Some((lang.to_string(), key.to_string(), value.to_string()))
}

/// 返回第一个**未被转义**的双引号的下标（值里可能出现 `\"`）。
fn find_unescaped_quote(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    for i in 0..b.len() {
        if b[i] != b'"' {
            continue;
        }
        let mut backslashes = 0;
        let mut j = i;
        while j > 0 && b[j - 1] == b'\\' {
            backslashes += 1;
            j -= 1;
        }
        if backslashes % 2 == 0 {
            return Some(i);
        }
    }
    None
}

/// 表里声明的 `{n}` 占位符下标集合（去重、升序）。
fn placeholders(value: &str) -> Vec<usize> {
    let b = value.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'{' {
            let mut j = i + 1;
            let mut num = String::new();
            while j < b.len() && b[j].is_ascii_digit() {
                num.push(b[j] as char);
                j += 1;
            }
            if !num.is_empty() && j < b.len() && b[j] == b'}' {
                out.push(num.parse::<usize>().unwrap());
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    out.sort_unstable();
    out.dedup();
    out
}

fn table() -> BTreeMap<String, Row> {
    let mut m: BTreeMap<String, Row> = BTreeMap::new();
    for line in TABLE.lines() {
        if let Some((lang, key, value)) = parse_table_line(line) {
            let row = m.entry(key).or_default();
            if lang == "En" {
                row.en = Some(value);
            } else {
                row.zh = Some(value);
            }
        }
    }
    m
}

/// 剥掉注释。**不做这一步会出事**：注释里写一行示例调用（比如本文档里解释 F8 的那段），
/// 就会被当成真实调用点扫进去，于是"守护测试"对着文档报错。字符串字面量内的 `//` 不会被误剥。
fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let b = src.as_bytes();
    let mut i = 0;
    let mut in_string = false;
    while i < b.len() {
        let c = b[i];
        if in_string {
            out.push(c as char);
            if c == b'\\' {
                if i + 1 < b.len() {
                    out.push(b[i + 1] as char);
                    i += 2;
                    continue;
                }
            } else if c == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if c == b'"' {
            in_string = true;
            out.push('"');
            i += 1;
            continue;
        }
        if c == b'/' && i + 1 < b.len() && b[i + 1] == b'/' {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == b'/' && i + 1 < b.len() && b[i + 1] == b'*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(b.len());
            continue;
        }
        out.push(c as char);
        i += 1;
    }
    out
}

/// 一个调用点：key 与其参数个数。
struct CallSite {
    file: String,
    key: String,
    argc: usize,
}

/// 扫描出所有 `t(<lang>, "literal-key", &[ ... ])` 调用点。
///
/// **只覆盖字面量 key**。变量 key（`ui_object` 里的 `t(lang, k, &[])`）扫不到，
/// 那一份由 `crates/lswasm/src/serialize.rs` 的测试单独守。
fn call_sites(src: &str, file: &str) -> Vec<CallSite> {
    let src = strip_comments(src);
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < b.len() {
        let is_call = b[i] == b't' && b[i + 1] == b'(';
        let prev_is_ident = i > 0
            && (b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_' || b[i - 1] == b'.');
        if !is_call || prev_is_ident {
            i += 1;
            continue;
        }
        if let Some((key, argc, consumed)) = parse_call(&src[i + 2..]) {
            out.push(CallSite {
                file: file.to_string(),
                key,
                argc,
            });
            i += 2 + consumed;
        } else {
            i += 1;
        }
    }
    out
}

/// 解析 `t(` 之后的内容。返回 (key, 参数个数, 消耗的字节数)；无法识别则 None。
fn parse_call(s: &str) -> Option<(String, usize, usize)> {
    // key 是第一个字符串字面量，且它前面（忽略空白后）必须紧跟一个逗号——
    // 这条判据把 `format!("...")` 之类的普通字符串排除掉。
    let q = s.find('"')?;
    if !s[..q].trim_end().ends_with(',') {
        return None;
    }
    let after = &s[q + 1..];
    let key_end = after.find('"')?;
    let key = &after[..key_end];

    // key 之后必须是 `, &[`
    let tail = s[q + 1 + key_end + 1..].trim_start();
    let tail = tail.strip_prefix(',')?.trim_start();
    let args = tail.strip_prefix("&[")?;
    let start = s.len() - args.len();

    // 括号配对 + 按顶层逗号数参数。
    // **必须用 pending 而不是直接数逗号**：Rust 里多行参数列表几乎都带尾随逗号
    // （`&[a, b, c,]`），数逗号会把它算成 4 个参数，于是"守护测试"对着正确的代码报错。
    let b = args.as_bytes();
    let mut depth = 1usize;
    let mut argc = 0usize;
    let mut pending = false;
    let mut in_string = false;
    let mut i = 0usize;
    while i < b.len() {
        let c = b[i];
        if in_string {
            if c == b'\\' {
                i += 2;
                continue;
            }
            if c == b'"' {
                in_string = false;
            }
            pending = true;
            i += 1;
            continue;
        }
        match c {
            b'"' => {
                in_string = true;
                pending = true;
            }
            b'[' | b'(' | b'{' => {
                depth += 1;
                pending = true;
            }
            b']' | b')' | b'}' => {
                depth -= 1;
                if depth == 0 {
                    if pending {
                        argc += 1;
                    }
                    return Some((key.to_string(), argc, start + i + 1));
                }
                pending = true;
            }
            b',' if depth == 1 => {
                if pending {
                    argc += 1;
                    pending = false;
                }
            }
            _ => {
                if !c.is_ascii_whitespace() {
                    pending = true;
                }
            }
        }
        i += 1;
    }
    None
}

// ---------------------------------------------------------------------------
// 1. 键集：两种语言必须**同键**
// ---------------------------------------------------------------------------

#[test]
fn key_sets_are_identical_across_languages() {
    let m = table();
    assert!(
        m.len() >= 100,
        "解析出的 key 只有 {} 个——表格式可能变了，解析器已失效（测试必须响亮失败，不能静默通过）",
        m.len()
    );
    let only_en: Vec<_> = m
        .iter()
        .filter(|(_, r)| r.en.is_some() && r.zh.is_none())
        .map(|(k, _)| k.clone())
        .collect();
    let only_zh: Vec<_> = m
        .iter()
        .filter(|(_, r)| r.zh.is_some() && r.en.is_none())
        .map(|(k, _)| k.clone())
        .collect();
    assert!(
        only_en.is_empty() && only_zh.is_empty(),
        "两种语言的键集必须相同。只在 EN: {only_en:?}；只在 ZH: {only_zh:?} \
         —— 缺一边的 key 会走 `_ => k` 分支，把 key 本身当成句子显示出来"
    );
}

/// 每个 key 必须在**至少一种语言**下渲染出与 key 不同的字符串。
///
/// 为什么不是"两种语言都不同"：英文侧确实存在值与 key 相同的情形
/// （`(Lang::En, "unknown") => "unknown"`），那是**正确**的翻译，不是缺陷。
/// 而一个拼错/漏定义的 key 会在**两种语言下都原样返回 key**——所以判据是"任一不同"。
#[test]
fn every_key_is_defined_not_merely_echoed() {
    let m = table();
    let mut echoed = Vec::new();
    for (key, _) in m.iter() {
        let en = t(Lang::En, key, &[]);
        let zh = t(Lang::ZhCn, key, &[]);
        if en == *key && zh == *key {
            echoed.push(key.clone());
        }
    }
    assert!(
        echoed.is_empty(),
        "这些 key 在两种语言下都原样返回了自己（等于没定义）: {echoed:?}"
    );
}

// ---------------------------------------------------------------------------
// 2. 占位符：跨语言一致、且从 0 连续
// ---------------------------------------------------------------------------

#[test]
fn placeholders_match_across_languages() {
    let m = table();
    let mut bad = Vec::new();
    for (key, row) in m.iter() {
        let (Some(en), Some(zh)) = (&row.en, &row.zh) else {
            continue;
        };
        let pe = placeholders(en);
        let pz = placeholders(zh);
        if pe != pz {
            bad.push(format!("{key}: EN{pe:?} vs ZH{pz:?}"));
        }
    }
    assert!(
        bad.is_empty(),
        "同一 key 的 EN/ZH 占位符集合必须一致，否则插值会静默错位或丢内容：\n  {}",
        bad.join("\n  ")
    );
}

#[test]
fn placeholders_are_contiguous_from_zero() {
    let m = table();
    let mut bad = Vec::new();
    for (key, row) in m.iter() {
        for (lang, value) in [("EN", &row.en), ("ZH", &row.zh)] {
            let Some(v) = value else { continue };
            let ps = placeholders(v);
            let expected: Vec<usize> = (0..ps.len()).collect();
            if ps != expected {
                bad.push(format!("{key} ({lang}): {ps:?}"));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "占位符必须从 {{0}} 连续编号，否则调用的第 n 个参数不会被替换：\n  {}",
        bad.join("\n  ")
    );
}

// ---------------------------------------------------------------------------
// 3. 调用点：key 必须存在，参数个数必须与模板匹配
// ---------------------------------------------------------------------------

/// 扫描调用点的共同前置。
///
/// **自守**：解析器要是失灵了，下面两个测试会"通过"却什么也没检查——这本身正是本项目
/// 最反对的那种"无人守的声明"。所以这里断言扫描到的调用点数量下限，宁可响亮失败。
fn all_call_sites() -> Vec<CallSite> {
    let mut sites = Vec::new();
    for (file, src) in CALL_SITE_SOURCES {
        sites.extend(call_sites(src, file));
    }
    assert!(
        sites.len() >= 70,
        "只解析出 {} 个调用点——解析器可能已失效（测试必须响亮失败，不能静默通过）",
        sites.len()
    );
    sites
}

#[test]
fn every_call_site_uses_a_key_that_exists() {
    let m = table();
    let missing: BTreeSet<String> = all_call_sites()
        .iter()
        .filter(|s| !m.contains_key(&s.key))
        .map(|s| format!("{}: \"{}\"", s.file, s.key))
        .collect();
    assert!(
        missing.is_empty(),
        "这些调用点用了表里没有的 key，会把 key 本身当成句子显示：\n  {}",
        missing.into_iter().collect::<Vec<_>>().join("\n  ")
    );
}

#[test]
fn every_call_site_passes_the_arguments_its_template_needs() {
    let m = table();
    let mut bad = Vec::new();
    for s in all_call_sites() {
        let Some(row) = m.get(&s.key) else { continue };
        for (lang, value) in [("En", &row.en), ("ZhCn", &row.zh)] {
            let Some(v) = value else { continue };
            let ps = placeholders(v);
            let need = ps.last().map(|n| n + 1).unwrap_or(0);
            if need != s.argc {
                bad.push(format!(
                    "{}: \"{}\" ({lang}) 模板需要 {need} 个参数，调用点给了 {}",
                    s.file, s.key, s.argc
                ));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "参数个数与模板占位符不匹配 —— 多给无害，**少给会把字面量的 {{0}} 留在用户看到的句子里**：\n  {}",
        bad.join("\n  ")
    );
}
