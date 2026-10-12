/*
 * G3 补测 · 阶段 E：mock 注资 → 构建支出 → 下载「Download Unsigned PSBT」
 *
 * 链上数据为 mock（合成），Caravan 构建/导出为真实路径。此声明须写入报告。
 * 产出：g3c-E-unsigned-*.psbt（Caravan 导出的未签名 PSBT）+ 截图 + JSON 观测记录
 */
const path = require("path");
const fs = require("fs");
const M = process.env.CARAVAN_MODULES || "D:/work/_g3-caravan/node_modules/";
const { chromium } = require(M + "playwright");
const btc = require(M + "bitcoinjs-lib");
const { buildWalletConfig } = require("./g3c_probe_export.js");

const BASE = process.env.G3C_BASE || "http://localhost:4174";
const OUT = path.join(__dirname, "..", "evidence");
const CHROME = "C:/Program Files/Google/Chrome/Application/chrome.exe";

const mock = { targetAddress: null, utxos: [], fundingHex: null, fundingTxid: null, hits: [], unhandled: [] };

function buildFunding(address, valueSats) {
  const p2sh = btc.payments.p2sh({ address, network: btc.networks.bitcoin });
  const tx = new btc.Transaction();
  tx.version = 2;
  tx.addInput(Buffer.alloc(32, 0x33), 0, 0xffffffff, Buffer.from("03cafe01", "hex"));
  tx.addOutput(p2sh.output, valueSats);
  tx.locktime = 0;
  return { txid: tx.getId(), hex: tx.toHex() };
}

