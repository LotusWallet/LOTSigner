/*
 * G3 补测 · 阶段 D：自建钱包 → 读取地址 → mock 链上数据 → 走到「构建支出」
 *
 * 说明：本阶段的链上数据（UTXO/余额/费率）是 **mock** 的（合成资交易，无真实资金）；
 * Caravan 的构建/展示/导出路径是**真实代码**。此声明须原样写入补测报告。
 */
const path = require("path");
const fs = require("fs");
const M = process.env.CARAVAN_MODULES || "D:/work/_g3-caravan/node_modules/";
const { chromium } = M + "playwright" && require(M + "playwright");
const btc = require(M + "bitcoinjs-lib");
const { buildWalletConfig } = require("./g3c_probe_export.js");

const BASE = process.env.G3C_BASE || "http://localhost:4174";
const OUT = path.join(__dirname, "..", "evidence");
const CHROME = "C:/Program Files/Google/Chrome/Application/chrome.exe";

// mock 状态（先声明，读到地址后再填）
const mock = {
  address: null,
  utxos: [],
  fundingTxHex: null,
  fundingTxid: null,
  hits: [],
};

function buildFunding(address, valueSats) {
  const p2sh = btc.payments.p2sh({ address, network: btc.networks.bitcoin });
  const tx = new btc.Transaction();
  tx.version = 2;
  tx.addInput(Buffer.alloc(32, 0x22), 0, 0xffffffff, Buffer.from("03beef01", "hex"));
  tx.addOutput(p2sh.output, valueSats);
  tx.locktime = 0;
  return { txid: tx.getId(), hex: tx.toHex() };
}

