# G1 · 判定记录 —— BDK-WASM 离线能力实测

| 项 | 内容 |
|---|---|
| 上位文件 | `g1/G1-预注册-BDK-WASM离线能力实测.md`（🔒 §2–§6 未改动） |
| 执行 | 主持人（顾问 C） |
| 日期 | 2026-10-12 |
| 被测对象 | `bitcoindevkit/bdk-wasm` commit **`73cb4df46f6f51ceea034e81b4f379cc83333435`**（2026-05-12）+ 已发布 npm 包 `@bitcoindevkit/bdk-wallet-web@0.3.0` / `-node@0.3.0` |
| 本机环境 | rustc/cargo 1.99.0；`wasm32-unknown-unknown` 已装；`wasm-pack` **未装**；**clang 未装**（PATH 与常见安装路径均无）；Node v24.21.0；Chrome（系统安装）+ Playwright |
| 证据目录 | `g1/evidence/` ｜ 探针脚本 `g1/tools/` |

---

## §0 结论摘要

| 项 | 值 |
|---|---|
| **门槛判定（预注册 §4.1 字面）** | **⚪ 不可判定** —— 两个原因见 §2 |
| **能力层面的实际结论** | **离线加载 / 离线描述符与地址派生 / 无打包器集成（需自写 ~10 行加载器）三项均成立**；其**官方入口与预构建产物**形态不成立 |
| 对 P1 形态的影响 | **无**（预注册 §1.2 已预登记：两种结局都不改变 P1 形态） |
| 真正决定采用与否的 | **§7 的 X1/X2 两项宪法级冲突** —— 不在本闸门门槛内，主持人**不得裁决**，须三方讨论 |
| 是否阻塞后续 | **否**（本闸门已降级、无否决权） |

> **一句话**：BDK-wasm **在能力上确实可以离线工作**（并且我们用异源对照把它验到了地址级）；
> 但它**不是**我们的形态——2.3 MB 体积（本项目 89 KB 的约 23 倍）、预构建产物**含联网代码**、
> 官方入口在"零工具链"下不可用、自建还需额外装 clang。
> 它能不能进 P1，是一个**宪法问题（X1/X2）**，不是一个**可行性问题**。

---

## §1 逐项记录（三字段，Q1–Q5）

> **密钥材料说明（先声明，免误读）**：Q3 的证据文件中会出现形如 `tprv…` 的 **testnet 扩展私钥**。
> 它由**脚本中公开写明的合成种子**（`0a1b2c…5566`）确定性派生而来，**testnet、无真实资金、无链上数据**，
> 与 G3 补测使用的合成种子同类。**不是任何真实密钥**，无法用于获取资产。
> 保留原文是为了让第三方能逐字符复现验证；若不需要复现，请勿将其视作秘密使用。

### Q1a 离线构建（配置 / 依赖层面）｜**成立**

| 字段 | 记录 |
|---|---|
| ① **功能事实** | 默认特性（`default = []`，即**不启用 esplora**）下，`cargo tree --target wasm32-unknown-unknown -e normal` 中 **`esplora` / `reqwest` / `tokio` / `hyper` / `async-https` 全部无命中**；显式 `-F esplora` 时才出现 `bdk_esplora v0.22.2 → esplora-client v0.12.3 → reqwest v0.12.22`。<br>→ **"无联网组件的最小构建"在这份配置里是真实存在的。** |
| ② **对本项目的实际影响** | 说明"离线"在源码层面**不是**事后裁剪，而是默认形态——这一点对我们有利 |
| ③ **证据等级 / 底层原因** | **A**（`cargo tree` 原始输出）｜根因：联网能力被放在可选特性 `esplora` 里 |

### Q1b 本机产出 wasm 产物｜**⚪ 环境阻塞（未取得观测）**

