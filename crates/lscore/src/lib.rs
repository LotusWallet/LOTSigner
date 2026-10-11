//! Lotus Signer · 权威内核
//!
//! **这是唯一的实现。** 网页端与命令行端都由它提供结果，禁止各自重写解析或判定逻辑。
//! 第三方 PSBT 库（如 `@caravan/psbt` / `rust-bitcoin`）只允许出现在对照测试中，不得进入此解析链路。
//! 所有面向用户的句子出自 `i18n` 文案表——加语言只改那一处（宪法第 8 条的延伸）。

pub mod base64;
pub mod i18n;
pub mod psbt;
pub mod script;
pub mod state;
pub mod tx;

use crate::i18n::{t, Lang};
use crate::psbt::*;
use crate::script::{classify, script_to_address, script_summary, ScriptType};
use crate::state::*;
use crate::tx::{hex, parse_unsigned_tx, txid_hex, UnsignedTx};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewError {
    Decode(String),
    Parse(String),
}

impl core::fmt::Display for ReviewError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ReviewError::Decode(e) => write!(f, "{}", t(Lang::En, "err_decode", &[e])),
            ReviewError::Parse(e) => write!(f, "{}", t(Lang::En, "err_parse", &[e])),
        }
    }
}

/// 审阅入口：吃 base64 文本，吐四层结构（英文，默认语言）。
pub fn review_psbt_base64(input: &str) -> Result<Review, ReviewError> {
    review_psbt_base64_lang(input, Lang::En)
}

pub fn review_psbt_base64_lang(input: &str, lang: Lang) -> Result<Review, ReviewError> {
    let raw = crate::base64::decode(input)
        .map_err(|e| ReviewError::Decode(t(lang, "err_decode", &[&format!("{:?}", e)])))?;
    review_psbt_bytes_lang(&raw, lang)
}

pub fn review_psbt_bytes(raw: &[u8]) -> Result<Review, ReviewError> {
    review_psbt_bytes_lang(raw, Lang::En)
}

pub fn review_psbt_bytes_lang(raw: &[u8], lang: Lang) -> Result<Review, ReviewError> {
    let psbt = psbt::parse_psbt(raw)
        .map_err(|e| ReviewError::Parse(t(lang, "err_parse", &[&format!("{:?}", e)])))?;
    let raw_tx = psbt.unsigned_tx_raw.as_ref().ok_or_else(|| {
        ReviewError::Parse(t(lang, "err_no_global_tx", &[]))
    })?;
    let utx = parse_unsigned_tx(raw_tx)
        .map_err(|e| ReviewError::Parse(t(lang, "err_parse", &[&format!("{:?}", e)])))?;
    Ok(build_review(&psbt, &utx, lang))
}

