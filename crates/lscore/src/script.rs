//! 脚本分类与地址呈现的最小实现
//!
//! 说明：仅为「把一堆字节变成人能对照的形态」所需的最低限度的编解码，
//! 不含任何策略判断。它不知道什么是"安全"，只回答"这是什么类型、对应哪个地址串"。

use crate::tx::{hex, sha256d};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptType {
    P2Pkh,
    P2Sh,
    P2Wpkh,
    P2Wsh,
    P2Tr,
    OpReturn,
    Unknown,
}

impl ScriptType {
    pub fn name(self) -> &'static str {
        match self {
            ScriptType::P2Pkh => "P2PKH",
            ScriptType::P2Sh => "P2SH",
            ScriptType::P2Wpkh => "P2WPKH",
            ScriptType::P2Wsh => "P2WSH",
            ScriptType::P2Tr => "P2TR",
            ScriptType::OpReturn => "OP_RETURN",
            ScriptType::Unknown => "未知脚本形态",
        }
    }
}

pub fn classify(spk: &[u8]) -> ScriptType {
    if spk.len() == 25 && spk[0] == 0x76 && spk[1] == 0xa9 && spk[2] == 0x14 && spk[23] == 0x88 && spk[24] == 0xac
    {
        return ScriptType::P2Pkh;
    }
    if spk.len() == 23 && spk[0] == 0xa9 && spk[1] == 0x14 && spk[22] == 0x87 {
        return ScriptType::P2Sh;
    }
    if spk.len() == 22 && spk[0] == 0x00 && spk[1] == 0x14 {
        return ScriptType::P2Wpkh;
    }
    if spk.len() == 34 && spk[0] == 0x00 && spk[1] == 0x20 {
        return ScriptType::P2Wsh;
    }
    if spk.len() == 34 && spk[0] == 0x51 && spk[1] == 0x20 {
        return ScriptType::P2Tr;
    }
    if !spk.is_empty() && spk[0] == 0x6a {
        return ScriptType::OpReturn;
    }
    ScriptType::Unknown
}

// ---------------- base58check ----------------

const B58_ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

fn base58check(payload: &[u8]) -> String {
    let mut data = payload.to_vec();
    let checksum = sha256d(payload);
    data.extend_from_slice(&checksum[..4]);

    // 前导零计数
    let zeros = data.iter().take_while(|&&b| b == 0).count();
    let mut num = data.clone();
    let mut out = String::new();
    for _ in 0..zeros {
        out.push('1');
    }

    while !num.is_empty() && num[0] == 0 {
        num.remove(0);
    }

    let mut digits: Vec<u8> = Vec::new();
    while !num.is_empty() {
        let mut carry = 0u32;
        let mut new_num = Vec::with_capacity(num.len());
        for &byte in num.iter() {
            let cur = (byte as u32) + carry * 256;
            let q = cur / 58;
            carry = cur % 58;
            if !new_num.is_empty() || q > 0 {
                new_num.push(q as u8);
            }
        }
        digits.push(carry as u8);
        num = new_num;
    }
    for d in digits.iter().rev() {
        out.push(B58_ALPHABET[*d as usize] as char);
    }
    out
}

// ---------------- bech32 / bech32m ----------------

const BECH32: &str = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";
const BECH32M_CONST: u32 = 0x2bc830a3;
const BECH32_CONST: u32 = 1;

fn polymod(values: &[u8]) -> u32 {
    const GEN: [u32; 5] = [0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3];
    let mut chk: u32 = 1;
    for &v in values {
        let b = chk >> 25;
        chk = (chk & 0x1ffffff) << 5 ^ (v as u32);
        for i in 0..5 {
            if (b >> i) & 1 == 1 {
                chk ^= GEN[i];
            }
        }
    }
    chk
}

fn hrp_expand(hrp: &str) -> Vec<u8> {
    let mut v: Vec<u8> = hrp.bytes().map(|b| b >> 5).collect();
    v.push(0);
    v.extend(hrp.bytes().map(|b| b & 31));
    v
}

