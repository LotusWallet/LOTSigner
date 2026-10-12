//! BIP-174 PSBT 容器解析
//!
//! 设计约束（来自产品宪法）：
//!   - 不支持的键类型必须**记录**而非丢弃，供上层标注 `Unsupported` 态；
//!   - PSBTv2 结构（无 `GLOBAL_UNSIGNED_TX`）不做推测性解析，**明确报错**；
//!   - 任何字节错位都报错返回，绝不静默降级。

use crate::tx::{read_varint, TxError};

pub const PSBT_MAGIC: [u8; 5] = [0x70, 0x73, 0x62, 0x74, 0xff]; // "psbt\xff"

// ---- 全局键类型 ----
pub const GLOBAL_UNSIGNED_TX: u64 = 0x00;
pub const GLOBAL_XPUB: u64 = 0x01;
pub const GLOBAL_PROPRIETARY: u64 = 0x0f;
/// PSBTv2 专有（出现即表明不是 v0）
pub const GLOBAL_TX_VERSION: u64 = 0x02;

// ---- 输入键类型 ----
pub const IN_NON_WITNESS_UTXO: u64 = 0x00;
pub const IN_WITNESS_UTXO: u64 = 0x01;
pub const IN_PARTIAL_SIG: u64 = 0x02;
pub const IN_SIGHASH_TYPE: u64 = 0x03;
pub const IN_REDEEM_SCRIPT: u64 = 0x04;
pub const IN_WITNESS_SCRIPT: u64 = 0x05;
pub const IN_BIP32_DERIVATION: u64 = 0x06;
pub const IN_FINAL_SCRIPTSIG: u64 = 0x07;
pub const IN_FINAL_SCRIPTWITNESS: u64 = 0x08;
pub const IN_PREVIOUS_TXID: u64 = 0x0d; // v2
pub const IN_OUTPUT_INDEX: u64 = 0x0e; // v2
pub const IN_PROPRIETARY: u64 = 0x0f;

// ---- 输出键类型 ----
pub const OUT_REDEEM_SCRIPT: u64 = 0x00;
pub const OUT_WITNESS_SCRIPT: u64 = 0x01;
pub const OUT_BIP32_DERIVATION: u64 = 0x02;
pub const OUT_AMOUNT: u64 = 0x03; // v2
pub const OUT_SCRIPT: u64 = 0x04; // v2
pub const OUT_PROPRIETARY: u64 = 0x0f;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PsbtError {
    BadMagic,
    /// 报文在某个 map 内部意外终止。带定位：哪个 map、期望字节处的偏移。
    /// 宪法第 5 条：报错必须点名出问题的位置，不得只给一句笼统的"解析失败"。
    Truncated { map: String, offset: usize },
    /// 嵌入的 unsigned tx 本身解析失败
    Tx(TxError),
    /// 出现了 PSBTv2 结构。MVP 明确不支持，且**不允许**推测性解析。
    UnsupportedVersion { key_type: u64 },
    /// 最后一个期望的 map 读完仍有残余字节
    TrailingBytes { total: usize },
}

impl PsbtError {
    /// 按语言产出一条点名问题的错误句子。所有句子出自 i18n 文案表。
    pub fn message(&self, lang: crate::i18n::Lang) -> String {
        use crate::i18n::t;
        match self {
            PsbtError::BadMagic => t(lang, "err_bad_magic", &[]),
            PsbtError::Truncated { map, offset } => {
                t(lang, "err_truncated", &[map, &offset.to_string()])
            }
            PsbtError::Tx(e) => t(lang, "err_tx_in_global", &[&e.message(lang)]),
            PsbtError::UnsupportedVersion { key_type } => {
                t(lang, "err_unsupported_version", &[&format!("{:x}", key_type)])
            }
            PsbtError::TrailingBytes { total } => {
                t(lang, "err_keycount", &[&total.to_string()])
            }
        }
    }
}

impl From<TxError> for PsbtError {
    fn from(e: TxError) -> Self {
        PsbtError::Tx(e)
    }
}

