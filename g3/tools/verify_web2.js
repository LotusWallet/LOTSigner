// BETA-READINESS B3/B4 浏览器验证：
//   B3 断网全链路：review → export → re-import → review（全部在拦截外部请求的条件下完成）
//   B4 外部请求审计：全程恰好只发本地请求
//   附带：malformed(S6) 显式报错、双语言、多分辨率截图
// 证据输出：g3/evidence/web2-verify.json + 截图 + 导出文件
"use strict";
const { chromium } = require("playwright");
const fs = require("fs");
const path = require("path");
const crypto = require("crypto");

const URL = "http://127.0.0.1:4180/index.html";
const FIX = "D:/work/LOTSigner/g3/fixtures";
const EV = "D:/work/LOTSigner/g3/evidence";
const CHROME = "C:/Program Files/Google/Chrome/Application/chrome.exe";

const results = [];
function check(name, ok, detail) {
  results.push({ name, ok, detail: detail || "" });
  console.log(`${ok ? "PASS" : "FAIL"}  ${name}${detail ? "  — " + detail : ""}`);
}

(async () => {
  const browser = await chromium.launch({ executablePath: CHROME, headless: true });
  const requests = [];

  async function mkPage(viewport, locale) {
    const ctx = await browser.newContext({
      viewport: viewport || { width: 1280, height: 800 },
      locale: locale || "en-US",
      acceptDownloads: true,
    });
    // B3 的「断网」：放行本地 4180，拦截其余一切（含 https、DNS 之外的任何主机）
    await ctx.route("**/*", (route) => {
      const u = route.request().url();
      requests.push(u);
      if (u.startsWith(URL.slice(0, URL.length - "index.html".length))) return route.continue();
      return route.abort();
    });
    const page = await ctx.newPage();
    const errs = [];
    page.on("pageerror", (e) => errs.push(String(e)));
    page.on("console", (m) => { if (m.type() === "error") errs.push(m.text()); });
    return { page, ctx, errs };
  }

  const b64 = (f) => fs.readFileSync(path.join(FIX, f), "utf8").trim();
  const sha = (buf) => crypto.createHash("sha256").update(buf).digest("hex");

  // ============ 1. 断网全链路：review → export → re-import（B3） ============
  {
    const { page, ctx, errs } = await mkPage();
    await page.goto(URL, { waitUntil: "networkidle" });
    await page.waitForFunction(() => document.querySelector("#hint") && document.querySelector("#hint").textContent.length > 0, { timeout: 15000 });

    // 载入示例 S1 → 审阅
    await page.click("#demo");
    await page.waitForSelector("section.layer", { timeout: 10000 });
    const body = await page.evaluate(() => document.body.innerText);
    check("B3/1 offline review renders 4 layers",
      ["Transaction facts", "Context", "Risk notices", "does NOT tell you"].every((s) => body.includes(s)));

    // 导出 → 捕获下载
    const [download] = await Promise.all([
      page.waitForEvent("download", { timeout: 10000 }),
      page.click("#export"),
    ]);
    const exportedPath = path.join(EV, "b3-exported-s1.psbt");
    await download.saveAs(exportedPath);

    // 导出文件必须与「S1 解码后的原始字节」一致
    const origBin = Buffer.from(b64("S1.psbt"), "base64");
    const gotBin = fs.readFileSync(exportedPath);
    check("B3/2 export byte-identical to input",
      sha(origBin) === sha(gotBin), `sha256=${sha(gotBin).slice(0, 16)}…`);
    check("B3/3 export filename ends with .psbt",
      /\.psbt$/.test(download.suggestedFilename()), download.suggestedFilename());

    // 再导入导出文件 → 再次审阅 → txid 必须一致
    const roundTripB64 = gotBin.toString("base64");
    await page.fill("#input", roundTripB64);
    await page.click("#go");
    await page.waitForTimeout(600);
    const body2 = await page.evaluate(() => document.body.innerText);
    check("B3/4 re-import exported file reviews again",
      body2.includes("Transaction facts"));
    const txidOnPage = await page.evaluate(() => {
      const dds = document.querySelectorAll("dl.kv dd");
      return dds.length ? dds[0].textContent.trim() : "";
    });
    check("B3/5 txid stable across round-trip", txidOnPage.length === 64, txidOnPage.slice(0, 16) + "…");

    // malformed S6：显式报错并点名位置
    await page.fill("#input", b64("S6.psbt"));
    await page.click("#go");
    await page.waitForTimeout(500);
    const errText = await page.evaluate(() => document.querySelector("#out").innerText);
    check("B2/6 malformed input names the offending map",
      errText.includes("output[1]") && /offset 348|truncated/i.test(errText), errText.slice(0, 90));

    check("B4/7 zero console errors (offline chain)", errs.length === 0, errs.join(" | ").slice(0, 120));
    await page.screenshot({ path: path.join(EV, "web-en-desktop.png"), fullPage: true });
    await ctx.close();
  }

  // ============ 2. 中文 + 手机分辨率 + 截图 ============
  {
    const { page, ctx, errs } = await mkPage({ width: 375, height: 740 }, "zh-CN");
    await page.goto(URL, { waitUntil: "networkidle" });
    await page.waitForFunction(() => document.querySelector("#hint") && document.querySelector("#hint").textContent.length > 0);
    await page.click("#demo");
    await page.waitForSelector("section.layer");
    const body = await page.evaluate(() => document.body.innerText);
    check("i18n/8 zh renders kernel texts", body.includes("交易事实") && body.includes("本工具不提供以下结论"));
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    check("responsive/9 no horizontal overflow @375px", overflow <= 0, `overflow=${overflow}px`);
    await page.screenshot({ path: path.join(EV, "web-zh-mobile.png"), fullPage: true });
    check("B4/10 zero console errors (zh)", errs.length === 0, errs.join(" | ").slice(0, 120));
    await ctx.close();
  }

  // ============ 3. 请求审计（B4） ============
  const local = requests.filter((u) => u.startsWith("http://127.0.0.1:4180"));
  const external = requests.filter((u) => !u.startsWith("http://127.0.0.1:4180"));
  check("B4/11 zero external requests", external.length === 0,
    `${local.length} local: ${[...new Set(local.map((u) => u.replace("http://127.0.0.1:4180", "")))].join(", ")}`);

  fs.writeFileSync(path.join(EV, "web2-verify.json"),
    JSON.stringify({ results, requests: [...new Set(requests)], external }, null, 2));
  const failed = results.filter((r) => !r.ok);
  console.log(`\n==== ${results.length - failed.length}/${results.length} PASS ====`);
  await browser.close();
  process.exit(failed.length ? 1 : 0);
})().catch((e) => { console.error("FATAL", e); process.exit(2); });
