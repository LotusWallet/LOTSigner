/*
 * G3 实测 · UI 探测脚本
 *
 * 用途：以真实浏览器加载本地构建的 Caravan coordinator，抓取各路由的可见控件与文本，
 *       作为差异项判定的原始输出（A 级证据：实测遇到的界面事实）。
 *
 * 说明：
 *   - 不做任何断言，只记录事实。判定由人工依据预注册文档作出。
 *   - 使用系统 Chrome（executablePath），规避 ms-playwright 缓存浏览器版本不匹配。
 *   - 每个路由输出：标题、控件清单（按钮/链接/输入）、正文摘要、截图。
 *
 * 运行：
 *   export PATH="/c/Users/simon/.workbuddy/binaries/node/versions/24.21.0:$PATH"
 *   export NODE_PATH="C:/Users/simon/.caravan-g3/node_modules"
 *   node probe_ui.js
 */

const { chromium } = require("playwright");
const fs = require("fs");
const path = require("path");

const BASE = "http://127.0.0.1:4173";
const OUT = "D:/work/LOTSigner/g3/evidence";
const CHROME = "C:/Program Files/Google/Chrome/Application/chrome.exe";

// 探测目标：首页 + 各路由。/#/xxx 为 Caravan 的 HashRouter 路由。
const ROUTES = [
  { p: "/", name: "00-landing" },
  { p: "/#/setup", name: "01-setup" },
  { p: "/#/wallet", name: "02-wallet" },
  { p: "/#/script", name: "03-script-explorer" },
  { p: "/#/test", name: "04-testsuite" },
  { p: "/#/address", name: "05-create-address" },
];

(async () => {
  fs.mkdirSync(OUT, { recursive: true });

  const browser = await chromium.launch({
    executablePath: fs.existsSync(CHROME) ? CHROME : undefined,
    headless: true,
  });
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
  const page = await ctx.newPage();

  const consoleErrors = [];
  const requests = [];
  page.on("console", m => {
    if (m.type() === "error") consoleErrors.push(m.text().slice(0, 200));
  });
  page.on("request", r => {
    const u = r.url();
    // 只记录非本地、非静态资源的请求，用于判定 ④ 离线可用性
    if (!u.startsWith(BASE) && !/\.(js|css|png|webp|wasm|svg|woff2?)$/i.test(u)) {
      requests.push(u);
    }
  });

  const report = { base: BASE, routes: [], externalRequests: [], consoleErrors: [] };

  for (const r of ROUTES) {
    console.log("\n=============== " + r.p + " ===============");
    try {
      await page.goto(BASE + r.p, { waitUntil: "domcontentloaded", timeout: 45000 });
      await page.waitForTimeout(3500); // SPA 渲染 + wasm 初始化
    } catch (e) {
      console.log("  导航失败: " + e.message.slice(0, 160));
      report.routes.push({ route: r.p, error: e.message.slice(0, 200) });
      continue;
    }

    const info = await page.evaluate(() => {
      const txt = s => (s || "").trim();
      const controls = Array.from(
        document.querySelectorAll(
          'button, a[href], [role="button"], input, textarea, [class*="MuiButton"], [class*="MuiTab"]'
        )
      )
        .map(e => {
          const label =
            txt(e.innerText) ||
            txt(e.getAttribute("aria-label")) ||
            txt(e.placeholder) ||
            txt(e.getAttribute("href")) ||
            txt(e.type) ||
            e.tagName;
          return label.replace(/\s+/g, " ").slice(0, 60);
        })
        .filter(Boolean);
      const body = txt(document.body.innerText).replace(/\n{2,}/g, "\n");
      return {
        title: document.title,
        controls: [...new Set(controls)].slice(0, 70),
        bodyHead: body.slice(0, 1200),
        // 是否存在文件上传输入（PSBT 导入的关键信号）
        fileInputs: Array.from(document.querySelectorAll('input[type="file"]')).map(i =>
          txt(i.accept)
        ),
      };
    });

    const shot = path.join(OUT, r.name + ".png");
    await page.screenshot({ path: shot, fullPage: false });

    console.log("  title: " + info.title);
    console.log("  文件输入 accept: " + JSON.stringify(info.fileInputs));
    console.log("  控件 (" + info.controls.length + "): " + info.controls.join(" | "));
    console.log("  --- 正文摘要 ---");
    console.log(
      info.bodyHead
        .split("\n")
        .slice(0, 14)
        .map(l => "  " + l.slice(0, 110))
        .join("\n")
    );

    report.routes.push({ route: r.p, screenshot: r.name + ".png", ...info });
  }

  report.externalRequests = [...new Set(requests)];
  report.consoleErrors = [...new Set(consoleErrors)].slice(0, 20);

  console.log("\n=============== 外部网络请求（判定④用）===============");
  console.log(report.externalRequests.length ? report.externalRequests.join("\n") : "  （无外部请求）");

  fs.writeFileSync(
    path.join(OUT, "probe-report.json"),
    JSON.stringify(report, null, 2),
    "utf8"
  );
  console.log("\n报告已写入: " + path.join(OUT, "probe-report.json"));

  await browser.close();
})();
