# Lotus Signer

> 别人让你签的东西，先过一遍 Lotus Signer。

**Lotus Signer —— 离线签名审阅与协作台。**

当有人把一份待签数据交给你时，Lotus Signer 在本地把它翻译成人能读懂的话、标注风险、由你确认后用你自己的钱包或硬件签名，再把签名结果交还发起方。全程本地计算，不联网也能用，不托管任何密钥。

## 状态

| 项 | 内容 |
|---|---|
| 当前阶段 | 立项论证完成，进入 P1（MVP）详细设计 |
| 论证结论 | 方向成立，批准进入 P1 |
| 主打场景 | BTC PSBT 审阅 + 签名 + 协作 |
| 开发投入 | 核心开发 1–2 人 / 10–12 周 |

## 产品宪法

1. **只处理"签名的内容"与"签名的结果"，不处理"账户"与"上链"。**
2. 不创建、不托管、不接触用户私钥。
3. 无 Lotus 自有业务服务器；核心功能离线可用。
4. **代码分发可信是 P0 要求，不因工期裁剪。**
5. 不支持的数据格式必须明确报错，不允许静默降级。
6. 只输出事实还原与风险提示，不输出"安全/危险"终局判定。

## 明确不做

自研 MPC · 自研硬件设备 · 浏览器扩展 · 移动端（P3 前）· 与 LotusWallet 主站代码级集成（P3 前）· 代币叙事

## 目录

```
doc/          论证与技术文档
packages/     源码（core / parsers / risk / transport / web / cli）
```

## 文档

| 文档 | 说明 |
|---|---|
| [`doc/2026-10-10_Lotus-Signer_论证会V1_主持人裁决与决议.md`](doc/2026-10-10_Lotus-Signer_论证会V1_主持人裁决与决议.md) | **第一轮专家论证会裁决与决议（V2 修订稿）**（立项依据，必读） |
| [`doc/discuss-chatgpt-v1.md`](doc/discuss-chatgpt-v1.md) | 顾问 A 论证稿 V1（产品战略与市场） |
| [`doc/workbuddy-expert1-V1.md`](doc/workbuddy-expert1-V1.md) | 顾问 B 背景材料与开放议题 |

## 关于 LotusWallet

Lotus Signer 由 LotusWallet 团队发起，以独立产品形态开发，作为 LotusWallet 的技术信誉资产与获客漏斗。成熟后按 P3 阶段回灌主站。

## 远程仓库

GitHub 目标仓库：`https://github.com/LotusWallet/LOTSinger`（截至 2026-10-10 尚未创建，本地提交待其创建后推送）
