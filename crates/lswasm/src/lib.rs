//! Lotus Signer · wasm 绑定层
//!
//! **本层不得包含任何解析、交易事实判定或五态逻辑** —— 它只做一件事：
//! 把 JS 传来的字节交给 `lscore`，把 `Review` 序列化成 JSON 回传。
//! 所有权威语义在 `lscore` 里，网页与命令行共用同一份实现（宪法第 8 条）。

use std::alloc::{alloc as rust_alloc, dealloc as rust_dealloc, Layout};

mod json;
mod serialize;

/// 分配一块线性内存给 JS 写入输入数据。
/// # Safety
/// 调用方须保证 `size > 0`，并在使用后调用 `ls_dealloc` 归还。
#[no_mangle]
pub extern "C" fn ls_alloc(size: usize) -> *mut u8 {
    if size == 0 {
        return std::ptr::null_mut();
    }
    unsafe {
        match Layout::from_size_align(size, 1) {
            Ok(layout) => rust_alloc(layout),
            Err(_) => std::ptr::null_mut(),
        }
    }
}

/// 归还 `ls_alloc` 分配的内存。
/// # Safety
/// `ptr` 必须来自 `ls_alloc`，`size` 必须与分配时一致。
#[no_mangle]
pub unsafe extern "C" fn ls_dealloc(ptr: *mut u8, size: usize) {
    if ptr.is_null() || size == 0 {
        return;
    }
    if let Ok(layout) = Layout::from_size_align(size, 1) {
        rust_dealloc(ptr, layout);
    }
}

/// 审阅入口。
///
/// 输入：`ptr`/`len` 指向 UTF-8 的 base64 PSBT 文本；`lang_ptr`/`lang_len` 指向语言标签
/// （"en"、"zh-CN"…，空串或未知标签回退英文）。
/// 输出：指向一块**长度前缀缓冲区**的指针 —— 头 4 字节为小端 u32 长度，其后为 JSON 字节。
/// 返回空指针表示分配失败。调用方读完须用 `ls_dealloc(ptr, len + 4)` 释放。
///
/// # Safety
/// `ptr` 必须指向 `len` 字节、`lang_ptr` 必须指向 `lang_len` 字节的有效可读内存。
#[no_mangle]
pub unsafe extern "C" fn ls_review(
    ptr: *const u8,
    len: usize,
    lang_ptr: *const u8,
    lang_len: usize,
) -> *mut u8 {
    let input = std::slice::from_raw_parts(ptr, len);
    let lang = if lang_ptr.is_null() || lang_len == 0 {
        lscore::i18n::Lang::En
    } else {
        let lang_bytes = std::slice::from_raw_parts(lang_ptr, lang_len);
        match std::str::from_utf8(lang_bytes) {
            Ok(s) => lscore::i18n::Lang::parse(s),
            Err(_) => lscore::i18n::Lang::En,
        }
    };

    let text = match std::str::from_utf8(input) {
        Ok(t) => t,
        Err(e) => {
            let body = format!(
                "{{\"ok\":false,\"error\":{}}}",
                json::string(&format!("invalid UTF-8 input: {}", e))
            );
            return pack(&body);
        }
    };

    let body = match lscore::review_psbt_base64_lang(text, lang) {
        Ok(review) => serialize::review_to_json(&review),
        Err(e) => format!("{{\"ok\":false,\"error\":{}}}", json::string(&e.to_string())),
    };

    pack(&body)
}