| 字段 | 记录 |
|---|---|
| ① **功能事实** | `cargo build --release --target wasm32-unknown-unknown` **失败，退出码 101**：<br>`secp256k1-sys v0.10.1: Compiler family detection failed due to error: ToolNotFound: failed to find tool "clang": program not found`<br>本机**无 clang、无 Visual Studio**（已逐一核查 PATH 与常见安装路径）。 |
| ② **对本项目的实际影响** | 若采用 BDK，**自建需要额外的 C 工具链（LLVM/clang）** —— 这是我们"零工具链"形态之外的一项新依赖。**安装 LLVM 属系统级变更，本报告不代做**，留待项目负责人决定。 |
| ③ **证据等级 / 底层原因** | **A**（构建日志原文）｜根因：`secp256k1-sys` 为 wasm32 编译 libsecp256k1 的 C 代码，需要 C 编译器 —— **属本机环境，不是被測对象的属性** |

### Q2 离线加载（无网络前提下模块可用）｜**成立**

| 字段 | 记录 |
|---|---|
| ① **功能事实** | **Node 侧**：把 `globalThis.fetch` 与 `XMLHttpRequest` 替换为抛错函数后，`@bitcoindevkit/bdk-wallet-node@0.3.0` 模块**加载成功**（导出 45 个符号）；**`fetch`/XHR 被调用次数 = 0**。<br>**浏览器侧**：拦截一切非本地请求（全部 abort），页面加载过程**外部请求数 = 0，且未观察到任何联网尝试**。 |
| ② **对本项目的实际影响** | 这些 API 本身不触网 —— **与我们"运行时零外部请求"的宪法属性不冲突** |
| ③ **证据等级 / 底层原因** | **A** ｜根因：`Wallet.create` 不需要区块链后端/持久化器，故不触发任何同步 |

### Q3 离线描述符解析 / 地址派生｜**成立（且经异源对照）**

| 字段 | 记录 |
|---|---|
| ① **功能事实** | 全程无网络下成功调用：<br>· `seed_to_descriptor(seed,'testnet','p2wpkh')` → 返回 `wpkh(tprv…/84'/1'/0'/0/*)#ykq3zhza` 形式的 DescriptorPair<br>· `xpub_to_descriptor(xpub,'8236dc33','bitcoin','p2wpkh')` → 成功<br>· `Wallet.create(network, ext, int)` —— **无需任何区块链后端**<br>· `reveal_next_address('external')` → `tb1qqp33kh5v4j7kjpstmtu5maz4qdeu6p095mzns3`<br>**异源对照**：用 bitcoinjs-lib 5.2.1 + bip32（与 BDK 无共享代码）独立派生同一 seed 的同一路径：<br>`external[0]` → `tb1qqp33kh5v4j7kjpstmtu5maz4qdeu6p095mzns3`（**逐字符一致**）；`external[1]` 亦一致。 |
| ② **对本项目的实际影响** | 离线描述符/地址派生**在能力上成立**，且结果可被独立实现交叉验证 |
| ③ **证据等级 / 底层原因** | **A** ｜根因：描述符解析与 BIP-32 派生是纯本地计算 |

### Q4 无打包器（bundler）集成｜**官方入口 ❌ / 自写加载器 ✅**