#[derive(Debug, Clone)]
pub struct Pair {
    pub key_type: u64,
    pub key_data: Vec<u8>,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct PsbtInput {
    pub pairs: Vec<Pair>,
}

#[derive(Debug, Clone)]
pub struct PsbtOutput {
    pub pairs: Vec<Pair>,
}

#[derive(Debug, Clone)]
pub struct Psbt {
    pub global_pairs: Vec<Pair>,
    pub inputs: Vec<PsbtInput>,
    pub outputs: Vec<PsbtOutput>,
    pub unsigned_tx_raw: Option<Vec<u8>>,
}

impl PsbtInput {
    pub fn get(&self, kt: u64) -> Option<&Pair> {
        self.pairs.iter().find(|p| p.key_type == kt)
    }
    pub fn count(&self, kt: u64) -> usize {
        self.pairs.iter().filter(|p| p.key_type == kt).count()
    }
}

impl PsbtOutput {
    pub fn get(&self, kt: u64) -> Option<&Pair> {
        self.pairs.iter().find(|p| p.key_type == kt)
    }
}

impl Psbt {
    pub fn global_get(&self, kt: u64) -> Option<&Pair> {
        self.global_pairs.iter().find(|p| p.key_type == kt)
    }
}

/// 读取一个 map：连续键值序列以 0x00 终止。
/// `label` 用于错误定位（"global" / "input[0]" / "output[1]"）。
fn read_map(b: &[u8], mut i: usize, label: &str) -> Result<(Vec<Pair>, usize), PsbtError> {
    let mut pairs = Vec::new();
    loop {
        if i >= b.len() {
            return Err(PsbtError::Truncated {
                map: label.to_string(),
                offset: i,
            });
        }
        let (key_len, n) = read_varint(b, i)?;
        i += n;
        if key_len == 0 {
            return Ok((pairs, i));
        }
        // key = <keytype varint><keydata>
        let (key_type, kn) = read_varint(b, i)?;
        i += kn;
        let data_len = key_len as usize - kn;
        if i + data_len > b.len() {
            return Err(PsbtError::Truncated {
                map: label.to_string(),
                offset: i,
            });
        }
        let key_data = b[i..i + data_len].to_vec();
        i += data_len;

        let (value_len, vn) = read_varint(b, i)?;
        i += vn;
        if i + value_len as usize > b.len() {
            return Err(PsbtError::Truncated {
                map: label.to_string(),
                offset: i,
            });
        }
        let value = b[i..i + value_len as usize].to_vec();
        i += value_len as usize;

        pairs.push(Pair {
            key_type,
            key_data,
            value,
        });
    }
}

pub fn parse_psbt(b: &[u8]) -> Result<Psbt, PsbtError> {
    if b.len() < PSBT_MAGIC.len() || b[..5] != PSBT_MAGIC {
        return Err(PsbtError::BadMagic);
    }
    let mut i = PSBT_MAGIC.len();

    let (global_pairs, next) = read_map(b, i, "global")?;
    i = next;

    // PSBTv2 检测：没有 GLOBAL_UNSIGNED_TX，却有 v2 专有键
    let unsigned_tx_raw = global_pairs
        .iter()
        .find(|p| p.key_type == GLOBAL_UNSIGNED_TX)
        .map(|p| p.value.clone());
    if unsigned_tx_raw.is_none() {
        if let Some(p) = global_pairs
            .iter()
            .find(|p| matches!(p.key_type, GLOBAL_TX_VERSION))
        {
            return Err(PsbtError::UnsupportedVersion {
                key_type: p.key_type,
            });
        }
        return Err(PsbtError::Truncated {
            map: "global".to_string(),
            offset: i,
        });
    }

    let unsigned_tx = unsigned_tx_raw.clone().expect("checked above");
    {
        use crate::tx::parse_unsigned_tx;
        parse_unsigned_tx(&unsigned_tx)?;
    }

    let n_in = {
        use crate::tx::parse_unsigned_tx;
        parse_unsigned_tx(&unsigned_tx)?.inputs.len()
    };
    let n_out = {
        use crate::tx::parse_unsigned_tx;
        parse_unsigned_tx(&unsigned_tx)?.outputs.len()
    };

    let mut inputs = Vec::with_capacity(n_in);
    for idx in 0..n_in {
        let label = format!("input[{}]", idx);
        let (pairs, next) = read_map(b, i, &label)?;
        i = next;
        inputs.push(PsbtInput { pairs });
    }

    let mut outputs = Vec::with_capacity(n_out);
    for idx in 0..n_out {
        let label = format!("output[{}]", idx);
        let (pairs, next) = read_map(b, i, &label)?;
        i = next;
        outputs.push(PsbtOutput { pairs });
    }

    if i != b.len() {
        return Err(PsbtError::TrailingBytes { total: b.len() });
    }

    Ok(Psbt {
        global_pairs,
        inputs,
        outputs,
        unsigned_tx_raw,
    })
}

/// 把「未知键类型」识别出来 —— 上层据此给出 `Unsupported` 五态
///
/// 判定原则（2026-10-12 修正）：**BIP-174 / BIP-371 定义的标准字段一律算"已知"**，
/// 即便本工具并不消费它（如 bip32_derivation、global_xpub）。只有**真正无法解释**
/// 的键类型才报 R001：
///   - `0x0f` proprietary：含义由第三方自定义，任何工具都无法凭空解释；
///   - 表外的数值类型：标准未定义。
///
/// 为什么必须这样修：真实协调器（Caravan）导出的 PSBT 里，每个签名者都会带一条
/// `GLOBAL_XPUB(0x01)` 与 `IN_BIP32_DERIVATION(0x06)`。若把它们算作"未知"，
/// **几乎每个真实 PSBT 都会刷出 R001**——风险提示被噪声淹没，恰好训练用户忽略告警，
/// 与"诚实有壁垒"的定位相反。此缺陷由 Caravan 真实导出物暴露。
pub fn unknown_key_type(is_input: bool, kt: u64) -> bool {
    if kt == 0x0f {
        return true; // proprietary 一律视为不支持（本工具无法解释其含义）
    }
    if is_input {
        !matches!(
            kt,
            IN_NON_WITNESS_UTXO
                | IN_WITNESS_UTXO
                | IN_PARTIAL_SIG
                | IN_SIGHASH_TYPE
                | IN_REDEEM_SCRIPT
                | IN_WITNESS_SCRIPT
                | IN_BIP32_DERIVATION
                | IN_FINAL_SCRIPTSIG
                | IN_FINAL_SCRIPTWITNESS
                | 0x09 // PSBT_IN_POR_COMMITMENT
                | 0x0a // PSBT_IN_RIPEMD160
                | 0x0b // PSBT_IN_SHA256
                | 0x0c // PSBT_IN_HASH160
                | 0x13 // PSBT_IN_TAP_KEY_SIG (BIP-371)
                | 0x14 // PSBT_IN_TAP_SCRIPT_SIG
                | 0x15 // PSBT_IN_TAP_LEAF_SCRIPT
                | 0x16 // PSBT_IN_TAP_BIP32_DERIVATION
                | 0x17 // PSBT_IN_TAP_INTERNAL_KEY
                | 0x18 // PSBT_IN_TAP_MERKLE_ROOT
        )
    } else {
        !matches!(
            kt,
            OUT_REDEEM_SCRIPT
                | OUT_WITNESS_SCRIPT
                | OUT_BIP32_DERIVATION
                | 0x05 // PSBT_OUT_TAP_INTERNAL_KEY (BIP-371)
                | 0x06 // PSBT_OUT_TAP_TREE
                | 0x07 // PSBT_OUT_TAP_BIP32_DERIVATION
        )
    }
}

/// 全局 map 的未知键判定（与输入/输出分开，避免把标准全局字段误判为不支持）
pub fn unknown_global_key_type(kt: u64) -> bool {
    !matches!(
        kt,
        GLOBAL_UNSIGNED_TX      // 0x00
            | GLOBAL_XPUB       // 0x01（BIP-174 标准：标记主密钥指纹，本工具不消费但**认得**）
            | 0x02              // PSBT_GLOBAL_TX_VERSION (v2)
            | 0x03              // PSBT_GLOBAL_FALLBACK_LOCKTIME (v2)
            | 0x04              // PSBT_GLOBAL_INPUT_COUNT (v2)
            | 0x05              // PSBT_GLOBAL_OUTPUT_COUNT (v2)
            | 0x06              // PSBT_GLOBAL_TX_MODIFIABLE (v2)
            | 0xfb // PSBT_GLOBAL_VERSION
    )
}

pub fn global_key_name(kt: u64) -> &'static str {
    match kt {
        GLOBAL_UNSIGNED_TX => "PSBT_GLOBAL_UNSIGNED_TX",
        GLOBAL_XPUB => "PSBT_GLOBAL_XPUB",
        0x02 => "PSBT_GLOBAL_TX_VERSION(v2)",
        0x03 => "PSBT_GLOBAL_FALLBACK_LOCKTIME(v2)",
        0x04 => "PSBT_GLOBAL_INPUT_COUNT(v2)",
        0x05 => "PSBT_GLOBAL_OUTPUT_COUNT(v2)",
        0x06 => "PSBT_GLOBAL_TX_MODIFIABLE(v2)",
        0xfb => "PSBT_GLOBAL_VERSION",
        GLOBAL_PROPRIETARY => "PSBT_GLOBAL_PROPRIETARY",
        _ => "PSBT_GLOBAL_UNKNOWN",
    }
}

