//! 把 `lscore::Review` 编码成前端消费的 JSON。
//!
//! 关键约定：**五态与四层都出现在数据模型里**，前端只负责展示，不做任何判定或补全。
//! 每个字段都带 `state` 与按语言渲染好的 `state_explain`。
//! `ui` 对象把界面壳文案也从内核带出——前端**不持有任何语义文案**，
//! 加语言只改 `lscore::i18n`（宪法第 8 条的延伸）。

use lscore::i18n::{t, Lang};
use lscore::state::{Field, Review};

use crate::json;

/// 界面壳文案（纯标签，无语义判断）。前端拿它渲染，不得自行拼句子。
pub fn ui_object(lang: Lang) -> String {
    let keys = [
        "app_title", "tagline", "offline_badge", "drop_title", "drop_sub", "input_placeholder",
        "btn_review", "btn_demo", "hint_ready", "hint_wasm_fail", "hint_demo_fail",
        "layer1", "layer2", "layer3", "layer4", "footer1", "footer2",
        "th_index", "th_outpoint", "th_type", "th_amount", "th_sigs", "th_value", "th_script_addr",
        "dt_txid", "dt_version", "dt_locktime", "dt_total_out", "dt_fee", "dt_sighash",
        "unknown", "not_provided_label", "no_sigs", "sig_count", "cannot_show_addr",
        "empty_risks", "err_review_fail", "err_core_fail",
    ];
    let items: Vec<String> = keys
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
