//! ASTM E1394 记录类型解析

/// 字段分隔符
pub const FIELD_SEP: char = '|';

/// 组件分隔符
pub const COMPONENT_SEP: char = '^';

/// 重复分隔符
pub const REPEAT_SEP: char = '\\';

/// 转义字符
pub const ESCAPE_CHAR: char = '&';

/// 记录类型
#[derive(Debug, Clone, PartialEq)]
pub enum RecordType {
    /// Header Record - 消息头
    Header,
    /// Patient Record - 患者信息
    Patient,
    /// Order Record - 检验申请/测试单
    Order,
    /// Result Record - 检验结果
    Result,
    /// Comment Record - 备注
    Comment,
    /// Request Record - 查询请求（双向）
    Request,
    /// Terminator Record - 结束记录
    Terminator,
    /// 未知类型
    Unknown(String),
}

impl RecordType {
    /// 从记录行首字符识别类型
    pub fn from_line(line: &str) -> Self {
        let trimmed = line.trim_start();
        if trimmed.starts_with("H|") {
            RecordType::Header
        } else if trimmed.starts_with("P|") {
            RecordType::Patient
        } else if trimmed.starts_with("O|") {
            RecordType::Order
        } else if trimmed.starts_with("R|") {
            RecordType::Result
        } else if trimmed.starts_with("C|") {
            RecordType::Comment
        } else if trimmed.starts_with("Q|") {
            RecordType::Request
        } else if trimmed.starts_with("L|") {
            RecordType::Terminator
        } else {
            RecordType::Unknown(trimmed.to_string())
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            RecordType::Header => "H - 头信息",
            RecordType::Patient => "P - 患者信息",
            RecordType::Order => "O - 检验申请",
            RecordType::Result => "R - 检验结果",
            RecordType::Comment => "C - 备注",
            RecordType::Request => "Q - 查询请求",
            RecordType::Terminator => "L - 结束",
            RecordType::Unknown(_) => "未知",
        }
    }
}

/// 解析后的记录
#[derive(Debug, Clone)]
pub struct ParsedRecord {
    pub record_type: RecordType,
    pub fields: Vec<String>,
    pub raw_line: String,
}

/// 按字段分隔符拆分记录
pub fn parse_record(line: &str) -> ParsedRecord {
    let record_type = RecordType::from_line(line);
    let fields: Vec<String> = line
        .split(FIELD_SEP)
        .map(|s| s.to_string())
        .collect();

    ParsedRecord {
        record_type,
        fields,
        raw_line: line.to_string(),
    }
}

/// 从解析后的记录中安全获取字段值
pub fn get_field(record: &ParsedRecord, index: usize) -> &str {
    record.fields.get(index).map(|s| s.as_str()).unwrap_or("")
}

// ═══════════════════════════════════════════════════════════════
// 高级解析：结构化记录提取
// ═══════════════════════════════════════════════════════════════

/// Header 记录解析结果
#[derive(Debug, Clone, Default)]
pub struct HeaderInfo {
    pub sender: String,       // 发送方 (如 R3M^0)
    pub message_type: String, // 消息类型 (PR/QR/CR/RQ/QA/SA)
    pub version: String,      // 协议版本 (如 RL_V1.3)
    pub timestamp: String,    // 消息时间
}

impl HeaderInfo {
    pub fn message_type_display(&self) -> &str {
        match self.message_type.as_str() {
            "PR" => "患者结果",
            "QR" => "质控结果",
            "CR" => "对照结果",
            "RQ" => "请求查询",
            "QA" => "查询确认",
            "SA" => "样本申请",
            _ => &self.message_type,
        }
    }
}

/// Patient 记录解析结果
#[derive(Debug, Clone, Default)]
pub struct PatientInfo {
    pub sequence: String,     // 序号
    pub patient_id: String,   // 患者ID
    pub name: String,         // 姓名
    pub age: String,          // 年龄
    pub sex: String,          // 性别 (U/M/F)
    pub visit_type: String,   // 就诊类型 (C/E/H/P/U)
    pub bed: String,          // 病床号
    pub doctor: String,       // 医生
    pub department: String,   // 科室
}

impl PatientInfo {
    pub fn sex_display(&self) -> &str {
        match self.sex.as_str() {
            "M" => "男",
            "F" => "女",
            _ => "未知",
        }
    }

    pub fn visit_type_display(&self) -> &str {
        match self.visit_type.as_str() {
            "C" => "门诊",
            "E" => "急诊",
            "H" => "住院",
            "P" => "体检",
            _ => "未知",
        }
    }
}

/// Result 记录解析结果
#[derive(Debug, Clone, Default)]
pub struct ResultInfo {
    pub sequence: String,     // 序号
    pub item_code: String,    // 项目代码
    pub value: String,        // 结果值
    pub unit: String,         // 单位
    pub ref_low: String,      // 参考值下限
    pub ref_high: String,     // 参考值上限
    pub test_time: String,    // 检测时间
    pub flag: String,         // 结果标志 (M/H/L/P/W/N/E/U)
    pub comment: String,      // 结果提示
}