fn build_review(psbt: &Psbt, utx: &UnsignedTx, lang: Lang) -> Review {
    let mut unsupported_keys: Vec<String> = Vec::new();
    let mut risks: Vec<RiskNote> = Vec::new();

    // ---------- 输入 ----------
    let mut inputs = Vec::new();
    let mut input_amounts: Vec<Option<u64>> = Vec::new();
    for (i, txin) in utx.inputs.iter().enumerate() {
        let p_in = psbt.inputs.get(i);
        let (has_w, has_nw) = match p_in {
            Some(p) => (
                p.get(IN_WITNESS_UTXO).is_some(),
                p.get(IN_NON_WITNESS_UTXO).is_some(),
            ),
            None => (false, false),
        };

        let script_type = match p_in {
            Some(p) => {
                let has_rs = p.get(IN_REDEEM_SCRIPT).is_some();
                let has_ws = p.get(IN_WITNESS_SCRIPT).is_some();
                Some(match (has_rs, has_ws) {
                    (true, true) => "P2SH-P2WSH".to_string(),
                    (false, true) => "P2WSH".to_string(),
                    (true, false) => "P2SH".to_string(),
                    (false, false) => t(lang, "unlabelled", &[]),
                })
            }
            None => None,
        };

        let amount_sats = match p_in {
            Some(p) => {
                if let Some(w) = p.get(IN_WITNESS_UTXO) {
                    if w.value.len() >= 8 {
                        let v = u64::from_le_bytes(w.value[0..8].try_into().unwrap());
                        Field::parsed_unverified(
                            v,
                            t(lang, "note_amount_witness", &[]),
                        )
                    } else {
                        Field::check_failed(t(lang, "check_witness_len", &[]))
                    }
                } else if let Some(nw) = p.get(IN_NON_WITNESS_UTXO) {
                    match parse_prev_output_value(&nw.value, txin.prev_vout, lang) {
                        Ok(v) => Field::with_state(v, FieldState::Verified, t(lang, "note_amount_nw", &[])),
                        Err(e) => Field::check_failed(e),
                    }
                } else {
                    Field::missing(t(lang, "missing_amount_no_utxo", &[]))
                }
            }
            None => Field::missing(t(lang, "missing_input_map", &[])),
        };

        input_amounts.push(amount_sats.value);

        let partial_sig_count = p_in.map(|p| p.count(IN_PARTIAL_SIG)).unwrap_or(0);

        inputs.push(InputFact {
            index: i,
            outpoint: format!("{}:{}", reverse_hex(&txin.prev_txid), txin.prev_vout),
            sequence: txin.sequence,
            script_type,
            amount_sats,
            partial_sig_count,
            has_non_witness_utxo: has_nw,
            has_witness_utxo: has_w,
        });
    }

    // ---------- 未知键 ----------
    for p in psbt.global_pairs.iter() {
        if p.key_type == GLOBAL_XPUB || p.key_type == GLOBAL_PROPRIETARY {
            unsupported_keys.push(format!("global/{}", crate::psbt::input_key_name(p.key_type)));
        }
    }
    for (i, p) in psbt.inputs.iter().enumerate() {
        for pair in p.pairs.iter() {
            if unknown_key_type(true, pair.key_type) {
                unsupported_keys.push(format!("input[{}]/{}", i, input_key_name(pair.key_type)));
            }
        }
    }
    for (i, p) in psbt.outputs.iter().enumerate() {
        for pair in p.pairs.iter() {
            if unknown_key_type(false, pair.key_type) {
                unsupported_keys.push(format!("output[{}]/{}", i, output_key_name(pair.key_type)));
            }
        }
    }
    if !unsupported_keys.is_empty() {
        risks.push(RiskNote {
            code: "R001",
            summary: t(lang, "r001_summary", &[]),
            trigger_reason: t(lang, "r001_trigger", &[&unsupported_keys.len().to_string()]),
            evidence: unsupported_keys.join(", "),
        });
    }

    // ---------- 输出 ----------
    let mut outputs = Vec::new();
    let mut total_out: u64 = 0;
    for (i, txout) in utx.outputs.iter().enumerate() {
        total_out = total_out.saturating_add(txout.value_sats);
        outputs.push(OutputFact {
            index: i,
            value_sats: txout.value_sats,
            script_type: classify(&txout.script_pubkey).name().to_string(),
            address: script_to_address(&txout.script_pubkey, true),
            script_hex: script_summary(&txout.script_pubkey),
        });
    }

    // ---------- 手续费（诚实标注的范例） ----------
    let fee_sats = if input_amounts.iter().any(|a| a.is_none()) {
        Field::missing(t(lang, "missing_fee", &[]))
    } else {
        let total_in: u64 = input_amounts.iter().map(|a| a.unwrap_or(0)).sum();
        match total_in.checked_sub(total_out) {
            Some(fee) => Field::parsed_unverified(fee, t(lang, "note_fee", &[])),
            None => Field::check_failed(t(lang, "check_out_exceed_in", &[])),
        }
    };

    if fee_sats.state == FieldState::Missing {
        let states = input_amounts
            .iter()
            .enumerate()
            .map(|(i, a)| match a {
                Some(v) => t(lang, "in_amount", &[&i.to_string(), &v.to_string()]),
                None => t(lang, "in_unknown", &[&i.to_string()]),
            })
            .collect::<Vec<_>>()
            .join(", ");
        risks.push(RiskNote {
            code: "R002",
            summary: t(lang, "r002_summary", &[]),
            trigger_reason: t(lang, "r002_trigger", &[]),
            evidence: t(lang, "r002_evidence", &[&states]),
        });
    }
    if fee_sats.state == FieldState::CheckFailed {
        risks.push(RiskNote {
            code: "R003",
            summary: t(lang, "r003_summary", &[]),
            trigger_reason: t(lang, "r003_trigger", &[]),
            evidence: t(lang, "r003_evidence", &[&total_out.to_string()]),
        });
    }

    // ---------- 既有签名 ----------
    for inp in inputs.iter() {
        if inp.partial_sig_count > 0 {
            risks.push(RiskNote {
                code: "R004",
                summary: t(lang, "r004_summary", &[]),
                trigger_reason: t(lang, "r004_trigger", &[&inp.index.to_string(), &inp.partial_sig_count.to_string()]),
                evidence: t(lang, "r004_evidence", &[]),
            });
        }
    }

    // ---------- 未知脚本形态 ----------
    for o in outputs.iter() {
        if o.script_type == ScriptType::Unknown.name() {
            risks.push(RiskNote {
                code: "R005",
                summary: t(lang, "r005_summary", &[]),
                trigger_reason: t(lang, "r005_trigger", &[&o.index.to_string()]),
                evidence: o.script_hex.clone(),
            });
        }
    }

    // ---------- 第二层：上下文解释 ----------
    let locktime_meaning = if utx.locktime == 0 {
        t(lang, "locktime_0", &[])
    } else if utx.locktime < 500_000_000 {
        t(lang, "locktime_block", &[&utx.locktime.to_string()])
    } else {
        t(lang, "locktime_time", &[&utx.locktime.to_string()])
    };

    let mut context = Vec::new();
    context.push(t(lang, "ctx_summary", &[
        &inputs.len().to_string(),
        &outputs.len().to_string(),
        &total_out.to_string(),
    ]));
    context.push(t(lang, "ctx_locktime", &[&utx.locktime.to_string(), &locktime_meaning]));
    context.push(t(lang, "ctx_draft", &[]));
    if inputs.iter().all(|i| i.partial_sig_count == 0) {
        context.push(t(lang, "ctx_first_signer", &[]));
    }

    Review {
        lang,
        facts: TransactionFacts {
            txid: txid_hex(utx),
            version: utx.version,
            locktime: utx.locktime,
            locktime_meaning,
            inputs,
            outputs,
            total_out_sats: total_out,
            fee_sats,
            sighash_type: sighash_field(psbt, lang),
            sighash_policy: sighash_policy_text(psbt, lang),
        },
        context,
        risks,
        not_provided: not_provided_list(lang),
        unsupported_keys,
    }
}

