# G3 固件（PSBT 测试样本）

`make_psbt_fixtures.py` 生成，`verify_fixtures.py` 自校验，再经 **bip174（bitcoinjs 参考实现）异源交叉验证**。
三者均通过后方可用于实测。

```bash
cd ../tools
python make_psbt_fixtures.py     # 生成
python verify_fixtures.py        # 自校验（退出码 0 才继续）
# 异源验证：见文末
```

---

## 样本清单

| 样本 | 字节 | 内容 | 对应五态 |
|---|---|---|---|
| **S1** | 454 | 受托场景 2-of-3 P2WSH 多签，1 输入 2 输出（含找零），未签名 | `verified` 基线 |
| **S2a** | 408 | S1 移除 `witness_utxo` → 无法得知输入金额 | `missing` |
| **S2b** | 472 | S1 追加 `PSBT_IN_PROPRIETARY(0x0f)` 未知字段 | `unsupported` |
| **S3** | 561 | S1 追加 pk1 的一个 `partial_sig` → 已收集 1/2 签名 | 配合③导出部分签名 |
| **S4** | 483 | P2SH 对照样本：同一 2-of-3 脚本作 `redeemScript`，输入带完整前序交易 | `verified`（金额读自前序交易） |
| **S5** | 454 | 输入 500,000 sat 而输出合计 1,490,000 sat → 手续费无法成立 | `check_failed` |
| **S6** | 442 | S1 截去尾部 12 字节（map 未以 `0x00` 终止） | 必须显式报错（B2） |
| **S7** | 557 | **真实链上形态**：与 S4 同构，唯一区别是前序交易的输入**带非空 scriptSig** | `verified`（回归防护，见下） |
| **S8** | 557 | 前序交易 txid 与输入 outpoint **故意不匹配**（破坏 BIP-174 硬要求） | 输入金额必须 `check_failed` 并点名两值 |
| **S9** | 1020 | **第三方真实导出物**：Caravan coordinator 的 "Download Unsigned PSBT" 原始输出（含 `PSBT_GLOBAL_XPUB×3` + 3 条 `bip32_derivation`，**无专有字段**） | `verified`；标准字段**不得**被报成未知 |

共同参数：2-of-3 P2WSH，input 1,500,000 sat，send 1,200,000 sat，change 290,000 sat，fee 10,000 sat，locktime 0，tx version 2。

**固件为纯合成数据**：不含任何真实资金、真实 xpub 或真实私钥。
三个公钥由 secp256k1 生成元的整数倍导出，数学上是有效曲线点，但**无人持有对应私钥**。

---

## 为什么零第三方依赖

生成器不引入任何加密或 PSBT 库，纯字节构造。理由有二：

1. **隔离原则**：P0 要求第三方 PSBT 库不得进入解析链路。固件生成同样保持零依赖，
   避免任何"对照库 DNA 混入"的争议。
2. **可审计**：固件是 G3 全部观察的起点。若它由某个库产出，就等于把该库的行为假设
   带进了判据；手工构造让"样本是什么"这件事完全由我们负责。

---

## 实施中发现的两个缺陷（都已修复，留存为教训）
### 缺陷 1：txid 写成 33 字节（严重）

构造 prev outpoint 时，第二段 hex 手写成 34 个字符，导致 `prev_txid` 为 33 字节，
`unsigned tx` 从 outpoint 之后**整体偏移 1 字节**。

**为什么这条严重**：偏移会让任何标准解析器失败，而导入失败恰好落在差异项①（免配置导入，工作流阻断项）上。
**固件缺陷会被误读成被测工具的差异化证据**——这是能给 G3 造成方向性误判的一类 bug。

修复后加了断言：`assert len(prev_txid_hex) == 64`。**这类常量长度必须用断言拦住，不能靠肉眼数。**

### 缺陷 2：键顺序未排序，被参考实现重排

`S3` 的 `partial_sig` 追加在所有键值末尾，导致 bip174 反序列化→再序列化时把它挪到前面，
产生 13 字节的差异（总长度不变）。根因是 **BIP-174 规定键应按键类型升序排列（SHOULD）**。

