// Lotus Signer 网页端真实浏览器验证
// 验证三件事：① wasm 内核加载并返回结果 ② 四层都渲染 ③ 零外部请求（宪法 G-II）
const { chromium } = require('playwright');
const fs = require('fs');

const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const URL = 'http://127.0.0.1:4180/index.html';

(async () => {
  const browser = await chromium.launch({ executablePath: CHROME, headless: true });
  const page = await browser.newPage();

  // ③ 记录一切请求，区分同源 / 外部
  const external = [];
  const all = [];
  page.on('request', r => {
    const u = r.url();
    all.push(u);
    if (!u.startsWith('http://127.0.0.1:4180') && !u.startsWith('data:') && !u.startsWith('blob:')) {
      external.push(u);
    }
  });
  const errors = [];
  page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
  page.on('pageerror', e => errors.push('PAGEERROR: ' + e.message));

  await page.goto(URL, { waitUntil: 'networkidle' });
  await page.waitForTimeout(500);

  const readyText = await page.textContent('#hint');
  console.log('内核状态 =', JSON.stringify(readyText));

  // 点「载入示例」→ 触发完整链路
  await page.click('#demo');
  await page.waitForTimeout(800);

  const bodyText = await page.evaluate(() => document.body.innerText);

  // ② 四层是否都在
  const layers = {
    '第一层': bodyText.includes('第一层'),
    '第二层': bodyText.includes('第二层'),
    '第三层': bodyText.includes('第三层'),
    '第四层': bodyText.includes('第四层'),
  };
  console.log('四层渲染 =', JSON.stringify(layers));

  // 内核真实产物：txid 与金额应出自 S1
  const facts = {
    txid: bodyText.includes('100df7d5af345144fdf8f76b4884faea1bc4fd012ea8431c11afcc96fffc7bdc'),
    out0: /1,200,000 sat/.test(bodyText),
    out1: /290,000 sat/.test(bodyText),
    fee: /10,000 sat/.test(bodyText),
    addr0: bodyText.includes('bc1qp8sv9wkt0sx7rtl7cah44d3jhu0mk7xqpk3rfl'),
  };
  console.log('内核事实  =', JSON.stringify(facts));

  // 五态徽标是否出现（S1 应含 verified + parsed_unverified）
  const badges = {
    verified: bodyText.includes('verified'),
    parsed_unverified: bodyText.includes('parsed_unverified'),
  };
  console.log('状态徽标  =', JSON.stringify(badges));

  // ④ 第四层不得为空
  const refuseCount = await page.evaluate(() => document.querySelectorAll('.refuse li').length);
  console.log('第四层条目 =', refuseCount);

  console.log('外部请求  =', JSON.stringify(external));
  console.log('总请求数  =', all.length);
  console.log('控制台错误 =', JSON.stringify(errors));

  fs.writeFileSync('D:/work/LOTSigner/g3/evidence/web-verify.json', JSON.stringify({
    hint: readyText, layers, facts, badges, refuseCount, external, totalRequests: all.length, errors,
    allRequests: all
  }, null, 2));

  await page.screenshot({ path: 'D:/work/LOTSigner/g3/evidence/web-screenshot.png', fullPage: true });

  await browser.close();

  // 判定
  const pass = readyText === '内核就绪'
    && Object.values(layers).every(Boolean)
    && Object.values(facts).every(Boolean)
    && refuseCount === 5
    && external.length === 0
    && errors.length === 0;
  console.log('\n===== 判定: ' + (pass ? 'PASS' : 'FAIL') + ' =====');
  process.exit(pass ? 0 : 1);
})();