impl ResultInfo {
    pub fn flag_display(&self) -> &str {
        match self.flag.as_str() {
            "M" => "正常",
            "H" => "偏高",
            "L" => "偏低",
            "P" => "阳性",
            "W" => "弱阳性",
            "N" => "阴性",
            "E" => "错误",
            "U" => "未知",
            _ => &self.flag,
        }
    }

    pub fn ref_range_display(&self) -> String {
        if self.ref_low.is_empty() && self.ref_high.is_empty() {
            String::new()
        } else {
            format!("{}~{}", self.ref_low, self.ref_high)
        }
    }
}

/// Order 记录解析结果
#[derive(Debug, Clone, Default)]
pub struct OrderInfo {
    pub sequence: String,     // 序号
    pub sample_id: String,    // 样本ID
    pub item_code: String,    // 项目编号
    pub dilution: String,     // 稀释倍数
    pub repeat_count: String, // 重复次数
    pub request_time: String, // 申请时间
    pub sample_type: String,  // 样本类型
    pub request_type: String, // 申请类型
}

impl OrderInfo {
    pub fn sample_type_display(&self) -> &str {
        match self.sample_type.as_str() {
            "Serum" => "血清",
            "Blood" => "全血",
            "Urine" => "尿液",
            "Amniotic" => "羊水",
            "Urethral" => "尿道分泌物",
            "Saliva" => "唾液",
            "Cervical" => "宫颈分泌物",
            "Other" => "其他",
            _ => &self.sample_type,
        }
    }
}

/// Terminator 记录解析结果
#[derive(Debug, Clone, Default)]
pub struct TerminatorInfo {
    pub sequence: String,
    pub code: String,
}

impl TerminatorInfo {
    pub fn code_display(&self) -> &str {
        match self.code.as_str() {
            "N" => "正常结束",
            "I" => "无信息",
            "Q" => "错误请求",
            _ => &self.code,
        }
    }
}

/// 从 ParsedRecord 提取 HeaderInfo
pub fn extract_header(record: &ParsedRecord) -> HeaderInfo {
    HeaderInfo {
        sender: get_field(record, 2).to_string(),
        message_type: get_field(record, 3).to_string(),
        version: get_field(record, 4).to_string(),
        timestamp: get_field(record, 5).to_string(),
    }
}

/// 从 ParsedRecord 提取 PatientInfo
pub fn extract_patient(record: &ParsedRecord) -> PatientInfo {
    PatientInfo {
        sequence: get_field(record, 1).to_string(),
        patient_id: get_field(record, 2).to_string(),
        name: get_field(record, 3).to_string(),
        age: get_field(record, 4).to_string(),
        sex: get_field(record, 5).to_string(),
        visit_type: get_field(record, 6).to_string(),
        bed: get_field(record, 7).to_string(),
        doctor: get_field(record, 8).to_string(),
        department: get_field(record, 9).to_string(),
    }
}

/// 从 ParsedRecord 提取 ResultInfo
pub fn extract_result(record: &ParsedRecord) -> ResultInfo {
    ResultInfo {
        sequence: get_field(record, 1).to_string(),
        item_code: get_field(record, 2).to_string(),
        value: get_field(record, 3).to_string(),
        unit: get_field(record, 4).to_string(),
        ref_low: get_field(record, 5).to_string(),
        ref_high: get_field(record, 6).to_string(),
        test_time: get_field(record, 7).to_string(),
        flag: get_field(record, 8).to_string(),
        comment: get_field(record, 9).to_string(),
    }
}

/// 从 ParsedRecord 提取 OrderInfo
pub fn extract_order(record: &ParsedRecord) -> OrderInfo {
    OrderInfo {
        sequence: get_field(record, 1).to_string(),
        sample_id: get_field(record, 2).to_string(),
        item_code: get_field(record, 3).to_string(),
        dilution: get_field(record, 4).to_string(),
        repeat_count: get_field(record, 5).to_string(),
        request_time: get_field(record, 6).to_string(),
        sample_type: get_field(record, 7).to_string(),
        request_type: get_field(record, 8).to_string(),
    }
}