/// 界面壳文案：返回「4 字节小端长度 + JSON」——`{"ui":{...}}`（含 app_title/按钮/表头等）。
/// 前端在页面加载与切换语言时调用，**不持有任何语义文案**（宪法第 8 条的延伸）。
///
/// # Safety
/// `lang_ptr` 必须指向 `lang_len` 字节的有效可读内存（可为空指针/0）。
#[no_mangle]
pub unsafe extern "C" fn ls_ui(lang_ptr: *const u8, lang_len: usize) -> *mut u8 {
    let lang = if lang_ptr.is_null() || lang_len == 0 {
        lscore::i18n::Lang::En
    } else {
        let lang_bytes = std::slice::from_raw_parts(lang_ptr, lang_len);
        match std::str::from_utf8(lang_bytes) {
            Ok(s) => lscore::i18n::Lang::parse(s),
            Err(_) => lscore::i18n::Lang::En,
        }
    };
    let body = format!(
        "{{\"ok\":true,\"lang\":{},\"ui\":{}}}",
        json::string(lang.code()),
        serialize::ui_object(lang)
    );
    pack(&body)
}

/// 把字符串打包成「4 字节小端长度 + 内容」并返回指针。
fn pack(s: &str) -> *mut u8 {
    let bytes = s.as_bytes();
    let total = bytes.len() + 4;
    let out = unsafe {
        match Layout::from_size_align(total, 1) {
            Ok(layout) => rust_alloc(layout),
            Err(_) => return std::ptr::null_mut(),
        }
    };
    if out.is_null() {
        return out;
    }
    unsafe {
        let len_le = (bytes.len() as u32).to_le_bytes();
        std::ptr::copy_nonoverlapping(len_le.as_ptr(), out, 4);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out.add(4), bytes.len());
    }
    out
}

// ---------------------------------------------------------------------------
// FFI 层测试（2026-10-12 补）
//
// 为什么要有这一层：网页端的**错误路径**此前没有任何自动化测试。`serialize.rs` 的测试
// 覆盖的是成功路径的 JSON 结构；而"浏览器里那句错误提示是不是中文"完全没人守——
// 实测发现它曾经既漏了语言（中文界面显示 `PSBT parse failed: …`），又漏了前缀次数
// （`input decode failed: input decode failed: …`）。用 node 直接实例化 `.wasm` 能手工验，
// 但那是手工的；这组测试通过**与浏览器完全相同的调用序列**（`ls_alloc` 写内存 →
// `ls_review` → 读 4 字节长度前缀）把这条路径钉在 `cargo test` 里，不需要 Node、不需要 CI 新 job。
// ---------------------------------------------------------------------------
#[cfg(test)]
mod ffi_tests {
    use super::*;

    fn has_cjk(s: &str) -> bool {
        s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
    }

    /// 英文模板中**独有**的片段（与 `crates/lscore/tests/errors.rs` 的清单同源）。
    /// 中文句子允许出现 `PSBT` / `map` 这类术语拉丁字母，但不允许出现英文措辞。
    const EN_ONLY: &[&str] = &[
        "Review failed",
        "input decode failed",
        "PSBT parse failed",
        "not a PSBT",
        "the message ends abruptly",
        "truncated or corrupt",
        "does not contain",
        "will not guess",
        "key type",
    ];

    /// 走一遍浏览器实际走的调用序列，返回解包后的 JSON 文本。
    fn call_review(input: &str, lang: &str) -> String {
        let ib = input.as_bytes();
        let lb = lang.as_bytes();
        unsafe {
            let ip = ls_alloc(ib.len());
            assert!(!ip.is_null(), "ls_alloc failed for input");
            std::ptr::copy_nonoverlapping(ib.as_ptr(), ip, ib.len());

            let lp = ls_alloc(lb.len());
            assert!(!lp.is_null(), "ls_alloc failed for lang");
            std::ptr::copy_nonoverlapping(lb.as_ptr(), lp, lb.len());

            let out = ls_review(ip, ib.len(), lp, lb.len());
            assert!(!out.is_null(), "ls_review returned null");

            let mut len_le = [0u8; 4];
            std::ptr::copy_nonoverlapping(out, len_le.as_mut_ptr(), 4);
            let n = u32::from_le_bytes(len_le) as usize;
            let body = std::slice::from_raw_parts(out.add(4), n);
            let s = std::str::from_utf8(body)
                .expect("ls_review output must be valid UTF-8")
                .to_string();

            ls_dealloc(ip, ib.len());
            ls_dealloc(lp, lb.len());
            ls_dealloc(out, n + 4);
            s
        }
    }

