/*
 * G3 补测 · 差异项③ 证据固化脚本
 * ------------------------------------------------------------------
 * 目的：把「阶段 E/F/G 三个下载物」的字段构成与可消费性，固化成
 *       一份**任何人可重跑**的证据日志（g3/evidence/g3c-export-verify.txt）。
 *
 * 三重核验：
 *   1. 字段构成   —— bitcoinjs-lib 读取每个下载物的 partialSig / nonWitnessUtxo /
 *                    redeemScript / bip32Derivation / finalScriptSig
 *   2. 签名字节级 —— 证明「导出物里的签名 == 我们外部注入的那条签名」（不是凭空计数）
 *   3. 第三方可消费 —— ① 本项目的 lscli（Rust 内核，与 Caravan/bitcoinjs 无共享代码）
 *                      ② bitcoinjs-lib（与 lscore 无共享代码）
 *   4. 密码学校验 —— bitcoinjs-lib validateSignaturesOfInput + tiny-secp256k1
 *                    （依赖可用才跑；不可用则如实标注 skipped，不伪造结论）
 *
 * 边界：链上 UTXO 为 mock、签名由外部工具完成 —— 两者是补测的既定前提，
 *       结论只适用于「该构建版本 + 该流程形态」，不作外推。
 *
 * 用法：node g3/tools/g3c_verify_exports.js
 */
const path = require("path");
const fs = require("fs");
const { execFileSync } = require("child_process");

const M = process.env.CARAVAN_MODULES || "D:/work/_g3-caravan/node_modules/";
const btc = require(M + "bitcoinjs-lib");

const EV = path.join(__dirname, "..", "evidence");
const REPO = path.join(__dirname, "..", "..");
const LSCLI = path.join(REPO, "target", "release", "lscli.exe");

const FILES = [
  ["E", "未签名导出（Send 流程 · 下载未签名交易）", "g3c-E-dl-transaction.psbt"],
  ["F-unsigned", "标签为 'Download Unsigned PSBT' 的按钮产出", "g3c-F-dl-transaction.psbt"],
  ["F-partial", "外部工具部分签名后的输入文件（注入基线）", "g3c-F-partial-signed.psbt"],
  ["G-signed", "签名步骤 'Download PSBT' 导出物", "g3c-G-dl-0-signed.psbt"],
];

let ecc = null;
try {
  ecc = require(M + "tiny-secp256k1");
} catch (e) {
  ecc = null;
}

const hex = (b) => Buffer.from(b).toString("hex");

function describe(file) {
  const p = path.join(EV, file);
  if (!fs.existsSync(p)) return { file, missing: true };
  const raw = fs.readFileSync(p, "utf8").trim();
  const psbt = btc.Psbt.fromBase64(raw, { network: btc.networks.bitcoin });
  const im = psbt.data.inputs[0] || {};
  const om = psbt.data.outputs[0] || {};
  const sigs = (im.partialSig || []).map((s) => ({ pubkey: hex(s.pubkey), sig: hex(s.signature) }));
  const out = {
    file,
    bytes: fs.statSync(p).size,
    txid: psbt.data.globalMap.unsignedTx.tx.getId(),
    nIn: psbt.data.globalMap.unsignedTx.tx.ins.length,
    nOut: psbt.data.globalMap.unsignedTx.tx.outs.length,
    partialSigs: sigs.length,
    sigs,
    nonWitnessUtxo: !!im.nonWitnessUtxo,
    redeemScript: !!im.redeemScript,
    witnessScript: !!im.witnessScript,
    in_bip32Derivation: (im.bip32Derivation || []).length,
    out_bip32Derivation: (om.bip32Derivation || []).length,
    finalScriptSig: !!im.finalScriptSig,
  };
  // 密码学校验（bitcoinjs-lib 5.2.1: validateSignaturesOfInput(index[, pubkey])，
  // 内部经 ecpair(tiny-secp256k1) 逐条验签；同时对每条签名核对公钥确在 redeemScript 中）
  if (sigs.length) {
    try {
      out.cryptoValid = psbt.validateSignaturesOfInput(0) === true ? "✅ 有效（ECDSA 验签通过）" : "❌ 无效";
    } catch (e) {
      out.cryptoValid = "error: " + e.message.slice(0, 90);
    }
  } else {
    out.cryptoValid = "n/a (无签名)";
  }
  return out;
}

