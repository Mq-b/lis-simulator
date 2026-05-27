//! ASTM 协议端到端集成测试
//!
//! 模拟完整的仪器→LIS 通信流程，不需要物理串口。

use lis_simulator::astm::control::*;
use lis_simulator::astm::frame::*;
use lis_simulator::astm::record::*;
use lis_simulator::state::AppState;
use lis_simulator::state::LogEntry;

/// 模拟完整的单向通信流程
///
/// 仪器发送: ENQ -> LIS回ACK -> 仪器发送数据帧 -> LIS回ACK -> 仪器发送EOT
#[test]
fn test_full_oneway_communication() {
    let mut state = AppState::new();

    // ─── 第1步: 仪器发送 ENQ ───────────────────────
    state.add_log(LogEntry::rx(&[ENQ], "ENQ"));
    assert!(is_control_message(ENQ));

    // LIS 自动回 ACK
    state.add_log(LogEntry::tx(&[ACK], "ACK"));

    // ─── 第2步: 仪器发送数据帧 ─────────────────────
    let now = "20250518120000";
    let records = vec![
        format!("H|\\^&|INST^0|PR|V1.0|{}", now),
        "P|1|660467|张三|36|M|C||李四|骨科".to_string(),
        format!("R|1|cTnI|0.03|ng/ml|0|0.15|{}|U|", now),
        format!("R|2|CK-MB|4.3|ng/ml|0|6|{}|U|", now),
        format!("R|3|NT-proBNP|125.0|pg/ml|0|100.0|{}|H|偏高", now),
        "L|1|N".to_string(),
    ];

    let record_strs: Vec<&str> = records.iter().map(|s| s.as_str()).collect();
    let frame = build_frame(&record_strs);

    state.add_log(LogEntry::rx(&frame, "DATA"));

    // LIS 解析帧
    let (parsed, _) = try_parse_frame(&frame).expect("帧解析失败");
    assert!(parsed.checksum_valid, "校验和应通过");
    assert_eq!(parsed.records.len(), 6, "应有6条记录");

    // 解析各记录
    for record_line in &parsed.records {
        let record = parse_record(record_line);
        match &record.record_type {
            RecordType::Header => {
                let header = extract_header(&record);
                state.current_message.header = Some(header);
            }
            RecordType::Patient => {
                let patient = extract_patient(&record);
                state.current_message.patient = Some(patient);
            }
            RecordType::Result => {
                let result = extract_result(&record);
                state.current_message.results.push(result);
            }
            RecordType::Terminator => {
                let term = extract_terminator(&record);
                state.current_message.terminator = Some(term);
                state.finish_message();
            }
            _ => {}
        }
    }

    // LIS 回 ACK
    state.add_log(LogEntry::tx(&[ACK], "ACK"));

    // ─── 第3步: 仪器发送 EOT ───────────────────────
    state.add_log(LogEntry::rx(&[EOT], "EOT"));

    // ─── 验证结果 ─────────────────────────────────
    assert_eq!(state.messages.len(), 1, "应有1条完整消息");

    let msg = &state.messages[0];

    // 验证 Header
    let header = msg.header.as_ref().expect("应有 Header");
    assert_eq!(header.sender, "INST^0");
    assert_eq!(header.message_type, "PR");
    assert_eq!(header.message_type_display(), "患者结果");
    assert_eq!(header.version, "V1.0");

    // 验证 Patient
    let patient = msg.patient.as_ref().expect("应有 Patient");
    assert_eq!(patient.patient_id, "660467");
    assert_eq!(patient.name, "张三");
    assert_eq!(patient.age, "36");
    assert_eq!(patient.sex, "M");
    assert_eq!(patient.sex_display(), "男");
    assert_eq!(patient.visit_type, "C");
    assert_eq!(patient.visit_type_display(), "门诊");
    assert_eq!(patient.doctor, "李四");
    assert_eq!(patient.department, "骨科");

    // 验证 Results
    assert_eq!(msg.results.len(), 3, "应有3条结果");

    assert_eq!(msg.results[0].item_code, "cTnI");
    assert_eq!(msg.results[0].value, "0.03");
    assert_eq!(msg.results[0].unit, "ng/ml");
    assert_eq!(msg.results[0].flag, "U");
    assert_eq!(msg.results[0].flag_display(), "未知");
    assert_eq!(msg.results[0].ref_range_display(), "0~0.15");

    assert_eq!(msg.results[1].item_code, "CK-MB");
    assert_eq!(msg.results[1].value, "4.3");

    assert_eq!(msg.results[2].item_code, "NT-proBNP");
    assert_eq!(msg.results[2].value, "125.0");
    assert_eq!(msg.results[2].flag, "H");
    assert_eq!(msg.results[2].flag_display(), "偏高");

    // 验证 Terminator
    let term = msg.terminator.as_ref().expect("应有 Terminator");
    assert_eq!(term.code, "N");
    assert_eq!(term.code_display(), "正常结束");

    // 验证日志
    assert!(state.log_entries.len() >= 5, "应有至少5条日志");
    assert_eq!(state.msg_count, 1);
    assert_eq!(state.result_count, 3);
}