| 字段 | 记录 |
|---|---|
| ① **功能事实** | web 包入口 `bitcoindevkit.js`（770 B）仅两行核心：`import * as wasm from "./bitcoindevkit_bg.wasm";` + `import { __wbg_set_wasm } from "./bitcoindevkit_bg.js";`<br>· **主实验**：本地静态服务（`.wasm` → `application/wasm`）+ 裸 `<script type="module">` → **加载失败**：<br>`Failed to load module script: Expected a JavaScript-or-Wasm module script but the server responded with a MIME type of "application/wasm"`<br>· **对照**：把 `.wasm` 改以 `text/javascript` 提供 → 越过 MIME 门槛后**按 JS 解析 wasm 字节**，报 `SyntaxError: Invalid or unexpected token`。<br>→ 证实 Chrome 把该导入**当作普通 JS 模块导入**（无 WebAssembly ESM integration）。<br>· **补充实验（防外推）**：不改 BDK 产物，加一段 **~10 行自定义加载器**（`fetch` → `WebAssembly.instantiate(bytes, {'./bitcoindevkit_bg.js': bg})` → `__wbg_set_wasm(instance.exports)`）→ **成功**，且派生地址与 Q3 一致、外部请求 0、控制台无错误。 |
| ② **对本项目的实际影响** | 「用官方入口 + 裸 module 脚本」这条路**不通**；「自写小加载器」这条路**通**。故这**不是**"BDK-wasm 无法在无打包器下使用"（那是过头结论），而是"**其官方入口的形态与我们的零工具链形态不匹配，需自写加载层**" |
| ③ **证据等级 / 底层原因** | **A** ｜根因：新版 wasm-bindgen 产出的是 **ESM 原生 wasm 导入**形态，需打包器处理 `.wasm` 导入或浏览器支持 ESM integration |

### Q5 体积代价｜**记录（不参与判定）**

| 字段 | 记录 |
|---|---|
| ① **功能事实** | `bdk-wallet-web@0.3.0`：`bitcoindevkit_bg.wasm` **2,140,519 B** + `bitcoindevkit_bg.js` 164,899 B + 入口 770 B ≈ **2.3 MB**（包解包合计 2,352,410 B）。<br>`bdk-wallet-node@0.3.0` 同量级。<br>**对照**：本项目 `web/lswasm.wasm` = **91,013 B（~89 KB）** → **约 23 倍**。<br>另：npm 上该包**仅 0.2.0 / 0.3.0 两个版本**，0.3.0 发布于 **2026-03-16**。 |
| ② **对本项目的实际影响** | 若进网页端，是一个数量级的体积代价 |
| ③ **证据等级 / 底层原因** | **A** ｜根因：BDK 引入了 `bdk_wallet` + `bitcoin` + `secp256k1` + 完整 wasm-bindgen 运行时 |

---

## §2 门槛判定（按预注册 §4.1 字面，不作事后修改）

预注册写的门槛是：`能做 = Q1 ✅ 且 Q2 ✅ 且 Q3 ✅ 且 Q4 ✅`；`不能做 = 任一 ❌ 且阻塞点可复现`；`不可判定 = 环境导致无法取得观测`。

**实测对照：**

| 子问题 | 实测 | 是否满足门槛条件 |
|---|---|---|
| Q1 离线构建 | Q1a 成立（依赖树已证）；**Q1b 因缺 clang 未取得产物** | **不是 ✅**（也未取得观测） |
| Q2 离线加载 | 成立 | ✅ |
| Q3 离线派生/解析 | 成立（含异源对照） | ✅ |
| Q4 无打包器集成 | **官方入口 ❌**；自写加载器 ✅ | **按字面 = ❌** |

→ **判定：⚪ 不可判定**（依据预注册 §1.3：环境导致未能取得观测）。

**但必须同时说明门槛本身在此暴露的两处不足（如实登记，不改预注册）：**

1. **Q4 的设计没有区分"形态"与"能力"。** 按字面 `Q4 ❌ → 不能做`，会把"官方入口的 import 写法与我们的形态不匹配"误读成"BDK-wasm 无法在无打包器下使用"——**而补充实验已证明后者为假**。
   → 这正是 G3 教过的同一种错误（把形态/固件问题当成能力缺失）。**故不采信字面的"不能做"结论。**
2. **Q1 被写成一个单一问题，实际含两个不同层面**（配置层 vs 本机产出层）。前者可证、后者受环境阻塞。应分层记录——本报告已分为 Q1a/Q1b。

> **门槛修订建议**：下次同类闸门应把 Q1 拆为"配置层 / 产出层"，并在 Q4 明确区分"官方入口形态"与"集成可行性"。**本轮不修改已冻条款**（预注册 §5-F7）。

---

## §3 可达结论（能力层面，全部有 A 级证据）

