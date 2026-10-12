/*
 * G3 补测 · 阶段 B：探测 Caravan 的 "PSBT Interface" 与 "Hermit" 入口
 * 目的：找到能把「已签名 PSBT」交回 Caravan 并获得可导出 PSBT 的真实 UI 路径。
 * 只记录观测事实（页面文本、可点按钮、下载产物），不下结论。
 */
const path = require("path");
const fs = require("fs");
const MODS = process.env.CARAVAN_MODULES || "D:/work/_g3-caravan/node_modules/";
const { chromium } = require(MODS + "playwright");

const BASE = process.env.G3C_BASE || "http://localhost:4174";
const OUT = path.join(__dirname, "..", "evidence");
const CHROME = "C:/Program Files/Google/Chrome/Application/chrome.exe";

(async () => {
  fs.mkdirSync(OUT, { recursive: true });
  const browser = await chromium.launch({
    executablePath: fs.existsSync(CHROME) ? CHROME : undefined,
    headless: true,
  });
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 1400 } });
  const page = await ctx.newPage();

  const external = [];
  await page.route("**/*", async (route) => {
    const url = route.request().url();
    if (url.startsWith(BASE) || url.startsWith("data:") || url.startsWith("blob:")) return route.continue();
    external.push(url);
    return route.abort();
  });

  const snap = async (tag) => {
    const s = await page.evaluate(() => {
      const t = (x) => (x || "").trim().replace(/\s+/g, " ");
      return {
        url: location.href,
        body: t(document.body.innerText).slice(0, 3500),
        buttons: Array.from(document.querySelectorAll("button")).map((b) => t(b.innerText)).filter(Boolean),
        inputs: Array.from(document.querySelectorAll("input,textarea")).map((i) => (i.type || "textarea") + ":" + (i.name || i.id || i.getAttribute("placeholder") || "")),
        links: Array.from(document.querySelectorAll("a")).map((a) => t(a.innerText)).filter(Boolean).slice(0, 30),
      };
    });
    await page.screenshot({ path: path.join(OUT, `g3c-B-${tag}.png`), fullPage: true });
    fs.writeFileSync(path.join(OUT, `g3c-B-${tag}.json`), JSON.stringify(s, null, 2));
    return s;
  };

  const steps = {};
  await page.goto(BASE, { waitUntil: "domcontentloaded" });
  await page.waitForTimeout(2500);

  for (const [label, route] of [
    ["hermit-psbt", "/hermit-psbt"],
    ["script", "/script"],
    ["wallet", "/wallet"],
    ["setup", "/setup"],
  ]) {
    await page.goto(BASE + "/#" + route, { waitUntil: "domcontentloaded" });
    await page.waitForTimeout(2500);
    steps[label] = await snap(label);
    console.log(`=== ${label} (${route}) ===`);
    console.log("url:", steps[label].url);
    console.log("body:", steps[label].body.slice(0, 600));
    console.log("buttons:", steps[label].buttons.join(" | "));
    console.log("inputs:", steps[label].inputs.join(" | "));
  }

  fs.writeFileSync(path.join(OUT, "g3c-B-probe.json"), JSON.stringify({ external, steps }, null, 2));
  console.log("blocked external:", external.length);
  await browser.close();
})().catch((e) => {
  console.error("PROBE ERROR:", e.message);
  process.exit(1);
});
