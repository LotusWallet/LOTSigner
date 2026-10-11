//! 内核文案表（i18n）——**所有面向用户的句子都从这里出**
//!
//! 架构决策（宪法第 8 条的延伸）：风险提示、五态解释、上下文解释都是**权威语义的一部分**，
//! 因此文案表放在内核，CLI 与网页共用同一份——加一种语言 = 在本文件加一个 match 分支，
//! 两端自动同步，前端**永不持有**语义文案。
//!
//! 第一期：EN（默认）+ zh-CN。日语/韩语/法语/泰语的扩展位已就绪（加一个 Lang 变体 + 两个 match），
//! 但**在未经母语者校对前不加入**——风险提示的错误翻译等于误导用户。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    ZhCn,
}

impl Lang {
    /// 从语言标签解析（"en"/"en-US"→En，"zh"/"zh-CN"/"zh-Hans"→ZhCn）。未知一律回退 En。
    pub fn parse(s: &str) -> Lang {
        let l = s.trim().to_ascii_lowercase();
        if l.starts_with("zh") {
            Lang::ZhCn
        } else {
            Lang::En
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::ZhCn => "zh-CN",
        }
    }
}

/// 取文案并做 {0} {1} … 插值。未知 key 原样返回（开发期立刻可见，不静默吞掉）。
pub fn t(lang: Lang, key: &str, args: &[&str]) -> String {
    let raw: &str = match (lang, key) {
        // ---------------- UI 壳 ----------------
        (Lang::En, "app_title") => "Lotus Signer",
        (Lang::En, "tagline") => "Before you sign what someone else handed you, run it through Lotus Signer.",
        (Lang::En, "offline_badge") => "This page issues no external requests · runs fully offline",
        (Lang::En, "drop_title") => "Drop the PSBT you were asked to sign",
        (Lang::En, "drop_sub") => "or click to choose a file — a base64 .psbt text. The file never leaves this browser.",
        (Lang::En, "input_placeholder") => "…or paste base64 PSBT text directly here",
        (Lang::En, "btn_review") => "Review",
        (Lang::En, "btn_demo") => "Load sample",
        (Lang::En, "hint_ready") => "Core ready",
        (Lang::En, "hint_wasm_fail") => "Core load failed: {0}",
        (Lang::En, "hint_demo_fail") => "Sample failed to load",
        (Lang::En, "layer1") => "Transaction facts",
        (Lang::En, "layer2") => "Context",
        (Lang::En, "layer3") => "Risk notices",
        (Lang::En, "layer4") => "What this tool does NOT tell you",
        (Lang::En, "footer1") => "This is an evidence-based review. It is not a recommendation for anyone to sign or not to sign.",
        (Lang::En, "footer2") => "All parsing and judgement comes from the single Rust core; this page contains no parsing logic.",
        (Lang::En, "th_index") => "#",
        (Lang::En, "th_outpoint") => "Outpoint",
        (Lang::En, "th_type") => "Type",
        (Lang::En, "th_amount") => "Amount",
        (Lang::En, "th_sigs") => "Sigs",
        (Lang::En, "th_value") => "Value",
        (Lang::En, "th_script_addr") => "Recipient script / address",
        (Lang::En, "dt_txid") => "txid",
        (Lang::En, "dt_version") => "version",
        (Lang::En, "dt_locktime") => "nLockTime",
        (Lang::En, "dt_total_out") => "Total output",
        (Lang::En, "dt_fee") => "Fee",
        (Lang::En, "dt_sighash") => "sighash",
        (Lang::En, "unknown") => "unknown",
        (Lang::En, "unlabelled") => "script type not labelled",
        (Lang::En, "not_provided_label") => "—",
        (Lang::En, "no_sigs") => "—",
        (Lang::En, "sig_count") => "{0} sig(s) (unverified)",
        (Lang::En, "cannot_show_addr") => "cannot render an address ({0})",
        (Lang::En, "empty_risks") => "No known rule triggered this time. This does NOT mean \"safe\" — it only means nothing known was triggered.",
        (Lang::En, "err_review_fail") => "Review failed: {0}",
        (Lang::En, "err_core_fail") => "Core call failed: {0}",

        // ---------------- CLI ----------------
        (Lang::En, "cli_usage") => "Usage: lscli [--lang en|zh] <PSBT file>",
        (Lang::En, "cli_example") => "Example: lscli ../../g3/fixtures/S1.psbt",
        (Lang::En, "cli_read_fail") => "Cannot read file: {0} — {1}",
        (Lang::En, "cli_banner") => "Lotus Signer · entrusted-signature review",
        (Lang::En, "cli_inputs") => "Inputs",
        (Lang::En, "cli_outputs") => "Outputs",
        (Lang::En, "cli_total") => "Total output",
        (Lang::En, "cli_sig_note") => "{0} existing signature(s) (not cryptographically verified)",
        (Lang::En, "r_trigger") => "Trigger:",
        (Lang::En, "r_evidence") => "Evidence:",

        // ---------------- 内核：错误 ----------------
        (Lang::En, "err_decode") => "input decode failed: {0}",
        (Lang::En, "err_parse") => "PSBT parse failed: {0}",
        (Lang::En, "err_no_global_tx") => "PSBT_GLOBAL_UNSIGNED_TX missing; PSBTv2 is not supported in this version",
        (Lang::En, "err_prev_parse") => "previous transaction parse failed: {0}",
        (Lang::En, "err_prev_out_range") => "output #{0} does not exist in the previous transaction",
        (Lang::En, "check_witness_len") => "witness_utxo is shorter than 8 bytes",
        (Lang::En, "check_out_exceed_in") => "outputs exceed inputs in total value; the amounts in this transaction do not add up",
        (Lang::En, "check_sighash_len") => "sighash_type field is not 4 bytes long",

        // ---------------- 内核：字段 note ----------------
        (Lang::En, "note_amount_witness") => "This amount comes from the witness_utxo field attached to the PSBT. This tool does not go online, so it cannot confirm it matches the chain.",
        (Lang::En, "note_amount_nw") => "This amount was read out of the previous transaction attached to the PSBT (non_witness_utxo). Its structure checks out; existence on chain is not confirmed offline.",
        (Lang::En, "missing_amount_no_utxo") => "The PSBT attached neither witness_utxo nor non_witness_utxo for this input, and this tool does not query the network — so the input amount is unknown.",
        (Lang::En, "missing_input_map") => "this input has no matching PSBT input map",
        (Lang::En, "note_fee") => "Fee = total inputs − total outputs. Input amounts come from fields attached to the PSBT and were not verified against the chain.",
        (Lang::En, "missing_fee") => "At least one input amount is unknown (no previous-transaction data attached and this tool does not go online), so the fee cannot be computed. This tool will not treat it as 0 or fill in a guess.",
        (Lang::En, "missing_sighash") => "The PSBT does not state a sighash type; interpreting it as SIGHASH_ALL would be a guess — this tool does not fill guesses in.",

        // ---------------- 内核：sighash 策略 ----------------
        (Lang::En, "policy_all") => "SIGHASH_ALL: the signature covers every input and every output.",
        (Lang::En, "policy_none") => "SIGHASH_NONE: the signature covers no outputs — others may redirect the recipients.",
        (Lang::En, "policy_single") => "SIGHASH_SINGLE: the signature covers only the output with the same index.",
        (Lang::En, "policy_acp_all") => "SIGHASH_ALL | ANYONECANPAY: others may add more inputs.",
        (Lang::En, "policy_acp_none") => "SIGHASH_NONE | ANYONECANPAY: no outputs covered; others may add inputs.",
        (Lang::En, "policy_acp_single") => "SIGHASH_SINGLE | ANYONECANPAY: only the same-index output is covered; others may add inputs.",
        (Lang::En, "policy_unknown") => "unknown sighash value {0}",
        (Lang::En, "policy_unspecified") => "not specified",

        // ---------------- 内核：第二层 ----------------
        (Lang::En, "locktime_0") => "0: no timelock; the transaction may be mined immediately",
        (Lang::En, "locktime_block") => "{0}: block-height lock — it will not be mined before that block",
        (Lang::En, "locktime_time") => "{0}: timestamp lock — it cannot be mined before that UNIX time",
        (Lang::En, "ctx_summary") => "This is a PSBT awaiting signatures: {0} input(s), {1} output(s), {2} sat in total outputs.",
        (Lang::En, "ctx_locktime") => "nLockTime = {0} ({1})",
        (Lang::En, "ctx_draft") => "A PSBT is only a draft awaiting signatures: the values in it come from fields supplied by whoever handed it over. Without going online to check the previous transactions, this tool cannot guarantee they match the chain.",
        (Lang::En, "ctx_first_signer") => "No existing signature — you would probably be the first signer.",

        // ---------------- 内核：第三层（风险） ----------------
        (Lang::En, "r001_summary") => "The PSBT contains fields this tool cannot interpret",
        (Lang::En, "r001_trigger") => "{0} unknown/proprietary key(s) detected",
        (Lang::En, "r002_summary") => "The fee is unknown",
        (Lang::En, "r002_trigger") => "at least one input carries neither witness_utxo nor non_witness_utxo, and this tool does not go online",
        (Lang::En, "r002_evidence") => "input amount states: {0}",
        (Lang::En, "in_amount") => "in[{0}]={1} sat",
        (Lang::En, "in_unknown") => "in[{0}]=unknown",
        (Lang::En, "r003_summary") => "Outputs exceed inputs in total value",
        (Lang::En, "r003_trigger") => "total inputs < total outputs",
        (Lang::En, "r003_evidence") => "total outputs = {0} sat",
        (Lang::En, "r004_summary") => "This input already carries a signature",
        (Lang::En, "r004_trigger") => "input[{0}] contains {1} partial_sig(s)",
        (Lang::En, "r004_evidence") => "the signatures themselves are not cryptographically verified (this tool never touches or derives keys, and does not verify signatures)",
        (Lang::En, "r005_summary") => "An output script form could not be recognised",
        (Lang::En, "r005_trigger") => "the scriptPubKey of output[{0}] matched no known standard form",

        // ---------------- 内核：第四层 ----------------
        (Lang::En, "np1") => "This tool does not judge the transaction \"safe\" or \"dangerous\".",
        (Lang::En, "np2") => "This tool does not know who the counterparty is and cannot judge whether a recipient is trustworthy.",
        (Lang::En, "np3") => "This tool does not go online, so it cannot confirm that the previous transactions exist or remain unspent.",
        (Lang::En, "np4") => "This tool never touches private keys and cannot judge whether you are entitled to sign this transaction.",
        (Lang::En, "np5") => "This tool offers no price, tax, or compliance advice.",

        // ---------------- 内核：五态解释 ----------------
        (Lang::En, "st_verified") => "parsed by explicitly defined rules and passed structural checks",
        (Lang::En, "st_parsed") => "read out of the message, but its correctness was not verified here",
        (Lang::En, "st_missing") => "not provided in the message; this tool does not guess",
        (Lang::En, "st_unsupported") => "present in the message but its meaning cannot be interpreted by this tool",
        (Lang::En, "st_failed") => "present but failed a check",

        // ================= zh-CN =================
        (Lang::ZhCn, "app_title") => "Lotus Signer",
        (Lang::ZhCn, "tagline") => "别人让你签的东西，先过一遍 Lotus Signer。",
        (Lang::ZhCn, "offline_badge") => "本页不发起任何外部请求 · 完全离线运行",
        (Lang::ZhCn, "drop_title") => "把待签的 PSBT 拖到这里",
        (Lang::ZhCn, "drop_sub") => "或点击选择文件 —— 支持 base64 的 .psbt 文本。文件不会离开这台浏览器。",
        (Lang::ZhCn, "input_placeholder") => "……也可以直接把 base64 文本粘贴到这里",
        (Lang::ZhCn, "btn_review") => "审阅",
        (Lang::ZhCn, "btn_demo") => "载入示例",
        (Lang::ZhCn, "hint_ready") => "内核就绪",
        (Lang::ZhCn, "hint_wasm_fail") => "内核加载失败：{0}",
        (Lang::ZhCn, "hint_demo_fail") => "示例加载失败",
        (Lang::ZhCn, "layer1") => "交易事实",
        (Lang::ZhCn, "layer2") => "上下文解释",
        (Lang::ZhCn, "layer3") => "风险提示",
        (Lang::ZhCn, "layer4") => "本工具不提供以下结论",
        (Lang::ZhCn, "footer1") => "以上为「循证审阅」结果，不构成任何人签名或不签名的建议。",
        (Lang::ZhCn, "footer2") => "解析与判定由唯一的 Rust 内核提供，本页不含任何解析逻辑。",
        (Lang::ZhCn, "th_index") => "#",
        (Lang::ZhCn, "th_outpoint") => "Outpoint",
        (Lang::ZhCn, "th_type") => "类型",
        (Lang::ZhCn, "th_amount") => "金额",
        (Lang::ZhCn, "th_sigs") => "签名",
        (Lang::ZhCn, "th_value") => "金额",
        (Lang::ZhCn, "th_script_addr") => "收款脚本 / 地址",
        (Lang::ZhCn, "dt_txid") => "txid",
        (Lang::ZhCn, "dt_version") => "version",
        (Lang::ZhCn, "dt_locktime") => "nLockTime",
        (Lang::ZhCn, "dt_total_out") => "输出合计",
        (Lang::ZhCn, "dt_fee") => "手续费",
        (Lang::ZhCn, "dt_sighash") => "sighash",
        (Lang::ZhCn, "unknown") => "未知",
        (Lang::ZhCn, "unlabelled") => "未标注脚本类型",
        (Lang::ZhCn, "not_provided_label") => "—",
        (Lang::ZhCn, "no_sigs") => "—",
        (Lang::ZhCn, "sig_count") => "{0} 个（未验签）",
        (Lang::ZhCn, "cannot_show_addr") => "无法呈现地址（{0}）",
        (Lang::ZhCn, "empty_risks") => "本次未检出需要提示的问题。这不代表「安全」，只代表未触发已知规则。",
        (Lang::ZhCn, "err_review_fail") => "审阅失败：{0}",
        (Lang::ZhCn, "err_core_fail") => "调用内核出错：{0}",

        (Lang::ZhCn, "cli_usage") => "用法：lscli [--lang en|zh] <PSBT 文件>",
        (Lang::ZhCn, "cli_example") => "示例：lscli ../../g3/fixtures/S1.psbt",
        (Lang::ZhCn, "cli_read_fail") => "读取失败：{0} —— {1}",
        (Lang::ZhCn, "cli_banner") => "Lotus Signer · 受托签名审阅",
        (Lang::ZhCn, "cli_inputs") => "输入",
        (Lang::ZhCn, "cli_outputs") => "输出",
        (Lang::ZhCn, "cli_total") => "输出合计",
        (Lang::ZhCn, "cli_sig_note") => "既有签名 {0} 个（未验签）",
        (Lang::ZhCn, "r_trigger") => "触发原因：",
        (Lang::ZhCn, "r_evidence") => "依据：",

        (Lang::ZhCn, "err_decode") => "输入解码失败：{0}",
        (Lang::ZhCn, "err_parse") => "PSBT 解析失败：{0}",
        (Lang::ZhCn, "err_no_global_tx") => "缺少 PSBT_GLOBAL_UNSIGNED_TX，本版本暂不支持 PSBTv2",
        (Lang::ZhCn, "err_prev_parse") => "前序交易解析失败：{0}",
        (Lang::ZhCn, "err_prev_out_range") => "前序交易的第 {0} 个输出不存在",
        (Lang::ZhCn, "check_witness_len") => "witness_utxo 长度不足 8 字节",
        (Lang::ZhCn, "check_out_exceed_in") => "输出合计超过输入合计，这笔交易在数额上不成立",
        (Lang::ZhCn, "check_sighash_len") => "sighash_type 字段长度不为 4 字节",

        (Lang::ZhCn, "note_amount_witness") => "该金额来自 PSBT 随附的 witness_utxo 字段；本工具不联网，无法核实它是否与链上前序交易一致",
        (Lang::ZhCn, "note_amount_nw") => "该金额读自 PSBT 随附的前序交易（non_witness_utxo）；结构校验通过，但离线状态无法确认其在链上存在",
        (Lang::ZhCn, "missing_amount_no_utxo") => "PSBT 未随附 witness_utxo 或 non_witness_utxo，且本工具不联网查询，故输入金额未知",
        (Lang::ZhCn, "missing_input_map") => "该输入无对应的 PSBT 输入映射",
        (Lang::ZhCn, "note_fee") => "手续费 = 输入合计 - 输出合计；输入金额来自 PSBT 随附字段，未经链上核实",
        (Lang::ZhCn, "missing_fee") => "至少一个输入金额未知（PSBT 未随附前序交易信息且本工具不联网查询），因此手续费无法计算 —— 本工具不会把它当作 0，也不会用推测值填补",
        (Lang::ZhCn, "missing_sighash") => "PSBT 未标注 sighash 类型；默认按 SIGHASH_ALL 解释，但这是猜测——本工具不作自动填补",

        (Lang::ZhCn, "policy_all") => "SIGHASH_ALL：签名覆盖全部输入与输出",
        (Lang::ZhCn, "policy_none") => "SIGHASH_NONE：签名不覆盖任何输出（他人可改写收款方）",
        (Lang::ZhCn, "policy_single") => "SIGHASH_SINGLE：签名只覆盖同序号输出",
        (Lang::ZhCn, "policy_acp_all") => "SIGHASH_ALL | ANYONECANPAY：他人可追加输入",
        (Lang::ZhCn, "policy_acp_none") => "SIGHASH_NONE | ANYONECANPAY：不覆盖输出；他人可追加输入",
        (Lang::ZhCn, "policy_acp_single") => "SIGHASH_SINGLE | ANYONECANPAY：只覆盖同序号输出；他人可追加输入",
        (Lang::ZhCn, "policy_unknown") => "未知 sighash 值 {0}",
        (Lang::ZhCn, "policy_unspecified") => "未标注",

        (Lang::ZhCn, "locktime_0") => "0：不使用时间锁，交易可被立即打包",
        (Lang::ZhCn, "locktime_block") => "{0}：区块高度锁 —— 在该区块高度之前不会被打包",
        (Lang::ZhCn, "locktime_time") => "{0}：时间戳锁 —— 在该 UNIX 时间之后才可被打包",
        (Lang::ZhCn, "ctx_summary") => "这是一笔待签名的 PSBT：{0} 个输入、{1} 个输出，输出合计 {2} sat。",
        (Lang::ZhCn, "ctx_locktime") => "nLockTime = {0}（{1}）",
        (Lang::ZhCn, "ctx_draft") => "PSBT 只是待签名的草稿：其中的数值来自递交方随附的字段，在不联网核实前序交易的前提下，本工具无法保证它们与链上一致。",
        (Lang::ZhCn, "ctx_first_signer") => "当前没有任何既有签名 —— 你可能是第一个签名者。",

        (Lang::ZhCn, "r001_summary") => "报文中含有本工具无法解释的字段",
        (Lang::ZhCn, "r001_trigger") => "检出 {0} 个未知/专有键",
        (Lang::ZhCn, "r002_summary") => "手续费未知",
        (Lang::ZhCn, "r002_trigger") => "至少一个输入未提供 witness_utxo / non_witness_utxo，且本工具不联网",
        (Lang::ZhCn, "r002_evidence") => "输入金额状态：{0}",
        (Lang::ZhCn, "in_amount") => "in[{0}]={1} sat",
        (Lang::ZhCn, "in_unknown") => "in[{0}]=未知",
        (Lang::ZhCn, "r003_summary") => "输出数额超过输入数额",
        (Lang::ZhCn, "r003_trigger") => "输入合计 < 输出合计",
        (Lang::ZhCn, "r003_evidence") => "输出合计 = {0} sat",
        (Lang::ZhCn, "r004_summary") => "该输入已包含既有签名",
        (Lang::ZhCn, "r004_trigger") => "input[{0}] 检出 {1} 个 partial_sig",
        (Lang::ZhCn, "r004_evidence") => "签名本身未经密码学核验（本工具不接触或不推导密钥，也不做验签）",
        (Lang::ZhCn, "r005_summary") => "存在无法识别的输出脚本形态",
        (Lang::ZhCn, "r005_trigger") => "output[{0}] 的 scriptPubKey 未匹配任何已知标准形态",

        (Lang::ZhCn, "np1") => "本工具不判断这笔交易「安全」或「危险」。",
        (Lang::ZhCn, "np2") => "本工具不知道交易对手方是谁，无法判断收款方是否可信。",
        (Lang::ZhCn, "np3") => "本工具不联网，因此无法核实前序交易是否存在、是否已被花费。",
        (Lang::ZhCn, "np4") => "本工具不接触私钥，也无法判断你是否有权签署这笔交易。",
        (Lang::ZhCn, "np5") => "本工具不提供价格、税务或合规意见。",

        // ---------------- 内核：五态解释 ----------------
        (Lang::ZhCn, "st_verified") => "已按明确规则解析并通过结构校验",
        (Lang::ZhCn, "st_parsed") => "已从报文中读出，但其内容正确性未在此验证",
        (Lang::ZhCn, "st_missing") => "报文中未提供此项，本工具不作推测",
        (Lang::ZhCn, "st_unsupported") => "报文中存在但本工具无法解释此项含义",
        (Lang::ZhCn, "st_failed") => "此项存在但未能通过校验",

        // 未知 key：两种语言都原样返回
        (_, k) => k,
    };

    // {0} {1} … 插值（简单顺序替换，参数不含 } 不会误伤）
    let mut out = raw.to_string();
    for (i, a) in args.iter().enumerate() {
        out = out.replace(&format!("{{{}}}", i), a);
    }
    out
}