(async () => {
  fs.mkdirSync(OUT, { recursive: true });
  const config = buildWalletConfig();
  const configPath = path.join(OUT, "g3c-config.json");
  fs.writeFileSync(configPath, JSON.stringify(config, null, 2));

  const browser = await chromium.launch({
    executablePath: fs.existsSync(CHROME) ? CHROME : undefined,
    headless: true,
  });
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 1400 }, acceptDownloads: true });
  const page = await ctx.newPage();

  await page.route("**/*", async (route) => {
    const url = route.request().url();
    if (url.startsWith(BASE) || url.startsWith("data:") || url.startsWith("blob:")) return route.continue();
    mock.hits.push(url);
    const u = new URL(url);
    const p = u.pathname;

    // Esplora 兼容接口（unchained.mempool.space / blockstream.info）
    if (mock.address) {
      if (p === `/api/address/${mock.address}/utxo` || p === `/address/${mock.address}/utxo`) {
        return route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify(mock.utxos),
        });
      }
      if (p === `/api/address/${mock.address}` || p === `/address/${mock.address}`) {
        const funded = mock.utxos.reduce((a, x) => a + x.value, 0);
        return route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            address: mock.address,
            chain_stats: {
              funded_txo_count: mock.utxos.length,
              funded_txo_sum: funded,
              spent_txo_count: 0,
              spent_txo_sum: 0,
              tx_count: mock.utxos.length,
            },
            mempool_stats: { funded_txo_count: 0, funded_txo_sum: 0, spent_txo_count: 0, spent_txo_sum: 0, tx_count: 0 },
          }),
        });
      }
    }
    if (p.endsWith("/blocks/tip/height")) {
      return route.fulfill({ status: 200, contentType: "text/plain", body: "900000" });
    }
    if (p.endsWith("/fee-estimates")) {
      return route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ "1": 2, "2": 2, "3": 2, "4": 2, "5": 2, "6": 1, "10": 1, "144": 1, "1008": 1 }),
      });
    }
    if (mock.fundingTxid && (p === `/api/tx/${mock.fundingTxid}/hex` || p === `/tx/${mock.fundingTxid}/hex`)) {
      return route.fulfill({ status: 200, contentType: "text/plain", body: mock.fundingTxHex });
    }
    if (mock.fundingTxid && (p === `/api/tx/${mock.fundingTxid}` || p === `/tx/${mock.fundingTxid}`)) {
      return route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          txid: mock.fundingTxid,
          version: 2,
          locktime: 0,
          vin: [],
          vout: [{ scriptpubkey: "", value: 200000 }],
          size: 100,
          weight: 400,
          fee: 0,
          status: { confirmed: true, block_height: 899000, block_hash: "00".repeat(32), block_time: 1700000000 },
        }),
      });
    }
    // 其余一律拒绝（离线边界）
    return route.abort();
  });

  const dump = async (tag) => {
    const s = await page.evaluate(() => {
      const t = (x) => (x || "").trim().replace(/\s+/g, " ");
      return {
        url: location.href,
        body: t(document.body.innerText).slice(0, 5000),
        buttons: Array.from(document.querySelectorAll("button")).map((b) => t(b.innerText)).filter(Boolean),
        inputs: Array.from(document.querySelectorAll("input,textarea,select")).map(
          (i) => `${i.tagName}:${i.type || ""}:${i.name || i.id || i.getAttribute("placeholder") || ""}`
        ),
        alerts: Array.from(document.querySelectorAll('[class*="MuiAlert"],[role="alert"]'))
          .map((e) => t(e.innerText).slice(0, 300))
          .filter(Boolean),
      };
    });
    await page.screenshot({ path: path.join(OUT, `g3c-D-${tag}.png`), fullPage: true });
    return s;
  };

  const steps = {};
  await page.goto(`${BASE}/#/wallet`, { waitUntil: "domcontentloaded" });
  await page.waitForTimeout(2500);

  // 导入钱包配置（文件）
  const cfgInput = page.locator('input[type="file"]').first();
  await cfgInput.setInputFiles(configPath);
  await page.waitForTimeout(3500);
  steps.afterConfig = await dump("afterConfig");
  console.log("=== 配置导入后 ===");
  console.log(steps.afterConfig.body.slice(0, 900));
  console.log("buttons:", steps.afterConfig.buttons.join(" | "));

  // 完成钱包确认（Caravan 需要 Confirm 才把钱包置为可用）
  const confirm = page.getByRole("button", { name: /^confirm$/i }).first();
  if (await confirm.count()) {
    await confirm.click().catch(() => {});
    await page.waitForTimeout(3500);
    steps.afterConfirm = await dump("afterConfirm");
    console.log("=== Confirm 后 ===");
    console.log(steps.afterConfirm.body.slice(0, 1200));
    console.log("buttons:", steps.afterConfirm.buttons.join(" | "));
    const tabs = await page.locator('[role="tab"]').allInnerTexts();
    console.log("tabs:", tabs.join(" | "));
  } else {
    console.log("(未找到 Confirm 按钮)");
  }

  // 找地址：钱包页/收款页
  const bodyText = steps.afterConfig.body;
  const addrMatch = bodyText.match(/(bc1[a-z0-9]{20,}|[23][1-9A-HJ-NP-Za-km-z]{25,40})/);
  console.log("地址候选:", addrMatch ? addrMatch[0] : "(未在文本中发现)");

  // 尝试进入 Spend（不同版本命名不同：Spend / Send）
  for (const label of ["Spend", "Send", "Receive"]) {
    const tab = page.getByRole("tab", { name: new RegExp(label, "i") }).first();
    const btn = page.getByRole("button", { name: new RegExp(label, "i") }).first();
    const target = (await tab.count()) ? tab : (await btn.count()) ? btn : null;
    if (target) {
      await target.click().catch(() => {});
      await page.waitForTimeout(2500);
      steps[`after-${label}`] = await dump(`after-${label}`);
      console.log(`=== ${label} ===`);
      console.log(steps[`after-${label}`].body.slice(0, 800));
      console.log("inputs:", steps[`after-${label}`].inputs.join(" | "));
    } else {
      console.log(`(${label} 入口未找到)`);
    }
  }

  fs.writeFileSync(path.join(OUT, "g3c-D-result.json"), JSON.stringify({ mock, steps }, null, 2));
  console.log("被拦截的外部请求:", mock.hits.length);
  await browser.close();
})().catch((e) => {
  console.error("PROBE ERROR:", e.stack ? e.stack.split("\n").slice(0, 3).join(" | ") : e.message);
  process.exit(1);
});
