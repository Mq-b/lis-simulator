//! ASTM E1381 帧解析
//!
//! 处理数据帧的提取、校验和计算与验证。

use super::control::{STX, ETX, ETB, CR, LF};

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

/// 校验和转 hex 字符串（不补零）
pub fn checksum_to_hex(checksum: u8) -> String {
    format!("{:x}", checksum)
}

/// 从字节流中尝试提取一个完整的 ASTM 帧
///
/// 帧格式: `STX` + 帧号 + 数据 + `ETX` + 校验和(2字节hex) + `CR` + `LF`
///
/// # 返回
/// - `Some((帧, 消耗字节数))` - 成功解析
/// - `None` - 数据不完整或格式错误
pub fn try_parse_frame(buf: &[u8]) -> Option<(AstmFrame, usize)> {
    // 查找 STX
    let stx_pos = buf.iter().position(|&b| b == STX)?;
    let buf = &buf[stx_pos..];

    // 至少需要: STX + 帧号 + ETX + checksum(2) + CR + LF = 7 字节
    if buf.len() < 7 {
        return None;
    }

    // 查找 ETX 或 ETB（结束标记）
    let end_pos = buf.iter().position(|&b| b == ETX || b == ETB)?;

    // 需要 ETX 后面至少有 checksum(2) + CR + LF = 4 字节
    if buf.len() < end_pos + 5 {
        return None;
    }

    let cs_high = buf[end_pos + 1];
    let cs_low = buf[end_pos + 2];
    let cr_byte = buf[end_pos + 3];
    let lf_byte = buf[end_pos + 4];

    // 验证 CR LF
    if cr_byte != CR || lf_byte != LF {
        return None;
    }

    // 解析校验和（2个 ASCII hex 字符）
    let cs_str = String::from_utf8_lossy(&[cs_high, cs_low]).to_string();
    let expected_checksum = u8::from_str_radix(&cs_str, 16).ok();

    // 计算实际校验和 (帧号+数据+ETX/ETB)
    let checksum_data = &buf[1..=end_pos];
    let actual_checksum = calc_checksum(checksum_data);

    let checksum_valid = expected_checksum
        .map(|expected| expected == actual_checksum)
        .unwrap_or(false);

    // 提取帧号
    let frame_number = buf[1];

    // 提取数据部分（帧号之后到 ETX/ETB 之前）
    let data_bytes = &buf[2..end_pos];

    // 按 CR 分割为多条记录
    let records: Vec<String> = data_bytes
        .split(|&b| b == CR)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).to_string())
        .collect();

    let consumed = stx_pos + end_pos + 5;

    Some((
        AstmFrame {
            frame_number,
            records,
            checksum: actual_checksum,
            checksum_valid,
            raw_bytes: buf[..=end_pos + 4].to_vec(),
        },
        consumed,
    ))
}

/// 构建一个 ASTM 数据帧
///
/// 帧格式: `STX` + 帧号 + 记录(`CR`分隔) + `ETX` + 校验和 + `CR` + `LF`
pub fn build_frame(records: &[&str]) -> Vec<u8> {
    let mut frame_data = Vec::new();
    frame_data.push(b'1'); // 帧号
    for (i, record) in records.iter().enumerate() {
        frame_data.extend_from_slice(record.as_bytes());
        if i < records.len() - 1 {
            frame_data.push(CR);
        }
    }

    let mut checksum_payload = frame_data.clone();
    checksum_payload.push(ETX);
    let cs = calc_checksum(&checksum_payload);

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
        assert_eq!(checksum_to_hex(0x00), "0");
        assert_eq!(checksum_to_hex(0x0a), "a");
        assert_eq!(checksum_to_hex(0xff), "ff");
        assert_eq!(checksum_to_hex(0xe5), "e5");
    }

    /// 测试构建和解析帧的往返一致性
    #[test]
    fn test_build_and_parse_roundtrip() {
        let records = vec!["H|\\^&|LIS_SIM|PR|V1.0|20250101120000", "L|1|N"];
        let frame_bytes = build_frame(&records);

        // 验证帧结构
        assert_eq!(frame_bytes[0], STX);
        assert_eq!(frame_bytes[frame_bytes.len() - 2], CR);
        assert_eq!(frame_bytes[frame_bytes.len() - 1], LF);

        // 解析
        let (parsed, consumed) = try_parse_frame(&frame_bytes).expect("应该能解析帧");
        assert_eq!(consumed, frame_bytes.len());
        assert_eq!(parsed.frame_number, b'1');
        assert_eq!(parsed.records.len(), 2);
        assert_eq!(parsed.records[0], "H|\\^&|LIS_SIM|PR|V1.0|20250101120000");
        assert_eq!(parsed.records[1], "L|1|N");
        assert!(parsed.checksum_valid);
    }

    /// 测试解析不完整的数据（数据不足）
    #[test]
    fn test_parse_incomplete_data() {
        let incomplete = vec![STX, b'1', b'H', b'|'];
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
        let (parsed, _) = try_parse_frame(&frame).expect("应该能解析空数据帧");
        assert_eq!(parsed.records.len(), 0);
        assert!(parsed.checksum_valid);
    }
}
