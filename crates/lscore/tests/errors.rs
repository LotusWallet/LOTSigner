//! 错误通道的**语言**与**措辞**不变量。
//!
//! 为什么单列一个文件（2026-10-12）：这一族缺陷不是"解析算错了数"，而是**错误句子本身说错话**——
//! 句子落到了请求之外的语言（中文界面里出现 `PSBT parse failed:`），或者描述了并未发生的事件
//! （把"全局 map 完整、只缺一个必需键"说成"报文被截断或损坏"）。
//! 对一个主张"我只说读到的内容"的工具，错误通道和事实通道是**同等级**的正确性问题：
//! 用户读到的唯一一句话就是这句错误，它说错，工具的全部承诺就落空。
//! 因此这族不变量与固件套件分开守，见 PROGRESS 第十五节 / P1-SPEC-AUDIT F5·F6。

use lscore::i18n::{t, Lang};
use lscore::review_psbt_base64_lang;
use lscore::review_psbt_bytes_lang;

// ---------------------------------------------------------------------------
// 合成输入：只用到 BIP-174 的容器语法，不需要任何固件文件
// ---------------------------------------------------------------------------

const MAGIC: [u8; 5] = [0x70, 0x73, 0x62, 0x74, 0xff]; // "psbt\xff"

/// 魔数不对
fn bad_magic() -> Vec<u8> {
    vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05]
}

/// **真截断**：全局 map 里一个键值是完整的，但 value 声明的长度超出报文末尾。
/// 这是 `Truncated` 的正当用法——报文的确实在 map 中途断掉了。
fn truncated_global_map() -> Vec<u8> {
    let mut v = MAGIC.to_vec();
    v.extend_from_slice(&[0x01, 0x01, 0x00]); // key_len=1, key_type=0x01, key_data=0x00
    v.push(0x05); // value_len=5
    v.extend_from_slice(&[0x41, 0x42]); // 实际只给了 2 字节
    v
}

/// 全局 map 结构完整、正常以 0x00 终止，但**不含** v0 必需的 key 0x00。
/// 这不是截断：报文一个字节都没少，少的是一个必需键。
///
/// 键值编码是 `key_len | key_type | key_data | value_len | value`，其中
/// `key_len = 1 + len(key_data)`——这里 key_data 为空，故 key_len=1、value 占 1 字节。
/// （第一次写这条样本时把 value_len 写成了 0，于是它变成一个**真**截断，
/// 被下面的 `missing_unsigned_tx_is_not_reported_as_truncation` 当场揭穿。）
fn global_map_without_unsigned_tx() -> Vec<u8> {
    let mut v = MAGIC.to_vec();
    v.extend_from_slice(&[0x01, 0x01]); // key_len=1, key_type=0x01 (PSBT_GLOBAL_XPUB)
    v.extend_from_slice(&[0x01, 0x00]); // value_len=1, value=0x00
    v.push(0x00); // map 正常终止
    v
}

/// 空全局 map —— 同样缺 key 0x00，且格式完全合法。
fn empty_global_map() -> Vec<u8> {
    let mut v = MAGIC.to_vec();
    v.push(0x00);
    v
}

/// 只含 PSBTv2 专有键 0x02（GLOBAL_TX_VERSION）。
fn v2_tx_version_field() -> Vec<u8> {
    let mut v = MAGIC.to_vec();
    v.extend_from_slice(&[0x01, 0x02, 0x04, 0x00, 0x00, 0x00, 0x00]);
    v.push(0x00);
    v
}

// ---------------------------------------------------------------------------
// 辅助
// ---------------------------------------------------------------------------

fn err_of_bytes(raw: &[u8], lang: Lang) -> String {
    review_psbt_bytes_lang(raw, lang)
        .expect_err("this input must be rejected")
        .to_string()
}

fn err_of_b64(s: &str, lang: Lang) -> String {
    review_psbt_base64_lang(s, lang)
        .expect_err("this input must be rejected")
        .to_string()
}

fn has_cjk(s: &str) -> bool {
    s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
}

fn count(hay: &str, needle: &str) -> usize {
    hay.matches(needle).count()
}

/// 英文模板中**独有**的片段。中文句子允许出现拉丁字母（`PSBT` / `map` / `offset` 之类
/// 是术语，不译），但绝不允许出现英文模板的措辞——那说明有一条错误路径绕过了文案表，
/// 或者把另一语言的半成品拼了进来。
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
    "PSBTv2 field",
    "supports PSBT v0 only",
    "trailing byte",
    "embedded unsigned transaction",
];

/// 本次用来覆盖四条错误路径的样本：(说明, 取错误的闭包)
fn error_cases() -> Vec<(&'static str, fn(Lang) -> String)> {
    vec![
        ("base64 decode failure", (|l| err_of_b64("!!!not-base64!!!", l)) as fn(Lang) -> String),
        ("bad magic", |l| err_of_bytes(&bad_magic(), l)),
        ("genuine truncation", |l| err_of_bytes(&truncated_global_map(), l)),
        ("global map without unsigned tx", |l| {
            err_of_bytes(&global_map_without_unsigned_tx(), l)
        }),
        ("empty global map", |l| err_of_bytes(&empty_global_map(), l)),
        ("psbtv2 field", |l| err_of_bytes(&v2_tx_version_field(), l)),
    ]
}

// ---------------------------------------------------------------------------
// 1. 语言：错误句子的语言必须跟随请求，不得混入另一语言的模板
// ---------------------------------------------------------------------------

