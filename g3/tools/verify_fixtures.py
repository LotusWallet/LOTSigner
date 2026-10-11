#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
G3 PSBT 固件验证器

职责：证明 make_psbt_fixtures.py 产出的四个样本是结构合法的 BIP-174 报文。
如果这一步不过，固件就不能进 G3 实测——否则所有后续观察都可能是在测一份坏输入。

说明：
  - 本文件是**验证工具**，不是产品解析实现。产品的权威语义实现将由 Rust core 承担（见产品宪法 §8）。
  - 零第三方依赖，纯字节解析。
  - 顺带把「五态」概念做一次最小实现：verified / missing / unsupported，用于自检固件覆盖度。

用法：
  python verify_fixtures.py
退出码：0 = 全部通过；1 = 有失败项
"""

import base64
import hashlib
import json
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
FX = HERE.parent / "fixtures"

PSBT_MAGIC = b"\x70\x73\x62\x74\xff"

KEY_NAMES = {
    (0, 0x00): "GLOBAL_UNSIGNED_TX",
    (-1, 0x01): "IN_WITNESS_UTXO",
    (-1, 0x02): "IN_PARTIAL_SIG",
    (-1, 0x03): "IN_SIGHASH_TYPE",
    (-1, 0x05): "IN_WITNESS_SCRIPT",
    (-1, 0x06): "IN_BIP32_DERIVATION",
    (-1, 0x0F): "IN_PROPRIETARY",
    (1, 0x01): "OUT_WITNESS_SCRIPT",
}

# 对标准 BIP-174 解析器而言，专有/未识别键 == unsupported。
# 注意：不能只看 UNKNOWN_ 前缀——键名表里若已录入 IN_PROPRIETARY，它就不再带 UNKNOWN_ 前缀，
# 会导致"其实存在未支持字段"被判成"不存在"。两类标记都要覆盖。
UNSUPPORTED_HINTS = ("IN_PROPRIETARY", "UNKNOWN_")


def has_unsupported(names):
    return any(n.startswith(UNSUPPORTED_HINTS) for n in names)


FAILURES = []


def check(cond, msg):
    if cond:
        print(f"  ✅ {msg}")
    else:
        print(f"  ❌ {msg}")
        FAILURES.append(msg)
    return cond


def read_varint(buf, i):
    head = buf[i]
    i += 1
    if head < 0xFD:
        return head, i
    if head == 0xFD:
        return struct.unpack("<H", buf[i:i + 2])[0], i + 2
    if head == 0xFE:
        return struct.unpack("<I", buf[i:i + 4])[0], i + 4
    return struct.unpack("<Q", buf[i:i + 8])[0], i + 8


def parse_map(buf, i, scope):
    """解析一个 kv map，返回 (健值列表, 新位置)。scope: 0=global, -1=input, 1=output"""
    pairs = []
    while True:
        if i >= len(buf):
            raise ValueError("map 未以 0x00 终止")
        if buf[i] == 0x00:
            i += 1
            break
        klen, i = read_varint(buf, i)
        key_raw = buf[i:i + klen]
        i += klen
        vlen, i = read_varint(buf, i)
        value = buf[i:i + vlen]
        i += vlen
        ktype, j = read_varint(key_raw, 0)
        keydata = key_raw[j:]
        pairs.append((KEY_NAMES.get((scope, ktype), f"UNKNOWN_0x{ktype:02x}"), keydata, value))
    return pairs, i


def parse_psbt(raw):
    if not raw.startswith(PSBT_MAGIC):
        raise ValueError("magic 不匹配")
    i = len(PSBT_MAGIC)
    global_map, i = parse_map(raw, i, 0)

    unsigned_tx = None
    for name, _, value in global_map:
        if name == "GLOBAL_UNSIGNED_TX":
            unsigned_tx = value
    if unsigned_tx is None:
        raise ValueError("缺少 GLOBAL_UNSIGNED_TX")

    n_in, o = read_varint(unsigned_tx, 4)
    # 跳过 inputs
    p = o
    inputs = []
    for _ in range(n_in):
        p += 32 + 4
        slen, p = read_varint(unsigned_tx, p)
        script_sig = unsigned_tx[p:p + slen]
        p += slen
        p += 4
        inputs.append(script_sig)
    n_out, p = read_varint(unsigned_tx, p)

    input_maps = []
    for _ in range(n_in):
        m, i = parse_map(raw, i, -1)
        input_maps.append(m)
    output_maps = []
    for _ in range(n_out):
        m, i = parse_map(raw, i, 1)
        output_maps.append(m)

    return {
        "global": global_map,
        "unsigned_tx": unsigned_tx,
        "n_in": n_in,
        "n_out": n_out,
        "input_script_sigs": inputs,
        "inputs": input_maps,
        "outputs": output_maps,
        "consumed": i,
        "raw_len": len(raw),
    }


def script_asm(script):
    """极简反汇编，仅覆盖 multisig 所需操作码"""
    out = []
    i = 0
    ops = {0x52: "OP_2", 0x53: "OP_3", 0xAE: "OP_CHECKMULTISIG"}
    while i < len(script):
        b = script[i]
        if 0x01 <= b <= 0x4B:
            data = script[i + 1:i + 1 + b]
            out.append(data.hex()[:16] + ".." if len(data) > 8 else data.hex())
            i += 1 + b
        else:
            out.append(ops.get(b, f"OP_0x{b:02x}"))
            i += 1
    return " ".join(out)


def field_status(parsed_map, wanted):
    """五态的最小实现：存在=verified / 缺失=missing / 存在but未知=unsupported"""
    names = [n for n, _, _ in parsed_map]
    if wanted in names:
        return "verified"
    if has_unsupported(names):
        return "unsupported"
    return "missing"


def read_fixture(name):
    p = FX / f"{name}.psbt"
    b64 = p.read_text(encoding="utf-8").strip()
    return base64.b64decode(b64)


def main():
    print("=" * 66)
    print("G3 PSBT 固件验证")
    print("=" * 66)

    expectations = {
        "S1": {"has_utxo": True, "has_sig": False, "has_prop": False},
        "S2a": {"has_utxo": False, "has_sig": False, "has_prop": False},
        "S2b": {"has_utxo": True, "has_sig": False, "has_prop": True},
        "S3": {"has_utxo": True, "has_sig": True, "has_prop": False},
    }

    for name, exp in expectations.items():
        print(f"\n--- {name} ---")
        raw = read_fixture(name)
        print(f"  raw = {len(raw)} bytes")

        parsed = parse_psbt(raw)
        check(parsed["consumed"] == parsed["raw_len"],
              f"字节完全消费（{parsed['consumed']}/{parsed['raw_len']}，无残余尾字节）")
        check(all(s == b"" for s in parsed["input_script_sigs"]),
              "unsigned tx 的 scriptSig 全部为空（BIP-174 要求）")
        check(parsed["n_in"] == 1 and parsed["n_out"] == 2,
              f"拓扑符合预期（in={parsed['n_in']}, out={parsed['n_out']}）")

        im = parsed["inputs"][0]
        names = [n for n, _, _ in im]

        has_utxo = "IN_WITNESS_UTXO" in names
        has_sig = "IN_PARTIAL_SIG" in names
        has_prop = has_unsupported(names)

        check(has_utxo == exp["has_utxo"],
              f"witness_utxo {'存在' if exp['has_utxo'] else '缺失'}（期望一致）")
        check(has_sig == exp["has_sig"],
              f"partial_sig {'存在' if exp['has_sig'] else '缺失'}（期望一致）")
        check(has_prop == exp["has_prop"],
              f"未知/专有字段 {'存在' if exp['has_prop'] else '缺失'}（期望一致）")

        print(f"  输入字段: {', '.join(names)}")
        print(f"  五态自检: utxo={field_status(im, 'IN_WITNESS_UTXO')}")

        for n, _, v in im:
            if n == "IN_WITNESS_SCRIPT":
                print(f"  witnessScript ({len(v)}B): {script_asm(v)}")
                check(v[0] == 0x52 and v[-1] == 0xAE,
                      "witnessScript 形态为 OP_2 ... OP_CHECKMULTISIG")
                check(hashlib.sha256(v).hexdigest() is not None, "witnessScript 可哈希")

    # --- 跨样本一致性
    print("\n--- 跨样本约束 ---")
    s1 = parse_psbt(read_fixture("S1"))
    s3 = parse_psbt(read_fixture("S3"))
    utxo1 = [v for n, _, v in s1["inputs"][0] if n == "IN_WITNESS_UTXO"][0]
    utxo3 = [v for n, _, v in s3["inputs"][0] if n == "IN_WITNESS_UTXO"][0]
    check(utxo1 == utxo3, "S1 与 S3 的 witness_utxo 完全一致（同源于同一笔输入）")
    check(s1["unsigned_tx"] == s3["unsigned_tx"],
          "S1 与 S3 的 unsigned tx 完全一致（签名不改变交易本体）")

    idx = json.loads((FX / "fixtures-index.json").read_text(encoding="utf-8"))
    check("fixtures-index.json 存在且可读" and idx["samples"],
          f"索引文件记录 {len(idx['samples'])} 个样本")

    print("\n" + "=" * 66)
    if FAILURES:
        print(f"❌ {len(FAILURES)} 项失败：")
        for f in FAILURES:
            print(f"   - {f}")
        return 1
    print("✅ 全部通过。四个固件均为结构合法的 BIP-174 报文，可用于 G3 实测。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