    fn fixture(name: &str) -> String {
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.pop(); // crates/
        p.pop(); // repo root
        p.push("g3");
        p.push("fixtures");
        p.push(format!("{name}.psbt"));
        std::fs::read_to_string(&p)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()))
            .trim()
            .to_string()
    }

    /// 四条错误路径 × 两种语言。样本是硬编码的 base64，不依赖固件文件
    /// （除 S6——真实截断需要有 348 偏移的真实报文）。
    fn error_inputs() -> Vec<(&'static str, String)> {
        vec![
            ("base64 decode failure", "!!!not-base64!!!".to_string()),
            // 00 01 02 03 04 05 —— 魔数不对
            ("bad magic", "AAECAwQF".to_string()),
            // 70 73 62 74 ff 00 —— "psbt\xff" + 空的全局 map，合法但缺 key 0x00
            ("missing unsigned tx", "cHNidP8A".to_string()),
            ("genuine truncation", fixture("S6")),
        ]
    }

    /// 浏览器里读到的那句话，语言必须跟随 `lang` 参数。
    #[test]
    fn review_error_language_follows_the_lang_argument() {
        for (name, input) in error_inputs() {
            let en = call_review(&input, "en");
            assert!(
                en.contains("\"ok\":false"),
                "[{name}] must be reported as a failure: {en}"
            );
            assert!(
                !has_cjk(&en),
                "[{name}] the en payload must contain no CJK characters: {en}"
            );

            let zh = call_review(&input, "zh-CN");
            assert!(
                has_cjk(&zh),
                "[{name}] the zh payload must actually be in Chinese: {zh}"
            );
            for frag in EN_ONLY {
                assert!(
                    !zh.contains(frag),
                    "[{name}] zh payload leaked the English template fragment {frag:?}: {zh}"
                );
            }
        }
    }

    /// 前綴只允许出现一次（曾经是 `input decode failed: input decode failed: …`）。
    #[test]
    fn review_decode_error_is_not_double_prefixed() {
        let en = call_review("!!!not-base64!!!", "en");
        assert_eq!(
            en.matches("input decode failed:").count(),
            1,
            "the decode prefix must appear exactly once: {en}"
        );
        let zh = call_review("!!!not-base64!!!", "zh-CN");
        assert_eq!(
            zh.matches("输入解码失败：").count(),
            1,
            "the zh decode prefix must appear exactly once: {zh}"
        );
    }

    /// 缺必需键不得被描述成"报文被截断"（与内核 `crates/lscore/tests/errors.rs` 同一断言，
    /// 此处是网页端的那一份）。
    #[test]
    fn review_missing_unsigned_tx_is_not_reported_as_truncation() {
        let en = call_review("cHNidP8A", "en");
        assert!(
            !en.contains("truncated or corrupt") && !en.contains("ends abruptly"),
            "a complete message missing a required key is not truncated: {en}"
        );
        assert!(
            en.contains("PSBT_GLOBAL_UNSIGNED_TX"),
            "the error must name the key it did not find: {en}"
        );
        let zh = call_review("cHNidP8A", "zh-CN");
        assert!(
            !zh.contains("被截断或损坏") && !zh.contains("意外终止"),
            "zh payload must not claim a truncation that did not happen: {zh}"
        );
    }

    /// 修错误通道不能碰坏成功通道：正常报文经 FFI 仍返回 `ok:true`。
    #[test]
    fn review_success_path_still_works_through_ffi() {
        for lang in ["en", "zh-CN"] {
            let out = call_review(&fixture("S1"), lang);
            assert!(
                out.contains("\"ok\":true"),
                "[{lang}] a valid PSBT must review cleanly: {}",
                &out[..out.len().min(120)]
            );
        }
    }
}
