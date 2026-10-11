/**
 * G3 固件异源交叉验证（bip174 = bitcoinjs 参考实现）
 *
 * 为什么需要它：自校验（verify_fixtures.py）用的是我们自己的解析器，与固件生成器同源，
 * 只能证明"我写的和我读的一致"。要证明固件是**标准解析器也能消费的合法报文**，
 * 必须用异源实现读一遍——这正是 T4 独立对照的思想用在固件层。
 *
 * 运行（须在 Caravan 仓库根执行，以便解析到 bip174）：
 *   cd /d/work/_g3-caravan
 *   node <本文件路径>
 */
const fs = require('fs');
const path = require('path');
// bip174 只是 PSBT 容器，unsigned tx 的编解码需外部注入（txGetter），
// 直接用裸 bip174 会报 "txGetter is not a function"。
// 故改用 bitcoinjs-lib 的 Psbt（内部即 bip174 + tx codec），同属 bitcoinjs 系，
// 与我们自写的 Python 生成器仍是**异源**。
const { Psbt } = require('bitcoinjs-lib');

const FX = path.resolve(__dirname, '..', 'fixtures');
const SAMPLES = ['S1', 'S2a', 'S2b', 'S3', 'S4'];

let fail = 0;
function check(ok, msg) {
  console.log(`  ${ok ? '✅' : '❌'} ${msg}`);
  if (!ok) fail++;
}

for (const name of SAMPLES) {
  const p = path.join(FX, `${name}.psbt`);
  console.log(`\n--- ${name} (bip174 异源) ---`);
  if (!fs.existsSync(p)) { console.log('  (缺失，跳过)'); continue; }
  const b64 = fs.readFileSync(p, 'utf8').trim();
  let psbt;
  try {
    psbt = Psbt.fromBase64(b64);
  } catch (e) {
    check(false, `bip174 解析失败: ${e.message}`);
    continue;
  }
  check(true, 'bip174 可解析（合法 BIP-174 报文）');

  const txObj = psbt.data.globalMap.unsignedTx.tx;
  check(txObj.ins.length === 1 && txObj.outs.length === 2,
        `拓扑 in=${txObj.ins.length} out=${txObj.outs.length}`);

  const im = psbt.data.inputs[0] || {};
  const keys = Object.keys(im);
  console.log(`  输入字段: ${keys.length ? keys.join(', ') : '(空)'}`);
  const names = keys.map(k => k.replace('unknownKey_', 'UNKNOWN_0x'));
  console.log(`  归一化: ${names.join(', ')}`);

  // 逐样本关键期望（与 verify_fixtures.py 保持一致，但用异源实现判定）
  const has = (s) => keys.some(k => k.includes(s));
  if (name === 'S4') {
    check(has('nonWitnessUtxo'), 'nonWitnessUtxo 存在（P2SH 输入必需）');
    check(has('redeemScript'), 'redeemScript 存在');
    // 异源核对：前序交易 txid == outpoint txid
    if (im.nonWitnessUtxo) {
      const crypto = require('crypto');
      const d = crypto.createHash('sha256')
        .update(crypto.createHash('sha256').update(im.nonWitnessUtxo).digest())
        .digest();
      const op = Buffer.from(txObj.ins[0].hash);
      check(d.equals(op), '前序交易 txid 与 outpoint 一致（异源核对通过）');
    }
  } else {
    const wantUtxo = name !== 'S2a';
    check(has('witnessUtxo') === wantUtxo,
          `witnessUtxo ${wantUtxo ? '存在' : '缺失'}（期望一致）`);
    if (name === 'S3') check(has('partialSig'), 'partialSig 存在');
    if (name === 'S2b') {
      // bitcoinjs-lib 把未知键收进 unknownKeyVals 数组，键名本身不带 unknownKey_ 前缀
      const n = Array.isArray(im.unknownKeyVals) ? im.unknownKeyVals.length : 0;
      check(n > 0, `存在未知键 ${n} 个（unsupported）`);
    }
    if (name !== 'S2a') check(has('witnessScript'), 'witnessScript 存在');
  }

  // round-trip：序列化回去仍可解析（不比字节，只比可解析性 —— 键序会被重排）
  try {
    const out = psbt.toBase64();
    Psbt.fromBase64(out);
    check(true, 'round-trip 后仍可解析（不比字节：键序会被参考实现重排）');
  } catch (e) {
    check(false, `round-trip 失败: ${e.message}`);
  }
}

console.log('\n' + '='.repeat(66));
console.log(fail === 0 ? '✅ 异源验证全部通过。' : `❌ ${fail} 项失败。`);
process.exit(fail === 0 ? 0 : 1);
