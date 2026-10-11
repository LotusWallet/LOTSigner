//! Lotus Signer · 权威内核
//!
//! **这是唯一的实现。** 网页端与命令行端都由它提供结果，禁止各自重写解析或判定逻辑。
//! 第三方 PSBT 库（如 `@caravan/psbt` / `rust-bitcoin`）只允许出现在对照测试中，不得进入此解析链路。

pub mod base64;
pub mod psbt;
pub mod script;
pub mod state;
pub mod tx;

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
            ReviewError::Decode(e) => write!(f, "输入解码失败：{}", e),
            ReviewError::Parse(e) => write!(f, "PSBT 解析失败：{}", e),
        }
    }
}

/// 审阅入口：吃 base64 文本（或裸路径不重要，调用方负责读文件），吐四层结构。
pub fn review_psbt_base64(input: &str) -> Result<Review, ReviewError> {
    let raw = crate::base64::decode(input).map_err(|e| ReviewError::Decode(format!("{:?}", e)))?;
    review_psbt_bytes(&raw)
}

pub fn review_psbt_bytes(raw: &[u8]) -> Result<Review, ReviewError> {
    let psbt = psbt::parse_psbt(raw).map_err(|e| ReviewError::Parse(format!("{:?}", e)))?;
    let raw_tx = psbt.unsigned_tx_raw.as_ref().ok_or_else(|| {
        ReviewError::Parse("缺少 PSBT_GLOBAL_UNSIGNED_TX，本版本暂不支持 PSBTv2".to_string())
    })?;
    let utx = parse_unsigned_tx(raw_tx).map_err(|e| ReviewError::Parse(format!("{:?}", e)))?;
    Ok(build_review(&psbt, &utx))
}

