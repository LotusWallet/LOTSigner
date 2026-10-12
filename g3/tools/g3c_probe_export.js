/*
 * G3 补测 · 差异项③「导出部分签名 PSBT」判定探针
 * ---------------------------------------------------------------
 * 背景（预注册 §3 / 判定表 §③）：
 *   ③ 的分类是「工作流阻断」，判定标准 = **签名后无法导出可供发起方继续收集
 *   其余签名的 PSBT（或导出格式不可被第三方工具消费）**。
 *   上一轮实测止步于 `This address has a zero balance.`——固件用的是合成密钥，
 *   其派生地址在主网无 UTXO，Caravan 的 `performSpend` 守卫（fetchUTXOs 成功
 *   且余额 > 0）不成立，因此**拿不到任何观测**。
 *
 * 本补测的做法（相对预注册方案的最小偏离，须在报告中如实声明）：
 *   预注册写的是"本地 regtest bitcoind 全链路"。本机无 bitcoind，且 Caravan 的
 *   签名环节只能由硬件设备完成（本机无设备），因此改为：
 *     ① 用自建 2-of-3 P2SH 钱包（xpub 自持、私钥自持，合成密钥，无真实资金）
 *     ② **mock 链上数据源**（Esplora 兼容接口），给该地址造一笔合成的资交易 UTXO
 *     ③ Caravan 走真实代码路径构建交易 → 下载「Download Unsigned PSBT」
 *     ④ 在**外部**用自持私钥对其中一个密钥签名（模拟"某位签名人签完交回"）
 *     ⑤ 通过 Caravan 的**文件导入**入口把已签名 PSBT 交回 → 观察其是否能导出
 *        可供发起方继续收集其余签名的 PSBT，并检查导出物能否被第三方工具消费
 *
 * 声明边界（写入报告时必须原样保留）：
 *   - 链上数据是**合成**的（不触及真实资金）；Caravan 的构建/导入/导出是**真实**的。
 *   - 签名由外部工具完成（无硬件设备时唯一可行），因此本测试**不评估** Caravan
 *     与设备的交互，仅评估 **PSBT 的导出/导入形态**。
 *   - mock 只改变"链上数据从哪来"，不改变 Caravan 对 PSBT 的解析、拼装与导出逻辑。
 *
 * 用法：
 *   node g3c_probe_export.js setup     # 生成钱包/资交易/配置，写出 g3c-setup.json
 *   node g3c_probe_export.js run       # 跑完整浏览器流程（需先起 vite preview）
 */

const path = require("path");
const fs = require("fs");
const crypto = require("crypto");

const MODS = process.env.CARAVAN_MODULES || "D:/work/_g3-caravan/node_modules/";
const b32 = require(MODS + "bip32");
const btc = require(MODS + "bitcoinjs-lib");
const caravanBtc = require(MODS + "@caravan/bitcoin");

const EVIDENCE = path.join(__dirname, "..", "evidence");
const SETUP_FILE = path.join(EVIDENCE, "g3c-setup.json");

// 三把合成密钥：确定性（可复现），不涉及任何真实资金
const SEEDS = [
  "0a1b2c3d4e5f60718293a4b5c6d7e8f90011223344556677889900112233445566",
  "1122334455667788990011223344556677889900112233445566778899001122",
  "fedcba98765432100123456789abcdeffedcba98765432100123456789abcdef",
];
const ROOT_PATH = "m/45'/0'/100'"; // 与 Caravan 官方 P2SH 固件配置同构

function buildWalletConfig() {
  const extendedPublicKeys = SEEDS.map((seedHex, i) => {
    const master = b32.fromSeed(Buffer.from(seedHex, "hex"));
    const child = master.derivePath(ROOT_PATH);
    return {
      name: `signer${i + 1}`,
      bip32Path: ROOT_PATH,
      xpub: child.neutered().toBase58(),
      xfp: master.fingerprint.toString("hex"),
    };
  });

  return {
    name: "G3C-P2SH-2of3",
    addressType: "P2SH",
    network: "mainnet",
    client: { type: "public" },
    quorum: { requiredSigners: 2, totalSigners: 3 },
    extendedPublicKeys,
    startingAddressIndex: 0,
  };
}

