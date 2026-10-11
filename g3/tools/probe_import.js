/*
 * G3 实测 · 差异项①关键动作：免配置导入 PSBT
 *
 * 判定要点（预注册 §3 ①）：
 *   目标用户拿到一份"别人递来的"PSBT，能否在不配置钱包（不提供 xpub）的前提下
 *   直接导入并完成审阅。若必须先行配置，则记为工作流阻断。
 *
 * 本脚本只记录事实：
 *   1. 直接进入 /#/script（不做任何钱包配置），上传 S1.psbt
 *   2. 记录上传后界面状态：是否解析成功 / 是否报错 / 是否要求补充 redeem script 或 xpub
 *   3. 记录上传后发生的网络请求（同时服务于差异项④判定）
 *   4. 截图留证
 *
 * 运行：
 *   export PATH="/c/Users/simon/.workbuddy/binaries/node/versions/24.21.0:$PATH"
 *   export NODE_PATH="C:/Users/simon/.caravan-g3/node_modules"
 *   node probe_import.js
 */

const { chromium } = require("playwright");
const fs = require("fs");
const path = require("path");

const BASE = "http://127.0.0.1:4173";
const OUT = "D:/work/LOTSigner/g3/evidence";
const CHROME = "C:/Program Files/Google/Chrome/Application/chrome.exe";
const FIXTURE = "D:/work/LOTSigner/g3/fixtures/S1.psbt";

const snap = async (page, tag) => {
  const info = await page.evaluate(() => {
    const txt = s => (s || "").trim();
    const body = txt(document.body.innerText).replace(/\n{2,}/g, "\n");
    return {
      body: body.slice(0, 2500),
      // 错误提示类元素
      alerts: Array.from(
        document.querySelectorAll('[class*="MuiAlert"], [role="alert"], .error, [class*="error"]')
      )
        .map(e => txt(e.innerText).replace(/\s+/g, " ").slice(0, 200))
        .filter(Boolean),
      textareas: Array.from(document.querySelectorAll("textarea")).map(t =>
        (t.value || "").slice(0, 120)
      ),
      buttons: Array.from(document.querySelectorAll("button"))
        .map(b => txt(b.innerText).replace(/\s+/g, " ").slice(0, 40))
        .filter(Boolean),
    };
  });
  const shot = path.join(OUT, tag + ".png");
  await page.screenshot({ path: shot, fullPage: false });
  return { tag, screenshot: tag + ".png", ...info };
};

(async () => {
  fs.mkdirSync(OUT, { recursive: true });
  const browser = await chromium.launch({
    executablePath: fs.existsSync(CHROME) ? CHROME : undefined,
    headless: true,
  });
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 1100 } });
  const page = await ctx.newPage();

  const net = [];
  page.on("request", r => {
    const u = r.url();
    if (!u.startsWith(BASE) && !/\.(js|css|png|webp|wasm|svg|woff2?)$/i.test(u)) net.push(u);
  });
  const errs = [];
  page.on("console", m => {
    if (m.type() === "error") errs.push(m.text().slice(0, 200));
  });

  const out = { fixture: "S1.psbt", steps: [], network: [], consoleErrors: [] };

  // ---- 步骤 1：直接进入 Script Explorer，不做任何钱包配置 ----
  await page.goto(BASE + "/#/script", { waitUntil: "domcontentloaded", timeout: 45000 });
  await page.waitForTimeout(3500);
  out.steps.push({ step: "1-进入ScriptExplorer(未配置钱包)", ...(await snap(page, "10-script-before-import")) });

  // ---- 步骤 2：定位 PSBT 文件输入并上传 ----
  const fileInputs = await page.locator('input[type="file"]').all();
  console.log("发现文件输入数量: " + fileInputs.length);
  let uploaded = false;
  for (const fi of fileInputs) {
    const accept = await fi.getAttribute("accept");
    console.log("  accept=" + accept);
    if (accept && accept.includes("base64")) {
      await fi.setInputFiles(FIXTURE);
      uploaded = true;
      console.log("  → 已上传 S1.psbt 到该输入");
      break;
    }
  }
  if (!uploaded && fileInputs.length) {
    await fileInputs[0].setInputFiles(FIXTURE);
    uploaded = true;
    console.log("  → 未找到 base64 专用输入，退而使用第一个文件输入");
  }
  out.uploaded = uploaded;

  await page.waitForTimeout(5000); // 等待解析 + 可能的 wasm 初始化
  out.steps.push({ step: "2-上传S1.psbt之后", ...(await snap(page, "11-script-after-import")) });

  // ---- 步骤 3：若存在 Import PSBT 按钮，显式点击后再观察 ----
  try {
    const btn = page.locator('button:has-text("Import PSBT")').first();
    if (await btn.count()) {
      await btn.click();
      await page.waitForTimeout(3000);
      out.steps.push({ step: "3-点击Import PSBT按钮后", ...(await snap(page, "12-after-import-click")) });
    }
  } catch (e) {
    out.clickError = e.message.slice(0, 150);
  }

  out.network = [...new Set(net)];
  out.consoleErrors = [...new Set(errs)].slice(0, 15);

  fs.writeFileSync(
    path.join(OUT, "import-report.json"),
    JSON.stringify(out, null, 2),
    "utf8"
  );

  // 控制台摘要输出
  for (const s of out.steps) {
    console.log("\n=============== " + s.step + " ===============");
    console.log("  alerts: " + JSON.stringify(s.alerts));
    console.log("  buttons: " + (s.buttons || []).join(" | "));
    console.log("  --- body ---");
    console.log(
      (s.body || "")
        .split("\n")
        .slice(0, 22)
        .map(l => "  " + l.slice(0, 110))
        .join("\n")
    );
  }
  console.log("\n=============== 网络请求 ===============");
  console.log(out.network.length ? out.network.join("\n") : "  （无）");
  console.log("\n报告: " + path.join(OUT, "import-report.json"));

  await browser.close();
})();
