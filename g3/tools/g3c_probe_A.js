/*
 * G3 补测 · 阶段 A：配置注入冒烟测试
 * 目的：确认自建钱包配置能通过 sessionStorage 注入并被 Caravan 接受，
 *       并把首地址从 UI 读回来（后续据此造资交易与 mock 链上数据）。
 * 只做观测与截图，不下任何结论。
 */
const path = require("path");
const fs = require("fs");
const MODS = process.env.CARAVAN_MODULES || "D:/work/_g3-caravan/node_modules/";
const { chromium } = require(MODS + "playwright");
const { buildWalletConfig } = require("./g3c_probe_export.js");

const BASE = process.env.G3C_BASE || "http://localhost:4174";
const OUT = path.join(__dirname, "..", "evidence");
const CHROME = "C:/Program Files/Google/Chrome/Application/chrome.exe";
const CONFIG_KEY = "caravan_config";

const txt = (s) => (s || "").trim().replace(/\s+/g, " ");

(async () => {
  fs.mkdirSync(OUT, { recursive: true });
  const config = buildWalletConfig();

  const browser = await chromium.launch({
    executablePath: fs.existsSync(CHROME) ? CHROME : undefined,
    headless: true,
  });
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 1200 } });
  const page = await ctx.newPage();

  const requests = [];
  await page.route("**/*", async (route) => {
    const url = route.request().url();
    if (url.startsWith(BASE)) return route.continue();
    requests.push(url);
    return route.abort();
  });

  await page.goto(BASE, { waitUntil: "domcontentloaded" });
  await page.evaluate(
    ([k, v]) => window.sessionStorage.setItem(k, v),
    [CONFIG_KEY, JSON.stringify(config)],
  );
  await page.reload({ waitUntil: "domcontentloaded" });
  await page.waitForTimeout(3000);

  const steps = [];
  const dump = async (tag) => {
    const s = await page.evaluate(() => {
      const t = (x) => (x || "").trim().replace(/\s+/g, " ");
      return {
        url: location.href,
        body: t(document.body.innerText).slice(0, 2000),
        tabs: Array.from(document.querySelectorAll('[role="tab"]')).map((e) => t(e.innerText)),
        buttons: Array.from(document.querySelectorAll("button")).map((b) => t(b.innerText)).filter(Boolean),
      };
    });
    await page.screenshot({ path: path.join(OUT, `g3c-A-${tag}.png`), fullPage: false });
    steps.push({ tag, ...s });
    return s;
  };

  await dump("landing");

  // 进入应用：点第一个 "Get started"
  const gs = page.getByRole("button", { name: /get started/i }).first();
  if (await gs.count()) {
    await gs.click();
    await page.waitForTimeout(2500);
    await dump("after-getstarted");
  }

  const info = await page.evaluate(() => {
    const t = (s) => (s || "").trim().replace(/\s+/g, " ");
    return {
      body: t(document.body.innerText).slice(0, 2500),
      tabs: Array.from(document.querySelectorAll('[role="tab"]')).map((e) => t(e.innerText)),
      buttons: Array.from(document.querySelectorAll("button")).map((b) => t(b.innerText)).filter(Boolean).slice(0, 40),
      links: Array.from(document.querySelectorAll("a")).map((a) => t(a.innerText) + " -> " + a.getAttribute("href")).slice(0, 20),
      configError: t(document.body.innerText).includes("Invalid") || t(document.body.innerText).includes("error"),
    };
  });

  await page.screenshot({ path: path.join(OUT, "g3c-A-smoke.png"), fullPage: false });

  const result = { base: BASE, injectedConfigOk: !info.configError, requests, steps, ...info };
  fs.writeFileSync(
    path.join(OUT, "g3c-A-smoke.json"),
    JSON.stringify(result, null, 2),
  );
  console.log("blocked external requests:", requests.length);
  console.log("tabs:", info.tabs.join(" | "));
  console.log("buttons:", info.buttons.join(" | ").slice(0, 400));
  console.log("body head:", info.body.slice(0, 400));

  await browser.close();
})().catch((e) => {
  console.error("PROBE ERROR:", e.message);
  process.exit(1);
});
