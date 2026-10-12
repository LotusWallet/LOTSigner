/*
 * G1 · Q4 补充实测：**不引入打包器**，用 ~10 行自定义加载器绕开
 *                  `import * as wasm from "./x.wasm"` 这一形态
 * --------------------------------------------------------------------------
 * 动机（防 F5 外推）：Q4 主实验只证明「**已发布入口**在裸 <script type="module"> 下加载失败」。
 * 若就此写成"BDK-wasm 无法在无打包器下使用"，那是过头结论。必须再问一层：
 *   能不能不改 BDK 产物、只加一小段加载代码，把它跑起来？
 *
 * 做法：wasm-bindgen 的 bg.js 通过 `__wbg_set_wasm(x)` 接收 wasm 导出对象。
 *       于是我们可以自己 fetch + instantiate，把 imports 指向同一个 bg.js 模块，
 *       再把手写的导出对象交给 __wbg_set_wasm —— 完全不需要打包器。
 *
 * 用法：node g1_q4b_altloader.mjs
 */
import { createRequire } from "module";
import path from "path";
import fs from "fs";
import http from "http";

const require = createRequire(import.meta.url);
const MODS = process.env.CARAVAN_MODULES || "D:/work/_g3-caravan/node_modules/";
const { chromium } = require(MODS + "playwright");

const PKG_DIR = process.env.G1_PKG_ROOT ? path.join(process.env.G1_PKG_ROOT, "x-web", "package") : path.join(import.meta.dirname, "x-web", "package");
const CHROME = "C:/Program Files/Google/Chrome/Application/chrome.exe";
const lines = [];
const w = (s) => {
  lines.push(s);
  console.log(s);
};

const HTML = `<!doctype html>
<meta charset="utf-8">
<title>G1 Q4b — custom loader, no bundler</title>
<script type="module">
window.__result = null;
try {
  // ── 自定义加载器（约 10 行，无打包器）──────────────────────────────
  const bg = await import('/bitcoindevkit_bg.js');
  const bytes = await (await fetch('/bitcoindevkit_bg.wasm')).arrayBuffer();
  const { instance } = await WebAssembly.instantiate(bytes, { './bitcoindevkit_bg.js': bg });
  bg.__wbg_set_wasm(instance.exports);
  // 然后就可直接用 bg.js 导出的 API（与入口 bitcoindevkit.js 导出同一批符号）
  const bdk = bg;
  // ────────────────────────────────────────────────────────────────────
  const SEED = Uint8Array.from([0x0a,0x1b,0x2c,0x3d,0x4e,0x5f,0x60,0x71,0x82,0x93,0xa4,0xb5,0xc6,0xd7,0xe8,0xf9,0x00,0x11,0x22,0x33,0x44,0x55,0x66,0x77,0x88,0x99,0x00,0x11,0x22,0x33,0x44,0x55,0x66]);
  const out = {};
  // 阶段 1：与 Q3 完全同一路径（testnet 描述符来自 seed）
  const dp = bdk.seed_to_descriptor(SEED, 'testnet', 'p2wpkh');
  const w1 = bdk.Wallet.create('testnet', dp.external, dp.internal);
  out.testnetAddr = w1.reveal_next_address('external').address.toString();
  // 阶段 2：主网 xpub → 主网描述符（网络必须自洽，否则 BDK 会拒绝）
  const mxpub = 'xpub6Cns4aAYhrJGMhdYDw8aCZ7NjVtfpsdEaf36sqsddUY8r9YizQN4Wst79dkWn497482zbKzdT34HchWwVJF2m4DEG2X5mUsWkMv7pNYw9gd';
  const dp2 = bdk.xpub_to_descriptor(mxpub, '8236dc33', 'bitcoin', 'p2wpkh');
  const w2 = bdk.Wallet.create('bitcoin', dp2.external, dp2.internal);
  out.mainnetAddr = w2.reveal_next_address('external').address.toString();
  window.__result = { ok: true, ...out, exports: Object.keys(bdk).length };
} catch (e) {
  window.__result = { ok: false, error: String((e && e.stack) || e), name: e && e.name };
}
</script>
`;

const MIME = { ".html": "text/html; charset=utf-8", ".js": "text/javascript; charset=utf-8", ".wasm": "application/wasm" };

const server = http.createServer((req, res) => {
  const p = decodeURIComponent(req.url.split("?")[0]);
  if (p === "/" || p === "/index.html") {
    res.writeHead(200, { "Content-Type": MIME[".html"] });
    return res.end(HTML);
  }
  const f = path.join(PKG_DIR, p.replace(/^\/+/, ""));
  if (!f.startsWith(PKG_DIR) || !fs.existsSync(f) || fs.statSync(f).isDirectory()) {
    res.writeHead(404);
    return res.end("not found");
  }
  res.writeHead(200, { "Content-Type": MIME[path.extname(f)] || "application/octet-stream" });
  res.end(fs.readFileSync(f));
});
await new Promise((r) => server.listen(4391, "127.0.0.1", r));
const BASE = "http://127.0.0.1:4391";

w("G1 · Q4 补充实测 —— 无打包器 + 自定义加载器（~10 行）");
w("时间: " + new Date().toISOString());
w("=".repeat(74));

const external = [], localReqs = [], consoleMsgs = [];
const browser = await chromium.launch({ executablePath: fs.existsSync(CHROME) ? CHROME : undefined, headless: true });
const ctx = await browser.newContext();
const page = await ctx.newPage();
page.on("console", (m) => consoleMsgs.push(`[${m.type()}] ${m.text()}`));
page.on("pageerror", (e) => consoleMsgs.push(`[pageerror] ${e.message}`));

await page.route("**/*", async (route) => {
  const url = route.request().url();
  if (url.startsWith(BASE) || url.startsWith("data:") || url.startsWith("blob:")) {
    localReqs.push(url.replace(BASE, ""));
    return route.continue();
  }
  external.push(url);
  return route.abort();
});

await page.goto(BASE + "/", { waitUntil: "load" });
await page.waitForTimeout(4000);
const result = await page.evaluate(() => window.__result);
await browser.close();
server.close();

w("");
w("【Q4b-1】结果");
if (result && result.ok) {
  w("  ✅ 成功（无打包器）");
  w("  testnet 地址: " + result.testnetAddr);
  w("  mainnet 地址: " + result.mainnetAddr);
} else {
  w("  ❌ 失败  name=" + (result && result.name));
  w("  " + String(result && result.error).split("\n").slice(0, 8).join("\n  "));
}
w("");
w("【Q4b-2】本地请求: " + localReqs.join(" , "));
w("【Q4b-3】外部请求数: " + external.length + (external.length ? " → " + JSON.stringify(external) : " → ✅ 无（且未尝试）"));
w("【Q4b-4】控制台/错误: " + (consoleMsgs.length ? consoleMsgs.join(" | ") : "（无）"));
w("");
w("对照：BDK 自己产出的同一地址应为 tb1qqp33kh5v4j7kjpstmtu5maz4qdeu6p095mzns3（见 g1-q3-offline.txt）");
if (result && result.ok) w("→ 与 Q3 结果一致: " + (result.testnetAddr === "tb1qqp33kh5v4j7kjpstmtu5maz4qdeu6p095mzns3" ? "✅ 是" : "❌ 否"));

fs.writeFileSync(path.join(process.env.G1_OUT_DIR || import.meta.dirname, "g1-q4b-altloader.txt"), lines.join("\n") + "\n");
console.log("\n[written] g1-q4b-altloader.txt");