/// 模拟连续发送多条消息
#[test]
fn test_multiple_messages() {
    let mut state = AppState::new();

    let patients = vec![
        ("20250518001", "张三", "cTnI", "0.03"),
        ("20250518002", "李四", "CK-MB", "8.5"),
        ("20250518003", "王五", "NT-proBNP", "200.0"),
    ];

    for (i, (sid, name, item, value)) in patients.iter().enumerate() {
        let now = format!("20250518120{:02}00", i);
        let records = vec![
            format!("H|\\^&|INST^0|PR|V1.0|{}", now),
            format!("P|1|{}|{}|30|M|H|BED{}|Dr.Zhao|Internal", sid, name, i),
            format!("R|1|{}|{}|ng/ml|0|10|{}|U|", item, value, now),
            "L|1|N".to_string(),
        ];

        let record_strs: Vec<&str> = records.iter().map(|s| s.as_str()).collect();
        let frame = build_frame(&record_strs);
        let (parsed, _) = try_parse_frame(&frame).unwrap_or_else(|| {
            panic!("帧解析失败, frame bytes: {:02X?}", &frame[..frame.len().min(50)])
        });
        assert!(parsed.checksum_valid);

        for record_line in &parsed.records {
            let record = parse_record(record_line);
            match &record.record_type {
                RecordType::Header => {
                    state.current_message.header = Some(extract_header(&record));
                }
                RecordType::Patient => {
                    state.current_message.patient = Some(extract_patient(&record));
                }
                RecordType::Result => {
                    state.current_message.results.push(extract_result(&record));
                }
                RecordType::Terminator => {
                    state.current_message.terminator = Some(extract_terminator(&record));
                    state.finish_message();
                }
                _ => {}
            }
        }
    }

    assert_eq!(state.messages.len(), 3, "应有3条完整消息");
    assert_eq!(state.msg_count, 3);
    assert_eq!(state.result_count, 3);

    // 验证每条消息的患者信息
    assert_eq!(state.messages[0].patient.as_ref().unwrap().name, "张三");
    assert_eq!(state.messages[1].patient.as_ref().unwrap().name, "李四");
    assert_eq!(state.messages[2].patient.as_ref().unwrap().name, "王五");
    // 验证医生和科室
    assert_eq!(state.messages[0].patient.as_ref().unwrap().doctor, "Dr.Zhao");
    assert_eq!(state.messages[0].patient.as_ref().unwrap().department, "Internal");
}

/// 测试校验和错误的帧被拒绝
#[test]
fn test_bad_checksum_rejected() {
    let records = vec!["H|\\^&|INST|PR|V1.0|20250101", "L|1|N"];
    let mut frame = build_frame(&records);

    // 篡改校验和
    let len = frame.len();
    frame[len - 4] = b'f';
    frame[len - 3] = b'f';

    let (parsed, _) = try_parse_frame(&frame).expect("帧应能解析");
    assert!(!parsed.checksum_valid, "校验和应失败");

    // 模拟器应拒绝此帧
    let state = AppState::new();
    if parsed.checksum_valid {
        // 不应进入这里
        panic!("校验和错误的帧不应通过验证");
    }
    assert_eq!(state.messages.len(), 0, "不应有解析的消息");
}

/// 测试 Q 记录解析（双向模式查询）
#[test]
fn test_query_record_parsing() {
    let record = parse_record("Q|1||20250518001||20250518000000|O");
    assert_eq!(record.record_type, RecordType::Request);
    assert_eq!(get_field(&record, 1), "1");
    assert_eq!(get_field(&record, 2), "");
    assert_eq!(get_field(&record, 3), "20250518001");
    assert_eq!(get_field(&record, 6), "O");
}

/// 测试 Comment 记录解析
#[test]
fn test_comment_record_parsing() {
    let record = parse_record("C|1|S|仪器自检通过|C");
    assert_eq!(record.record_type, RecordType::Comment);
    assert_eq!(get_field(&record, 2), "S");
    assert_eq!(get_field(&record, 3), "仪器自检通过");
    assert_eq!(get_field(&record, 4), "C");
}

/// 测试多种样本类型的 Order 记录
#[test]
fn test_order_sample_types() {
    let test_cases = vec![
        ("Serum", "血清"),
        ("Blood", "全血"),
        ("Urine", "尿液"),
        ("Other", "其他"),
    ];

    for (stype, expected) in test_cases {
        let record = parse_record(&format!("O|1|123456|test||1|20250101|{}|Q", stype));
        let order = extract_order(&record);
        assert_eq!(order.sample_type_display(), expected, "样本类型 '{}' 应显示为 '{}'", stype, expected);
    }
}

/// 测试所有结果标志
#[test]
fn test_all_result_flags() {
    let flags = vec![
        ("M", "正常"), ("H", "偏高"), ("L", "偏低"),
        ("P", "阳性"), ("W", "弱阳性"), ("N", "阴性"),
        ("E", "错误"), ("U", "未知"),
    ];

    for (flag, expected) in flags {
        let record = parse_record(&format!("R|1|test|1.0|U|||20250101|{}|", flag));
        let result = extract_result(&record);
        assert_eq!(result.flag_display(), expected);
    }
}

/// 测试 AppState 日志缓冲区限制
#[test]
fn test_log_buffer_limit() {
    let mut state = AppState::new();
    state.max_log_entries = 5;

    for i in 0..10 {
        state.add_log(LogEntry::rx(&[i as u8], "TEST"));
    }

    assert_eq!(state.log_entries.len(), 5, "日志应被限制为5条");
    // 最早的应该被淘汰
    assert_eq!(state.log_entries[0].ctrl_type, "TEST");
}

/// 测试 AppState 消息清理
#[test]
fn test_state_clear() {
    let mut state = AppState::new();

    // 添加一些数据
    state.add_log(LogEntry::rx(&[ENQ], "ENQ"));
    state.current_message.header = Some(HeaderInfo {
        sender: "TEST".to_string(),
        ..Default::default()
    });
    state.finish_message();

    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.msg_count, 1);

    // 清理
    state.clear_messages();
    assert_eq!(state.messages.len(), 0);
    assert_eq!(state.msg_count, 0);
    assert_eq!(state.result_count, 0);

    state.clear_log();
    assert_eq!(state.log_entries.len(), 0);
}
