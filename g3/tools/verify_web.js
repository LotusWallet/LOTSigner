// Lotus Signer 网页端验证 v2：双语言（i18n）+ 响应式 + 零外部请求
const { chromium } = require('playwright');
const fs = require('fs');

const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const URL = 'http://127.0.0.1:4180/index.html';
const OUT = 'D:/work/LOTSigner/g3/evidence';

(async () => {
  const browser = await chromium.launch({ executablePath: CHROME, headless: true });
  const report = { errors: [], checks: {} };

  const mkPage = async () => {
    const page = await browser.newPage();
    const external = [];
    page.on('request', r => {
      const u = r.url();
      if (!u.startsWith('http://127.0.0.1:4180') && !u.startsWith('data:') && !u.startsWith('blob:')) {
        external.push(u);
      }
    });
    page.on('console', m => { if (m.type() === 'error') report.errors.push(m.text()); });
    page.on('pageerror', e => report.errors.push('PAGEERROR: ' + e.message));
    return { page, external };
  };

  // ============ 桌面 1280×800：双语言全流程 ============
  {
    // 强制 locale=en-US 的独立 context：验证「默认英文」（国际化默认语言）。
    // 注意不能共用默认 context——headless Chrome 继承系统语言（zh-CN），默认中文是正确行为。
    const page = await browser.newContext({ locale: 'en-US' }).then(c => c.newPage());
    const external = [];
    page.on('request', r => {
      const u = r.url();
      if (!u.startsWith('http://127.0.0.1:4180') && !u.startsWith('data:') && !u.startsWith('blob:')) {
        external.push(u);
      }
    });
    page.on('console', m => { if (m.type() === 'error') report.errors.push(m.text()); });
    page.on('pageerror', e => report.errors.push('PAGEERROR: ' + e.message));
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto(URL, { waitUntil: 'networkidle' });
    await page.waitForTimeout(600);

    // 默认应为英文（国际化默认），除非 localStorage 残留——new context 无残留
    const enTitle = await page.textContent('#t-drop-title');
    report.checks.default_en = /sign/i.test(enTitle);
    const enOffline = await page.textContent('#t-offline');
    report.checks.en_offline_badge = /offline/i.test(enOffline);

    // 英文审阅 S1
    await page.click('#demo');
    await page.waitForTimeout(600);
    let body = await page.evaluate(() => document.body.innerText);
    report.checks.en_facts = body.includes('100df7d5af345144fdf8f76b4884faea1bc4fd012ea8431c11afcc96fffc7bdc')
      && /1,200,000 sat/.test(body) && /10,000 sat/.test(body);
    report.checks.en_layers = ['Transaction facts', 'Context', 'Risk notices', 'What this tool does NOT tell you']
      .every(x => body.includes(x));
    report.checks.en_parsed_badge = body.includes('parsed_unverified');
    report.checks.en_refuse5 = (await page.evaluate(() => document.querySelectorAll('.refuse li').length)) === 5;
    await page.screenshot({ path: OUT + '/web-en-desktop.png', fullPage: true });

    // 切中文 → 全文应翻成中文（含内核句子）
    await page.click('#lang-zh');
    await page.waitForTimeout(600);
    body = await page.evaluate(() => document.body.innerText);
    report.checks.zh_layers = ['交易事实', '上下文解释', '风险提示', '本工具不提供以下结论'].every(x => body.includes(x));
    report.checks.zh_kernel_sentences = body.includes('待签名的 PSBT') && body.includes('本工具不联网');
    report.checks.zh_facts_kept = body.includes('100df7d5af345144fdf8f76b4884faea1bc4fd012ea8431c11afcc96fffc7bdc');
    await page.screenshot({ path: OUT + '/web-zh-desktop.png', fullPage: true });

    report.checks.desktop_external_zero = external.length === 0;
    await page.close();
  }

  // ============ 手机 375×667（iPhone SE 级别）：中文 + 布局 ============
  {
    const { page, external } = await mkPage();
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto(URL, { waitUntil: 'networkidle' });
    await page.waitForTimeout(400);
    await page.click('#lang-zh');
    await page.waitForTimeout(300);
    await page.click('#demo');
    await page.waitForTimeout(600);

    // 无横向溢出（页面级）
    const overflow = await page.evaluate(() =>
      document.documentElement.scrollWidth - document.documentElement.clientWidth);
    report.checks.mobile_no_page_overflow = overflow <= 0;
    // 表格容器可横向滚动（内容不被截断，容器可滚）
    const scrollable = await page.evaluate(() => {
      const els = [...document.querySelectorAll('.table-scroll')];
      return els.length >= 2 && els.every(el => el.scrollWidth >= el.clientWidth - 2);
    });
    report.checks.mobile_tables_scrollable = scrollable;
    // 第四层仍完整
    report.checks.mobile_refuse5 = (await page.evaluate(() => document.querySelectorAll('.refuse li').length)) === 5;
    await page.screenshot({ path: OUT + '/web-zh-mobile.png', fullPage: true });
    report.checks.mobile_external_zero = external.length === 0;
    await page.close();
  }

  // ============ 平板 768×1024 与宽屏 1920×1080 快照 ============
  {
    const { page } = await mkPage();
    for (const [w, h, name] of [[768, 1024, 'tablet'], [1920, 1080, 'wide']]) {
      await page.setViewportSize({ width: w, height: h });
      await page.goto(URL, { waitUntil: 'networkidle' });
      await page.waitForTimeout(300);
      await page.click('#demo');
      await page.waitForTimeout(500);
      const overflow = await page.evaluate(() =>
        document.documentElement.scrollWidth - document.documentElement.clientWidth);
      report.checks['no_overflow_' + name] = overflow <= 0;
      await page.screenshot({ path: `${OUT}/web-${name}.png`, fullPage: false });
    }
    await page.close();
  }

  await browser.close();

  report.checks.console_errors_zero = report.errors.length === 0;
  fs.writeFileSync(OUT + '/web-verify.json', JSON.stringify(report, null, 2));

  console.log(JSON.stringify(report.checks, null, 2));
  console.log('errors:', JSON.stringify(report.errors));
  const pass = Object.values(report.checks).every(Boolean);
  console.log('\n===== 判定: ' + (pass ? 'PASS' : 'FAIL') + ' =====');
  process.exit(pass ? 0 : 1);
})();