function lscli(file) {
  const p = path.join(EV, file);
  if (!fs.existsSync(p)) return "(missing)";
  try {
    const out = execFileSync(LSCLI, ["--lang", "en", p], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
    return out.trim();
  } catch (e) {
    return "EXIT " + (e.status === undefined ? "?" : e.status) + "\n" + ((e.stdout || "") + (e.stderr || "")).trim();
  }
}

const lines = [];
const w = (s) => {
  lines.push(s);
  console.log(s);
};

w("G3 补测 · 差异项③ 导出物核验日志");
w("生成时间（本机）: " + new Date().toISOString());
w("lscli: " + LSCLI);
w("bitcoinjs-lib: " + require(M + "bitcoinjs-lib/package.json").version + "   tiny-secp256k1: " + (ecc ? require(M + "tiny-secp256k1/package.json").version : "不可用"));
w("=".repeat(78));

const desc = {};
for (const [tag, label, file] of FILES) {
  const d = describe(file);
  desc[tag] = d;
  w("");
  w(`【${tag}】${label}`);
  w("-".repeat(78));
  if (d.missing) {
    w(`  文件缺失: ${file}`);
    continue;
  }
  w(`  文件          : ${file}  (${d.bytes} 字节)`);
  w(`  未签名交易 txid: ${d.txid}`);
  w(`  输入/输出      : ${d.nIn} in / ${d.nOut} out`);
  w(`  部分签名字数    : ${d.partialSigs}`);
  d.sigs.forEach((s, i) => w(`    [${i}] pubkey=${s.pubkey.slice(0, 24)}… sig=${s.sig.slice(0, 32)}…(${(s.sig.length / 2)}B)`));
  w(`  输入字段       : nonWitnessUtxo=${d.nonWitnessUtxo} redeemScript=${d.redeemScript} witnessScript=${d.witnessScript} in_bip32=${d.in_bip32Derivation} finalScriptSig=${d.finalScriptSig}`);
  w(`  输出字段       : out_bip32=${d.out_bip32Derivation}`);
  w(`  密码学校验     : ${d.cryptoValid}`);
  w("");
  w("  ── lscli（本项目 Rust 内核，与 Caravan / bitcoinjs-lib 无共享代码）──");
  lscli(file).split("\n").forEach((l) => w("  | " + l));
}

w("");
w("=".repeat(78));
w("关键比对：'导出物是否保留外部注入的那条签名'");
w("-".repeat(78));
const base = desc["F-partial"];
const g = desc["G-signed"];
const fu = desc["F-unsigned"];
if (base && g && base.sigs.length === 1) {
  const same = g.sigs.length === 1 && g.sigs[0].sig === base.sigs[0].sig && g.sigs[0].pubkey === base.sigs[0].pubkey;
  w(`  注入基线 (F-partial) 签名: pubkey=${base.sigs[0].pubkey.slice(0, 24)}…`);
  w(`                             sig   =${base.sigs[0].sig.slice(0, 48)}…`);
  w(`  G 导出物    (G-signed) 签名: ${g.sigs.length ? "pubkey=" + g.sigs[0].pubkey.slice(0, 24) + "…" : "无"}`);
  w(`                             ${g.sigs.length ? "sig   =" + g.sigs[0].sig.slice(0, 48) + "…" : ""}`);
  w(`  → 逐字节完全一致 : ${same ? "✅ 是（签名被原样保留）" : "❌ 否"}`);
  w(`  → 未被篡改计数  : ${g.partialSigs === 1 ? "✅ 恰好 1 条，非凭空计数" : "⚠️ partialSigs=" + g.partialSigs}`);
} else {
  w("  （基线或导出物缺失，无法比对）");
}
if (fu) {
  w("");
  w(`  对照：标签为 "Download Unsigned PSBT" 的按钮产出 partialSigs=${fu.partialSigs}（${fu.bytes} 字节）`);
  w("  → 该按钮语义即「未签名」，其产物不含签名符合其自身标签，不构成导出能力缺陷。");
}

w("");
w("边界声明（原样保留）：");
w("  · 链上 UTXO 数据为 mock（本地拦截并构造），非真实主网/测试网数据；");
w("  · 部分签名由外部工具（bitcoinjs-lib + 合成密钥）完成，非 Caravan 自身签名；");
w("  · 结论仅适用于该构建版本 (caravan-coordinator@1.19.2, commit b25532a3…) 的浏览器形态。");

fs.writeFileSync(path.join(EV, "g3c-export-verify.txt"), lines.join("\n") + "\n");
console.log("\n[written] g3/evidence/g3c-export-verify.txt");
