//! 把 `lscore::Review` 编码成前端消费的 JSON。
//!
//! 关键约定：**五态与四层都出现在数据模型里**，前端只负责展示，不做任何判定或补全。
//! 每个字段都带 `state` 与按语言渲染好的 `state_explain`。
//! `ui` 对象把界面壳文案也从内核带出——前端**不持有任何语义文案**，
//! 加语言只改 `lscore::i18n`（宪法第 8 条的延伸）。

use lscore::i18n::{t, Lang};
use lscore::state::{Field, Review};

use crate::json;

/// 界面壳标签清单。
///
/// **抽成常量是为了能被测试守**（2026-10-12）：这是一份声明式清单，而 `t()` 对未知 key
/// 的行为是「原样返回 key」——清单里少写/拼错一个 key，前端拿到的标签就会是 `th_value`
/// 这样的裸标识符，而且**安静地**显示出来。它也是本文件里唯一用**变量 key** 调用 `t()` 的地方，
/// 因此 `crates/lscore/tests/i18n_table.rs` 的调用点审计**扫不到它**，必须单独守。
pub(crate) const UI_SHELL_KEYS: [&str; 40] = [
    "app_title", "tagline", "offline_badge", "drop_title", "drop_sub", "input_placeholder",
    "btn_review", "btn_demo", "btn_export", "hint_export_done", "hint_ready",
    "hint_wasm_fail", "hint_demo_fail",
    "layer1", "layer2", "layer3", "layer4", "footer1", "footer2",
    "th_index", "th_outpoint", "th_type", "th_amount", "th_sigs", "th_value", "th_script_addr",
    "dt_txid", "dt_version", "dt_locktime", "dt_total_out", "dt_fee", "dt_sighash",
    "unknown", "not_provided_label", "no_sigs", "sig_count", "cannot_show_addr",
    "empty_risks", "err_review_fail", "err_core_fail",
];

/// 界面壳文案（纯标签，无语义判断）。前端拿它渲染，不得自行拼句子。
///
/// 注意：这里对**所有**标签统一传 `&[]`。带 `{0}` 的标签（见
/// `ui_shell_keys_with_placeholders_are_the_substituted_ones`）**由前端做替换**——
/// 内核给的是模板，填值是数据。这是有意的分工，不是漏了参数。
pub fn ui_object(lang: Lang) -> String {
    let items: Vec<String> = UI_SHELL_KEYS
        .iter()
        .map(|k| format!("{}:{}", json::string(k), json::string(&t(lang, k, &[]))))
        .collect();
    format!("{{{}}}", items.join(","))
}

fn field_json<T: std::fmt::Display>(lang: Lang, f: &Field<T>) -> String {
    format!(
        "{{\"state\":{},\"value\":{},\"note\":{},\"state_explain\":{}}}",
        json::string(f.state.as_str()),
        match &f.value {
            Some(v) => v.to_string(),
            None => "null".to_string(),
        },
        json::opt_string(&f.note),
        json::string(&f.state.explain(lang)),
    )
}

fn field_u64_json(lang: Lang, f: &Field<u64>) -> String {
    format!(
        "{{\"state\":{},\"value\":{},\"note\":{},\"state_explain\":{}}}",
        json::string(f.state.as_str()),
        json::opt_u64(&f.value),
        json::opt_string(&f.note),
        json::string(&f.state.explain(lang)),
    )
}

