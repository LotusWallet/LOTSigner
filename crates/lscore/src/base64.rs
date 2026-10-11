//! Base64 解码（标准字母表 + 填充）
//!
//! 为何自带：保持内核零第三方依赖（见 workspace Cargo.toml 的依赖策略说明）。

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum B64Error {
    InvalidChar(u8),
    InvalidLength,
    InvalidPadding,
    TrailingBits,
}

const ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn val(c: u8) -> Result<u8, B64Error> {
    if let Some(i) = ALPHABET.iter().position(|&a| a == c) {
        Ok(i as u8)
    } else {
        Err(B64Error::InvalidChar(c))
    }
}

/// 解码标准 base64。允许输入含空白字符（CR/LF/空格），会在解码前剔除——
/// 这一点是 Caravan 实测的教训：其 `readAsText` 不做 trim，固件末尾一个换行
/// 就会让整个导入失败（“Input cannot be bufferized.”）。
pub fn decode(input: &str) -> Result<Vec<u8>, B64Error> {
    let cleaned: Vec<u8> = input
        .bytes()
        .filter(|b| !matches!(b, b'\r' | b'\n' | b' ' | b'\t'))
        .collect();

    if cleaned.len() % 4 != 0 {
        return Err(B64Error::InvalidLength);
    }

    let mut out: Vec<u8> = Vec::with_capacity(cleaned.len() / 4 * 3);
    for chunk in cleaned.chunks(4) {
        if chunk.len() < 4 {
            return Err(B64Error::InvalidLength);
        }
        // 计算本组的有效字符数（去掉尾部填充）
        let mut n = 4usize;
        if chunk[3] == b'=' {
            n = 3;
            if chunk[2] == b'=' {
                n = 2;
            }
        }
        if n < 2 {
            return Err(B64Error::InvalidPadding);
        }
        // 填充之后不得再有非 '=' 字符
        for &c in &chunk[n..] {
            if c != b'=' {
                return Err(B64Error::InvalidPadding);
            }
        }

        let mut bits: u32 = 0;
        for (i, &c) in chunk.iter().enumerate().take(n) {
            bits |= (val(c)? as u32) << (18 - 6 * i);
        }
        // n 个字符 → n*6 位 → 输出 n-1 字节，剩余 6-(2*(n-1) mod ... ) 位须为 0
        let byte_count = n - 1;
        for i in 0..byte_count {
            out.push((bits >> (16 - 8 * i)) as u8);
        }
        // 校验被丢弃的尾比特必须为 0（严格模式：不做推测性填补）
        let leftover_bits = 6 * n - 8 * byte_count;
        if leftover_bits > 0 {
            let mask = (1u32 << leftover_bits) - 1;
            if bits & mask != 0 {
                return Err(B64Error::TrailingBits);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_known_vector() {
        assert_eq!(decode("").unwrap(), Vec::<u8>::new());
        assert_eq!(decode("QQ==").unwrap(), vec![0x41]);
        assert_eq!(decode("QUJD").unwrap(), vec![0x41, 0x42, 0x43]);
        assert_eq!(decode("QUJDRFG=").unwrap().len(), 5);
    }

    #[test]
    fn tolerates_whitespace() {
        // CR/LF 必须被容忍 —— 否则会重蹈 Caravan readAsText 不 trim 的覆辙
        assert_eq!(decode("QUJD\r\n").unwrap(), vec![0x41, 0x42, 0x43]);
        assert_eq!(decode("QUJ\nD").unwrap(), vec![0x41, 0x42, 0x43]);
    }

    #[test]
    fn rejects_bad_input() {
        assert!(decode("QUJ").is_err()); // 长度非 4 的倍数
        assert!(decode("QU!D").is_err()); // 非法字符
        assert!(decode("QQ=A").is_err()); // 填充位置非法
    }
}
