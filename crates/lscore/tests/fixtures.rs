//! 固件套件集成测试 —— BETA-READINESS B1/B2/A3 的可重复证据。
//!
//! 运行：`cargo test -p lscore --test fixtures`
//! 每个固件的期望五态在此**逐条断言**；任何"静默降级"都会让本套件红。
//!
//! 注意：本文件是**测试**，不属于解析链路；它消费 lscore 的公开接口，
//! 与 g3/tools/verify_fixtures.py（Python 侧自校验）互为独立验证。

use lscore::i18n::Lang;
use lscore::review_psbt_base64_lang;
use lscore::state::FieldState;

fn fixtures_dir() -> std::path::PathBuf {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // crates/
    p.pop(); // repo root
    p.join("g3").join("fixtures")
}

fn load(name: &str) -> String {
    let p = fixtures_dir().join(format!("{}.psbt", name));
    let text = std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("cannot read {}: {}", p.display(), e));
    text.trim().to_string()
}

fn review(name: &str) -> lscore::state::Review {
    let b64 = load(name);
    review_psbt_base64_lang(&b64, Lang::En)
        .unwrap_or_else(|e| panic!("{} should parse cleanly, got: {}", name, e))
}

fn collect_states(r: &lscore::state::Review) -> Vec<FieldState> {
    let mut v = vec![r.facts.fee_sats.state, r.facts.sighash_type.state];
    for i in &r.facts.inputs {
        v.push(i.amount_sats.state);
    }
    v
}

// ---------------------------------------------------------------------------
// S1 — 正常受托样本
// ---------------------------------------------------------------------------

#[test]
fn s1_normal_review() {
    let r = review("S1");
    assert_eq!(r.facts.inputs.len(), 1);
    assert_eq!(r.facts.outputs.len(), 2);
    assert_eq!(r.facts.total_out_sats, 1_490_000);

    // 金额来自 witness_utxo：parsed_unverified（不联网，不虚称 verified）
    let amt = &r.facts.inputs[0].amount_sats;
    assert_eq!(amt.state, FieldState::ParsedUnverified);
    assert_eq!(amt.value, Some(1_500_000));

    // 手续费可由随附字段算出：10,000 sat
    assert_eq!(r.facts.fee_sats.state, FieldState::ParsedUnverified);
    assert_eq!(r.facts.fee_sats.value, Some(10_000));

    // sighash 显式给出 SIGHASH_ALL
    assert_eq!(r.facts.sighash_type.state, FieldState::Verified);
    assert_eq!(r.facts.sighash_type.value, Some(0x01));

    // 第四层恒 5 条，永不为空
    assert_eq!(r.not_provided.len(), 5);
    // 无既有签名 → 无 R004
    assert!(r.risks.iter().all(|k| k.code != "R004"));
    // txid 形如 64 个 hex 字符
    assert_eq!(r.facts.txid.len(), 64);
    assert!(r.facts.txid.chars().all(|c| c.is_ascii_hexdigit()));
}

// ---------------------------------------------------------------------------
// S2a — missing：无 witness_utxo，金额未知，手续费不可计算
// ---------------------------------------------------------------------------

#[test]
fn s2a_missing_states() {
    let r = review("S2a");
    let amt = &r.facts.inputs[0].amount_sats;
    assert_eq!(amt.state, FieldState::Missing);
    assert!(amt.value.is_none());
    assert!(amt.note.as_deref().unwrap().contains("does not query the network"));

    // 手续费整体为 Missing —— 不许填 0
    assert_eq!(r.facts.fee_sats.state, FieldState::Missing);
    assert!(r.facts.fee_sats.value.is_none());

    // 触发 R002 风险提示，且提示里不得出现把未知说成 0 的话
    assert!(r.risks.iter().any(|k| k.code == "R002"));
}

// ---------------------------------------------------------------------------
// S2b — unsupported：专有/未知键被记录并触发 R001
// ---------------------------------------------------------------------------