const esplora = (route, p) => {
  const addrUtxo = p.match(/^\/(?:api\/)?address\/([^/]+)\/utxo$/);
  if (addrUtxo) {
    const addr = addrUtxo[1];
    const body = addr === mock.targetAddress ? JSON.stringify(mock.utxos) : "[]";
    return route.fulfill({ status: 200, contentType: "application/json", body });
  }
  const addrInfo = p.match(/^\/(?:api\/)?address\/([^/]+)$/);
  if (addrInfo) {
    const addr = addrInfo[1];
    const isTarget = addr === mock.targetAddress;
    const funded = isTarget ? mock.utxos.reduce((a, x) => a + x.value, 0) : 0;
    const n = isTarget ? mock.utxos.length : 0;
    return route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        address: addr,
        chain_stats: { funded_txo_count: n, funded_txo_sum: funded, spent_txo_count: 0, spent_txo_sum: 0, tx_count: n },
        mempool_stats: { funded_txo_count: 0, funded_txo_sum: 0, spent_txo_count: 0, spent_txo_sum: 0, tx_count: 0 },
      }),
    });
  }
  if (/\/blocks\/tip\/height$/.test(p)) return route.fulfill({ status: 200, contentType: "text/plain", body: "900000" });
  if (/\/fee-estimates$/.test(p))
    return route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ "1": 2, "2": 2, "3": 2, "4": 2, "5": 2, "6": 1, "10": 1, "144": 1, "1008": 1 }),
    });
  if (mock.fundingTxid && new RegExp(`/(?:api/)?tx/${mock.fundingTxid}/hex$`).test(p))
    return route.fulfill({ status: 200, contentType: "text/plain", body: mock.fundingHex });
  if (mock.fundingTxid && new RegExp(`/(?:api/)?tx/${mock.fundingTxid}$`).test(p))
    return route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        txid: mock.fundingTxid,
        version: 2,
        locktime: 0,
        vin: [],
        vout: [{ scriptpubkey: "", value: mock.utxos[0] ? mock.utxos[0].value : 0 }],
        size: 100,
        weight: 400,
        fee: 0,
        status: { confirmed: true, block_height: 899000, block_hash: "11".repeat(32), block_time: 1700000000 },
      }),
    });
  mock.unhandled.push(p);
  return route.fulfill({ status: 404, contentType: "application/json", body: "{}" });
};

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

  const downloads = [];
  page.on("download", async (d) => {
    const target = path.join(OUT, `g3c-E-dl-${d.suggestedFilename()}`);
    await d.saveAs(target);
    downloads.push({ filename: d.suggestedFilename(), saved: target, size: fs.statSync(target).size });
    console.log("DOWNLOAD:", d.suggestedFilename(), fs.statSync(target).size, "bytes");
  });

  await page.route("**/*", async (route) => {
    const url = route.request().url();
    if (url.startsWith(BASE) || url.startsWith("data:") || url.startsWith("blob:")) return route.continue();
    mock.hits.push(url);
    return esplora(route, new URL(url).pathname);
  });

  const dump = async (tag) => {
    const s = await page.evaluate(() => {
      const t = (x) => (x || "").trim().replace(/\s+/g, " ");
      return {
        url: location.href,
        body: t(document.body.innerText).slice(0, 5000),
        buttons: Array.from(document.querySelectorAll("button")).map((b) => t(b.innerText)).filter(Boolean),
        alerts: Array.from(document.querySelectorAll('[class*="MuiAlert"],[role="alert"]'))
          .map((e) => t(e.innerText).slice(0, 300))
          .filter(Boolean),
      };
    });
    await page.screenshot({ path: path.join(OUT, `g3c-E-${tag}.png`), fullPage: true });
    fs.writeFileSync(path.join(OUT, `g3c-E-${tag}.json`), JSON.stringify(s, null, 2));
    return s;
  };

  const steps = {};
  await page.goto(`${BASE}/#/wallet`, { waitUntil: "domcontentloaded" });
  await page.waitForTimeout(2500);
  await page.locator('input[type="file"]').first().setInputFiles(configPath);
  await page.waitForTimeout(3000);
  await page.getByRole("button", { name: /^confirm$/i }).first().click();
  await page.waitForTimeout(3500);

  // ---- 读首地址（Receive 标签）
  await page.getByRole("tab", { name: /receive/i }).first().click();
  await page.waitForTimeout(2000);
  steps.receive = await dump("receive");
  const mAddr = steps.receive.body.match(/(bc1[a-z0-9]{20,}|[23][1-9A-HJ-NP-Za-km-z]{25,40})/);
  const addr = mAddr ? mAddr[0] : null;
  console.log("首地址:", addr);
  if (!addr) throw new Error("未能从 UI 读到地址");

  // ---- 造资交易 + mock
  const FUND = 200000;
  const f = buildFunding(addr, FUND);
  mock.targetAddress = addr;
  mock.fundingTxid = f.txid;
  mock.fundingHex = f.hex;
  mock.utxos = [
    { txid: f.txid, vout: 0, value: FUND, status: { confirmed: true, block_height: 899000, block_hash: "22".repeat(32), block_time: 1700000000 } },
  ];
  console.log("mock 资交易:", f.txid, FUND, "sat →", addr);

  // 刷新让 Caravan 重新拉数据
  await page.getByRole("button", { name: /refresh/i }).first().click().catch(() => {});
  await page.waitForTimeout(3000);
  await page.getByRole("tab", { name: /addresses/i }).first().click().catch(() => {});
  await page.waitForTimeout(2500);
  steps.afterRefresh = await dump("afterRefresh");
  console.log("=== 刷新后 ===");
  console.log(steps.afterRefresh.body.slice(0, 700));

  // ---- Send 构建支出
  await page.getByRole("tab", { name: /^send$/i }).first().click();
  await page.waitForTimeout(2500);
  steps.send = await dump("send");
  console.log("=== Send 页 ===");
  console.log(steps.send.body.slice(0, 700));

  // 目的地址：用钱包第二个地址（Receive → Next Address），避免"发给外部"的额外确认
  await page.getByRole("tab", { name: /receive/i }).first().click();
  await page.waitForTimeout(1500);
  await page.getByRole("button", { name: /next address/i }).first().click().catch(() => {});
  await page.waitForTimeout(2000);
  const recv2 = await dump("receive2");
  const addrs = recv2.body.match(/(bc1[a-z0-9]{20,}|[1-3][1-9A-HJ-NP-Za-km-z]{25,40})/g) || [];
  const dest = addrs.find((a) => a !== addr) || addr;
  console.log("目的地址:", dest);

  await page.getByRole("tab", { name: /^send$/i }).first().click();
  await page.waitForTimeout(2000);
  await page.locator('input[name="destination"]').fill(dest);
  await page.waitForTimeout(500);
  await page.locator('input[name="amount"]').fill("0.001");
  await page.waitForTimeout(1500);
  steps.sendFilled = await dump("sendFilled");
  console.log("=== 填表后 ===");
  console.log(steps.sendFilled.body.slice(0, 900));
  console.log("alerts:", steps.sendFilled.alerts.join(" || "));

  // 预览交易
  const prev = page.getByRole("button", { name: /preview transaction/i }).first();
  if (await prev.count()) {
    await prev.click().catch(() => {});
    await page.waitForTimeout(3500);
    steps.preview = await dump("preview");
    console.log("=== 预览 ===");
    console.log(steps.preview.body.slice(0, 1500));
    console.log("buttons:", steps.preview.buttons.join(" | "));
  }

  // 下载未签名 PSBT
  const dl = page.locator("button", { hasText: /download unsigned psbt/i }).first();
  if (await dl.count()) {
    await dl.click({ timeout: 8000 }).catch((e) => console.log("点击下载失败:", e.message.slice(0, 60)));
    await page.waitForTimeout(3500);
    steps.afterDownload = await dump("afterDownload");
  } else {
    console.log("(未见 Download Unsigned PSBT 按钮)");
  }

  fs.writeFileSync(
    path.join(OUT, "g3c-E-result.json"),
    JSON.stringify({ config, mock: { ...mock, hits: mock.hits.length }, downloads, steps }, null, 2),
  );
  console.log("--- 摘要 ---");
  console.log("外部请求数:", mock.hits.length, "| 未处理路径:", [...new Set(mock.unhandled)].slice(0, 8).join(" "));
  console.log("下载:", downloads.map((d) => d.filename + " " + d.size + "B").join(" | "));
  await browser.close();
})().catch((e) => {
  console.error("PROBE ERROR:", e.stack ? e.stack.split("\n").slice(0, 4).join(" | ") : e.message);
  process.exit(1);
});
