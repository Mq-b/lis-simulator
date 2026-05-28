//! ASTM E1381 帧解析
//!
//! 处理数据帧的提取、校验和计算与验证。

use super::control::{STX, ETX, ETB, CR, LF};
use crate::config::get_config;

/// ASTM 帧解析结果
#[derive(Debug, Clone)]
pub struct AstmFrame {
    /// 帧序号 (0-9)
    pub frame_number: u8,
    /// 帧内数据（多条记录，以 CR 分隔）
    pub records: Vec<String>,
    /// 校验和（原始字节值）
    pub checksum: u8,
    /// 校验是否通过
    pub checksum_valid: bool,
    /// 原始字节（用于 HEX 显示）
    pub raw_bytes: Vec<u8>,
}

/// 计算校验和
///
/// 范围：从帧号到 ETX/ETB（含）所有字节之和，模 256
pub fn calc_checksum(data: &[u8]) -> u8 {
    data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b))
}

/// 校验和转 hex 字符串
///
/// 根据配置决定是否补零
pub fn checksum_to_hex(checksum: u8) -> String {
    if get_config().astm.checksum_zero_padded {
        format!("{:02x}", checksum)
    } else {
        format!("{:x}", checksum)
    }
}

/// 从字节流中尝试提取一个完整的 ASTM 帧
///
/// 帧格式根据配置决定：
/// - 标准: `STX` + 帧号 + 数据 + `ETX` + 校验和(2字节hex) + `CR` + `LF`
/// - 非标: `STX` + 数据 + `ETX` + 校验和 + `CR` + `LF`
///
/// # 返回
/// - `Some((帧, 消耗字节数))` - 成功解析
/// - `None` - 数据不完整或格式错误
pub fn try_parse_frame(buf: &[u8]) -> Option<(AstmFrame, usize)> {
    let cfg = &get_config().astm;

    // 查找 STX
    let stx_pos = buf.iter().position(|&b| b == STX)?;
    let buf = &buf[stx_pos..];

    // 最小长度：STX + ETX + checksum(至少1字节) + CR + LF = 5 字节
    // 有帧号时额外 +1 字节
    let min_len = if cfg.has_frame_number { 7 } else { 6 };
    if buf.len() < min_len {
        return None;
    }

    // 查找 ETX 或 ETB（结束标记）
    let end_pos = buf.iter().position(|&b| b == ETX || b == ETB)?;

    // 查找 CR LF（校验和后面）
    // 校验和可能是 1 或 2 字节，所以从 ETX 后面开始找 CR
    let after_etx = &buf[end_pos + 1..];
    let cr_offset = after_etx.iter().position(|&b| b == CR)?;

    // CR 后面必须是 LF
    if cr_offset + 1 >= after_etx.len() || after_etx[cr_offset + 1] != LF {
        return None;
    }

    // 提取校验和 hex 字符串（ETX 后到 CR 前）
    let cs_bytes = &after_etx[..cr_offset];
    let cs_str = String::from_utf8_lossy(cs_bytes).to_string();
    let expected_checksum = u8::from_str_radix(&cs_str, 16).ok();

    // 计算实际校验和
    let actual_checksum = if cfg.checksum_includes_stx {
        // 非标模式：从 STX 到 ETX/ETB（含）
        calc_checksum(&buf[..=end_pos])
    } else if cfg.has_frame_number {
        // 标准模式：从帧号到 ETX/ETB（含）
        calc_checksum(&buf[1..=end_pos])
    } else {
        // 无帧号但不含 STX：从数据开始到 ETX/ETB（含）
        calc_checksum(&buf[1..=end_pos])
    };

    let checksum_valid = expected_checksum
        .map(|expected| expected == actual_checksum)
        .unwrap_or(false);

    // 提取帧号和数据
    let (frame_number, data_start) = if cfg.has_frame_number {
        (buf[1], 2) // 帧号在 buf[1]，数据从 buf[2] 开始
    } else {
        (b'0', 1) // 无帧号，数据从 buf[1] 开始，默认帧号为 '0'
    };

    // 提取数据部分（到 ETX/ETB 之前）
    let data_bytes = &buf[data_start..end_pos];

    // 按 CR 分割为多条记录
    let records: Vec<String> = data_bytes
        .split(|&b| b == CR)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).to_string())
        .collect();

    let total_len = end_pos + 1 + cr_offset + 2; // ETX + checksum + CR + LF
    let consumed = stx_pos + total_len;

    Some((
        AstmFrame {
            frame_number,
            records,
            checksum: actual_checksum,
            checksum_valid,
            raw_bytes: buf[..total_len].to_vec(),
        },
        consumed,
    ))
}