/// 第四层的固定内容，任何情况下都不得为空。
pub fn not_provided_list(lang: Lang) -> Vec<String> {
    ["np1", "np2", "np3", "np4", "np5"]
        .iter()
        .map(|k| t(lang, k, &[]))
        .collect()
}

fn sighash_field(psbt: &Psbt, lang: Lang) -> Field<u32> {
    match psbt.inputs.first().and_then(|p| p.get(IN_SIGHASH_TYPE)) {
        Some(p) if p.value.len() == 4 => {
            Field::verified(u32::from_le_bytes(p.value[0..4].try_into().unwrap()))
        }
        Some(_) => Field::check_failed(t(lang, "check_sighash_len", &[])),
        None => Field::missing(t(lang, "missing_sighash", &[])),
    }
}

fn sighash_policy_text(psbt: &Psbt, lang: Lang) -> String {
    match sighash_field(psbt, lang).value {
        Some(0x01) => t(lang, "policy_all", &[]),
        Some(0x02) => t(lang, "policy_none", &[]),
        Some(0x03) => t(lang, "policy_single", &[]),
        Some(0x81) => t(lang, "policy_acp_all", &[]),
        Some(0x82) => t(lang, "policy_acp_none", &[]),
        Some(0x83) => t(lang, "policy_acp_single", &[]),
        Some(v) => t(lang, "policy_unknown", &[&format!("0x{:02x}", v)]),
        None => t(lang, "policy_unspecified", &[]),
    }
}

fn reverse_hex(b: &[u8; 32]) -> String {
    let mut v = b.to_vec();
    v.reverse();
    hex(&v)
}

/// 从前序交易报文中取出第 vout 个输出的金额
fn parse_prev_output_value(prev_tx: &[u8], vout: u32, lang: Lang) -> Result<u64, String> {
    let prev = parse_unsigned_tx(prev_tx)
        .map_err(|e| t(lang, "err_prev_parse", &[&format!("{:?}", e)]))?;
    if vout as usize >= prev.outputs.len() {
        return Err(t(lang, "err_prev_out_range", &[&vout.to_string()]));
    }
    Ok(prev.outputs[vout as usize].value_sats)
}
