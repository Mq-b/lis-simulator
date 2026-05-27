/// ASTM E1381 控制字符定义

/// 请求通信 (0x05)
pub const ENQ: u8 = 0x05;

/// 确认/准备好 (0x06)
pub const ACK: u8 = 0x06;

/// 否定确认/未准备好 (0x15)
pub const NAK: u8 = 0x15;

/// 传输结束 (0x04)
pub const EOT: u8 = 0x04;

/// 帧数据开始 (0x02)
pub const STX: u8 = 0x02;

/// 帧数据结束 (0x03)
pub const ETX: u8 = 0x03;

/// 分帧结束符 (0x17)
pub const ETB: u8 = 0x17;

/// 回车 (0x0D)
pub const CR: u8 = 0x0D;

/// 换行 (0x0A)
pub const LF: u8 = 0x0A;

/// 控制字符类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ControlChar {
    Enq,
    Ack,
    Nak,
    Eot,
    Unknown(u8),
}

/// 识别单个字节是否为控制字符
pub fn identify_control(byte: u8) -> ControlChar {
    match byte {
        ENQ => ControlChar::Enq,
        ACK => ControlChar::Ack,
        NAK => ControlChar::Nak,
        EOT => ControlChar::Eot,
        other => ControlChar::Unknown(other),
    }
}

/// 控制字符转显示名称
pub fn control_name(byte: u8) -> &'static str {
    match byte {
        ENQ => "ENQ",
        ACK => "ACK",
        NAK => "NAK",
        EOT => "EOT",
        STX => "STX",
        ETX => "ETX",
        ETB => "ETB",
        CR => "CR",
        LF => "LF",
        _ => "",
    }
}

/// 检查是否为单字节控制消息（ENQ/ACK/NAK/EOT）
pub fn is_control_message(byte: u8) -> bool {
    matches!(byte, ENQ | ACK | NAK | EOT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identify_control() {
        assert_eq!(identify_control(ENQ), ControlChar::Enq);
        assert_eq!(identify_control(ACK), ControlChar::Ack);
        assert_eq!(identify_control(NAK), ControlChar::Nak);
        assert_eq!(identify_control(EOT), ControlChar::Eot);
        assert_eq!(identify_control(0x00), ControlChar::Unknown(0x00));
        assert_eq!(identify_control(0xFF), ControlChar::Unknown(0xFF));
    }

    #[test]
    fn test_control_name() {
        assert_eq!(control_name(ENQ), "ENQ");
        assert_eq!(control_name(ACK), "ACK");
        assert_eq!(control_name(NAK), "NAK");
        assert_eq!(control_name(EOT), "EOT");
        assert_eq!(control_name(STX), "STX");
        assert_eq!(control_name(ETX), "ETX");
        assert_eq!(control_name(CR), "CR");
        assert_eq!(control_name(LF), "LF");
        assert_eq!(control_name(0x00), "");
    }

    #[test]
    fn test_is_control_message() {
        assert!(is_control_message(ENQ));
        assert!(is_control_message(ACK));
        assert!(is_control_message(NAK));
        assert!(is_control_message(EOT));
        assert!(!is_control_message(STX));
        assert!(!is_control_message(ETX));
        assert!(!is_control_message(0x00));
        assert!(!is_control_message(0xFF));
    }

    #[test]
    fn test_control_char_constants() {
        assert_eq!(ENQ, 0x05);
        assert_eq!(ACK, 0x06);
        assert_eq!(NAK, 0x15);
        assert_eq!(EOT, 0x04);
        assert_eq!(STX, 0x02);
        assert_eq!(ETX, 0x03);
        assert_eq!(ETB, 0x17);
        assert_eq!(CR, 0x0D);
        assert_eq!(LF, 0x0A);
    }
}