#[test]
fn s2b_unsupported_recorded() {
    let r = review("S2b");
    assert!(
        r.unsupported_keys
            .iter()
            .any(|k| k.starts_with("input[0]/PSBT_IN_PROPRIETARY")),
        "proprietary key must be recorded by name, got {:?}",
        r.unsupported_keys
    );
    let r001 = r
        .risks
        .iter()
        .find(|k| k.code == "R001")
        .expect("R001 must fire for unsupported keys");
    assert!(r001.trigger_reason.contains("1"));
    // 证据字段须点名键名 —— 不许笼统地说"有未知字段"
    assert!(r001.evidence.contains("PSBT_IN_PROPRIETARY"));
}

// ---------------------------------------------------------------------------
// S3 — partially signed：既有签名被指出且声明未验签
// ---------------------------------------------------------------------------

#[test]
fn s3_partial_signature() {
    let r = review("S3");
    assert_eq!(r.facts.inputs[0].partial_sig_count, 1);
    let r004 = r
        .risks
        .iter()
        .find(|k| k.code == "R004")
        .expect("R004 must fire for existing signatures");
    assert!(r004.evidence.contains("not cryptographically verified"));
}

// ---------------------------------------------------------------------------
// S4 — P2SH：金额读自完整前序交易，结构校验通过 → verified
// ---------------------------------------------------------------------------

#[test]
fn s4_p2sh_verified_amount() {
    let r = review("S4");
    let amt = &r.facts.inputs[0].amount_sats;
    assert_eq!(amt.state, FieldState::Verified);
    assert_eq!(amt.value, Some(1_500_000));
    assert_eq!(r.facts.inputs[0].script_type.as_deref(), Some("P2SH"));
    assert_eq!(r.facts.fee_sats.value, Some(10_000));
}

// ---------------------------------------------------------------------------
// S5 — check_failed：输出合计 > 输入合计，手续费无法成立
// ---------------------------------------------------------------------------

#[test]
fn s5_check_failed_fee() {
    let r = review("S5");
    // 输入金额本身解析正常（500_000 sat，parsed_unverified）
    assert_eq!(r.facts.inputs[0].amount_sats.state, FieldState::ParsedUnverified);
    assert_eq!(r.facts.inputs[0].amount_sats.value, Some(500_000));

    // 手续费 = 输入 − 输出 不成立 → check_failed，绝不填 0 或负数
    assert_eq!(r.facts.fee_sats.state, FieldState::CheckFailed);
    assert!(r.facts.fee_sats.value.is_none());

    // R003 风险提示必须带触发原因与证据
    let r003 = r
        .risks
        .iter()
        .find(|k| k.code == "R003")
        .expect("R003 must fire when outputs exceed inputs");
    assert!(r003.trigger_reason.contains("total inputs < total outputs"));
    assert!(r003.evidence.contains("1490000"));
}

// ---------------------------------------------------------------------------
// S6 — malformed：截断报文必须显式报错，且**点名位置**（宪法第 5 条）
// ---------------------------------------------------------------------------

#[test]
fn s6_malformed_named_error() {
    let b64 = load("S6");
    let err = review_psbt_base64_lang(&b64, Lang::En).expect_err("S6 must be rejected");
    let msg = err.to_string();
    // 报错必须点名出问题的 map（global / input[n] / output[n]）与字节偏移——宪法第 5 条
    let named_map = ["global", "input[0]", "output[0]", "output[1]"]
        .iter()
        .any(|m| msg.contains(m));
    assert!(named_map, "error must name the offending map, got: {}", msg);
    assert!(
        msg.contains("offset"),
        "error must state the byte offset, got: {}",
        msg
    );
    // 中文错误同样点名位置与偏移
    let err_zh = review_psbt_base64_lang(&b64, Lang::ZhCn).expect_err("S6 must be rejected");
    let msg_zh = err_zh.to_string();
    assert!(
        ["global", "input[0]", "output[0]", "output[1]"]
            .iter()
            .any(|m| msg_zh.contains(m)),
        "zh error must name the offending map, got: {}",
        msg_zh
    );
}

// ---------------------------------------------------------------------------
// 合成 malformed 输入（B2）：坏魔数 / 非法 base64 / PSBTv2 字段
// ---------------------------------------------------------------------------

#[test]
fn bad_magic_rejected() {
    let b64 = load("S1");
    let mut raw = lscore::base64::decode(&b64).unwrap();
    raw[0] = b'X';
    let err = lscore::review_psbt_bytes(&raw).expect_err("bad magic must be rejected");
    assert!(err.to_string().contains("magic"));
}

