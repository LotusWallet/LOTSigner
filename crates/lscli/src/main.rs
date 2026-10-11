//! Lotus Signer · 命令行审阅端
//!
//! 本文件**不得**包含任何 PSBT 解析、交易事实判定或五态逻辑——宪法第 8 条（权威语义实现唯一）。
//! 它只负责读文件、调用 lscore、把结果排版输出。

use std::env;
use std::fs;
use std::process::ExitCode;

use lscore::state::FieldState;
use lscore::review_psbt_base64;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return ExitCode::SUCCESS;
    }
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("lscli {}", VERSION);
        return ExitCode::SUCCESS;
    }

    let path = match args.get(1) {
        Some(p) => p,
        None => {
            eprintln!("用法：lscli <PSBT 文件>");
            eprintln!("示例：lscli ../../g3/fixtures/S1.psbt");
            return ExitCode::FAILURE;
        }
    };

    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("读取失败：{} —— {}", path, e);
            return ExitCode::FAILURE;
        }
    };

    let review = match review_psbt_base64(&text) {
        Ok(r) => r,
        Err(e) => {
            // 宪法第 5 条：不支持的格式必须明确报错，不允许静默降级
            eprintln!("审阅失败：{}", e);
            return ExitCode::FAILURE;
        }
    };

    let f = &review.facts;
    println!("==================================================");
    println!(" Lotus Signer · 受托签名审阅");
    println!("==================================================");
    println!("txid      {}", f.txid);
    println!("version   {}     nLockTime {}（{}）", f.version, f.locktime, f.locktime_meaning);

    // ---- 第一层：交易事实 ----
    println!("\n── 第一层 · 交易事实 ─────────────────────────────");
    println!("输入");
    for i in &f.inputs {
        let amount = match i.amount_sats.value {
            Some(v) => format!("{} sat", v),
            None => "未知".to_string(),
        };
        let note = i.amount_sats.note.clone().unwrap_or_default();
        println!(
            "  in[{}] {}   seq=0x{:08x}   类型={}",
            i.index,
            i.outpoint,
            i.sequence,
            i.script_type.clone().unwrap_or_else(|| "未提供".to_string()),
        );
        println!(
            "        金额 {} [{}]",
            amount,
            i.amount_sats.state.as_str()
        );
        if !note.is_empty() {
            println!("              ↳ {}", note);
        }
        if i.partial_sig_count > 0 {
            println!("        既有签名 {} 个（未验签）", i.partial_sig_count);
        }
    }

    println!("输出");
    for o in &f.outputs {
        println!(
            "  out[{}] {:>12} sat   {}",
            o.index,
            o.value_sats,
            o.address
                .clone()
                .unwrap_or_else(|| format!("无法呈现地址（{}）", o.script_type))
        );
    }
    println!("  {:>46}", "─────────────");
    println!("  输出合计 {:>11} sat", f.total_out_sats);

    let fee_txt = match f.fee_sats.value {
        Some(v) => format!("{} sat", v),
        None => "未知".to_string(),
    };
    println!(
        "  手续费    {:>11} [{}]",
        fee_txt,
        f.fee_sats.state.as_str()
    );
    if let Some(n) = &f.fee_sats.note {
        println!("              ↳ {}", n);
    }

    println!(
        "  sighash   {} [{}]\n              ↳ {}",
        match f.sighash_type.value {
            Some(v) => format!("0x{:02x}", v),
            None => "未标注".to_string(),
        },
        f.sighash_type.state.as_str(),
        f.sighash_policy
    );

    // ---- 第二层：上下文 ----
    println!("\n── 第二层 · 上下文解释 ───────────────────────────");
    for c in &review.context {
        println!("  · {}", c);
    }

    // ---- 第三层：风险提示（每条须带触发原因） ----
    println!("\n── 第三层 · 风险提示 ─────────────────────────────");
    if review.risks.is_empty() {
        println!("  （本次未检出需要提示的问题。这不代表「安全」，只代表未触发已知规则。）");
    } else {
        for r in &review.risks {
            println!("  [{}] {}", r.code, r.summary);
            println!("        触发原因：{}", r.trigger_reason);
            println!("        依据：{}", r.evidence);
        }
    }

    // ---- 第四层：不提供的结论（永远不得为空） ----
    println!("\n── 第四层 · 本工具不提供以下结论 ─────────────────");
    for n in &review.not_provided {
        println!("  × {}", n);
    }

    println!("\n==================================================");
    println!(" 以上为「循证审阅」结果，不构成任何人签名或不签名的建议。");
    println!("==================================================");

    ExitCode::SUCCESS
}

fn print_help() {
    println!(
        "lscli {} —— Lotus Signer 命令行审阅端\n\
         \n\
         用法:\n  \
           lscli <PSBT 文件>     审阅一份 base64（或已用 CRLF/换行包裹的）PSBT\n  \
           lscli --version\n  \
           lscli --help\n\
         \n\
         本工具不联网、不接触私钥、不输出「安全/危险」结论。\n\
         所有解析与判定由 lscore 内核提供，本端不含任何解析逻辑。",
        VERSION
    );
}