/// 从 ParsedRecord 提取 TerminatorInfo
pub fn extract_terminator(record: &ParsedRecord) -> TerminatorInfo {
    TerminatorInfo {
        sequence: get_field(record, 1).to_string(),
        code: get_field(record, 2).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试记录类型识别
    #[test]
    fn test_record_type_from_line() {
        assert_eq!(RecordType::from_line("H|\\^&|R3M^0|PR"), RecordType::Header);
        assert_eq!(RecordType::from_line("P|1|123456|张三"), RecordType::Patient);
        assert_eq!(RecordType::from_line("O|1|123456|cTnI"), RecordType::Order);
        assert_eq!(RecordType::from_line("R|1|cTnI|0.03|ng/ml"), RecordType::Result);
        assert_eq!(RecordType::from_line("C|1|S|备注|C"), RecordType::Comment);
        assert_eq!(RecordType::from_line("Q|1||123456|||O"), RecordType::Request);
        assert_eq!(RecordType::from_line("L|1|N"), RecordType::Terminator);
        assert!(matches!(RecordType::from_line("X|unknown"), RecordType::Unknown(_)));
    }

    /// 测试字段拆分
    #[test]
    fn test_parse_record_fields() {
        let record = parse_record("H|\\^&|INST^0|PR|V1.0|20250518103000");
        assert_eq!(record.record_type, RecordType::Header);
        assert_eq!(record.fields.len(), 6);
        assert_eq!(record.fields[0], "H");
        assert_eq!(record.fields[1], "\\^&");
        assert_eq!(record.fields[2], "INST^0");
        assert_eq!(record.fields[5], "20250518103000");
    }

    /// 测试 Header 信息提取
    #[test]
    fn test_extract_header() {
        let record = parse_record("H|\\^&|INST^0|PR|V1.0|20250518103000");
        let header = extract_header(&record);
        assert_eq!(header.sender, "INST^0");
        assert_eq!(header.message_type, "PR");
        assert_eq!(header.message_type_display(), "患者结果");
        assert_eq!(header.version, "V1.0");
        assert_eq!(header.timestamp, "20250518103000");
    }

    /// 测试 Patient 信息提取
    #[test]
    fn test_extract_patient() {
        let record = parse_record("P|1|660467|张三|36|M|C||李四|骨科");
        let patient = extract_patient(&record);
        assert_eq!(patient.patient_id, "660467");
        assert_eq!(patient.name, "张三");
        assert_eq!(patient.age, "36");
        assert_eq!(patient.sex, "M");
        assert_eq!(patient.sex_display(), "男");
        assert_eq!(patient.visit_type, "C");
        assert_eq!(patient.visit_type_display(), "门诊");
        assert_eq!(patient.bed, "");
        assert_eq!(patient.doctor, "李四");
        assert_eq!(patient.department, "骨科");
    }

    /// 测试 Result 信息提取
    #[test]
    fn test_extract_result() {
        let record = parse_record("R|1|cTnI|0.03|ng/ml|0|0.15|20250518103000|U|");
        let result = extract_result(&record);
        assert_eq!(result.item_code, "cTnI");
        assert_eq!(result.value, "0.03");
        assert_eq!(result.unit, "ng/ml");
        assert_eq!(result.ref_low, "0");
        assert_eq!(result.ref_high, "0.15");
        assert_eq!(result.ref_range_display(), "0~0.15");
        assert_eq!(result.flag, "U");
        assert_eq!(result.flag_display(), "未知");
    }

    /// 测试 Result 标志显示
    #[test]
    fn test_result_flag_display() {
        let test_cases = vec![
            ("M", "正常"),
            ("H", "偏高"),
            ("L", "偏低"),
            ("P", "阳性"),
            ("W", "弱阳性"),
            ("N", "阴性"),
            ("E", "错误"),
            ("U", "未知"),
            ("X", "X"), // 未知标志原样返回
        ];
        for (flag, expected) in test_cases {
            let record = parse_record(&format!("R|1|test|1.0|U|||20250101|{}|", flag));
            let result = extract_result(&record);
            assert_eq!(result.flag_display(), expected, "标志 '{}' 应显示为 '{}'", flag, expected);
        }
    }

    /// 测试 Terminator 信息提取
    #[test]
    fn test_extract_terminator() {
        let record = parse_record("L|1|N");
        let term = extract_terminator(&record);
        assert_eq!(term.code, "N");
        assert_eq!(term.code_display(), "正常结束");

        let record2 = parse_record("L|1|I");
        let term2 = extract_terminator(&record2);
        assert_eq!(term2.code_display(), "无信息");
    }

    /// 测试 Order 信息提取
    #[test]
    fn test_extract_order() {
        let record = parse_record("O|1|5868739000|NT-proBNP||1|20250418185452|Blood|U");
        let order = extract_order(&record);
        assert_eq!(order.sample_id, "5868739000");
        assert_eq!(order.item_code, "NT-proBNP");
        assert_eq!(order.sample_type, "Blood");
        assert_eq!(order.sample_type_display(), "全血");
    }

    /// 测试空字段处理
    #[test]
    fn test_empty_field_handling() {
        let record = parse_record("P|1||||||||");
        let patient = extract_patient(&record);
        assert_eq!(patient.patient_id, "");
        assert_eq!(patient.name, "");
        assert_eq!(patient.sex_display(), "未知");
    }

    /// 测试 get_field 越界
    #[test]
    fn test_get_field_out_of_bounds() {
        let record = parse_record("H|test");
        assert_eq!(get_field(&record, 0), "H");
        assert_eq!(get_field(&record, 1), "test");
        assert_eq!(get_field(&record, 2), ""); // 越界返回空
        assert_eq!(get_field(&record, 100), "");
    }
}