pub fn review_to_json(r: &Review) -> String {
    let lang = r.lang;
    let f = &r.facts;

    let inputs: Vec<String> = f
        .inputs
        .iter()
        .map(|i| {
            format!(
                "{{\"index\":{},\"outpoint\":{},\"sequence\":{},\"script_type\":{},\"amount\":{},\"partial_sig_count\":{},\"has_witness_utxo\":{},\"has_non_witness_utxo\":{}}}",
                i.index,
                json::string(&i.outpoint),
                i.sequence,
                json::opt_string(&i.script_type),
                field_u64_json(lang, &i.amount_sats),
                i.partial_sig_count,
                i.has_witness_utxo,
                i.has_non_witness_utxo,
            )
        })
        .collect();

    let outputs: Vec<String> = f
        .outputs
        .iter()
        .map(|o| {
            format!(
                "{{\"index\":{},\"value_sats\":{},\"script_type\":{},\"address\":{},\"script_hex\":{}}}",
                o.index,
                o.value_sats,
                json::string(&o.script_type),
                json::opt_string(&o.address),
                json::string(&o.script_hex),
            )
        })
        .collect();

    let risks: Vec<String> = r
        .risks
        .iter()
        .map(|k| {
            format!(
                "{{\"code\":{},\"summary\":{},\"trigger_reason\":{},\"evidence\":{}}}",
                json::string(k.code),
                json::string(&k.summary),
                json::string(&k.trigger_reason),
                json::string(&k.evidence),
            )
        })
        .collect();

    format!(
        "{{\"ok\":true,\"lang\":{},\"ui\":{},\"facts\":{{\"txid\":{},\"version\":{},\"locktime\":{},\"locktime_meaning\":{},\
\"inputs\":[{}],\"outputs\":[{}],\"total_out_sats\":{},\"fee_sats\":{},\
\"sighash_type\":{},\"sighash_policy\":{}}},\
\"context\":{},\"risks\":[{}],\"not_provided\":{},\"unsupported_keys\":{}}}",
        json::string(lang.code()),
        ui_object(lang),
        json::string(&f.txid),
        f.version,
        f.locktime,
        json::string(&f.locktime_meaning),
        inputs.join(","),
        outputs.join(","),
        f.total_out_sats,
        field_u64_json(lang, &f.fee_sats),
        field_json(lang, &f.sighash_type),
        json::string(&f.sighash_policy),
        json::strings(&r.context),
        risks.join(","),
        json::strings(&r.not_provided),
        json::strings(&r.unsupported_keys),
    )
}