/// 构建一个 ASTM 数据帧
///
/// 帧格式根据配置决定：
/// - 标准: `STX` + 帧号 + 记录(`CR`分隔) + `ETX` + 校验和 + `CR` + `LF`
/// - 非标: `STX` + 记录(`CR`分隔) + `ETX` + 校验和 + `CR` + `LF`
pub fn build_frame(records: &[&str]) -> Vec<u8> {
    let cfg = &get_config().astm;

    let mut frame_data = Vec::new();

    // 添加帧号（如果配置要求）
    if cfg.has_frame_number {
        frame_data.push(b'1');
    }

    // 添加记录
    for (i, record) in records.iter().enumerate() {
        frame_data.extend_from_slice(record.as_bytes());
        if i < records.len() - 1 {
            frame_data.push(CR);
        }
    }

    // 计算校验和
    let cs = if cfg.checksum_includes_stx {
        // 非标模式：包含 STX
        let mut checksum_payload = vec![STX];
        checksum_payload.extend_from_slice(&frame_data);
        checksum_payload.push(ETX);
        calc_checksum(&checksum_payload)
    } else {
        // 标准模式：帧号到 ETX
        let mut checksum_payload = frame_data.clone();
        checksum_payload.push(ETX);
        calc_checksum(&checksum_payload)
    };

    // 构建完整帧
    let mut full_frame = vec![STX];
    full_frame.extend_from_slice(&frame_data);
    full_frame.push(ETX);
    full_frame.extend_from_slice(checksum_to_hex(cs).as_bytes());
    full_frame.push(CR);
    full_frame.push(LF);
    full_frame
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试校验和计算
    #[test]
    fn test_calc_checksum_basic() {
        // 简单数据: "1L|1|N" + ETX
        // 0x31+0x4C+0x7C+0x31+0x7C+0x4E+0x03 = 0x1F7, mod 256 = 0xF7
        let data = vec![b'1', b'L', b'|', b'1', b'|', b'N', ETX];
        let cs = calc_checksum(&data);
        assert_eq!(cs, 0xF7);
    }

    /// 测试校验和 hex 转换
    #[test]
    fn test_checksum_to_hex() {
        let cfg = &get_config().astm;
        if cfg.checksum_zero_padded {
            assert_eq!(checksum_to_hex(0x00), "00");
            assert_eq!(checksum_to_hex(0x0a), "0a");
            assert_eq!(checksum_to_hex(0xff), "ff");
            assert_eq!(checksum_to_hex(0xe5), "e5");
        } else {
            assert_eq!(checksum_to_hex(0x00), "0");
            assert_eq!(checksum_to_hex(0x0a), "a");
            assert_eq!(checksum_to_hex(0xff), "ff");
            assert_eq!(checksum_to_hex(0xe5), "e5");
        }
    }

    /// 测试构建和解析帧的往返一致性
    #[test]
    fn test_build_and_parse_roundtrip() {
        let cfg = &get_config().astm;
        let records = vec!["H|\\^&|LIS_SIM|PR|V1.0|20250101120000", "L|1|N"];
        let frame_bytes = build_frame(&records);

        // 验证帧结构
        assert_eq!(frame_bytes[0], STX);
        assert_eq!(frame_bytes[frame_bytes.len() - 2], CR);
        assert_eq!(frame_bytes[frame_bytes.len() - 1], LF);

        // 解析
        let (parsed, consumed) = try_parse_frame(&frame_bytes).expect("应该能解析帧");
        assert_eq!(consumed, frame_bytes.len());

        // 根据配置验证帧号
        if cfg.has_frame_number {
            assert_eq!(parsed.frame_number, b'1');
        } else {
            assert_eq!(parsed.frame_number, b'0'); // 默认帧号
        }

        assert_eq!(parsed.records.len(), 2);
        assert_eq!(parsed.records[0], "H|\\^&|LIS_SIM|PR|V1.0|20250101120000");
        assert_eq!(parsed.records[1], "L|1|N");
        assert!(parsed.checksum_valid);
    }

    /// 测试解析不完整的数据（数据不足）
    #[test]
    fn test_parse_incomplete_data() {
        let cfg = &get_config().astm;
        let incomplete = if cfg.has_frame_number {
            vec![STX, b'1', b'H', b'|']
        } else {
            vec![STX, b'H', b'|']
        };
        assert!(try_parse_frame(&incomplete).is_none());
    }

    /// 测试解析校验和错误的帧
    #[test]
    fn test_parse_bad_checksum() {
        let mut frame = build_frame(&["L|1|N"]);
        // 篡改校验和
        let len = frame.len();
        frame[len - 4] = b'f';
        frame[len - 3] = b'f';

        let (parsed, _) = try_parse_frame(&frame).expect("应该能解析帧");
        assert!(!parsed.checksum_valid);
    }

    /// 测试从噪声数据中提取帧
    #[test]
    fn test_parse_frame_with_prefix_noise() {
        let mut noise = vec![0x00, 0xFF, 0x12, 0x34];
        let frame = build_frame(&["R|1|cTnI|0.03|ng/ml"]);
        noise.extend_from_slice(&frame);

        let (parsed, consumed) = try_parse_frame(&noise).expect("应该能从噪声中提取帧");
        assert_eq!(consumed, noise.len());
        assert!(parsed.checksum_valid);
        assert_eq!(parsed.records[0], "R|1|cTnI|0.03|ng/ml");
    }

    /// 测试多条记录的帧
    #[test]
    fn test_parse_multi_record_frame() {
        let records = vec![
            "H|\\^&|INST^0|PR|V1.0|20250518103000",
            "P|1|123456|张三|36|M|C||李四|骨科",
            "R|1|cTnI|0.03|ng/ml|0|0.15|20250518103000|U|",
            "R|2|CK-MB|4.3|ng/ml|0|6|20250518103000|U|",
            "L|1|N",
        ];
        let frame = build_frame(&records);
        let (parsed, _) = try_parse_frame(&frame).expect("应该能解析多记录帧");

        assert_eq!(parsed.records.len(), 5);
        assert!(parsed.checksum_valid);
        assert_eq!(parsed.records[0], records[0]);
        assert_eq!(parsed.records[4], records[4]);
    }

    /// 测试空记录列表
    #[test]
    fn test_build_empty_frame() {
        let frame = build_frame(&[]);

        // 空帧可能无法解析（需要至少 STX + ETX + checksum + CR + LF）
        if let Some((parsed, _)) = try_parse_frame(&frame) {
            assert_eq!(parsed.records.len(), 0);
            assert!(parsed.checksum_valid);
        } else {
            // 如果无法解析，验证帧结构正确
            assert!(frame.len() >= 5); // STX + ETX + checksum(至少1) + CR + LF
        }
    }
}