修复：生成时统一按 `(keytype, keydata)` 排序。

> **这条的价值超出固件本身** —— 它是对 **T0「canonical 比对基准」必要性的一次实证**：
> 连 bitcoinjs 的参考实现都会重排键序。所以在 native 与 wasm 之间做比对时，
> **若比字节，一定会产生纯格式层面的假失败**；必须比结构化语义，并把键序、整数表示等
> 记入「非语义差异清单」。这与 V4 §4.4 预注册的判断一致，此处获得了现实证据。

---

## 双重验证

### 1. 自校验（`verify_fixtures.py`）

零依赖反向解析，断言：magic 正确、字节完全消费无残余、scriptSig 全空（BIP-174 要求）、
拓扑 1-in/2-out、各字段存在性符合预期、跨样本一致性（S1 与 S3 的 unsigned tx 完全相同）。

### 2. 异源交叉验证（bip174）

自生成自解析存在**循环论证**风险：生成器与自验证器出自同一套对 PSBT 的理解，
若该理解本身有偏差，两者会一致地错。因此引入 bitcoinjs 官方参考实现 `bip174` 打破循环。

验证位于**项目外**（`C:/Users/simon/.psbtxcheck`），不进仓库，避免污染项目依赖树：

```bash
export NODE_PATH="C:/Users/simon/.psbtxcheck/node_modules"
node C:/Users/simon/.psbtxcheck/check.js
```

判据：`Psbt.fromBase64()` 成功，且 `toBase64()` 与原文**逐字节一致**。

```
S1  ✅  S2a ✅  S2b ✅  S3 ✅
全部通过：固件为结构合法的 BIP-174 报文，可用于 G3 实测。
```

> 注：`bip174` 是纯 PSBT 容器库，不含交易反序列化器，须外部提供 `txGetter`。
> 脚本内以独立方式实现了交易计数（不复用 Python 生成器逻辑），保持两方独立。

---

## 由第三方真实产物暴露的两个内核缺陷（2026-10-12，均已修复）

**S7 / S8 / S9 不是构造出来的假想场景，而是被一个第三方工具的真实导出物逼出来的。**

在做 G3 差异项③补测时，我们让 Caravan（本地构建 v1.19.2）通过其真实 UI 路径导出了一笔
未签名 PSBT（`g3/evidence/g3c-E-dl-transaction.psbt`，现固化为 **S9**），再拿本工具去审阅它。
结果两个缺陷当场暴露——**此前的全部固件都没能发现它们**：

| # | 缺陷 | 后果 | 为什么旧固件发现不了 | 修复 |
|---|---|---|---|---|
| 1 | 内核把「未签名交易的 scriptSig 必须为空」严格规则**复用**到 `non_witness_utxo` 的前序交易上 | 真实链上交易**本来就带 scriptSig** → **任何来自真实协调器/钱包的 P2SH 多签 PSBT，输入金额都被误判 `check_failed`**（虚假告警，直接毁掉主要使用场景） | 旧固件（含 S4）的前序交易一律填**空 scriptSig**，恰好绕过了该分支 | 拆分为 `parse_unsigned_tx`（严格）与 `parse_prev_tx`（宽松），并补 BIP-174 要求的 **txid 一致性校验**（S8） |
| 2 | 把 **BIP-174 标准字段** `PSBT_GLOBAL_XPUB(0x01)`、`PSBT_IN_BIP32_DERIVATION(0x06)` 判为"未知/专有"；且全局字段误用 `input_key_name()` 命名 | 几乎**每个真实 PSBT 都刷出 R001**（风险提示被噪声淹没，等于训练用户忽略告警）；且字段名显示为错误的 `PSBT_IN_WITNESS_UTXO` | 旧固件不含 `GLOBAL_XPUB`，且 S4 的 bip32_derivation 未被检查 | 新增 `global_key_name()` 与 `unknown_global_key_type()`；只有 `0x0f` proprietary 与表外类型才报 R001 |

**教训（已写入 T4 报告）**：构造的固件只能验证"我们想到了的情形"。
**必须拿真实第三方产物来打自己**——差异项③的补测顺带完成了这件事，收益超过补测本身。
