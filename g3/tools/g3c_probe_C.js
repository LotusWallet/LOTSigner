/*
 * G3 补测 · 阶段 C：把「部分签名 PSBT」交给 Caravan 的 PSBT 入口，观察其导出
 *
 * 观测点（对应预注册 §3 的 ③ 判定标准）：
 *   - 导入部分签名 PSBT 后，界面是否提供"导出/下载 PSBT"（供发起方继续收集签名）
 *   - 若提供，导出的 PSBT 是否保留已收集的签名、其余字段是否完整
 *   - 导出物能否被第三方工具消费（随后用 lscli / bitcoinjs-lib 验证）
 *
 * 只记录观测事实；判定在报告中由人填写。
 */
const path = require("path");
const fs = require("fs");
const MODS = process.env.CARAVAN_MODULES || "D:/work/_g3-caravan/node_modules/";
const { chromium } = require(MODS + "playwright");

const BASE = process.env.G3C_BASE || "http://localhost:4174";
const OUT = path.join(__dirname, "..", "evidence");
const CHROME = "C:/Program Files/Google/Chrome/Application/chrome.exe";
const FIXTURES = path.join(__dirname, "..", "fixtures");

const FIXTURE = process.argv[2] || "S3"; // S3 = 含 1 个 partial_sig 的 2-of-3 P2WSH
const ROUTE = process.argv[3] || "hermit-psbt";

(async () => {
  fs.mkdirSync(OUT, { recursive: true });
  const browser = await chromium.launch({
    executablePath: fs.existsSync(CHROME) ? CHROME : undefined,
    headless: true,
  });
  const ctx = await browser.newContext({
    viewport: { width: 1440, height: 1400 },
    acceptDownloads: true,
  });
  const page = await ctx.newPage();

  const external = [];
  const downloads = [];
  page.on("download", async (d) => {
    const target = path.join(OUT, `g3c-C-download-${d.suggestedFilename()}`);
    await d.saveAs(target);
    downloads.push({ filename: d.suggestedFilename(), saved: target, size: fs.statSync(target).size });
    console.log("DOWNLOAD:", d.suggestedFilename(), fs.statSync(target).size, "bytes");
  });

  await page.route("**/*", async (route) => {
    const url = route.request().url();
    if (url.startsWith(BASE) || url.startsWith("data:") || url.startsWith("blob:")) return route.continue();
    external.push(url);
    return route.abort();
  });

  const dump = async (tag) => {
    const s = await page.evaluate(() => {
      const t = (x) => (x || "").trim().replace(/\s+/g, " ");
      return {
        url: location.href,
        body: t(document.body.innerText).slice(0, 4000),
        buttons: Array.from(document.querySelectorAll("button")).map((b) => t(b.innerText)).filter(Boolean),
        alerts: Array.from(document.querySelectorAll('[class*="MuiAlert"],[role="alert"],[class*="error"]'))
          .map((e) => t(e.innerText).slice(0, 300))
          .filter(Boolean),
      };
    });
    await page.screenshot({ path: path.join(OUT, `g3c-C-${tag}.png`), fullPage: true });
    fs.writeFileSync(path.join(OUT, `g3c-C-${tag}.json`), JSON.stringify(s, null, 2));
    return s;
  };

  const steps = {};

  await page.goto(`${BASE}/#/${ROUTE}`, { waitUntil: "domcontentloaded" });
  await page.waitForTimeout(2500);
  steps.before = await dump("before");

  // 选择 P2WSH + Mainnet（与 S3 固件一致）
  const p2wsh = page.locator('input[type="radio"][value="P2WSH"]').first();
  if (await p2wsh.count()) await p2wsh.check().catch(() => {});
  await page.waitForTimeout(500);

  // 导入 PSBT 文件
  const fileInput = page.locator('input[type="file"]').first();
  if (await fileInput.count()) {
    await fileInput.setInputFiles(path.join(FIXTURES, `${FIXTURE}.psbt`));
    await page.waitForTimeout(4000);
    steps.imported = await dump("imported");
    console.log("=== after import ===");
    console.log("body:", steps.imported.body.slice(0, 1200));
    console.log("buttons:", steps.imported.buttons.join(" | "));
    console.log("alerts:", steps.imported.alerts.join(" || "));
  } else {
    console.log("(!) 未找到文件输入");
  }

  // 若有导出类按钮，逐个点开观察（名称含 download/export）
  const exportButtons = page.locator("button", { hasText: /download|export/i });
  const n = await exportButtons.count();
  console.log("export-ish buttons:", n);
  for (let i = 0; i < n; i++) {
    const b = exportButtons.nth(i);
    const label = (await b.innerText().catch(() => "")).trim();
    try {
      await b.click({ timeout: 5000 });
      await page.waitForTimeout(3000);
      steps[`after-click-${i}`] = await dump(`after-click-${i}`);
      console.log(`clicked #${i}: ${label}`);
    } catch (e) {
      console.log(`click #${i} failed (${label}): ${e.message.slice(0, 80)}`);
    }
  }

  fs.writeFileSync(
    path.join(OUT, `g3c-C-result-${FIXTURE}.json`),
    JSON.stringify({ route: ROUTE, fixture: FIXTURE, external, downloads, steps }, null, 2),
  );
  console.log("blocked external:", external.length, "| downloads:", downloads.length);
  await browser.close();
})().catch((e) => {
  console.error("PROBE ERROR:", e.message);
  process.exit(1);
});
