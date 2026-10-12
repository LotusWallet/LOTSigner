/*
 * G3 补测 · 阶段 F：把「已部分签名」的 PSBT 交回 Caravan，观察其导出能力
 *
 * 对应预注册 §3 的 ③ 判定标准：
 *   「签名后无法导出可供发起方继续收集其余签名的 PSBT（或导出格式不可被第三方工具消费）」
 *
 * 步骤：
 *   1) 用自持私钥（signer1，路径取自 PSBT 内的 bip32Derivation）对 Caravan 导出的
 *      未签名 PSBT 做**部分签名**（保持未 finalize）
 *   2) 在 Caravan 的 Send 标签 → "Import PSBT" 把该 PSBT 交回
 *   3) 观察其界面提供了什么（能否继续签名 / 能否导出 PSBT）
 *   4) 若导出，保存导出物并交给第三方工具（lscli）验证可消费性
 *
 * 链上数据为 mock（合成资交易），Caravan 的导入/导出为真实路径。此声明须写入报告。
 */
const path = require("path");
const fs = require("fs");
const M = process.env.CARAVAN_MODULES || "D:/work/_g3-caravan/node_modules/";
const { chromium } = require(M + "playwright");
const btc = require(M + "bitcoinjs-lib");
const b32 = require(M + "bip32");
const { buildWalletConfig, SEEDS } = require("./g3c_probe_export.js");

const BASE = process.env.G3C_BASE || "http://localhost:4174";
const OUT = path.join(__dirname, "..", "evidence");
const CHROME = "C:/Program Files/Google/Chrome/Application/chrome.exe";

const UNSIGNED = path.join(OUT, "g3c-E-dl-transaction.psbt");
const PARTIAL = path.join(OUT, "g3c-F-partial-signed.psbt");

// mock：需要为钱包地址提供那笔资交易 UTXO，否则 Caravan 会以
// "This PSBT does not contain any UTXOs from this wallet" 拒绝导入
// （该行为本身已是一次观测：导入校验是**按钱包已知 UTXO** 做的）
const mock = { hits: [], unhandled: [], targetAddress: null, utxos: [], fundingHex: null, fundingTxid: null };

function buildFunding(address, valueSats) {
  const p2sh = btc.payments.p2sh({ address, network: btc.networks.bitcoin });
  const tx = new btc.Transaction();
  tx.version = 2;
  tx.addInput(Buffer.alloc(32, 0x33), 0, 0xffffffff, Buffer.from("03cafe01", "hex"));
  tx.addOutput(p2sh.output, valueSats);
  tx.locktime = 0;
  return { txid: tx.getId(), hex: tx.toHex() };
}

function partiallySign() {
  const b64 = fs.readFileSync(UNSIGNED, "utf8").trim();
  const psbt = btc.Psbt.fromBase64(b64, { network: btc.networks.bitcoin });
  const der = psbt.data.inputs[0].bip32Derivation || [];
  // 签名者 1 = seed[0]（xfp 8236dc33）
  const master = b32.fromSeed(Buffer.from(SEEDS[0], "hex"));
  const xfp = master.fingerprint.toString("hex");
  const mine = der.find((d) => d.masterFingerprint.toString("hex") === xfp);
  if (!mine) throw new Error("PSBT 内未找到 signer1 的派生路径");
  const child = master.derivePath(mine.path);
  const key = btc.ECPair.fromPrivateKey(child.privateKey, { network: btc.networks.bitcoin });
  psbt.signInput(0, key);
  // 保持未 finalize：部分签名场景的规范形态
  const out = psbt.toBase64();
  fs.writeFileSync(PARTIAL, out);
  const check = btc.Psbt.fromBase64(out, { network: btc.networks.bitcoin });
  const sigs = Object.keys(check.data.inputs[0].partialSig || {});
  console.log("部分签名完成:", PARTIAL, "| partialSig 条数:", sigs.length, "| path:", mine.path);
  return out;
}