1. **BDK-wasm 的离线能力成立**：模块可在无网络下加载；描述符构造、钱包创建（无需区块链后端）、地址派生全部离线可用，**并经异源实现逐字符验证**。
2. **其官方分发形态与我们的零工具链形态不匹配**：入口依赖 ESM 原生 wasm 导入，裸 `<script type="module">` 加载失败；需自写约 10 行加载器。
3. **其预构建产物含联网代码**：`bitcoindevkit_bg.wasm` 二进制中出现 `esplora`(3) / `reqwest`(4) / `fetch`(2)。**注意这与 Q1a 是不同的两件事** —— 源码默认特性里没有这些 crate；**已发布的包不是按默认特性构建的**。
4. **体积代价一个数量级**：≈2.3 MB vs 本项目 ≈89 KB。

**受限说明**：以上第 1–4 条的"可用性"受 **X1/X2** 约束（见下），本闸门不裁决。

---

## §4 X1 / X2 冲突的登记（**主持人不得裁决**）

| # | 冲突 | 本次实测新增的事实 |
|---|---|---|
| **X1** | **零依赖内核 vs BDK 依赖树**：`lscore` 现为**零运行时依赖**；BDK 引入 `bdk_wallet 2.3.0` + `bitcoin 0.32.8` + `secp256k1` + wasm-bindgen 运行时 | **实测佐证**：体积从 89 KB 涨到 ≈2.3 MB（约 23 倍）；且**自建还需额外装 clang**（Q1b） |
| **X2** | **权威语义唯一 vs 第二解析器**：`CONTRIBUTING.md` 规定"唯一权威实现、禁止第二解析器" | **实测佐证**：BDK 确实能独立解析描述符并派生地址（Q3 已证），与 `lscore` 的地址语义**存在重叠** |

> 两条均为宪法级取舍，须三方讨论。**本报告只提供事实，不提供裁决。**

---

## §5 附带观察（不计入判定，按预注册 §9.4）

1. **`xpub_to_descriptor` 不校验扩展密钥的网络与 `network` 参数是否自洽。**
   传入**主网 xpub** 且 `network='testnet'` 时，它**不报错**，静默返回一个自相矛盾的描述符；错误延后到 `Wallet.create` 才以 `Key error: Invalid network` 抛出。
   → 与 G3 ④ 的"错误暴露晚/不清晰"**同类现象**。**如实登记，不计数**。
   （说明：本次我们自己的测试脚本踩到了这个坑，先归因到脚本再继续 —— 与 G3 处理 CRLF 固件陷阱同一纪律。）

2. **方法学教训（如果不记下来，将来会重犯）：对 wasm 做字符串检查必须用 `LC_ALL=C grep -a`，不能用 `strings | grep`。**
   本报告第一版用 `strings -a … | grep -c esplora` 得到 **0 命中**，据此差点写成"已发布产物不含联网代码"；
   改用 `LC_ALL=C grep -a` 后得到 `esplora`=3 / `reqwest`=4。**这是一次被自己抓住的错误结论。**

---

## §6 必拒错误自检（预注册 §9.3）

| # | 必拒错误 | 自检 |
|---|---|---|
| F1 | 以文档代替实测 | ✅ 全部结论来自实跑；README 的"仅支持 Esplora"等表述**未被用作结论依据**（反而被二进制检查修正：预构建包是带 esplora 的构建） |
| F2 | 网络请求未记录 / 假离线 | ✅ Node 侧把 fetch/XHR 打成抛错并计数（0）；浏览器侧拦截并列出全部请求（外部 0，本地逐个列出） |
| F3 | 构建成功当作离线可跑 | ✅ 未发生；且反过来**构建本身失败**（Q1b），已如实记为环境阻塞 |
| F4 | 工具链约束未区分 | ✅ Q4 明确区分「官方入口形态」与「自写加载器的可上线形态」；Q1b 明确指出自建需 clang 这一**额外工具链依赖** |
| F5 | 结论外推 | ✅ 未写成"BDK-wasm 无法在无打包器下使用"；**专门做了补充实验**以证其可用（Q4b） |
| F6 | 版本未锁定 | ✅ 源码 commit `73cb4df4…`；npm 包精确到 `0.3.0`（并记录发布时间 2026-03-16） |
| F7 | 未记许可与版本 | ✅ MIT OR Apache-2.0（GitHub API 显示 NOASSERTION，实际以 `LICENSE`+`Cargo.toml` 为准，已核实）；版本见上 |
| F8 | 三字段缺一 | ✅ Q1a–Q5 全部三字段齐全 |
| F9 | 无原始输出 | ✅ `g1/evidence/` 五份日志 + 一张截图；探针脚本在 `g1/tools/` |

