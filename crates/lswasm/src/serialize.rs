//! 把 `lscore::Review` 编码成前端消费的 JSON。
//!
//! 关键约定：**五态与四层都出现在数据模型里**，前端只负责展示，不做任何判定或补全。
//! 每个字段都带 `state`，前端据此决定徽标与措辞（宪法第 6、7 条）。

use lscore::state::{Field, Review};

use crate::json;

fn field_json<T: std::fmt::Display>(f: &Field<T>) -> String {
    format!(
        "{{\"state\":{},\"value\":{},\"note\":{},\"state_explain\":{}}}",
        json::string(f.state.as_str()),
        match &f.value {
            Some(v) => v.to_string(),
            None => "null".to_string(),
        },
        json::opt_string(&f.note),
        json::string(f.state.explain()),
    )
}

fn field_u64_json(f: &Field<u64>) -> String {
    format!(
        "{{\"state\":{},\"value\":{},\"note\":{},\"state_explain\":{}}}",
        json::string(f.state.as_str()),
        json::opt_u64(&f.value),
        json::opt_string(&f.note),
        json::string(f.state.explain()),
    )
}

pub fn review_to_json(r: &Review) -> String {
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
                field_u64_json(&i.amount_sats),
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
        "{{\"ok\":true,\"facts\":{{\"txid\":{},\"version\":{},\"locktime\":{},\"locktime_meaning\":{},\
\"inputs\":[{}],\"outputs\":[{}],\"total_out_sats\":{},\"fee_sats\":{},\
\"sighash_type\":{},\"sighash_policy\":{}}},\
\"context\":{},\"risks\":[{}],\"not_provided\":{},\"unsupported_keys\":{}}}",
        json::string(&f.txid),
        f.version,
        f.locktime,
        json::string(&f.locktime_meaning),
        inputs.join(","),
        outputs.join(","),
        f.total_out_sats,
        field_u64_json(&f.fee_sats),
        field_json(&f.sighash_type),
        json::string(&f.sighash_policy),
        json::strings(&r.context),
        risks.join(","),
        json::strings(&r.not_provided),
        json::strings(&r.unsupported_keys),
    )
}