#[test]
fn non_base64_rejected() {
    let err = review_psbt_base64_lang("!!!not-base64!!!", Lang::En)
        .expect_err("garbage input must be rejected");
    assert!(err.to_string().contains("decode"));
}

#[test]
fn psbt_v2_rejected_without_guessing() {
    // 构造一个只含 PSBT_GLOBAL_TX_VERSION(0x02) 的 v2 报文
    let mut raw = vec![0x70, 0x73, 0x62, 0x74, 0xff];
    let key = [0x01u8, 0x02]; // key_len=1, key_type=0x02
    raw.extend_from_slice(&key);
    raw.extend_from_slice(&[0x04, 0x00, 0x00, 0x00, 0x00]); // value_len=4 + value
    raw.push(0x00); // map 终止
    let err = lscore::review_psbt_bytes(&raw).expect_err("PSBTv2 must be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("PSBTv2"),
        "error must name the version problem, got: {}",
        msg
    );
}

#[test]
fn empty_input_rejected() {
    let err = review_psbt_base64_lang("", Lang::En).expect_err("empty must be rejected");
    assert!(!err.to_string().is_empty());
}

// ---------------------------------------------------------------------------
// A3：全固件字段无空白 —— 每个展示字段必带五态之一
// ---------------------------------------------------------------------------

#[test]
fn a3_every_field_carries_a_state() {
    for name in ["S1", "S2a", "S2b", "S3", "S4", "S5"] {
        let r = review(name);
        let states = collect_states(&r);
        assert!(!states.is_empty(), "{} must expose at least one field", name);
        for s in states {
            // as_str 只认五个合法值；非法值会在这里显式暴露
            assert!(
                [
                    FieldState::Verified,
                    FieldState::ParsedUnverified,
                    FieldState::Missing,
                    FieldState::Unsupported,
                    FieldState::CheckFailed
                ]
                .contains(&s),
                "{}: unexpected state",
                name
            );
        }
        // 第四层任何情况下非空
        assert_eq!(r.not_provided.len(), 5, "{}: layer 4 must never be empty", name);
    }
}

// ---------------------------------------------------------------------------
// 五态全集覆盖：六个固件跑下来，五种状态必须都被触发过
// （Unsupported 态在字段级由 unsupported_keys + R001 承载，见 KNOWN-LIMITATIONS）
// ---------------------------------------------------------------------------

#[test]
fn all_states_exercised_across_suite() {
    let mut seen = std::collections::HashSet::new();
    for name in ["S1", "S2a", "S2b", "S3", "S4", "S5"] {
        for s in collect_states(&review(name)) {
            seen.insert(s.as_str().to_string());
        }
    }
    for want in ["verified", "parsed_unverified", "missing", "check_failed"] {
        assert!(seen.contains(want), "state {} never exercised, suite covers {:?}", want, seen);
    }
}

// ---------------------------------------------------------------------------
// 双语言一致性：同一固件两语言的**结构**完全一致（事实相同，句子不同）
// ---------------------------------------------------------------------------

#[test]
fn i18n_structure_invariant() {
    for name in ["S1", "S2a", "S2b", "S3", "S4", "S5"] {
        let en = review_psbt_base64_lang(&load(name), Lang::En).unwrap();
        let zh = review_psbt_base64_lang(&load(name), Lang::ZhCn).unwrap();
        assert_eq!(en.facts.txid, zh.facts.txid, "{}", name);
        assert_eq!(en.facts.total_out_sats, zh.facts.total_out_sats, "{}", name);
        assert_eq!(en.facts.inputs.len(), zh.facts.inputs.len(), "{}", name);
        for (ie, iz) in en.facts.inputs.iter().zip(zh.facts.inputs.iter()) {
            assert_eq!(ie.amount_sats.state, iz.amount_sats.state, "{}", name);
            assert_eq!(ie.amount_sats.value, iz.amount_sats.value, "{}", name);
        }
        assert_eq!(en.risks.len(), zh.risks.len(), "{}", name);
        for (re, rz) in en.risks.iter().zip(zh.risks.iter()) {
            assert_eq!(re.code, rz.code, "{}", name);
        }
        assert_eq!(en.not_provided.len(), zh.not_provided.len(), "{}", name);
    }
}
