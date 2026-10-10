# Lotus Signer

> 别人让你签的东西，先过一遍 Lotus Signer。

**Lotus Signer —— 本地运行优先的待签数据审阅与签名协作工具。**

当有人把一份待签数据交给你时，Lotus Signer 在本地把它翻译成人能读懂的话、标注风险、由你确认后用你自己的钱包或硬件签名，再把签名结果交还发起方。不托管任何密钥，不设 Lotus 自有业务服务器。

> **承诺分四层**（终裁定义）：L-A 绝对承诺｜L-B 场景承诺（对明确支持且信息完整的 PSBT，审阅→签名→导出全链路离线）｜L-C 增强层（风险标注等，显著标注"需联网"，数据源用户自选）｜L-D 明确不承诺。**不使用"全程离线""完全不联网"的笼统表述。**

## 状态

| 项 | 内容 |
|---|---|
| 当前阶段 | **P1-0 技术验证闸门**（2 周 / 1 人） |
| 论证结论 | 方向成立，**P1-0 三条闸门（G1 BDK-WASM 实跑 / G2 WebHID 直连 / G3 Caravan 重合度实测）不全过则不进入 P1** |
| 主打场景 | BTC PSBT 受托审阅 + 签名 + 导出（无状态、不做钱包管理） |
| 目标人群 | 能自己看懂 PSBT 的技术型自托管多签用户（**刻意收窄**） |
| 开发投入 | 核心开发 1–2 人；止损线**前移至 P1-0** |
| 立项性质 | **技术信誉资产 + 战略期权**，不以收入 KPI 考核 |

## 产品宪法

1. **只处理"签名的内容"与"签名的结果"，不处理"账户"与"上链"。**
2. 不创建、不托管、不接触用户私钥。
3. 无 Lotus 自有业务服务器。
4. **代码分发可信是 P0 要求（九条措施），设为独立发布门槛，未通过不得发布，不因工期裁剪。**
5. 不支持的数据格式与字段必须**明确报错**，不允许静默降级、不允许用推测填补。
6. 每个关键字段须携带五态之一：`verified` / `parsed_unverified` / `missing` / `unsupported` / `check_failed`。
7. 输出分四层：交易事实 / 上下文解释 / 风险提示（须展示触发原因）/ 不提供的结论。**不输出"安全/危险"终局判定。**

## 明确不做

自研 MPC · 自研硬件设备 · 浏览器扩展 · 移动端（P3 前）· 与 LotusWallet 主站代码级集成（P3 前）· 代币叙事

## 目录

```
doc/          论证与技术文档
packages/     源码（core / parsers / risk / transport / web / cli）
```

## 文档索引（按阅读顺序）

| 文档 | 说明 |
|---|---|
| [`doc/2026-10-10_Lotus-Signer_论证会V2_第二轮终裁.md`](doc/2026-10-10_Lotus-Signer_论证会V2_第二轮终裁.md) | **★ 第二轮终裁（最新结论，冲突时以本文为准）** |
| [`doc/2026-10-10_Lotus-Signer_论证会V1_主持人裁决与决议.md`](doc/2026-10-10_Lotus-Signer_论证会V1_主持人裁决与决议.md) | 第一轮裁决与决议（V2 修订稿，上位依据） |
| [`doc/Lotus-Signer_论证会V1_第二轮讨论稿_顾问B-WorkBuddy.md`](doc/Lotus-Signer_论证会V1_第二轮讨论稿_顾问B-WorkBuddy.md) | 顾问 B 第二轮讨论稿（含 Caravan/BDK 实测） |
| [`doc/discuss-chatgpt-round2.md`](doc/discuss-chatgpt-round2.md) | 顾问 A 第二轮讨论稿 |
| [`doc/discuss-chatgpt-v1.md`](doc/discuss-chatgpt-v1.md) | 顾问 A 论证稿 V1 |
| [`doc/workbuddy-expert1-V1.md`](doc/workbuddy-expert1-V1.md) | 顾问 B 背景材料与开放议题 |

## 关于 LotusWallet

Lotus Signer 由 LotusWallet 团队发起，以独立产品形态开发，作为 LotusWallet 的技术信誉资产与获客漏斗。成熟后按 P3 阶段回灌主站。

## 远程仓库

GitHub 目标仓库：`https://github.com/LotusWallet/LOTSinger`（截至 2026-10-10 尚未创建，本地提交待其创建后推送）