pub fn input_key_name(kt: u64) -> &'static str {
    match kt {
        IN_NON_WITNESS_UTXO => "PSBT_IN_NON_WITNESS_UTXO",
        IN_WITNESS_UTXO => "PSBT_IN_WITNESS_UTXO",
        IN_PARTIAL_SIG => "PSBT_IN_PARTIAL_SIG",
        IN_SIGHASH_TYPE => "PSBT_IN_SIGHASH_TYPE",
        IN_REDEEM_SCRIPT => "PSBT_IN_REDEEM_SCRIPT",
        IN_WITNESS_SCRIPT => "PSBT_IN_WITNESS_SCRIPT",
        IN_BIP32_DERIVATION => "PSBT_IN_BIP32_DERIVATION",
        IN_FINAL_SCRIPTSIG => "PSBT_IN_FINAL_SCRIPTSIG",
        IN_FINAL_SCRIPTWITNESS => "PSBT_IN_FINAL_SCRIPTWITNESS",
        0x09 => "PSBT_IN_POR_COMMITMENT",
        0x0a => "PSBT_IN_RIPEMD160",
        0x0b => "PSBT_IN_SHA256",
        0x0c => "PSBT_IN_HASH160",
        IN_PREVIOUS_TXID => "PSBT_IN_PREVIOUS_TXID(v2)",
        IN_OUTPUT_INDEX => "PSBT_IN_OUTPUT_INDEX(v2)",
        0x13 => "PSBT_IN_TAP_KEY_SIG",
        0x14 => "PSBT_IN_TAP_SCRIPT_SIG",
        0x15 => "PSBT_IN_TAP_LEAF_SCRIPT",
        0x16 => "PSBT_IN_TAP_BIP32_DERIVATION",
        0x17 => "PSBT_IN_TAP_INTERNAL_KEY",
        0x18 => "PSBT_IN_TAP_MERKLE_ROOT",
        IN_PROPRIETARY => "PSBT_IN_PROPRIETARY",
        _ => "PSBT_IN_UNKNOWN",
    }
}

pub fn output_key_name(kt: u64) -> &'static str {
    match kt {
        OUT_REDEEM_SCRIPT => "PSBT_OUT_REDEEM_SCRIPT",
        OUT_WITNESS_SCRIPT => "PSBT_OUT_WITNESS_SCRIPT",
        OUT_BIP32_DERIVATION => "PSBT_OUT_BIP32_DERIVATION",
        OUT_AMOUNT => "PSBT_OUT_AMOUNT(v2)",
        OUT_SCRIPT => "PSBT_OUT_SCRIPT(v2)",
        0x05 => "PSBT_OUT_TAP_INTERNAL_KEY",
        0x06 => "PSBT_OUT_TAP_TREE",
        0x07 => "PSBT_OUT_TAP_BIP32_DERIVATION",
        OUT_PROPRIETARY => "PSBT_OUT_PROPRIETARY",
        _ => "PSBT_OUT_UNKNOWN",
    }
}
