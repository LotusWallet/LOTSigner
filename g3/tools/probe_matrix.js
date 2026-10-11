/*
 * G3 实测 · 差异项①细化判定矩阵
 *
 * 用法：node probe_matrix.js <mode> [端口]
 *   mode = type     → 先选 Address Type = P2WSH，再导入 S1（判 S1 失败是否只因未选类型）
 *   mode = offline  → 拦截并丢弃一切外部请求后导入 S4（判离线可用性 · 差异项④）
 *   mode = signed   → 导入 S3（已含 1 个 partial_sig），判能否识别既有签名状态
 *
 * 预注册纪律：只记录事实，不写结论。结论由判定表在事后统一填写。
 */

const { chromium } = require("playwright");
const fs = require("fs");
const path = require("path");

const MODE = process.argv[2] || "type";
const PORT = process.argv[3] || "4174";
const BASE = `http://127.0.0.1:${PORT}`;
const OUT = "D:/work/LOTSigner/g3/evidence";
const CHROME = "C:/Program Files/Google/Chrome/Application/chrome.exe";

const FIXTURE = {
  type: "S1",
  offline: "S4",
  signed: "S3",
}[MODE];

const dump = async (page, tag) => {
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
  fs.mkdirSync(OUT, { recursive: true });
  const browser = await chromium.launch({
    executablePath: fs.existsSync(CHROME) ? CHROME : undefined,
    headless: true,
  });
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 1200 } });
  const page = await ctx.newPage();

  const blocked = [];
  const net = [];
  const errs = [];

  if (MODE === "offline") {
    // 模拟断网：一切非本地请求直接丢弃，模拟离线环境下的行为
    await page.route("**/*", (route) => {
      const u = route.request().url();
      if (u.startsWith(BASE)) return route.continue();
      blocked.push(u);
      return route.abort();
    });
  } else {
    page.on("request", (r) => {
      const u = r.url();
      if (!u.startsWith(BASE) && !/\.(js|css|png|webp|wasm|svg|woff2?)$/i.test(u))
        net.push(u);
    });
  }

  page.on("console", (m) => {
    if (m.type() === "error") errs.push(m.text().slice(0, 300));
  });
  page.on("pageerror", (e) => errs.push("PAGEERROR: " + e.message.slice(0, 300)));

  const out = { mode: MODE, fixture: FIXTURE + ".psbt", steps: [] };

  await page.goto(BASE + "/#/script", {
    waitUntil: "domcontentloaded",
    timeout: 45000,
  });
  await page.waitForTimeout(4000);

  // ---- 模式 type：先把 Address Type 切到 P2WSH ----
  if (MODE === "type") {
    out.steps.push({ step: "0-进入ScriptExplorer", ...(await dump(page, `30-${MODE}-entry`)) });
    // Address Type 是 MUI Select，先点开再选 P2WSH
    try {
      const sel = page.locator('div[role="combobox"], div.MuiSelect-select').first();
      if (await sel.count()) {
        await sel.click();
        await page.waitForTimeout(1200);
        const opt = page.locator('li[role="option"]:has-text("P2WSH")').first();
        if (await opt.count()) {
          await opt.click();
          await page.waitForTimeout(1500);
          out.typeSelected = "P2WSH";
        } else {
          out.typeSelected = "未找到P2WSH选项";
        }
      } else {
        out.typeSelected = "未找到类型选择器";
      }
    } catch (e) {
      out.typeSelected = "切换失败: " + String(e.message).slice(0, 120);
    }
    out.steps.push({ step: "1-已选P2WSH", ...(await dump(page, `31-${MODE}-type-selected`)) });
  }

  // ---- 上传目标固件 ----
  const fpath = `D:/work/LOTSigner/g3/fixtures/${FIXTURE}.psbt`;
  if (!fs.existsSync(fpath)) {
    console.error("固件不存在: " + fpath);
    await browser.close();
    process.exit(2);
  }
  const inputs = await page.locator('input[type="file"]').all();
  let uploaded = false;
  for (const fi of inputs) {
    const a = (await fi.getAttribute("accept")) || "";
    if (a.includes("base64")) {
      await fi.setInputFiles(fpath);
      uploaded = true;
      break;
    }
  }
  if (!uploaded && inputs.length) {
    await inputs[0].setInputFiles(fpath);
    uploaded = true;
  }
  out.uploaded = uploaded;

  await page.waitForTimeout(7000);
  out.steps.push({ step: `2-导入${FIXTURE}后`, ...(await dump(page, `32-${MODE}-after-import`)) });

  out.blockedRequests = [...new Set(blocked)];
  out.networkObserved = [...new Set(net)];
  out.consoleErrors = [...new Set(errs)].slice(0, 15);

  fs.writeFileSync(
    path.join(OUT, `matrix-report-${MODE}.json`),
    JSON.stringify(out, null, 2),
    "utf8"
  );

  console.log("==================================================");
  console.log("MODE=" + MODE + "  FIXTURE=" + FIXTURE + "  uploaded=" + uploaded);
  if (MODE === "type") console.log("typeSelected=" + out.typeSelected);
  for (const s of out.steps) {
    console.log("\n----- " + s.step + " -----");
    console.log("  alerts : " + JSON.stringify(s.alerts));
    console.log("  buttons: " + (s.buttons || []).join(" | "));
    console.log("  --- body ---");
    console.log(
      (s.body || "")
        .split("\n")
        .slice(0, 28)
        .map((l) => "  " + l.slice(0, 120))
        .join("\n")
    );
  }
  console.log("\n----- 外部请求 -----");
  const ext = MODE === "offline" ? out.blockedRequests : out.networkObserved;
  console.log(ext.length ? out.blockedRequests.concat(out.networkObserved).join("\n") : "  （无）");
  console.log("\n----- 控制台错误 -----");
  console.log(out.consoleErrors.length ? out.consoleErrors.join("\n") : "  （无）");
  console.log("\n报告: g3/evidence/matrix-report-" + MODE + ".json");

  await browser.close();
})();