fn build_review(psbt: &Psbt, utx: &UnsignedTx) -> Review {
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
                    (false, false) => "未标注脚本类型".to_string(),
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
                            "该金额来自 PSBT 随附的 witness_utxo 字段；\
                             本工具不联网，无法核实它是否与链上前序交易一致",
                        )
                    } else {
                        Field::check_failed("witness_utxo 长度不足 8 字节")
                    }
                } else if let Some(nw) = p.get(IN_NON_WITNESS_UTXO) {
                    match parse_prev_output_value(&nw.value, txin.prev_vout) {
                        Ok(v) => Field::verified(v),
                        Err(e) => Field::check_failed(e),
                    }
                } else {
                    Field::missing(
                        "PSBT 未随附 witness_utxo 或 non_witness_utxo，\
                         且本工具不联网查询，故输入金额未知",
                    )
                }
            }
            None => Field::missing("该输入无对应的 PSBT 输入映射"),
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
            summary: "报文中含有本工具无法解释的字段".to_string(),
            trigger_reason: format!("检出 {} 个未知/专有键", unsupported_keys.len()),
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
        Field::missing(
            "至少一个输入金额未知（PSBT 未随附前序交易信息且本工具不联网查询），\
             因此手续费无法计算 —— 本工具不会把它当作 0，也不会用推测值填补",
        )
    } else {
        let total_in: u64 = input_amounts.iter().map(|a| a.unwrap_or(0)).sum();
        match total_in.checked_sub(total_out) {
            Some(fee) => Field::parsed_unverified(
                fee,
                "手续费 = 输入合计 - 输出合计；输入金额来自 PSBT 随附字段，未经链上核实",
            ),
            None => Field::check_failed("输出合计超过输入合计，这笔交易在数额上不成立"),
        }
    };

    if fee_sats.state == FieldState::Missing {
        risks.push(RiskNote {
            code: "R002",
            summary: "手续费未知".to_string(),
            trigger_reason: "至少一个输入未提供 witness_utxo / non_witness_utxo，且本工具不联网"
                .to_string(),
            evidence: format!(
                "输入金额状态：{}",
                input_amounts
                    .iter()
                    .enumerate()
                    .map(|(i, a)| match a {
                        Some(v) => format!("in[{}]={} sat", i, v),
                        None => format!("in[{}]=未知", i),
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        });
    }
    if fee_sats.state == FieldState::CheckFailed {
        risks.push(RiskNote {
            code: "R003",
            summary: "输出数额超过输入数额".to_string(),
            trigger_reason: "输入合计 < 输出合计".to_string(),
            evidence: format!("输出合计 = {} sat", total_out),
        });
    }

    // ---------- 既有签名 ----------
    for inp in inputs.iter() {
        if inp.partial_sig_count > 0 {
            risks.push(RiskNote {
                code: "R004",
                summary: "该输入已包含既有签名".to_string(),
                trigger_reason: format!("input[{}] 检出 {} 个 partial_sig", inp.index, inp.partial_sig_count),
                evidence: "签名本身未经密码学核验（本工具不接触或不推导密钥，也不做验签）".to_string(),
            });
        }
    }

    // ---------- 未知脚本形态 ----------
    for o in outputs.iter() {
        if o.script_type == ScriptType::Unknown.name() {
            risks.push(RiskNote {
                code: "R005",
                summary: "存在无法识别的输出脚本形态".to_string(),
                trigger_reason: format!(
                    "output[{}] 的 scriptPubKey 未匹配任何已知标准形态",
                    o.index
                ),
                evidence: o.script_hex.clone(),
            });
        }
    }

    // ---------- 第二层：上下文解释 ----------
    let locktime_meaning = if utx.locktime == 0 {
        "0：不使用时间锁，交易可被立即打包".to_string()
    } else if utx.locktime < 500_000_000 {
        format!("{}：区块高度锁 —— 在该区块高度之前不会被打包", utx.locktime)
    } else {
        format!("{}：时间戳锁 —— 在该 UNIX 时间之后才可被打包", utx.locktime)
    };

    let mut context = Vec::new();
    context.push(format!(
        "这是一笔待签名的 PSBT：{} 个输入、{} 个输出，输出合计 {} sat。",
        inputs.len(),
        outputs.len(),
        total_out
    ));
    context.push(format!("nLockTime = {}（{}）", utx.locktime, locktime_meaning));
    context.push(
        "PSBT 只是待签名的草稿：其中的数值来自递交方随附的字段，\
         在不联网核实前序交易的前提下，本工具无法保证它们与链上一致。"
            .to_string(),
    );
    if inputs.iter().all(|i| i.partial_sig_count == 0) {
        context.push("当前没有任何既有签名 —— 你可能是第一个签名者。".to_string());
    }

    Review {
        facts: TransactionFacts {
            txid: txid_hex(utx),
            version: utx.version,
            locktime: utx.locktime,
            locktime_meaning,
            inputs,
            outputs,
            total_out_sats: total_out,
            fee_sats,
            sighash_type: sighash_field(psbt),
            sighash_policy: sighash_policy_text(psbt),
        },
        context,
        risks,
        not_provided: Review::default_not_provided(),
        unsupported_keys,
    }
}

fn sighash_field(psbt: &Psbt) -> Field<u32> {
    match psbt.inputs.first().and_then(|p| p.get(IN_SIGHASH_TYPE)) {
        Some(p) if p.value.len() == 4 => {
            Field::verified(u32::from_le_bytes(p.value[0..4].try_into().unwrap()))
        }
        Some(_) => Field::check_failed("sighash_type 字段长度不为 4 字节"),
        None => Field::missing("PSBT 未标注 sighash 类型；默认按 SIGHASH_ALL 解释，但这是猜测——本工具不作自动填补"),
    }
}

fn sighash_policy_text(psbt: &Psbt) -> String {
    match sighash_field(psbt).value {
        Some(0x01) => "SIGHASH_ALL：签名覆盖全部输入与输出".to_string(),
        Some(0x02) => "SIGHASH_NONE：签名不覆盖任何输出（他人可改写收款方）".to_string(),
        Some(0x03) => "SIGHASH_SINGLE：签名只覆盖同序号输出".to_string(),
        Some(0x81) => "SIGHASH_ALL | ANYONECANPAY：他人可追加输入".to_string(),
        Some(0x82) => "SIGHASH_NONE | ANYONECANPAY".to_string(),
        Some(0x83) => "SIGHASH_SINGLE | ANYONECANPAY".to_string(),
        Some(v) => format!("未知 sighash 值 0x{:02x}", v),
        None => "未标注".to_string(),
    }
}

fn reverse_hex(b: &[u8; 32]) -> String {
    let mut v = b.to_vec();
    v.reverse();
    hex(&v)
}

/// 从前序交易报文中取出第 vout 个输出的金额
fn parse_prev_output_value(prev_tx: &[u8], vout: u32) -> Result<u64, String> {
    let prev = parse_unsigned_tx(prev_tx).map_err(|e| format!("前序交易解析失败：{:?}", e))?;
    if vout as usize >= prev.outputs.len() {
        return Err(format!("前序交易的第 {} 个输出不存在", vout));
    }
    Ok(prev.outputs[vout as usize].value_sats)
}
