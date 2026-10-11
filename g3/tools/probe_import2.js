/*
 * G3 实测 · 差异项①判定探针（参数化版）
 *
 * 用法：node probe_import2.js <样本名> [端口]
 *   node probe_import2.js S4      → 测 P2SH 对照样本
 *   node probe_import2.js S1      → 测 P2WSH 受托样本
 *
 * 为什么参数化：差异项①要判的是"免配置导入"是否成立，
 * 而 Caravan 的解析分支可能因 witnessScript / redeemScript 不同而不同。
 * 只测一个样本得出的结论不可靠 —— 必须覆盖两类脚本类型再下判。
 *
 * 记录原则（预注册 §8）：只记录功能事实，不做结论。
 * 结论在判定表中由人填写。
 */

const { chromium } = require("playwright");
const fs = require("fs");
const path = require("path");

const NAME = process.argv[2] || "S4";
const PORT = process.argv[3] || "4174";
const BASE = `http://127.0.0.1:${PORT}`;
const OUT = "D:/work/LOTSigner/g3/evidence";
const CHROME = "C:/Program Files/Google/Chrome/Application/chrome.exe";
const FIXTURE = `D:/work/LOTSigner/g3/fixtures/${NAME}.psbt`;

const snap = async (page, tag) => {
  const info = await page.evaluate(() => {
    const txt = (s) => (s || "").trim();
    return {
      body: txt(document.body.innerText).replace(/\n{2,}/g, "\n").slice(0, 3000),
      alerts: Array.from(
        document.querySelectorAll(
          '[class*="MuiAlert"], [role="alert"], .error, [class*="error"]'
        )
      )
        .map((e) => txt(e.innerText).replace(/\s+/g, " ").slice(0, 220))
        .filter(Boolean),
      buttons: Array.from(document.querySelectorAll("button"))
        .map((b) => txt(b.innerText).replace(/\s+/g, " ").slice(0, 40))
        .filter(Boolean),
    };
  });
  await page.screenshot({ path: path.join(OUT, tag + ".png"), fullPage: false });
  return { tag, screenshot: tag + ".png", ...info };
};

(async () => {
  if (!fs.existsSync(FIXTURE)) {
    console.error("固件不存在: " + FIXTURE);
    process.exit(2);
  }
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
  page.on("console", (m) => {
    if (m.type() === "error") errs.push(m.text().slice(0, 300));
  });
  page.on("pageerror", (e) => errs.push("PAGEERROR: " + e.message.slice(0, 300)));

  const out = { fixture: NAME + ".psbt", base: BASE, steps: [] };

  // ---- 步骤 1：直接进入 Script Explorer，零钱包配置 ----
  await page.goto(BASE + "/#/script", {
    waitUntil: "domcontentloaded",
    timeout: 45000,
  });
  await page.waitForTimeout(4000);
  out.steps.push({ step: "1-进入ScriptExplorer(未配置钱包)", ...(await snap(page, `20-${NAME}-before`)) });

  // ---- 步骤 2：两条导入路径都试 ----
  const inputs = await page.locator('input[type="file"]').all();
  out.fileInputCount = inputs.length;
  out.fileInputAccepts = [];
  for (const fi of inputs) {
    out.fileInputAccepts.push(await fi.getAttribute("accept"));
  }

  let route = "none";
  const candidates = inputs.filter((_) => true);
  // 优先 accept 含 base64 的；没有就取第一个
  let target = null;
  for (const fi of candidates) {
    const a = (await fi.getAttribute("accept")) || "";
    if (a.includes("base64")) { target = fi; route = "file-input(base64)"; break; }
  }
  if (!target && candidates.length) { target = candidates[0]; route = "file-input(first)"; }

  if (target) {
    await target.setInputFiles(FIXTURE);
    out.route = route;
    await page.waitForTimeout(6000);
    out.steps.push({ step: "2-上传" + NAME + ".psbt之后", ...(await snap(page, `21-${NAME}-after`)) });
  } else {
    out.route = "no-file-input";
  }

  // ---- 步骤 3：若有显式 Import PSBT 按钮，点击 ----
  try {
    const btn = page.locator('button:has-text("Import PSBT")').first();
    if (await btn.count()) {
      await btn.click();
      await page.waitForTimeout(4000);
      out.steps.push({ step: "3-点击Import PSBT按钮后", ...(await snap(page, `22-${NAME}-click`)) });
    } else {
      out.importButtonPresent = false;
    }
  } catch (e) {
    out.clickError = String(e.message).slice(0, 200);
  }

  out.network = [...new Set(net)];
  out.consoleErrors = [...new Set(errs)].slice(0, 15);

  fs.writeFileSync(
    path.join(OUT, `import-report-${NAME}.json`),
    JSON.stringify(out, null, 2),
    "utf8"
  );

  // ---- 控制台摘要 ----
  console.log("==================================================");
  console.log("样本: " + NAME + "   端口: " + PORT + "   路由: " + out.route);
  console.log("文件输入 accept 列表: " + JSON.stringify(out.fileInputAccepts));
  for (const s of out.steps) {
    console.log("\n----- " + s.step + " -----");
    console.log("  alerts : " + JSON.stringify(s.alerts));
    console.log("  buttons: " + (s.buttons || []).join(" | "));
    console.log("  --- body ---");
    console.log(
      (s.body || "")
        .split("\n")
        .slice(0, 26)
        .map((l) => "  " + l.slice(0, 120))
        .join("\n")
    );
  }
  console.log("\n----- 网络请求（非本地、非静态资源）-----");
  console.log(out.network.length ? out.network.join("\n") : "  （无）");
  console.log("\n----- 控制台错误 -----");
  console.log(out.consoleErrors.length ? out.consoleErrors.join("\n") : "  （无）");
  console.log("\n报告: g3/evidence/import-report-" + NAME + ".json");

  await browser.close();
})();
