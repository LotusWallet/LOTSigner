/*
 * G3 实测 · 差异项②「信息完整度」判定探针
 *
 * 目的：受托签名场景下，用户最需要知道的是"这笔钱要付给谁、付多少、手续费多少"。
 * 本探针在导入 S4(P2SH) 之后遍历后续可达路由，搜索界面是否呈现 PSBT 的输出事实。
 *
 * 期望值（固件 S4，经 bitcoinjs-lib 异源确认）：
 *   out[0] = 1,200,000 sat    out[1] = 290,000 sat    合计 1,490,000 sat
 *   常规表现形式可能为 "1200000" / "0.012" / "290000" / "0.0029" / "0.0149"
 *
 * 预注册纪律：只记录"界面出现了什么/没出现什么"，不代替判定。
 */

const { chromium } = require("playwright");
const fs = require("fs");
const path = require("path");

const PORT = process.argv[2] || "4174";
const BASE = `http://127.0.0.1:${PORT}`;
const OUT = "D:/work/LOTSigner/g3/evidence";
const CHROME = "C:/Program Files/Google/Chrome/Application/chrome.exe";

// 搜索标记：任何一处出现即说明界面呈现了输出事实
const MARKERS = [
  "1200000", "0.012", "290000", "0.0029", "0.0149", "1490000",
  "1,200,000", "290,000",
];

(async () => {
  fs.mkdirSync(OUT, { recursive: true });
  const browser = await chromium.launch({
    executablePath: fs.existsSync(CHROME) ? CHROME : undefined,
    headless: true,
  });
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 1200 } });
  const page = await ctx.newPage();

  const net = [];
  page.on("request", (r) => {
    const u = r.url();
    if (!u.startsWith(BASE) && !/\.(js|css|png|webp|wasm|svg|woff2?)$/i.test(u))
      net.push(u);
  });
  const errs = [];
  page.on("pageerror", (e) => errs.push("PAGEERROR: " + String(e.message).slice(0, 200)));

  const out = { steps: [], markerHits: {} };

  const capture = async (tag, label) => {
    const info = await page.evaluate(() => {
      const txt = (s) => (s || "").trim();
      return {
        body: txt(document.body.innerText).replace(/\n{2,}/g, "\n").slice(0, 4000),
        buttons: Array.from(document.querySelectorAll("button"))
          .map((b) => txt(b.innerText).replace(/\s+/g, " ").slice(0, 40))
          .filter(Boolean),
      };
    });
    await page.screenshot({ path: path.join(OUT, tag + ".png"), fullPage: false });
    const hits = MARKERS.filter((m) => (info.body || "").includes(m));
    out.steps.push({ step: label, screenshot: tag + ".png", hits, body: info.body, buttons: info.buttons });
    return hits;
  };

  // ---- 1. 导入 S4 ----
  await page.goto(BASE + "/#/script", { waitUntil: "domcontentloaded", timeout: 45000 });
  await page.waitForTimeout(4000);
  const inputs = await page.locator('input[type="file"]').all();
  for (const fi of inputs) {
    const a = (await fi.getAttribute("accept")) || "";
    if (a.includes("base64")) {
      await fi.setInputFiles("D:/work/LOTSigner/g3/fixtures/S4.psbt");
      break;
    }
  }
  await page.waitForTimeout(7000);
  await capture("40-scan-after-import", "A-导入S4后(Script Explorer)");

  // ---- 2. 点击 Spend from this address ----
  try {
    const btn = page.locator('button:has-text("Spend from this address")').first();
    if (await btn.count()) {
      await btn.click();
      await page.waitForTimeout(6000);
      await capture("41-scan-after-spend", "B-点击 Spend from this address 后");
    } else {
      out.steps.push({ step: "B-未找到 Spend 按钮", body: "", buttons: [] });
    }
  } catch (e) {
    out.steps.push({ step: "B-点击异常: " + String(e.message).slice(0, 120), body: "", buttons: [] });
  }

  // ---- 3. 遍历其他可达路由，看是否有页面会呈现该 PSBT 的输出 ----
  for (const [route, label] of [
    ["/#/wallet", "C-Wallet 页"],
    ["/#/hermit", "D-Hermit PSBT Interface"],
    ["/#/setup", "E-Setup 页"],
  ]) {
    await page.goto(BASE + route, { waitUntil: "domcontentloaded", timeout: 45000 });
    await page.waitForTimeout(4500);
    await capture("42-scan" + route.replace(/[^a-z]/gi, "-"), label);
  }

  out.network = [...new Set(net)];
  out.pageErrors = [...new Set(errs)];

  fs.writeFileSync(
    path.join(OUT, "scan-report-outputs.json"),
    JSON.stringify(out, null, 2),
    "utf8"
  );

  console.log("==================================================");
  console.log("目标：S4 输出 = 1200000 sat / 290000 sat  合计 1490000 sat");
  for (const s of out.steps) {
    console.log("\n----- " + s.step + " -----");
    console.log("  命中输出标记: " + (s.hits && s.hits.length ? s.hits.join(", ") : "【无】"));
    console.log("  buttons: " + (s.buttons || []).join(" | "));
  }
  console.log("\n----- 外部请求 -----");
  console.log(out.network.length ? out.network.join("\n") : "  （无）");
  console.log("\n----- 页面异常 -----");
  console.log(out.pageErrors.length ? out.pageErrors.join("\n") : "  （无）");
  console.log("\n报告: g3/evidence/scan-report-outputs.json");

  await browser.close();
})();
