/*
 * G1 · Q2/Q3 实测：BDK-wasm 在**无网络**前提下能否提供描述符解析 / 地址派生
 * --------------------------------------------------------------------------
 * 依据：g1/G1-预注册-BDK-WASM离线能力实测.md §3（Q2/Q3）、§5（F2/F3/F5）
 *
 * 方法（三层防伪）：
 *   1. **断网保险**：把 globalThis.fetch 打成抛错，并记录任何调用尝试。
 *      若代码路径真的需要联网，这里会立刻炸——而不是"看起来没连"。
 *   2. **功能实测**：调用 seed_to_descriptor / Wallet.create / reveal_next_address。
 *   3. **异源对照**：用 bitcoinjs-lib + bip32 独立派生同一 seed 的同一地址，
 *      逐字符比对。**不满足于"它打印了什么"**。
 *
 * 边界声明（须原样写进报告）：
 *   · 用的是**已发布的 npm 预构建包** @bitcoindevkit/bdk-wallet-node@0.3.0，
 *     不是从源码构建 —— 而预构建包经二进制检查**含 esplora/reqwest（联网代码）**。
 *     故本测试证明的是"**这些 API 本身不触网**"，不是"该产物整体无联网代码"。
 *   · 用的是合成 testnet 种子，无真实资金、无链上数据。
 *
 * 用法：node g1_q3_offline.mjs
 */
import { createRequire } from "module";
import path from "path";
import fs from "fs";
import { pathToFileURL } from "url";

const require = createRequire(import.meta.url);
const MODS = process.env.CARAVAN_MODULES || "D:/work/_g3-caravan/node_modules/";
const b32 = require(MODS + "bip32");
const btc = require(MODS + "bitcoinjs-lib");

const PKG_ROOT = process.env.G1_PKG_ROOT || import.meta.dirname;
const PKG = path.join(PKG_ROOT, "x-node", "package", "bitcoindevkit.js");
const lines = [];
const w = (s) => {
  lines.push(s);
  console.log(s);
};

// ── 1. 断网保险 ────────────────────────────────────────────────────────────
const netAttempts = [];
globalThis.fetch = (...a) => {
  netAttempts.push(String(a[0]));
  throw new Error("NETWORK BLOCKED by test harness");
};
globalThis.XMLHttpRequest = class {
  constructor() {
    netAttempts.push("XMLHttpRequest");
    throw new Error("NETWORK BLOCKED by test harness");
  }
};

// 同一把合成种子（与 G3 补测同源、确定性、无真实资金）
const SEED_HEX = "0a1b2c3d4e5f60718293a4b5c6d7e8f90011223344556677889900112233445566";

w("G1 · Q2/Q3 实测 —— BDK-wasm 离线能力");
w("时间: " + new Date().toISOString());
w("包: @bitcoindevkit/bdk-wallet-node@0.3.0（已发布预构建包）");
w("文件: " + PKG + "  (" + fs.statSync(PKG).size + " 字节)");
const wasmPath = path.join(PKG_ROOT, "x-node", "package", "bitcoindevkit_bg.wasm");
w("wasm: bitcoindevkit_bg.wasm  (" + fs.statSync(wasmPath).size + " 字节)");
w("=".repeat(74));

const bdk = await import(pathToFileURL(PKG).href);
w("");
w("【Q2-1】模块加载（在 fetch 被打死的前提下）");
w("  加载成功 ✅   导出符号数: " + Object.keys(bdk).length);
w("  含 EsploraClient 导出: " + (typeof bdk.EsploraClient === "function" ? "是（类存在）" : "否"));

// ── 2. 描述符（离线） ──────────────────────────────────────────────────────
w("");
w("【Q3-1】seed_to_descriptor（离线构造描述符）");
const seed = new Uint8Array(Buffer.from(SEED_HEX, "hex"));
const dp = bdk.seed_to_descriptor(seed, "testnet", "p2wpkh");
w("  external: " + dp.external);
w("  internal: " + dp.internal);

w("");
w("【Q3-2】xpub_to_descriptor（从 xpub 构造，离线）");
const master = b32.fromSeed(Buffer.from(SEED_HEX, "hex"));
const fp = master.fingerprint.toString("hex");
const acct = master.derivePath("m/84'/1'/0'");
const xpub = acct.neutered().toBase58();
w("  输入 xpub (" + fp + "): " + xpub.slice(0, 24) + "…");
const dp2 = bdk.xpub_to_descriptor(xpub, fp, "testnet", "p2wpkh");
w("  → external: " + dp2.external);

// ── 3. 钱包 + 地址派生（离线） ─────────────────────────────────────────────
w("");
w("【Q3-3】Wallet.create + reveal_next_address（离线派生地址）");
const wallet = bdk.Wallet.create("testnet", dp.external, dp.internal);
const addr0 = wallet.reveal_next_address("external");
const addr1 = wallet.reveal_next_address("external");
const gotAddress = addr0.address.toString();
w("  external[0]: " + gotAddress);
w("  external[1]: " + addr1.address.toString());

// ── 4. 异源对照 ────────────────────────────────────────────────────────────
w("");
w("【Q3-4】异源对照（bitcoinjs-lib + bip32 独立派生，与 BDK 无共享代码）");
const refKey = master.derivePath("m/84'/1'/0'/0/0");
const ref = btc.payments.p2wpkh({ pubkey: refKey.publicKey, network: btc.networks.testnet }).address;
const ref1Key = master.derivePath("m/84'/1'/0'/0/1");
const ref1 = btc.payments.p2wpkh({ pubkey: ref1Key.publicKey, network: btc.networks.testnet }).address;
w("  对照 external[0]: " + ref);
w("  对照 external[1]: " + ref1);
w("  → external[0] 一致: " + (gotAddress === ref ? "✅ 是" : "❌ 否"));
w("  → external[1] 一致: " + (addr1.address.toString() === ref1 ? "✅ 是" : "❌ 否"));

// ── 5. 网络尝试记录 ────────────────────────────────────────────────────────
w("");
w("【Q2-2】网络调用尝试记录");
w("  fetch/XHR 被调用次数: " + netAttempts.length + (netAttempts.length === 0 ? "  → ✅ 全程未尝试联网" : "  → ❌ " + JSON.stringify(netAttempts)));

w("");
w("=".repeat(74));
w("边界声明（原样）：用的是**已发布预构建 npm 包**（含 esplora/reqwest 代码，见二进制检查），");
w("故本测试证明的是「这些 API 本身不触网」，不是「该产物整体无联网代码」。");
w("用的是合成 testnet 种子，无真实资金、无链上数据。");

fs.writeFileSync(path.join(process.env.G1_OUT_DIR || import.meta.dirname, "g1-q3-offline.txt"), lines.join("\n") + "\n");
console.log("\n[written] g1-q3-offline.txt");
