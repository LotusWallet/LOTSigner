/*
 * G3 补测 · 阶段 G：定位「导出物是否携带已收集的部分签名」
 *
 * 背景：阶段 F 观察到 Caravan 接受外部部分签名 PSBT（界面显示 1 of 2），
 * 但点「Download Unsigned PSBT」得到的是**不含签名**的文件。
 * 由于该按钮标签本身写着 "Unsigned"，必须排除"点错按钮"才能下结论——
 * 故本阶段改点签名步骤里的「Download PSBT」，并逐一记录每个下载物的字段构成。
 *
 * 判定标准（预注册 §3 ③）：签名后能否导出**可供发起方继续收集其余签名**的 PSBT。
 * 只记录功能事实。结论在报告中由人填写。
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
const PARTIAL = path.join(OUT, "g3c-F-partial-signed.psbt");

const mock = { hits: [], targetAddress: null, utxos: [], fundingHex: null, fundingTxid: null };

function buildFunding(address, valueSats) {
  const p2sh = btc.payments.p2sh({ address, network: btc.networks.bitcoin });
  const tx = new btc.Transaction();
  tx.version = 2;
  tx.addInput(Buffer.alloc(32, 0x33), 0, 0xffffffff, Buffer.from("03cafe01", "hex"));
  tx.addOutput(p2sh.output, valueSats);
  tx.locktime = 0;
  return { txid: tx.getId(), hex: tx.toHex() };
}

const inspect = (file) => {
  const raw = fs.readFileSync(file, "utf8").trim();
  const p = btc.Psbt.fromBase64(raw, { network: btc.networks.bitcoin });
  const im = p.data.inputs[0];
  return {
    file: path.basename(file),
    bytes: fs.statSync(file).size,
    partialSigs: Object.keys(im.partialSig || {}).length,
    nonWitnessUtxo: !!im.nonWitnessUtxo,
    redeemScript: !!im.redeemScript,
    bip32Derivation: (im.bip32Derivation || []).length,
    finalScriptSig: !!im.finalScriptSig,
    txid: p.data.globalMap.unsignedTx.tx.getId(),
  };
};

(async () => {
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
    const target = path.join(OUT, `g3c-G-dl-${downloads.length}-${d.suggestedFilename()}`);
    await d.saveAs(target);
    downloads.push({ label: d.suggestedFilename(), saved: target, ...inspect(target) });
    console.log("DOWNLOAD:", downloads[downloads.length - 1]);
  });

  await page.route("**/*", async (route) => {
    const url = route.request().url();
    if (url.startsWith(BASE) || url.startsWith("data:") || url.startsWith("blob:")) return route.continue();
    mock.hits.push(url);
    const p = new URL(url).pathname;
    if (/\/blocks\/tip\/height$/.test(p)) return route.fulfill({ status: 200, contentType: "text/plain", body: "900000" });
    if (/fee-estimates$|\/fees\/recommended$/.test(p))
      return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify({ "1": 2, "6": 1 }) });
    if (/\/(?:api\/)?address\//.test(p)) {
      const isTarget = mock.targetAddress && p.includes(mock.targetAddress);
      const funded = isTarget ? mock.utxos.reduce((a, x) => a + x.value, 0) : 0;
      const cnt = isTarget ? mock.utxos.length : 0;
      return route.fulfill({
        status: 200,
        contentType: "application/json",
        body: /\/utxo$/.test(p)
          ? JSON.stringify(isTarget ? mock.utxos : [])
          : JSON.stringify({
              chain_stats: { funded_txo_count: cnt, funded_txo_sum: funded, spent_txo_count: 0, spent_txo_sum: 0, tx_count: cnt },
              mempool_stats: { funded_txo_count: 0, funded_txo_sum: 0, spent_txo_count: 0, spent_txo_sum: 0, tx_count: 0 },
            }),
      });
    }
    if (mock.fundingTxid && new RegExp(`/(?:api/)?tx/${mock.fundingTxid}/hex$`).test(p))
      return route.fulfill({ status: 200, contentType: "text/plain", body: mock.fundingHex });
    return route.fulfill({ status: 404, contentType: "application/json", body: "{}" });
  });

  const dump = async (tag) => {
    const s = await page.evaluate(() => {
      const t = (x) => (x || "").trim().replace(/\s+/g, " ");
      return {
        body: t(document.body.innerText).slice(0, 4000),
        buttons: Array.from(document.querySelectorAll("button")).map((b) => t(b.innerText)).filter(Boolean),
      };
    });
    await page.screenshot({ path: path.join(OUT, `g3c-G-${tag}.png`), fullPage: true });
    return s;
  };

  const steps = {};
  await page.goto(`${BASE}/#/wallet`, { waitUntil: "domcontentloaded" });
  await page.waitForTimeout(2500);
  await page.locator('input[type="file"]').first().setInputFiles(configPath);
  await page.waitForTimeout(3000);
  await page.getByRole("button", { name: /^confirm$/i }).first().click();
  await page.waitForTimeout(3000);

  // 读地址 + mock 注资 + Refresh
  await page.getByRole("tab", { name: /receive/i }).first().click();
  await page.waitForTimeout(1800);
  const recv = await dump("receive");
  const addr = (recv.body.match(/(bc1[a-z0-9]{20,}|[23][1-9A-HJ-NP-Za-km-z]{25,40})/) || [])[0];
  const f = buildFunding(addr, 200000);
  mock.targetAddress = addr;
  mock.fundingTxid = f.txid;
  mock.fundingHex = f.hex;
  mock.utxos = [{ txid: f.txid, vout: 0, value: 200000, status: { confirmed: true, block_height: 899000, block_hash: "44".repeat(32), block_time: 1700000000 } }];
  await page.getByRole("button", { name: /refresh/i }).first().click().catch(() => {});
  await page.waitForTimeout(3500);

  // 导入部分签名 PSBT
  await page.getByRole("tab", { name: /^send$/i }).first().click();
  await page.waitForTimeout(2000);
  const cb = page.locator('[role="combobox"]');
  await cb.nth((await cb.count()) - 1).click();
  await page.waitForTimeout(1000);
  await page.locator('[role="option"]', { hasText: /file upload/i }).first().click();
  await page.waitForTimeout(1500);
  await page.locator('input[type="file"]').first().setInputFiles(PARTIAL);
  await page.waitForTimeout(4500);
  steps.afterImport = await dump("afterImport-preview");
  console.log("=== 导入后（预览页）按钮 ===");
  console.log(steps.afterImport.buttons.join(" | "));

  // 进入签名步骤
  const signBtn = page.getByRole("button", { name: /^sign transaction$/i }).first();
  if (await signBtn.count()) {
    await signBtn.click();
    await page.waitForTimeout(3500);
    steps.signStep = await dump("sign-step");
    console.log("=== 签名步骤按钮 ===");
    console.log(steps.signStep.buttons.join(" | "));
    console.log("body:", steps.signStep.body.slice(0, 800));
  }

  // 点签名步骤里的 "Download PSBT"（不带 Unsigned 的那个）
  const dlStrict = page.locator("button", { hasText: /^download psbt$/i });
  const nStrict = await dlStrict.count();
  console.log("严格匹配 'Download PSBT' 的按钮数:", nStrict);
  for (let i = 0; i < nStrict; i++) {
    await dlStrict.nth(i).click({ timeout: 8000 }).catch((e) => console.log("点击失败:", e.message.slice(0, 60)));
    await page.waitForTimeout(3000);
  }

  // 再做一次全局兜底：所有含 download 的按钮
  const anyDl = page.locator("button", { hasText: /download/i });
  const nAny = await anyDl.count();
  for (let i = 0; i < nAny; i++) {
    const label = (await anyDl.nth(i).innerText().catch(() => "")).trim().replace(/\s+/g, " ");
    console.log(`  [兜底] 发现按钮: ${label}`);
  }

  fs.writeFileSync(
    path.join(OUT, "g3c-G-result.json"),
    JSON.stringify({ steps: Object.fromEntries(Object.entries(steps).map(([k, v]) => [k, { url: v.url, buttons: v.buttons }])), downloads, externalRequests: mock.hits.length }, null, 2),
  );
  console.log("--- 下载物汇总（关键：partialSigs）---");
  downloads.forEach((d) => console.log(`  ${d.label}: ${d.bytes}B | partialSigs=${d.partialSigs} | txid=${d.txid.slice(0, 16)}…`));
  await browser.close();
})().catch((e) => {
  console.error("PROBE ERROR:", e.stack ? e.stack.split("\n").slice(0, 4).join(" | ") : e.message);
  process.exit(1);
});