(async () => {
  partiallySign();

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
    const target = path.join(OUT, `g3c-F-dl-${d.suggestedFilename()}`);
    await d.saveAs(target);
    downloads.push({ filename: d.suggestedFilename(), saved: target, size: fs.statSync(target).size });
    console.log("DOWNLOAD:", d.suggestedFilename(), fs.statSync(target).size, "bytes");
  });

  await page.route("**/*", async (route) => {
    const url = route.request().url();
    if (url.startsWith(BASE) || url.startsWith("data:") || url.startsWith("blob:")) return route.continue();
    mock.hits.push(url);
    const p = new URL(url).pathname;
    if (/\/blocks\/tip\/height$/.test(p)) return route.fulfill({ status: 200, contentType: "text/plain", body: "900000" });
    if (/\/fee-estimates$/.test(p) || /\/fees\/recommended$/.test(p))
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
    if (mock.fundingTxid && new RegExp(`/(?:api/)?tx/${mock.fundingTxid}$`).test(p))
      return route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          txid: mock.fundingTxid, version: 2, locktime: 0, vin: [],
          vout: [{ scriptpubkey: "", value: mock.utxos[0] ? mock.utxos[0].value : 0 }],
          size: 100, weight: 400, fee: 0,
          status: { confirmed: true, block_height: 899000, block_hash: "33".repeat(32), block_time: 1700000000 },
        }),
      });
    mock.unhandled.push(p);
    return route.fulfill({ status: 404, contentType: "application/json", body: "{}" });
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
    await page.screenshot({ path: path.join(OUT, `g3c-F-${tag}.png`), fullPage: true });
    fs.writeFileSync(path.join(OUT, `g3c-F-${tag}.json`), JSON.stringify(s, null, 2));
    return s;
  };

  const steps = {};
  await page.goto(`${BASE}/#/wallet`, { waitUntil: "domcontentloaded" });
  await page.waitForTimeout(2500);
  await page.locator('input[type="file"]').first().setInputFiles(configPath);
  await page.waitForTimeout(3000);
  await page.getByRole("button", { name: /^confirm$/i }).first().click();
  await page.waitForTimeout(3000);

  // 读钱包首地址 → 造资交易并设置 mock（与阶段 E 同一构造逻辑，txid 可复现）
  await page.getByRole("tab", { name: /receive/i }).first().click();
  await page.waitForTimeout(2000);
  const recv = await dump("receive");
  const mAddr = recv.body.match(/(bc1[a-z0-9]{20,}|[23][1-9A-HJ-NP-Za-km-z]{25,40})/);
  const addr = mAddr ? mAddr[0] : null;
  console.log("钱包首地址:", addr);
  if (!addr) throw new Error("未能读到钱包地址");

  const f = buildFunding(addr, 200000);
  mock.targetAddress = addr;
  mock.fundingTxid = f.txid;
  mock.fundingHex = f.hex;
  mock.utxos = [
    { txid: f.txid, vout: 0, value: 200000, status: { confirmed: true, block_height: 899000, block_hash: "33".repeat(32), block_time: 1700000000 } },
  ];
  console.log("mock 资交易 txid:", f.txid);

  // 自洽校验：待导入 PSBT 的 outpoint 应当正是这笔资交易（否则就不是"本钱包的 PSBT"）
  const psbtOutpoint = (() => {
    const psbt = btc.Psbt.fromBase64(fs.readFileSync(PARTIAL, "utf8").trim(), { network: btc.networks.bitcoin });
    const tx = psbt.__CACHE.__TX || null;
    const raw = psbt.data.globalMap.unsignedTx.toBuffer();
    // 简化：直接取全局未签名交易的第一个输入的 prevout txid（小端 → 反转）
    const txidLE = raw.slice(5, 37);
    return Buffer.from(txidLE).reverse().toString("hex");
  })();
  console.log("PSBT 输入 outpoint txid:", psbtOutpoint, "| 与 mock 资交易一致:", psbtOutpoint === f.txid);

  // 关键：Caravan 的导入校验是 `selectAvailableInputsFromPSBT(getState())`——
  // 拿 PSBT 输入与**钱包状态里已拉取的 UTXO 集**比对，而不是现场查链。
  // 因此必须先 Refresh，让钱包知道这笔 mock UTXO，否则一律被拒。
  await page.getByRole("button", { name: /refresh/i }).first().click().catch(() => {});
  await page.waitForTimeout(3500);
  await page.getByRole("tab", { name: /addresses/i }).first().click().catch(() => {});
  await page.waitForTimeout(2500);
  steps.afterRefresh = await dump("afterRefresh");
  const balM = steps.afterRefresh.body.match(/([\d.]+) BTC/);
  console.log("刷新后钱包余额显示:", balM ? balM[1] + " BTC" : "(未识别)");

  // Send 标签 → Import PSBT
  await page.getByRole("tab", { name: /^send$/i }).first().click();
  await page.waitForTimeout(2500);
  steps.send = await dump("send");

  // 导入方式选择（Import Method）——先看有哪些选项
  const inputs = await page.locator("input,select,[role='combobox']").all();
  console.log("Send 页控件数:", inputs.length);

  // Import PSBT 区域：先在下拉里选 "File Upload"，文件输入才会出现
  const comboboxes = page.locator('[role="combobox"]');
  const ncb = await comboboxes.count();
  console.log("combobox 数:", ncb);
  let imported = false;
  if (ncb > 0) {
    // 最后一个 combobox = Import Method
    await comboboxes.nth(ncb - 1).click();
    await page.waitForTimeout(1200);
    const options = await page.locator('[role="option"]').allInnerTexts();
    console.log("Import Method 选项:", options.join(" | "));
    await page.locator('[role="option"]', { hasText: /file upload/i }).first().click();
    await page.waitForTimeout(2000);
  }

  const fileInputs = page.locator('input[type="file"]');
  const n = await fileInputs.count();
  console.log("文件输入数:", n);
  for (let i = 0; i < n; i++) {
    console.log(`  file[${i}] accept=${(await fileInputs.nth(i).getAttribute("accept")) || ""}`);
  }
  if (n > 0) {
    await fileInputs.nth(n - 1).setInputFiles(PARTIAL);
    await page.waitForTimeout(4500);
    steps.afterImport = await dump("afterImport");
    imported = true;
    console.log("=== 导入部分签名 PSBT 后 ===");
    console.log(steps.afterImport.body.slice(0, 1800));
    console.log("buttons:", steps.afterImport.buttons.join(" | "));
    console.log("alerts:", steps.afterImport.alerts.join(" || "));
  } else {
    console.log("(!) 选择 File Upload 后仍未出现文件输入");
  }

  // 逐个点开可能的导出/继续按钮
  if (imported) {
    for (const re of [/download.*psbt/i, /sign transaction/i, /show unsigned/i, /preview/i]) {
      const b = page.locator("button", { hasText: re }).first();
      if (await b.count()) {
        const label = (await b.innerText().catch(() => "")).trim();
        await b.click({ timeout: 8000 }).catch((e) => console.log(`点击 ${label} 失败: ${e.message.slice(0, 60)}`));
        await page.waitForTimeout(3000);
        steps[`after-${label.replace(/\s+/g, "-")}`] = await dump(`after-${label.replace(/\s+/g, "-")}`);
        console.log(`--- 点击「${label}」后 ---`);
        console.log(steps[`after-${label.replace(/\s+/g, "-")}`].body.slice(0, 1000));
        console.log("buttons:", steps[`after-${label.replace(/\s+/g, "-")}`].buttons.join(" | "));
      }
    }
  }

  fs.writeFileSync(
    path.join(OUT, "g3c-F-result.json"),
    JSON.stringify({ mock: { hits: mock.hits.length, unhandled: [...new Set(mock.unhandled)] }, downloads, steps }, null, 2),
  );
  console.log("--- 摘要 ---");
  console.log("下载:", downloads.map((d) => d.filename + " " + d.size + "B").join(" | ") || "(无)");
  console.log("未处理路径:", [...new Set(mock.unhandled)].slice(0, 6).join(" "));
  await browser.close();
})().catch((e) => {
  console.error("PROBE ERROR:", e.stack ? e.stack.split("\n").slice(0, 4).join(" | ") : e.message);
  process.exit(1);
});
