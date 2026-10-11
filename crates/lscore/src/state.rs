//! 五态模型与四层输出 —— 本产品的核心数据契约
//!
//! **五态**（字段的证据状态）与**四层**（给用户的四类信息）不可互替，两者都进数据模型而非只存在于界面上。
//! 这是宪法第 6、7 条的直接落实。


/// 字段的证据状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldState {
    /// 通过**明确定义的**解析与校验规则
    Verified,
    /// 已从报文中取出，但其真实性未经验证（例如签名未经密钥核验）
    ParsedUnverified,
    /// 报文中不存在该字段
    Missing,
    /// 报文中存在，但本工具无法解释其语义
    Unsupported,
    /// 存在且已知其类型，但未能通过校验
    CheckFailed,
}

impl FieldState {
    pub fn as_str(self) -> &'static str {
        match self {
            FieldState::Verified => "verified",
            FieldState::ParsedUnverified => "parsed_unverified",
            FieldState::Missing => "missing",
            FieldState::Unsupported => "unsupported",
            FieldState::CheckFailed => "check_failed",
        }
    }

    /// 面向普通用户的一句解释。刻意避免"安全/危险"措辞。
    pub fn explain(self) -> &'static str {
        match self {
            FieldState::Verified => "已按明确规则解析并通过结构校验",
            FieldState::ParsedUnverified => "已从报文中读出，但其内容正确性未在此验证",
            FieldState::Missing => "报文中未提供此项，本工具不作推测",
            FieldState::Unsupported => "报文中存在但本工具无法解释此项含义",
            FieldState::CheckFailed => "此项存在但未能通过校验",
        }
    }
}

/// 带五态的字段容器
#[derive(Debug, Clone)]
pub struct Field<T> {
    pub state: FieldState,
    pub value: Option<T>,
    pub note: Option<String>,
}

impl<T> Field<T> {
    pub fn missing(note: impl Into<String>) -> Self {
        Field { state: FieldState::Missing, value: None, note: Some(note.into()) }
    }
    pub fn verified(v: T) -> Self {
        Field { state: FieldState::Verified, value: Some(v), note: None }
    }
    pub fn parsed_unverified(v: T, note: impl Into<String>) -> Self {
        Field { state: FieldState::ParsedUnverified, value: Some(v), note: Some(note.into()) }
    }
    pub fn unsupported(note: impl Into<String>) -> Self {
        Field { state: FieldState::Unsupported, value: None, note: Some(note.into()) }
    }
    pub fn check_failed(note: impl Into<String>) -> Self {
        Field { state: FieldState::CheckFailed, value: None, note: Some(note.into()) }
    }
    pub fn with_state(v: T, s: FieldState, note: impl Into<String>) -> Self {
        Field { state: s, value: Some(v), note: Some(note.into()) }
    }
}

// ---------------- 第一层：交易事实 ----------------

#[derive(Debug, Clone)]
pub struct InputFact {
    pub index: usize,
    pub outpoint: String,
    pub sequence: u32,
    pub script_type: Option<String>,
    /// 输入金额。PSBT 的前序交易信息缺失时即为 Missing —— 这是诚实标注的关键场景。
    pub amount_sats: Field<u64>,
    pub partial_sig_count: usize,
    pub has_non_witness_utxo: bool,
    pub has_witness_utxo: bool,
}

#[derive(Debug, Clone)]
pub struct OutputFact {
    pub index: usize,
    pub value_sats: u64,
    pub script_type: String,
    pub address: Option<String>,
    pub script_hex: String,
}

#[derive(Debug, Clone)]
pub struct TransactionFacts {
    pub txid: String,
    pub version: i32,
    pub locktime: u32,
    /// nLockTime 是否实际生效（< 500_000_000 视为区块高度，否则时间戳）
    pub locktime_meaning: String,
    pub inputs: Vec<InputFact>,
    pub outputs: Vec<OutputFact>,
    pub total_out_sats: u64,
    /// 手续费。任一输入金额缺失即整体为 Missing —— 不允许用 0 或将会计差异抹平。
    pub fee_sats: Field<u64>,
    pub sighash_type: Field<u32>,
    /// sighash 策略的含义说明（第二层内容，但依附于该字段，避免 UI 各自解释）
    pub sighash_policy: String,
}

// ---------------- 第三层：风险提示（须展示触发原因） ----------------

#[derive(Debug, Clone)]
pub struct RiskNote {
    pub code: &'static str,
    pub summary: String,
    pub trigger_reason: String,
    pub evidence: String,
}

// ---------------- 总输出容器 ----------------

#[derive(Debug, Clone)]
pub struct Review {
    /// 第一层：交易事实
    pub facts: TransactionFacts,
    /// 第二层：上下文解释
    pub context: Vec<String>,
    /// 第三层：风险提示（每条必带触发原因）
    pub risks: Vec<RiskNote>,
    /// 第四层：本工具明确不提供的结论
    pub not_provided: Vec<String>,
    /// 未知/专有键清单
    pub unsupported_keys: Vec<String>,
}

impl Review {
    /// 第四层的固定内容，任何情况下都不得为空。
    pub fn default_not_provided() -> Vec<String> {
        vec![
            "本工具不判断这笔交易「安全」或「危险」。".to_string(),
            "本工具不知道交易对手方是谁，无法判断收款方是否可信。".to_string(),
            "本工具不联网，因此无法核实前序交易是否存在、是否已被花费。".to_string(),
            "本工具不接触私钥，也无法判断你是否有权签署这笔交易。".to_string(),
            "本工具不提供价格、税务或合规意见。".to_string(),
        ]
    }
}