---

## §7 复现方式

```bash
export PATH="/c/Users/simon/.workbuddy/binaries/node/versions/24.21.0:$PATH"

# ── 环境（关键：cargo 联网须把 CARGO_HOME 放到工作区内；沙箱会干扰 ~/.cargo）──
export CARGO_HOME=/d/work/_g1-cargohome
unset http_proxy https_proxy all_proxy
export HTTPS_PROXY=http://127.0.0.1:8800        # 经 VPN 代理

# ── Q1：源码与依赖树（证据 g1/evidence/g1-q1-offline-build.txt）──
git -c http.proxy=http://127.0.0.1:8800 clone --depth 1 \
    https://github.com/bitcoindevkit/bdk-wasm.git /d/work/_g1-bdkwasm
cd /d/work/_g1-bdkwasm
cargo tree --target wasm32-unknown-unknown -e normal | grep -iE 'esplora|reqwest|tokio|hyper'   # 期望：无命中
cargo tree --target wasm32-unknown-unknown -e normal -F esplora | grep -iE 'esplora|reqwest'      # 期望：有
cargo build --release --target wasm32-unknown-unknown    # 本机：退出码 101（缺 clang）

# ── Q5：已发布产物（证据 g1-q5-size-and-artifact.txt）──
mkdir -p /d/work/_g1-npm && cd /d/work/_g1-npm
for n in bdk-wallet-web bdk-wallet-node; do
  curl -s -x http://127.0.0.1:8800 -O "https://registry.npmjs.org/@bitcoindevkit/$n/-/$n-0.3.0.tgz"
done
mkdir -p x-web x-node && tar -xzf bdk-wallet-web.tgz -C x-web && tar -xzf bdk-wallet-node.tgz -C x-node

# ── Q2/Q3：离线派生 + 异源对照（证据 g1-q3-offline.txt）──
node g1_q3_offline.mjs          # CARAVAN_MODULES 指向含 playwright/bitcoinjs-lib/bip32 的 node_modules

# ── Q4：无打包器浏览器加载（证据 g1-q4-browser.txt / -MIME-control.txt）──
node g1_q4_browser.mjs
WASM_MIME="text/javascript; charset=utf-8" node g1_q4_browser.mjs      # 对照实验

# ── Q4b：自写加载器（证据 g1-q4b-altloader.txt）──
node g1_q4b_altloader.mjs
```

---

## §8 对后续的含义

- **本闸门不阻塞任何步骤**（预注册 §1.2 已预登记：两种结局都不改变 P1 形态）。
- **是否采用 BDK 是一个宪法问题（X1/X2），须三方讨论**；本报告只交付事实。
- **需要项目负责人决定的一项系统级变更**：是否安装 LLVM/clang 以闭合 Q1b（本报告不代做）。
- **已交付的可复用资产**：`g1/tools/g1_q3_offline.mjs` 的"异源对照 + 断网保险"模式，
  可直接复用为将来任何"增强层候选"的准入检验（**这是本次最有价值的副产品**）。

---

*本文为 G1 实测判定记录。预注册 `g1/G1-预注册-BDK-WASM离线能力实测.md` 的 §2–§6 未作任何修改；
文中指出的门槛不足与修订建议已单列于 §2，未用于修改已冻条款。*
