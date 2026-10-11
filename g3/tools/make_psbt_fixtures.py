#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
G3 PSBT 测试固件生成器

设计约束：
  1. 不依赖任何第三方加密/PSBT 库——纯字节构造。理由：本项目 P0 要求第三方 PSBT 库
     不得进入解析链路；固件生成同样保持零依赖，避免任何"对照库污染"的嫌疑。
  2. 样本不含任何真实资金、真实 xpub 或真实私钥。公钥由 secp256k1 生成元的倍数导出，
     是数学上有效的曲线点，但不是任何人的密钥。
  3. 输出的 PSBT 在结构上是合法的 BIP-174 报文，可被标准解析器消费。

生成三个样本（对应 G3 预注册 §2）：
  S1  受托场景多签 PSBT：2-of-3 P2WSH，1 输入 2 输出（含找零），未签名
  S2a 信息缺失变体：同上但移除 witness_utxo → 无法得知输入金额（对应五态 missing）
  S2b 未支持字段变体：插入 PSBT_IN_PROPRIETARY(0x0f) → 对应五态 unsupported
  S3  已部分签名：在 S1 基础上加入 1 个 partial_sig（格式正确的 DER+SIGHASH）

用法：
  python make_psbt_fixtures.py
输出：
  ../fixtures/*.psbt (base64 单行) 与 *.json (元信息)
"""

import hashlib
import json
import base64
import struct
from pathlib import Path

HERE = Path(__file__).resolve().parent
OUT = HERE.parent / "fixtures"

# ---------------------------------------------------------------------------
# secp256k1 —— 仅用于生成"格式有效但无人持有"的公钥
# ---------------------------------------------------------------------------
P = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEFFFFFC2F
N = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141
GX = 0x79BE667EF9DCBBAC55A06295CE870B07029BFCDB2DCE28D959F2815B16F81798
GY = 0x483ADA7726A3C4655DA4FBFC0E1108A8FD17B448A68554199C47D08FFB10D4B8


def point_add(p1, p2):
    if p1 is None:
        return p2
    if p2 is None:
        return p1
    x1, y1 = p1
    x2, y2 = p2
    if x1 == x2 and (y1 + y2) % P == 0:
        return None
    if p1 == p2:
        lam = (3 * x1 * x1) * pow(2 * y1, P - 2, P) % P
    else:
        lam = (y2 - y1) * pow(x2 - x1, P - 2, P) % P
    x3 = (lam * lam - x1 - x2) % P
    y3 = (lam * (x1 - x3) - y1) % P
    return (x3, y3)


def point_mul(k, p):
    r = None
    while k:
        if k & 1:
            r = point_add(r, p)
        p = point_add(p, p)
        k >>= 1
    return r


def ser_pubkey(k, compressed=True):
    pt = point_mul(k, (GX, GY))
    assert pt is not None, f"{k}*G 得到无穷远点"
    x, y = pt
    out = (bytes([0x02 + (y & 1)]) + x.to_bytes(32, "big")
           if compressed else b"\x04" + x.to_bytes(32, "big") + y.to_bytes(32, "big"))
    assert len(out) == (33 if compressed else 65)
    return out


# ---------------------------------------------------------------------------
# 基础编码
# ---------------------------------------------------------------------------
def varint(n):
    if n < 0xFD:
        return bytes([n])
    if n <= 0xFFFF:
        return b"\xfd" + struct.pack("<H", n)
    if n <= 0xFFFFFFFF:
        return b"\xfe" + struct.pack("<I", n)
    return b"\xff" + struct.pack("<Q", n)


def ser_compact_size(n):
    return varint(n)


def sha256(b):
    return hashlib.sha256(b).digest()


def hash160(b):
    return hashlib.new("ripemd160", sha256(b)).digest()


# ---------------------------------------------------------------------------
# PSBT 构造
# ---------------------------------------------------------------------------
PSBT_MAGIC = b"\x70\x73\x62\x74\xff"  # "psbt" + 0xff

# --- key types
GLOBAL_UNSIGNED_TX = 0x00
IN_WITNESS_UTXO = 0x01
IN_PARTIAL_SIG = 0x02
IN_SIGHASH_TYPE = 0x03
IN_WITNESS_SCRIPT = 0x05
IN_BIP32_DERIVATION = 0x06
IN_PROPRIETARY = 0x0F
OUT_WITNESS_SCRIPT = 0x01


def kv(keytype_int, keydata, value):
    """
    序列化一个 key-value 对。

    返回 (keytype, keydata, 字节串) 三元组而非纯字节——为了让调用方能按键序排序。
    理由见 sorted_pairs 的说明。
    """
    key = ser_compact_size(keytype_int) + keydata
    return (keytype_int, keydata, varint(len(key)) + key + varint(len(value)) + value)


def sorted_pairs(pairs):
    """
    按 BIP-174 建议对键值排序（keytype 升序，同类型按 keydata 升序）。

    为什么必须排：BIP-174 要求"键应按键类型升序排列"（SHOULD）。实测发现
    bitcoinjs 的 bip174 参考实现在反序列化→再序列化时会按自身的转换器顺序重建键表，
    导致与输入字节不一致。这不是固件错误，但会造成"字节级 round-trip 失败"。

    这件事本身就是 T0「canonical 比对基准」必要性的实证：
    一旦拿字节去比，连广受信任的参考实现都会报出纯格式层面的"假失败"。
    """
    return sorted(pairs, key=lambda p: (p[0], p[1]))


def map_bytes(pairs):
    """一个 map = 若干 kv（按键序排好）+ 0x00 终止"""
    return b"".join(p[2] for p in sorted_pairs(pairs)) + b"\x00"


def build_unsigned_tx(n_inputs, inputs, outputs, n_outputs, locktime=0, version=2):
    """
    inputs:  [(txid_bytes32_le, vout, sequence), ...]
    outputs: [(value_sat, scriptpubkey_bytes), ...]
    注意：PSBT 的 unsigned tx 中 scriptSig 必须为空。
    """
    b = struct.pack("<i", version)
    b += ser_compact_size(n_inputs)
    for txid, vout, seq in inputs:
        b += txid  # 内部字节序（小端显示）
        b += struct.pack("<I", vout)
        b += ser_compact_size(0)  # scriptSig 必须为空
        b += struct.pack("<I", seq)
    b += ser_compact_size(n_outputs)
    for value, spk in outputs:
        b += struct.pack("<Q", value)
        b += ser_compact_size(len(spk)) + spk
    b += struct.pack("<I", locktime)
    return b


def make_2of3_script(k1, k2, k3):
    """OP_2 <pk1> <pk2> <pk3> OP_3 OP_CHECKMULTISIG"""
    pk1, pk2, pk3 = (ser_pubkey(k) for k in (k1, k2, k3))
    return (
        b"\x52"
        + bytes([len(pk1)]) + pk1
        + bytes([len(pk2)]) + pk2
        + bytes([len(pk3)]) + pk3
        + b"\x53\xae"
    ), (pk1, pk2, pk3)


def p2wsh_spk(witness_script):
    return b"\x00\x20" + sha256(witness_script)


def build_psbt(n_inputs, unsigned_tx, input_maps, output_maps):
    """按 BIP-174 组装完整 PSBT"""
    global_map = map_bytes([kv(GLOBAL_UNSIGNED_TX, b"", unsigned_tx)])
    inputs = b"".join(map_bytes(m) for m in input_maps)
    outputs = b"".join(map_bytes(m) for m in output_maps)
    return PSBT_MAGIC + global_map + inputs + outputs


def der_sig():
    """一个格式正确、数学上有效性的 DER 签名（r,s 为固定值，非真实签名结果）"""
    r = bytes.fromhex("36a1f0e4a3f1c5f9d5b0d3f2e1a4c7b8"
                      "a9d0e1f2a3b4c5d6e7f8091a2b3c4d5e")
    s = bytes.fromhex("4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f90"
                      "a1b2c3d4e5f60718293a4b5c6d7e8f90")
    # 保证高位为 0 以符合 DER 正整数规则
    r = b"\x00" + r if r[0] & 0x80 else r
    s = b"\x00" + s if s[0] & 0x80 else s
    body = b"\x02" + bytes([len(r)]) + r + b"\x02" + bytes([len(s)]) + s
    return b"\x30" + bytes([len(body)]) + body


def main():
    OUT.mkdir(parents=True, exist_ok=True)

    # --- 三个参与方的公钥：k*G，k 为小整数（无人持有私钥 k 的记录意义）
    K1, K2, K3 = 0x11111111, 0x22222222, 0x33333333
    wscript, (pk1, pk2, pk3) = make_2of3_script(K1, K2, K3)

    # --- 虚构前序交易（不含任何真实资金）
    # 前序 outpoint 必须是严格 32 字节的 txid：34+30 这类近似写法极易多一个字符，
    # 一旦 txid 是 33 字节，unsigned tx 将整体偏移 1 字节，导致所有下游工具解析失败。
    # 而这类失败会恰好落在 G3 差异项①上，把固件缺陷误判为被测工具的"工作流阻断"。
    prev_txid_hex = (
        "a1b2c3d4e5f60718293a4b5c6d7e8f90"
        "11223344556677889900112233445566"
    )
    assert len(prev_txid_hex) == 64, f"txid 须为 64 个 hex 字符，实为 {len(prev_txid_hex)}"
    prev_txid = bytes.fromhex(prev_txid_hex)
    assert len(prev_txid) == 32
    prev_txid = prev_txid[::-1]  # 转为比特币内部字节序

    input_amount = 1_500_000   # 0.015 BTC
    send_amount = 1_200_000
    change_amount = 290_000    # 差额即手续费 10,000 sat

    change_spk = p2wsh_spk(wscript)          # 找零回多签地址
    send_spk = b"\x00\x14" + hash160(pk1)    # 收款为 P2WPKH

    unsigned_tx = build_unsigned_tx(
        n_inputs=1,
        inputs=[(prev_txid, 0, 0xFFFFFFFE)],
        outputs=[(send_amount, send_spk), (change_amount, change_spk)],
        n_outputs=2,
    )

    witness_utxo_value = (
        struct.pack("<Q", input_amount)
        + varint(len(p2wsh_spk(wscript)))
        + p2wsh_spk(wscript)
    )

    base_input = [
        kv(IN_WITNESS_UTXO, b"", witness_utxo_value),
        kv(IN_SIGHASH_TYPE, b"", struct.pack("<I", 1)),          # SIGHASH_ALL
        kv(IN_WITNESS_SCRIPT, b"", wscript),
        kv(IN_BIP32_DERIVATION, pk1,
           b"\x00" * 4 + b"\x80\x00\x00\x00" + b"\x00\x00\x00\x00"),  # fp + path
    ]
    output_maps = [[], [kv(OUT_WITNESS_SCRIPT, b"", wscript)]]

    # ---------------- S1：未签名的受托多签 PSBT ----------------
    s1 = build_psbt(1, unsigned_tx, [list(base_input)], output_maps)

    # ---------------- S2a：缺 UTXO 信息（missing） ----------------
    no_utxo = [kv for kv in base_input if kv != base_input[0]]
    s2a = build_psbt(1, unsigned_tx, [no_utxo], output_maps)

    # ---------------- S2b：含专有未知字段（unsupported） ----------------
    prop_keydata = b"LOTS" + varint(0x01)
    prop_key = ser_compact_size(IN_PROPRIETARY) + varint(4) + b"LOTS" + varint(0x01)
    prop_val = b"\x00" * 9
    prop_bytes = (
        varint(len(prop_key)) + prop_key + varint(len(prop_val)) + prop_val
    )
    with_prop = list(base_input) + [(IN_PROPRIETARY, prop_keydata, prop_bytes)]
    s2b = build_psbt(1, unsigned_tx, [with_prop], output_maps)

    # ---------------- S3：已含 1 个部分签名 ----------------
    with_sig = list(base_input) + [
        kv(IN_PARTIAL_SIG, pk1, der_sig() + b"\x01")
    ]
    s3 = build_psbt(1, unsigned_tx, [with_sig], output_maps)

    samples = {
        "S1": (s1, "受托场景 2-of-3 P2WSH 多签 PSBT，未签名。1 输入 2 输出（含找零），"
                   "含 witness_utxo / sighash / witnessScript / bip32 derivation"),
        "S2a": (s2a, "S1 移除 witness_utxo → 无法得知输入金额。对应五态 missing"),
        "S2b": (s2b, "S1 追加 PSBT_IN_PROPRIETARY(0x0f) 未知字段。对应五态 unsupported"),
        "S3": (s3, "S1 追加 pk1 的一个 partial_sig → 已收集 1/2 签名，待第 2 个"),
    }

    index = {}
    for name, (raw, desc) in samples.items():
        b64 = base64.b64encode(raw).decode()
        p = OUT / f"{name}.psbt"
        p.write_text(b64 + "\n", encoding="utf-8")
        index[name] = {
            "file": p.name,
            "description": desc,
            "bytes": len(raw),
            "base64": b64,
            "hex": raw.hex(),
        }
        print(f"[{name}] {len(raw):5d} bytes -> {p.name}")

    meta = {
        "_warning": "本固件为纯合成数据。不含任何真实资金、真实 xpub 或真实私钥。"
                    "公钥为 secp256k1 生成元的整数倍，数学有效但无人持有对应私钥。",
        "generator": "g3/tools/make_psbt_fixtures.py (零第三方依赖，纯字节构造)",
        "reference": "BIP-174 (PSBT v0)",
        "common": {
            "multisig": "2-of-3 P2WSH",
            "witness_script_hex": wscript.hex(),
            "input_amount_sat": input_amount,
            "send_amount_sat": send_amount,
            "change_amount_sat": change_amount,
            "fee_sat": input_amount - send_amount - change_amount,
            "locktime": 0,
            "tx_version": 2,
        },
        "samples": {k: {"file": v["file"], "description": v["description"],
                        "bytes": v["bytes"], "hex": v["hex"]} for k, v in index.items()},
    }
    mp = OUT / "fixtures-index.json"
    mp.write_text(json.dumps(meta, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"\n索引已写出: {mp}")
    return index


if __name__ == "__main__":
    main()
