/*
 * G1 · Q2/Q4 实测：**无打包器（bundler）**前提下，浏览器能否直接加载 web 包
 * --------------------------------------------------------------------------
 * 依据：g1/G1-预注册-BDK-WASM离线能力实测.md §3（Q2/Q4）、§5（F2/F4）
 *
 * 为什么这一问是关键的：
 *   我们的网页端 web/ 是**零工具链**形态（手写 extern "C" wasm，无 npm、无打包器、
 *   纯静态文件 + 一个 serve 脚本）。而已发布的 web 包入口是：
 *       import * as wasm from "./bitcoindevkit_bg.wasm";
 *   —— 这是「ESM 原生 wasm 导入」，不是 fetch 初始化。
 *   它需要 ① 打包器处理 .wasm 导入，或 ② 浏览器支持 WebAssembly ESM integration，
 *   或 ③ 两者皆无 → 直接加载失败。**这一条必须实测，不能假设。**
 *
 * 方法：
 *   · 本地 Node 静态服务器（精确控制 MIME：.wasm → application/wasm）
 *   · headless Chrome（系统 Chrome）+ Playwright
 *   · **拦截并记录一切非本地请求**（Q2）
 *   · 页面用**裸 <script type="module">**，不做任何打包/转译
 *
 * 用法：node g1_q4_browser.mjs
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
<title>G1 Q4 — no-bundler load test</title>
<script type="module">
window.__result = null;
try {
  const bdk = await import('/bitcoindevkit.js');
  const xpub = 'xpub6Cns4aAYhrJGMhdYDw8aCZ7NjVtfpsdEaf36sqsddUY8r9YizQN4Wst79dkWn497482zbKzdT34HchWwVJF2m4DEG2X5mUsWkMv7pNYw9gd';
  const dp = bdk.xpub_to_descriptor(xpub, '8236dc33', 'testnet', 'p2wpkh');
  const wl = bdk.Wallet.create('testnet', dp.external, dp.internal);
  const a = wl.reveal_next_address('external').address.toString();
  window.__result = { ok: true, address: a, exports: Object.keys(bdk).length };
} catch (e) {
  window.__result = { ok: false, error: String((e && e.stack) || e), name: e && e.name };
}
</script>
`;

const MIME = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  // 对照实验：允许用 WASM_MIME 覆盖 .wasm 的 MIME，以区分"能力缺失"与"MIME 门槛"
  ".wasm": process.env.WASM_MIME || "application/wasm",
  ".json": "application/json",
  ".d.ts": "text/plain",
  ".md": "text/plain",
};

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

await new Promise((r) => server.listen(4390, "127.0.0.1", r));
const BASE = "http://127.0.0.1:4390";

w("G1 · Q2/Q4 实测 —— 无打包器浏览器加载");
w("时间: " + new Date().toISOString());
w("被检包: @bitcoindevkit/bdk-wallet-web@0.3.0（已发布预构建包，ESM + ESM-wasm-import 形态）");
w("静态服务: " + BASE + "（.wasm → application/wasm）");
w("浏览器: " + (fs.existsSync(CHROME) ? CHROME : "(playwright bundled)"));
w("=".repeat(74));

const external = [];
const localReqs = [];
const consoleMsgs = [];

const browser = await chromium.launch({ executablePath: fs.existsSync(CHROME) ? CHROME : undefined, headless: true });
const ctx = await browser.newContext();
const page = await ctx.newPage();

page.on("console", (m) => consoleMsgs.push(`[${m.type()}] ${m.text()}`));
page.on("pageerror", (e) => consoleMsgs.push(`[pageerror] ${e.message}`));
page.on("requestfailed", (r) => consoleMsgs.push(`[requestfailed] ${r.url()} :: ${r.failure()?.errorText}`));

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
await page.screenshot({ path: path.join(process.env.G1_OUT_DIR || import.meta.dirname, "g1-q4-browser.png"), fullPage: true });
await browser.close();
server.close();

w("");
w("【Q4-1】裸 <script type=\"module\"> 加载结果");
if (result && result.ok) {
  w("  ✅ 加载成功（无需打包器）");
  w("  派生地址: " + result.address);
  w("  导出符号数: " + result.exports);
} else {
  w("  ❌ 加载失败");
  w("  name : " + (result && result.name));
  w("  error: " + String(result && result.error).split("\n").slice(0, 6).join("\n         "));
}

w("");
w("【Q4-2】本地请求清单（应当只有模块与 wasm 本身）");
localReqs.forEach((u) => w("  " + u));
w("  合计 " + localReqs.length + " 个本地请求");

w("");
w("【Q2-3】外部请求（应全被拦截，且不应出现任何联网尝试）");
w("  外部请求数: " + external.length + (external.length ? "  → " + JSON.stringify(external.slice(0, 5)) : "  → ✅ 无"));

w("");
w("【Q4-3】控制台 / 页面错误");
if (consoleMsgs.length === 0) w("  （无）");
else consoleMsgs.slice(0, 12).forEach((m) => w("  " + m));

w("");
w("=".repeat(74));
w("边界声明（原样）：用的是**已发布预构建 npm 包**（二进制含 esplora/reqwest 代码）；");
w("本测试只回答「该产物能否在无打包器下被浏览器加载」，不评价其内部实现。");

fs.writeFileSync(path.join(process.env.G1_OUT_DIR || import.meta.dirname, "g1-q4-browser.txt"), lines.join("\n") + "\n");
console.log("\n[written] g1-q4-browser.txt");
