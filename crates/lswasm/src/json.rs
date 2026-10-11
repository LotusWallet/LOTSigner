//! 极小的 JSON 序列化器（零依赖）
//!
//! 为什么手写而不引 serde：内核依赖越少，可复核性越高（Cargo.toml 顶部已写明策略）。
//! 这里只服务一个用途——把 `Review` 结构编码成前端可解析的 JSON。

pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

pub fn string(s: &str) -> String {
    format!("\"{}\"", escape(s))
}

pub fn opt_string(s: &Option<String>) -> String {
    match s {
        Some(v) => string(v),
        None => "null".to_string(),
    }
}

pub fn opt_u64(v: &Option<u64>) -> String {
    match v {
        Some(v) => v.to_string(),
        None => "null".to_string(),
    }
}

pub fn strings(v: &[String]) -> String {
    let items: Vec<String> = v.iter().map(|s| string(s)).collect();
    format!("[{}]", items.join(","))
}
