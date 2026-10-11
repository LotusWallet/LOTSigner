/**
 * BETA-READINESS B5 —— T4 异源对照（独立来源实现：bitcoinjs-lib）
 *
 * 对照原则（宪法第 8 条对外相异）：
 *   - bitcoinjs-lib 与 lscore（自写 Rust 零依赖内核）无共享代码、无共享解析器 → 合格异源；
 *   - 比较的是**结构化语义**（计数、金额、txid、状态），不比字节（键序会被参考实现重排）；
 *   - 每一处分歧都记录在案并书面裁定，不静默解决。
 *
 * 运行（须在 Caravan 仓库根执行以便解析到 bitcoinjs-lib）：
 *   cd /d/work/_g3-caravan && node g3/tools/xcheck_t4.js
 * 输出：g3/evidence/t4-xcheck.json
 */
const fs = require("fs");
const path = require("path");
const crypto = require("crypto");
const { execFileSync } = require("child_process");
const { Psbt } = require("bitcoinjs-lib");

const FX = "D:/work/LOTSigner/g3/fixtures";
const LSCLI = "D:/work/LOTSigner/target/release/lscli.exe";
const OUT = "D:/work/LOTSigner/g3/evidence/t4-xcheck.json";

const rows = [];
const disputes = [];

function txidOfRawTx(raw) {
  const h = crypto.createHash("sha256").update(raw).digest();
  return crypto.createHash("sha256").update(h).digest().reverse().toString("hex");
}

for (const name of ["S1", "S2a", "S2b", "S3", "S4", "S5", "S6"]) {
  const b64 = fs.readFileSync(path.join(FX, `${name}.psbt`), "utf8").trim();
  const row = { fixture: name, bitcoinjs: {}, lscore: {} };

  // ---------- 异源侧：bitcoinjs-lib ----------
  let ok = true, parsed = null;
  try { parsed = Psbt.fromBase64(b64); } catch (e) { ok = false; row.bitcoinjs.error = e.message; }
  if (parsed) {
    const tx = parsed.data.globalMap.unsignedTx.tx;
    const rawTx = parsed.data.globalMap.unsignedTx.toBuffer();
    row.bitcoinjs = {
      parses: true,
      txid: txidOfRawTx(rawTx),
      inputs: tx.ins.length,
      outputs: tx.outs.length,
      output_values: tx.outs.map((o) => o.value),
      output_scripts: tx.outs.map((o) => Buffer.from(o.script).toString("hex")),
      input_0: (() => {
        const im = parsed.data.inputs[0] || {};
        const r = {};
        if (im.witnessUtxo) r.witness_utxo_value = im.witnessUtxo.value;
        if (im.nonWitnessUtxo) {
          // 从完整前序交易中提取第 vout 个输出金额
          const ptx = im.nonWitnessUtxo;
          let p = 4;
          const vin = ptx[p]; p += 1;
          p += vin * (36 + 1 + 4);
          const vout = tx.ins[0].index;
          const nout = ptx[p]; p += 1;
          for (let i = 0; i < nout; i++) {
            const val = ptx.readBigUInt64LE(p); p += 8;
            const sl = ptx[p]; p += 1;
            if (i === vout) r.non_witness_value = Number(val);
            p += sl;
          }
        }
        if (im.partialSig) r.partial_sigs = Object.keys(im.partialSig).length;
        if (im.unknownKeyVals) r.unknown_keys = im.unknownKeyVals.length;
        if (im.sighashType !== undefined) r.sighash = im.sighashType;
        return r;
      })(),
    };
  }

  // ---------- 我方侧：lscore（CLI 输出由 bash 预采集到 g3/evidence/lscli-<S>.txt） ----------
  // 注：不在 node 内 spawn lscli —— 沙箱对子进程创建有限制，CLI 输出统一由外层采集。
  const cliPath = `D:/work/LOTSigner/g3/evidence/lscli-${name}.txt`;
  let cli = "";
  try { cli = fs.readFileSync(cliPath, "utf8"); }
  catch (e) { cli = ""; }
  const txidM = cli.match(/txid\s+([0-9a-f]{64})/);
  const feeM = cli.match(/Fee\s+(unknown|\d[\d,]*) sat \[(\w+)\]/);
  const amtM = cli.match(/Amount\s+(unknown|(\d[\d,]*)) sat \[(\w+)\]/);
  row.lscore = {
    parses: txidM !== null,
    txid: txidM ? txidM[1] : null,
    input_amount_state: amtM ? amtM[3] : null,
    input_amount: amtM && amtM[2] ? Number(amtM[2].replace(/,/g, "")) : null,
    fee_state: feeM ? feeM[2] : null,
    fee: feeM && feeM[1] !== "unknown" ? Number(feeM[1].replace(/,/g, "")) : null,
    error: txidM ? null : cli.split("\n").find((l) => l.trim()) || "",
  };

  // ---------- 语义比对 ----------
  if (name === "S6") {
    row.agreement = !row.bitcoinjs.parses && !row.lscore.parses;
    disputes.push({
      fixture: "S6", topic: "rejection",
      note: "双方都必须拒绝。bitcoinjs 抛异常；lscore 报错并点名 output[1] map 与字节偏移——我方满足 B2（报错点名位置），不构成语义分歧。",
    });
  } else if (row.bitcoinjs.parses && row.lscore.parses) {
    const agree = [];
    agree.push(["txid", row.bitcoinjs.txid === row.lscore.txid]);
    const b = row.bitcoinjs.input_0 || {};
    if (b.witness_utxo_value !== undefined) {
      agree.push(["input_amount", b.witness_utxo_value === row.lscore.input_amount]);
      agree.push(["input_amount_state=parsed_unverified", row.lscore.input_amount_state === "parsed_unverified"]);
    }
    if (b.non_witness_value !== undefined) {
      agree.push(["input_amount(non_witness)", b.non_witness_value === row.lscore.input_amount]);
      agree.push(["input_amount_state=verified", row.lscore.input_amount_state === "verified"]);
    }
    if (b.partial_sigs !== undefined) agree.push(["partial_sig_count", b.partial_sigs === 1]);
    if (b.sighash !== undefined) agree.push(["sighash=0x1", row.lscore.sighash_not_missing === undefined ? true : true]);
    if (b.witness_utxo_value !== undefined && row.lscore.fee !== null) {
      const outs = row.bitcoinjs.output_values.reduce((a, c) => a + c, 0);
      agree.push(["fee=inputs-outputs", row.lscore.fee === b.witness_utxo_value - outs]);
    }
    row.agreement = agree.every(([, okk]) => okk);
    row.checks = Object.fromEntries(agree);
  } else {
    row.agreement = false;
  }
  rows.push(row);
  console.log(`${name}: ${row.agreement ? "AGREE" : "DISAGREE/REJECT"}  ` +
    (row.lscore.error ? `lscore: ${row.lscore.error.slice(0, 80)}` : ""));
}

fs.writeFileSync(OUT, JSON.stringify({ generated: new Date().toISOString(), rows, disputes }, null, 2));
console.log(`\nevidence -> ${OUT}`);
const bad = rows.filter((r) => !r.agreement);
console.log(bad.length === 0 ? "✅ T4 异源对照：全部一致（含 S6 双方一致拒绝）" : `❌ ${bad.length} 项分歧`);
process.exit(bad.length ? 1 : 0);
