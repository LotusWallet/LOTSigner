# 对外发布内容队列

> **用途**：所有要发到 X / Telegram 的内容，**先写在这里，再由项目负责人手工复制发布。**
> 不做 API 集成、不建 bot（2026-10-11 决定：发布不是高频工作）。
> 本文件随 git 提交，因此"某条信息是什么时候、以什么措辞发出的"**可追溯**——这本身就是
> 「凡需下发的信息都以 GitHub 开源库方式承载」的落地形式。

---

## 写稿纪律（写之前必读，源自产品宪法）

| # | 禁止 | 原因 |
|---|---|---|
| 1 | **不得出现"更安全""防诈骗""保护你的资产"等终局判定式宣传** | 宪法第 7 条：不输出安全/危险终局判定。这类措辞正是我们批评的**虚假安全感** |
| 2 | **不得声称"全程离线""完全不联网"** | 承诺分四层（L-A 绝对 / L-B 场景 / L-C 增强并标注需联网 / L-D 不承诺）。笼统绝对表述必然失真 |
| 3 | **未过 T0 与签名三要件的产物，不得称为"官方发布"** | 宪法第 4 条。提前提供的只能是"源码 + 自建构建说明"，须标注**未经官方签名核验** |
| 4 | **待核验事项一律不得写进对外内容** | 未核实即按未知处理，不得坐实 |
| 5 | **不得暗示能替代硬件钱包或任何其他工具** | 定位是"第二意见/独立验证证据"，不是替代品 |
| 6 | 引用第三方项目（Caravan / BDK / Sparrow 等）须**写明版本与许可** | 许可差异会影响下游用户（如 Nunchuk 是 GPLv3） |
| 7 | **不得出现"竞品做不到/没有竞品/只有我们能"式暗示** | G3 实测（`g3/G3-判定表-补测修订-③.md`）：我们预设的三个差异点 ①③⑤ **全部被反证**。判定为**有条件通过**（仅质量项达标）→ 护城河强度低一个量级。此类暗示既是 E5 式外推，也与实测不符。可陈述的只有**可被任何人核验的现象**（见 G3 判定表 §4） |

**每条发布须附**：关联 commit 或 PR、可核验来源链接、发布状态。

---

## 队列

> 状态标记：`⬜ 草稿` → `🟡 待发布` → `✅ 已发布`

### 🟡 [2026-10-12] Lotus Signer 公开内测：源码 + 自建构建

**平台**：X / Telegram / 两者（同稿）
**关联**：commit `9381f7d`（Track-1 冲刺）· 仓库 https://github.com/LotusWallet/LOTSigner
**状态**：✅ 已发布（2026-10-12，项目负责人手工发布于 X / Telegram，仓库同日转 public）

**正文（中文）**
> Lotus Signer 已公开（github.com/LotusWallet/LOTSigner）。
>
> 它做的事很简单：别人让你签的东西——先过一遍 Lotus Signer。本地还原待签交易的事实，标出缺什么、不支持什么，然后由**你自己的钱包或硬件**去签。它不托管密钥、没有我们的业务服务器、审阅全链路零外部请求。
>
> 当前为内测形态：**仅提供源码 + 一条命令自建**（`git clone` + `./build.sh`），**尚无官方签名产物**——请只从源码自行构建，勿从其他渠道获取可执行文件。
>
> 已知边界在 KNOWN-LIMITATIONS.md 里写明：目前只处理 BTC PSBT；不支持的内容会明确报错，不会静默降级；`verified` 表示"通过了明确的解析与校验规则"，**不表示交易已验证安全**。
>
> 使用前请先读 README 与 KNOWN-LIMITATIONS.md。发现问题请开 issue。

**正文（English）**
> Lotus Signer is now public: github.com/LotusWallet/LOTSigner
>
> It does one thing: before you sign what someone else handed you, run it through Lotus Signer. It reconstructs the transaction facts locally, marks what is missing and what is unsupported, and you sign with **your own** wallet or hardware. It never custodies a key, there is no Lotus-operated server, and the review chain issues zero external requests.
>
> This is a **beta offered as source + one-command build only** (`git clone` + `./build.sh`). **No officially signed artifact exists yet** — build it yourself from source; do not trust binaries from any other channel.
>
> Boundaries are written down in KNOWN-LIMITATIONS.md: BTC PSBT only; unsupported content raises an explicit error, never silent degradation; `verified` means "passed the defined parsing and verification rules" — **it does not mean the transaction has been verified as safe**.
>
> Read the README and KNOWN-LIMITATIONS.md before use. Issues welcome.

**附链接**：仓库主页 · KNOWN-LIMITATIONS.md · BETA-READINESS.md（判定依据）
**自查**：[x] 无终局判定　[x] 无绝对离线表述（写"审阅全链路零外部请求"，属 L-B 场景描述，未称"完全不联网"）　[x] 无未核验事实　[x] 无"官方发布"措辞（明示无官方签名产物）　[x] 未暗示替代硬件钱包　[x] 无竞品比较暗示（未提及任何第三方项目）

### 模板（新增条目时照抄）

```markdown
### 🟡 [YYYY-MM-DD] 标题

**平台**：X / Telegram / 两者
**关联**：commit `xxxxxxx` · PR #n
**状态**：⬜ 草稿

**正文（中文）**
> ……

**正文（English）**
> ……

**附链接**：……
**自查**：[ ] 无终局判定　[ ] 无绝对离线表述　[ ] 无未核验事实　[ ] 附可核验来源　[ ] 无竞品比较暗示
```

---

## 已发布归档

（空）