// ===========================================================================
// 契约测试（P1-SPEC 附录 A）
//
// 为什么需要：附录 A 把"前端消费的 JSON 形状"写成了一份**契约**，但在本次之前
// **没有任何测试守着它**（`lswasm` 此前 0 项测试）。一份无人守的契约会悄悄漂移，
// 而前端只会在浏览器里失败——那正是"构建通过 ≠ 正确"的老问题。
//
// 这里不引任何 JSON 库（本项目零依赖），自带一个极小的良构校验器：
// 它能证明"发出的确实是合法 JSON"，而不只是"含有某些子串"。
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use lscore::i18n::Lang;

    // ---------- 极小 JSON 良构校验器（零依赖） ----------

    fn skip_ws(b: &[u8], i: &mut usize) {
        while *i < b.len() && matches!(b[*i], b' ' | b'\t' | b'\n' | b'\r') {
            *i += 1;
        }
    }

    fn expect_lit(b: &[u8], i: &mut usize, lit: &[u8]) -> Result<(), String> {
        if b.len() >= *i + lit.len() && &b[*i..*i + lit.len()] == lit {
            *i += lit.len();
            Ok(())
        } else {
            Err(format!(
                "expected {:?} at offset {}",
                core::str::from_utf8(lit).unwrap(),
                i
            ))
        }
    }

    fn parse_string(b: &[u8], i: &mut usize) -> Result<(), String> {
        if *i >= b.len() || b[*i] != b'"' {
            return Err(format!("expected a string at offset {}", i));
        }
        *i += 1;
        while *i < b.len() {
            match b[*i] {
                b'"' => {
                    *i += 1;
                    return Ok(());
                }
                b'\\' => {
                    *i += 1;
                    if *i >= b.len() {
                        return Err("truncated escape sequence".into());
                    }
                    match b[*i] {
                        b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => *i += 1,
                        b'u' => {
                            if *i + 4 >= b.len() {
                                return Err("truncated \\u escape".into());
                            }
                            for k in 1..=4 {
                                if !b[*i + k].is_ascii_hexdigit() {
                                    return Err("non-hex digit in \\u escape".into());
                                }
                            }
                            *i += 5;
                        }
                        e => return Err(format!("invalid escape \\{}", e as char)),
                    }
                }
                0x00..=0x1f => return Err("raw control character inside a string".into()),
                _ => *i += 1,
            }
        }
        Err("unterminated string".into())
    }

    fn parse_number(b: &[u8], i: &mut usize) -> Result<(), String> {
        let start = *i;
        if b[*i] == b'-' {
            *i += 1;
        }
        while *i < b.len() && b[*i].is_ascii_digit() {
            *i += 1;
        }
        if *i == start || (b[start] == b'-' && *i == start + 1) {
            return Err(format!("malformed number at offset {}", start));
        }
        Ok(())
    }

    fn parse_value(b: &[u8], i: &mut usize) -> Result<(), String> {
        if *i >= b.len() {
            return Err("unexpected end of input".into());
        }
        match b[*i] {
            b'{' => parse_object(b, i),
            b'[' => parse_array(b, i),
            b'"' => parse_string(b, i),
            b't' => expect_lit(b, i, b"true"),
            b'f' => expect_lit(b, i, b"false"),
            b'n' => expect_lit(b, i, b"null"),
            b'-' | b'0'..=b'9' => parse_number(b, i),
            c => Err(format!("unexpected byte {:?} at offset {}", c as char, i)),
        }
    }

    fn parse_object(b: &[u8], i: &mut usize) -> Result<(), String> {
        *i += 1;
        skip_ws(b, i);
        if *i < b.len() && b[*i] == b'}' {
            *i += 1;
            return Ok(());
        }
        loop {
            skip_ws(b, i);
            parse_string(b, i)?;
            skip_ws(b, i);
            if *i >= b.len() || b[*i] != b':' {
                return Err(format!("expected ':' at offset {}", i));
            }
            *i += 1;
            skip_ws(b, i);
            parse_value(b, i)?;
            skip_ws(b, i);
            if *i >= b.len() {
                return Err("unexpected end inside an object".into());
            }
            match b[*i] {
                b',' => *i += 1,
                b'}' => {
                    *i += 1;
                    return Ok(());
                }
                c => return Err(format!("expected ',' or '}}' but got {:?} at {}", c as char, i)),
            }
        }
    }

    fn parse_array(b: &[u8], i: &mut usize) -> Result<(), String> {
        *i += 1;
        skip_ws(b, i);
        if *i < b.len() && b[*i] == b']' {
            *i += 1;
            return Ok(());
        }
        loop {
            skip_ws(b, i);
            parse_value(b, i)?;
            skip_ws(b, i);
            if *i >= b.len() {
                return Err("unexpected end inside an array".into());
            }
            match b[*i] {
                b',' => *i += 1,
                b']' => {
                    *i += 1;
                    return Ok(());
                }
                c => return Err(format!("expected ',' or ']' but got {:?} at {}", c as char, i)),
            }
        }
    }

    fn validate_json(s: &str) -> Result<(), String> {
        let b = s.as_bytes();
        let mut i = 0usize;
        skip_ws(b, &mut i);
        parse_value(b, &mut i)?;
        skip_ws(b, &mut i);
        if i != b.len() {
            return Err(format!("trailing bytes from offset {}", i));
        }
        Ok(())
    }

    // ---------- 夹具 ----------

    /// 可解析固件全集（与 crates/lscore/tests/fixtures.rs 的 PARSEABLE 一致；
    /// S6 是 malformed，必须报错，故不在此列）。
    const PARSEABLE: [&str; 9] = ["S1", "S2a", "S2b", "S3", "S4", "S5", "S7", "S8", "S9"];

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

    fn review_json(name: &str, lang: Lang) -> String {
        let r = lscore::review_psbt_base64_lang(&fixture(name), lang)
            .unwrap_or_else(|e| panic!("{name} should parse cleanly: {e}"));
        review_to_json(&r)
    }

    fn count(hay: &str, needle: &str) -> usize {
        hay.matches(needle).count()
    }

    // ---------- 测试 ----------

    /// 每种固件、每种语言下，发出的都必须是**合法 JSON**（而非"含有某些子串"）。
    #[test]
    fn emitted_json_is_well_formed_for_every_fixture() {
        for name in PARSEABLE {
            for lang in [Lang::En, Lang::ZhCn] {
                let s = review_json(name, lang);
                if let Err(e) = validate_json(&s) {
                    panic!("{name}/{lang:?}: emitted JSON is malformed ({e})\n{s}");
                }
            }
        }
    }

    /// P1-SPEC 附录 A 的顶层与 facts 键必须存在。
    #[test]
    fn json_contract_matches_p1_spec_appendix_a() {
        let s = review_json("S1", Lang::En);

        for k in [
            "\"ok\"", "\"lang\"", "\"ui\"", "\"facts\"", "\"context\"", "\"risks\"",
            "\"not_provided\"", "\"unsupported_keys\"",
        ] {
            assert!(s.contains(k), "top-level key {k} missing");
        }
        assert!(s.contains("\"ok\":true"), "ok must be true on success");

        for k in [
            "\"txid\"", "\"version\"", "\"locktime\"", "\"locktime_meaning\"", "\"inputs\"",
            "\"outputs\"", "\"total_out_sats\"", "\"fee_sats\"", "\"sighash_type\"",
            "\"sighash_policy\"",
        ] {
            assert!(s.contains(k), "facts key {k} missing");
        }
        for k in [
            "\"index\"", "\"outpoint\"", "\"sequence\"", "\"script_type\"", "\"amount\"",
            "\"partial_sig_count\"", "\"has_witness_utxo\"", "\"has_non_witness_utxo\"",
        ] {
            assert!(s.contains(k), "input key {k} missing");
        }
        for k in ["\"value_sats\"", "\"address\"", "\"script_hex\""] {
            assert!(s.contains(k), "output key {k} missing");
        }

        // 每个"字段对象"必须同时带 state / value / note / state_explain —— 前端据此渲染，
        // 缺任何一个都会让界面少一格或崩掉。
        let n_state = count(&s, "\"state\":");
        assert!(n_state > 0, "at least one field object expected");
        assert_eq!(n_state, count(&s, "\"state_explain\":"), "state 与 state_explain 数量不一致");
        assert_eq!(n_state, count(&s, "\"note\":"), "state 与 note 数量不一致");
        assert_eq!(n_state, count(&s, "\"value\":"), "state 与 value 数量不一致");
    }

    /// 风险对象四字段齐备（用 S2a 触发 R002）。
    #[test]
    fn risk_objects_carry_all_four_fields() {
        let s = review_json("S2a", Lang::En);
        for k in ["\"code\":\"R002\"", "\"summary\":", "\"trigger_reason\":", "\"evidence\":"] {
            assert!(s.contains(k), "risk object missing {k}\n{s}");
        }
    }

    /// 第四层与语言码如实透出（前端不得自行改写状态或文案）。
    #[test]
    fn not_provided_and_lang_are_exposed() {
        assert!(review_json("S1", Lang::En).contains("\"lang\":\"en\""));
        assert!(review_json("S1", Lang::ZhCn).contains("\"lang\":\"zh-CN\""));
        // 第四层的固定内容必须在两种语言下都非空且随语言变化
        let en = review_json("S1", Lang::En);
        let zh = review_json("S1", Lang::ZhCn);
        assert!(en.contains("does not judge the transaction"));
        assert!(zh.contains("不判断这笔交易"));
    }

    /// 界面壳文案由内核提供，前端不持有语义文案（宪法第 8 条的延伸）。
    #[test]
    fn ui_object_exposes_shell_labels() {
        let s = ui_object(Lang::En);
        for k in [
            "app_title", "layer1", "layer2", "layer3", "layer4", "btn_review", "btn_export",
            "drop_title", "empty_risks", "err_review_fail",
        ] {
            assert!(s.contains(&format!("\"{k}\":")), "ui key {k} missing");
        }
        assert!(validate_json(&s).is_ok(), "ui object must itself be well-formed JSON");
        // 五态解释随语言走，且不出现"安全/危险"终局判定措辞
        let ui = ui_object(Lang::ZhCn);
        assert!(ui.contains("本工具不提供以下结论"));
    }

    /// 转义：含引号/反斜杠/换行的字符串必须被正确转义（否则前端解析会崩）。
    #[test]
    fn escaping_keeps_json_well_formed() {
        let nasty = "quote\" backslash\\ newline\n tab\t ctrl\u{1}";
        let s = format!("{{\"s\":{}}}", json::string(nasty));
        assert!(validate_json(&s).is_ok(), "escaped string broke the JSON: {s}");
        assert!(!s.contains('\n'), "raw newline leaked into JSON output");
    }

    /// 界面壳清单里的每个 key 都必须在表里有定义。
    ///
    /// 这是 `crates/lscore/tests/i18n_table.rs` 的**调用点审计扫不到的那一份**：
    /// 那里只扫字面量 key，而 `ui_object` 是拿变量 key 去调 `t()` 的。
    /// 一个拼错的 key 会让前端把裸标识符当标签显示，且悄无声息。
    #[test]
    fn ui_shell_keys_are_defined_in_the_table() {
        for k in UI_SHELL_KEYS {
            let en = t(Lang::En, k, &[]);
            let zh = t(Lang::ZhCn, k, &[]);
            assert!(
                en != k || zh != k,
                "ui shell key {k:?} is defined in neither language — the page would render the raw key"
            );
        }
        // 自守：清单本身不能退化（少了 key 这个测试会"通过"却覆盖不到东西）
        assert_eq!(UI_SHELL_KEYS.len(), 40, "ui shell key list changed size");
        let uniq: std::collections::BTreeSet<_> = UI_SHELL_KEYS.iter().collect();
        assert_eq!(uniq.len(), UI_SHELL_KEYS.len(), "ui shell key list has duplicates");
    }

    /// 带 `{0}` 的界面标签**必须**是前端会做替换的那几个。
    ///
    /// 为什么单列一条：`ui_object` 对每个标签统一传 `&[]`，所以任何带 `{0}` 的模板
    /// 到了页面上都是字面量 `{0}`——**除非前端自己 `replace("{0}", …)`**。
    /// 于是"往清单里加一个带占位符的标签"这个动作，必须同时改前端，否则页面显示 `{0}`。
    /// 这条断言把那个跨层契约钉在明面上（前端替换点见 `web/index.html`）。
    #[test]
    fn ui_shell_keys_with_placeholders_are_the_substituted_ones() {
        let mut with_ph: Vec<&str> = UI_SHELL_KEYS
            .iter()
            .copied()
            .filter(|k| t(Lang::En, k, &[]).contains("{0}"))
            .collect();
        with_ph.sort_unstable();
        assert_eq!(
            with_ph,
            ["cannot_show_addr", "err_core_fail", "err_review_fail", "hint_wasm_fail", "sig_count"],
            "带占位符的界面标签集合变了。若你新加了一个带 {{0}} 的标签：前端必须自己替换它，\
             否则页面上会显示字面量 {{0}} —— 请到 web/index.html 补 replace，并更新本断言与注释。"
        );
        // 现状说明（2026-10-12 核实，见 P1-SPEC-AUDIT F9）：
        //   sig_count / cannot_show_addr / err_review_fail / err_core_fail  —— 前端已替换；
        //   hint_wasm_fail —— **前端从未使用**，且按构造不可达：要显示它就需要 UI，
        //   而 UI 由同一个 wasm 的 ls_ui 提供——wasm 挂了就没有 ls_ui。故它是死标签，
        //   登记而不删除（它会出现在 UI 对象里，动它属于对外契约变更）。
    }
}