/** 用 Caravan 自己的派生函数算地址——保证与 UI 内部完全一致 */
function deriveAddress(config, index) {
  const ms = caravanBtc.deriveMultisigByIndex(config.extendedPublicKeys, index);
  return {
    address: ms.address,
    redeemScriptHex: ms.redeemScript,
    pubkeys: ms.pubkeys,
    bip32Derivation: ms.bip32Derivation,
  };
}

/** 造一笔"资交易"：支付到该多签地址，作为 UTXO 的来源（合成，非链上真实） */
function buildFundingTx(scriptPubKeyHex, valueSats) {
  const tx = new btc.Transaction();
  tx.version = 2;
  // 一个合成输入（coinbase 风格，无所谓共识有效性——它只需与自身 txid 自洽）
  tx.addInput(
    Buffer.alloc(32, 0x11),
    0,
    0xffffffff,
    Buffer.from("03abcd01", "hex"), // 合成 scriptSig
  );
  tx.addOutput(Buffer.from(scriptPubKeyHex, "hex"), valueSats);
  tx.locktime = 0;
  return { txid: tx.getId(), hex: tx.toHex() };
}

function setup() {
  fs.mkdirSync(EVIDENCE, { recursive: true });
  const config = buildWalletConfig();
  const addr = deriveAddress(config, 0);

  // P2SH scriptPubKey（用 Caravan 的 payments 结果反推 scriptPubKey）
  const p2sh = btc.payments.p2sh({
    address: addr.address,
    network: btc.networks.bitcoin,
  });
  const spkHex = p2sh.output.toString("hex");

  const FUND_SATS = 200000;
  const funding = buildFundingTx(spkHex, FUND_SATS);

  const out = {
    generatedAt: new Date().toISOString(),
    note: "合成数据：密钥自持、链上数据 mock，不含真实资金",
    config,
    address: addr.address,
    redeemScriptHex: addr.redeemScriptHex,
    scriptPubKeyHex: spkHex,
    pubkeys: addr.pubkeys,
    fundingTxid: funding.txid,
    fundingTxHex: funding.hex,
    fundingValueSats: FUND_SATS,
  };
  fs.writeFileSync(SETUP_FILE, JSON.stringify(out, null, 2));
  console.log("wallet config written:", SETUP_FILE);
  console.log("address      :", addr.address);
  console.log("funding txid :", funding.txid);
  console.log("funding value:", FUND_SATS, "sat");
  return out;
}

/** 用第 n 把密钥对一笔 PSBT 签名（模拟某位签名人交回签名） */
function signPsbtWithKey(psbtBase64, seedIndex, config, addressIndex) {
  const master = b32.fromSeed(Buffer.from(SEEDS[seedIndex], "hex"));
  const child = master.derivePath(`${ROOT_PATH}/${addressIndex}`);
  const keyPair = btc.ECPair.fromPrivateKey(child.privateKey, {
    network: btc.networks.bitcoin,
  });
  const psbt = btc.Psbt.fromBase64(psbtBase64, {
    network: btc.networks.bitcoin,
  });
  psbt.signAllInputs(keyPair);
  // 不 finalize：部分签名场景必须保持未 finalize 的 PSBT 形态
  return psbt.toBase64();
}

module.exports = { buildWalletConfig, deriveAddress, setup, signPsbtWithKey, SETUP_FILE, SEEDS, ROOT_PATH };

if (require.main === module) {
  const cmd = process.argv[2] || "setup";
  if (cmd === "setup") setup();
  else console.error("unknown command:", cmd);
}