#[test]
fn error_sentences_follow_the_requested_language() {
    for (name, f) in error_cases() {
        let en = f(Lang::En);
        assert!(
            !has_cjk(&en),
            "[{}] an English error must contain no CJK characters, got: {}",
            name,
            en
        );
        assert!(!en.trim().is_empty(), "[{}] English error is empty", name);

        let zh = f(Lang::ZhCn);
        assert!(
            has_cjk(&zh),
            "[{}] a Chinese error must actually be in Chinese, got: {}",
            name,
            zh
        );
        for frag in EN_ONLY {
            assert!(
                !zh.contains(frag),
                "[{}] Chinese error leaked the English template fragment {:?}: {}",
                name,
                frag,
                zh
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 2. 双重前缀：前缀模板只允许补一次
// ---------------------------------------------------------------------------

#[test]
fn decode_failure_states_the_cause_exactly_once() {
    // 修复前：构造处先用 err_decode 包一层、Display 再包一层，
    // 输出 "input decode failed: input decode failed: InvalidChar(33)"。
    let en = err_of_b64("!!!not-base64!!!", Lang::En);
    assert_eq!(
        count(&en, "input decode failed:"),
        1,
        "the decode prefix must appear exactly once, got: {}",
        en
    );
    // 宪法第 5 条：报错必须点名问题，不能只说"解码失败"。
    assert!(
        en.contains("InvalidChar") || en.contains("InvalidLength"),
        "the decoder's own diagnostic must survive (the cause is the observation), got: {}",
        en
    );

    let zh = err_of_b64("!!!not-base64!!!", Lang::ZhCn);
    assert_eq!(
        count(&zh, "输入解码失败："),
        1,
        "the zh decode prefix must appear exactly once, got: {}",
        zh
    );
    assert!(
        !zh.contains("input decode failed"),
        "no English prefix may survive in the zh error, got: {}",
        zh
    );
}

// ---------------------------------------------------------------------------
// 3. CLI 实际打印的那一行：内核 + err_review_fail 包装后仍然单语
// ---------------------------------------------------------------------------

#[test]
fn cli_composed_error_line_is_single_language() {
    // lscli 打印的是 t(lang, "err_review_fail", &[&e.to_string()])，
    // 这一层包装也曾把英文前缀粘在中文句子上。
    for (name, f) in error_cases() {
        let en_line = t(Lang::En, "err_review_fail", &[&f(Lang::En)]);
        assert!(
            !has_cjk(&en_line),
            "[{}] composed English line leaked CJK: {}",
            name,
            en_line
        );

        let zh_line = t(Lang::ZhCn, "err_review_fail", &[&f(Lang::ZhCn)]);
        assert!(zh_line.starts_with("审阅失败"), "[{}] zh line: {}", name, zh_line);
        for frag in EN_ONLY {
            assert!(
                !zh_line.contains(frag),
                "[{}] composed Chinese line leaked {:?}: {}",
                name,
                frag,
                zh_line
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 4·5. 措辞必须与观察一致：缺键 ≠ 截断，而真截断仍如实报截断
// ---------------------------------------------------------------------------

#[test]
fn missing_unsigned_tx_is_not_reported_as_truncation() {
    for raw in [global_map_without_unsigned_tx(), empty_global_map()] {
        let en = err_of_bytes(&raw, Lang::En);
        assert!(
            !en.contains("truncated or corrupt"),
            "a complete message that merely lacks a required key must not be called truncated: {}",
            en
        );
        assert!(
            !en.contains("ends abruptly"),
            "nothing ends abruptly here; the global map terminated normally: {}",
            en
        );
        assert!(
            en.contains("PSBT_GLOBAL_UNSIGNED_TX"),
            "the error must name the key it did not find: {}",
            en
        );

        let zh = err_of_bytes(&raw, Lang::ZhCn);
        assert!(
            !zh.contains("被截断或损坏") && !zh.contains("意外终止"),
            "zh error must not claim a truncation that did not happen: {}",
            zh
        );
        assert!(
            zh.contains("PSBT_GLOBAL_UNSIGNED_TX"),
            "zh error must name the missing key: {}",
            zh
        );
    }
}

#[test]
fn genuine_truncation_is_still_reported_as_truncation() {
    // 反向守卫：修掉"缺键误报截断"时不能顺手把真截断也改口。
    let en = err_of_bytes(&truncated_global_map(), Lang::En);
    assert!(
        en.contains("truncated or corrupt"),
        "a genuinely truncated message must still be called truncated: {}",
        en
    );
    assert!(
        en.contains("offset"),
        "a truncation must point at the byte offset: {}",
        en
    );

    let zh = err_of_bytes(&truncated_global_map(), Lang::ZhCn);
    assert!(zh.contains("被截断或损坏"), "zh truncation message: {}", zh);
    assert!(zh.contains("偏移"), "zh truncation must state the offset: {}", zh);
}

// ---------------------------------------------------------------------------
// 6. 两条相邻路径不得合并：v2 字段是"不支持的版本"，不是"缺键"
// ---------------------------------------------------------------------------

#[test]
fn psbtv2_field_still_reported_as_unsupported_version() {
    let en = err_of_bytes(&v2_tx_version_field(), Lang::En);
    assert!(
        en.contains("PSBTv2"),
        "a v2-specific key must be named as such, got: {}",
        en
    );
    assert!(
        !en.contains("PSBT_GLOBAL_UNSIGNED_TX"),
        "the v2 path and the missing-key path must stay distinguishable, got: {}",
        en
    );
    let zh = err_of_bytes(&v2_tx_version_field(), Lang::ZhCn);
    assert!(zh.contains("PSBTv2"), "zh v2 message: {}", zh);
}