fn convert_bits(data: &[u8], from: u32, to: u32, pad: bool) -> Option<Vec<u8>> {
    let mut acc = 0u32;
    let mut bits = 0u32;
    let mut ret: Vec<u8> = Vec::new();
    let maxv = (1u32 << to) - 1;
    for &value in data {
        acc = (acc << from) | (value as u32);
        bits += from;
        while bits >= to {
            bits -= to;
            ret.push(((acc >> bits) & maxv) as u8);
        }
    }
    if pad {
        if bits > 0 {
            ret.push(((acc << (to - bits)) & maxv) as u8);
        }
    } else if bits >= from || ((acc << (to - bits)) & maxv) != 0 {
        return None;
    }
    Some(ret)
}

fn bech32_encode(hrp: &str, data: &[u8], const_val: u32) -> String {
    let mut values = hrp_expand(hrp);
    values.extend(data);
    let pm = polymod(&values) ^ const_val;
    let mut checksum = Vec::new();
    for i in 0..6 {
        checksum.push(((pm >> (5 * (5 - i))) & 31) as u8);
    }
    values.extend(checksum);
    let mut out = String::from(hrp);
    out.push('1');
    for &v in &values[hrp.len() * 2 + 1..] {
        out.push(BECH32.as_bytes()[v as usize] as char);
    }
    out
}

/// 把 scriptPubKey 呈现为地址字符串。无法安全呈现时返回 None（绝不推测）。
pub fn script_to_address(spk: &[u8], mainnet: bool) -> Option<String> {
    match classify(spk) {
        ScriptType::P2Pkh => {
            let prefix = if mainnet { 0x00 } else { 0x6f };
            let mut payload = vec![prefix];
            payload.extend_from_slice(&spk[3..23]);
            Some(base58check(&payload))
        }
        ScriptType::P2Sh => {
            let prefix = if mainnet { 0x05 } else { 0xc4 };
            let mut payload = vec![prefix];
            payload.extend_from_slice(&spk[2..22]);
            Some(base58check(&payload))
        }
        ScriptType::P2Wpkh | ScriptType::P2Wsh => {
            // witness v0，用 bech32
            let hrp = if mainnet { "bc" } else { "tb" };
            let conv = convert_bits(&spk[2..], 8, 5, true)?;
            let mut data = vec![0u8]; // witness version 0
            data.extend(conv);
            Some(bech32_encode(hrp, &data, BECH32_CONST))
        }
        ScriptType::P2Tr => {
            let hrp = if mainnet { "bc" } else { "tb" };
            let conv = convert_bits(&spk[2..], 8, 5, true)?;
            let mut data = vec![1u8]; // witness version 1 → bech32m
            data.extend(conv);
            Some(bech32_encode(hrp, &data, BECH32M_CONST))
        }
        ScriptType::OpReturn | ScriptType::Unknown => None,
    }
}

pub fn script_summary(spk: &[u8]) -> String {
    if spk.len() <= 42 {
        format!("{} ({})", classify(spk).name(), hex(spk))
    } else {
        format!("{} ({}…)", classify(spk).name(), hex(&spk[..8]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bech32_p2wpkh_vector() {
        // BIP-173 官方测试向量之一（全部小写）
        let spk: Vec<u8> = hex_to_bytes("0014751e76e8199196d454941c45d1b3a323f1433bd6");
        let addr = script_to_address(&spk, true).unwrap();
        assert_eq!(addr, "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4");
    }

    #[test]
    fn base58_p2sh_vector() {
        // Caravan 实测中出现的脚本：OP_HASH160 2475ffa90c... OP_EQUAL → 351oc99Eva2AoWUfMyBgJjRS2DPcVfA1Pg
        let spk: Vec<u8> = hex_to_bytes("a9142475ffa90c61bdedab5f6b1079fe57a2fd2e79e987");
        let addr = script_to_address(&spk, true).unwrap();
        assert_eq!(addr, "351oc99Eva2AoWUfMyBgJjRS2DPcVfA1Pg");
    }

    #[test]
    fn p2wsh_address_is_stable() {
        let spk: Vec<u8> = hex_to_bytes("0020bd3d29be575e4966b6bddd59eab54593cb9cfad4");
        let a1 = script_to_address(&spk, true).unwrap();
        let a2 = script_to_address(&spk, true).unwrap();
        assert_eq!(a1, a2);
        assert!(a1.starts_with("bc1q"));
    }

    fn hex_to_bytes(s: &str) -> Vec<u8> {
        (0..s.len() / 2)
            .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap())
            .collect()
    }
}
