//! Lotus Signer · 命令行审阅端
//!
//! 本文件**不得**包含任何 PSBT 解析、交易事实判定或五态逻辑——宪法第 8 条（权威语义实现唯一）。
//! 它只负责读文件、调参数、调用 lscore、把结果排版输出。所有句子出自内核文案表。

use std::env;
use std::fs;
use std::process::ExitCode;

use lscore::i18n::{t, Lang};
use lscore::review_psbt_base64_lang;

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

    // --lang en|zh（默认 en，国际化默认语言）
    let mut lang = Lang::En;
    let mut positional: Vec<String> = Vec::new();
    let mut it = args.iter().skip(1);
    while let Some(a) = it.next() {
        if a == "--lang" {
            lang = it.next().map(|v| Lang::parse(v)).unwrap_or(Lang::En);
        } else if let Some(v) = a.strip_prefix("--lang=") {
            lang = Lang::parse(v);
        } else {
            positional.push(a.clone());
        }
    }

    let path = match positional.first() {
        Some(p) => p,
        None => {
            eprintln!("{}", t(lang, "cli_usage", &[]));
            eprintln!("{}", t(lang, "cli_example", &[]));
            return ExitCode::FAILURE;
        }
    };

    let text = match fs::read_to_string(path) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("{}", t(lang, "cli_read_fail", &[path, &e.to_string()]));
            return ExitCode::FAILURE;
        }
    };

    let review = match review_psbt_base64_lang(&text, lang) {
        Ok(r) => r,
        Err(e) => {
            // 宪法第 5 条：不支持的格式必须明确报错，不允许静默降级
            eprintln!("{} {}", t(lang, "err_review_fail", &[]), e);
            return ExitCode::FAILURE;
        }
    };

    let f = &review.facts;
    println!("==================================================");
    println!(" {}", t(lang, "cli_banner", &[]));
    println!("==================================================");
    println!("{}      {}", t(lang, "dt_txid", &[]), f.txid);
    println!(
        "{}   {}     {} {}",
        t(lang, "dt_version", &[]),
        f.version,
        t(lang, "dt_locktime", &[]),
        f.locktime
    );
    println!("        {}", f.locktime_meaning);

    // ---- 第一层：交易事实 ----
    println!("\n── {} ─────────────────────────────", t(lang, "layer1", &[]));
    println!("{}", t(lang, "cli_inputs", &[]));
    for i in &f.inputs {
        let amount = match i.amount_sats.value {
            Some(v) => format!("{} sat", v),
            None => t(lang, "unknown", &[]),
        };
        let note = i.amount_sats.note.clone().unwrap_or_default();
        println!(
            "  in[{}] {}   seq=0x{:08x}   {} {}",
            i.index,
            i.outpoint,
            i.sequence,
            t(lang, "th_type", &[]),
            i.script_type.clone().unwrap_or_else(|| "—".to_string()),
        );
        println!(
            "        {} {} [{}]",
            t(lang, "th_amount", &[]),
            amount,
            i.amount_sats.state.as_str()
        );
        if !note.is_empty() {
            println!("              ↳ {}", note);
        }
        if i.partial_sig_count > 0 {
            println!("        {}", t(lang, "cli_sig_note", &[&i.partial_sig_count.to_string()]));
        }
    }

    println!("{}", t(lang, "cli_outputs", &[]));
    for o in &f.outputs {
        println!(
            "  out[{}] {:>12} sat   {}",
            o.index,
            o.value_sats,
            o.address.clone().unwrap_or_else(|| {
                t(lang, "cannot_show_addr", &[&o.script_type])
            })
        );
    }
    println!("  {:>46}", "─────────────");
    println!("  {} {:>11} sat", t(lang, "cli_total", &[]), f.total_out_sats);

    let fee_txt = match f.fee_sats.value {
        Some(v) => format!("{} sat", v),
        None => t(lang, "unknown", &[]),
    };
    println!(
        "  {}    {:>11} [{}]",
        t(lang, "dt_fee", &[]),
        fee_txt,
        f.fee_sats.state.as_str()
    );
    if let Some(n) = &f.fee_sats.note {
        println!("              ↳ {}", n);
    }

    println!(
        "  {}   {} [{}]\n              ↳ {}",
        t(lang, "dt_sighash", &[]),
        match f.sighash_type.value {
            Some(v) => format!("0x{:02x}", v),
            None => t(lang, "policy_unspecified", &[]),
        },
        f.sighash_type.state.as_str(),
        f.sighash_policy
    );

    // ---- 第二层：上下文 ----
    println!("\n── {} ───────────────────────────", t(lang, "layer2", &[]));
    for c in &review.context {
        println!("  · {}", c);
    }

    // ---- 第三层：风险提示（每条须带触发原因） ----
    println!("\n── {} ─────────────────────────────", t(lang, "layer3", &[]));
    if review.risks.is_empty() {
        println!("  {}", t(lang, "empty_risks", &[]));
    } else {
        for r in &review.risks {
            println!("  [{}] {}", r.code, r.summary);
            println!("        {} {}", t(lang, "r_trigger", &[]), r.trigger_reason);
            println!("        {} {}", t(lang, "r_evidence", &[]), r.evidence);
        }
    }

    // ---- 第四层：不提供的结论（永远不得为空） ----
    println!("\n── {} ─────────────────", t(lang, "layer4", &[]));
    for n in &review.not_provided {
        println!("  × {}", n);
    }

    println!("\n==================================================");
    println!(" {}", t(lang, "footer1", &[]));
    println!("==================================================");

    ExitCode::SUCCESS
}

fn print_help() {
    // 帮助文本双语并列，避免帮本身再引入一层参数
    println!(
        "lscli {} —— Lotus Signer command-line review / 命令行审阅端\n\
         \n\
         Usage / 用法:\n  \
           lscli [--lang en|zh] <PSBT file>\n  \
           lscli --version\n  \
           lscli --help\n\
         \n\
         Default language is English; `--lang zh` switches to Chinese.\n\
         默认语言为英文，`--lang zh` 切换到中文。\n\
         \n\
         This tool does not go online, never touches private keys, and never outputs\n\
         a \"safe/dangerous\" verdict. All parsing and judgement lives in the lscore core.\n\
         本工具不联网、不接触私钥、不输出「安全/危险」结论；\n\
         所有解析与判定由 lscore 内核提供，本端不含任何解析逻辑。",
        VERSION
    );
}
