// g3/tools/check_wasm_error_lang.js
//
// 目的：在**真实构建产物**这一层检查网页端的错误句子。
//
// 为什么需要它：`cargo test` 里的 FFI 层测试（crates/lswasm/src/lib.rs）覆盖的是**源码**；
// 而用户实际加载的是 `web/lswasm.wasm` 这个构建产物。`verify_web.js` / `verify_web2.js`
// 走的是成功路径（双语 UI、零外部请求、断网全链路），谁也没断言过**错误**句子的语言。
// 2026-10-12 正是靠这个脚本确认网页端与 CLI 有完全相同的两个缺陷（见 PROGRESS 第十五节 /
// P1-SPEC-AUDIT F5）。它检查的是产物，不是源码，因此构建完 wasm 后值得再跑一次。
//
// 用法：
//   node g3/tools/check_wasm_error_lang.js [path-to-wasm]     # 默认 web/lswasm.wasm
//   退出码 0 = 通过；1 = 有错误句子落到了非请求语言
//
// 依赖：Node ≥ 18（用 WebAssembly.instantiate 直接加载 .wasm，无 npm 依赖）。
// 注意：本脚本**不参与构建**——build.sh 不需要 Node。

const fs = require('fs');
const path = require('path');

const wasmPath = process.argv[2] || path.join(__dirname, '..', '..', 'web', 'lswasm.wasm');
const root = path.join(__dirname, '..', '..');

// 英文模板中独有的片段。中文句子允许出现 PSBT / map 这类术语拉丁字母，但不允许出现英文措辞。
const EN_ONLY = [
  'Review failed',
  'input decode failed',
  'PSBT parse failed',
  'not a PSBT',
  'the message ends abruptly',
  'truncated or corrupt',
  'does not contain',
  'will not guess',
  'key type',
];
const hasCJK = (s) => /[\u4e00-\u9fff]/.test(s);

const fixture = (name) =>
  fs.readFileSync(path.join(root, 'g3', 'fixtures', `${name}.psbt`), 'utf8').trim();

(async () => {
  const bytes = fs.readFileSync(wasmPath);
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const ex = instance.exports;

  function put(str) {
    const b = Buffer.from(str, 'utf8');
    const p = ex.ls_alloc(b.length);
    new Uint8Array(ex.memory.buffer).set(b, p);
    return [p, b.length];
  }
  function readPacked(p) {
    const dv = new DataView(ex.memory.buffer);
    const n = dv.getUint32(p, true);
    return Buffer.from(new Uint8Array(ex.memory.buffer, p + 4, n)).toString('utf8');
  }
  /** 走一遍浏览器实际走的调用序列。 */
  function review(text, lang) {
    const [ip, il] = put(text);
    const [lp, ll] = put(lang);
    const out = ex.ls_review(ip, il, lp, ll);
    if (out === 0) return '<ls_review returned null>';
    const s = readPacked(out);
    ex.ls_dealloc(ip, il);
    ex.ls_dealloc(lp, ll);
    return s;
  }

  const cases = [
    ['base64 decode failure', '!!!not-base64!!!'],
    ['bad magic', Buffer.from([0, 1, 2, 3, 4, 5]).toString('base64')],
    ['missing unsigned tx', Buffer.from([0x70, 0x73, 0x62, 0x74, 0xff, 0x00]).toString('base64')],
    ['genuine truncation (S6)', fixture('S6')],
    ['valid PSBT (S1)', fixture('S1')],
  ];

  console.log(`wasm: ${wasmPath} (${bytes.length} bytes)\n`);
  let failures = 0;
  for (const [name, input] of cases) {
    for (const lang of ['en', 'zh-CN']) {
      const raw = review(input, lang);
      let shown;
      try {
        const j = JSON.parse(raw);
        shown = j.ok === false ? j.error : '<ok:true>';
      } catch (e) {
        shown = `<NOT VALID JSON: ${e.message}>`;
        failures++;
      }
      let flag = '';
      if (lang === 'en' && hasCJK(shown)) flag = '  !! English payload contains CJK';
      if (lang === 'zh-CN') {
        const leaked = EN_ONLY.find((f) => shown.includes(f));
        if (leaked) flag = `  !! zh payload leaked English template: ${leaked}`;
      }
      if (flag) failures++;
      console.log(`[${lang.padEnd(5)}] ${name.padEnd(26)} ${shown}${flag}`);
    }
  }
  console.log(
    failures === 0
      ? '\nWASM-ERROR-LANG OK'
      : `\nWASM-ERROR-LANG FAILED (${failures} problem(s))`
  );
  process.exit(failures === 0 ? 0 : 1);
})().catch((e) => {
  console.error('check failed:', e);
  process.exit(1);
});
