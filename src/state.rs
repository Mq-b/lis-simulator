use std::collections::VecDeque;
use chrono::Local;

use crate::astm::record::{
    HeaderInfo, PatientInfo, ResultInfo, TerminatorInfo,
};

/// 原始日志条目
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub direction: String,  // "RX" / "TX"
    pub ctrl_type: String,  // "ENQ"/"ACK"/"NAK"/"EOT"/"DATA"
    pub raw_data: String,
    pub hex_data: String,
}

impl LogEntry {
    pub fn rx(data: &[u8], ctrl_type: &str) -> Self {
        Self {
            timestamp: Local::now().format("%H:%M:%S%.3f").to_string(),
            direction: "RX".to_string(),
            ctrl_type: ctrl_type.to_string(),
            raw_data: format_control_readable(data),
            hex_data: hex_string(data),
        }
    }

    pub fn tx(data: &[u8], ctrl_type: &str) -> Self {
        Self {
            timestamp: Local::now().format("%H:%M:%S%.3f").to_string(),
            direction: "TX".to_string(),
            ctrl_type: ctrl_type.to_string(),
            raw_data: format_control_readable(data),
            hex_data: hex_string(data),
        }
    }
}

/// 一条完整消息的解析结果
#[derive(Debug, Clone, Default)]
pub struct MessageData {
    pub header: Option<HeaderInfo>,
    pub patient: Option<PatientInfo>,
    pub results: Vec<ResultInfo>,
    pub terminator: Option<TerminatorInfo>,
    pub raw_records: Vec<String>,
}

/// 应用状态
#[derive(Debug)]
pub struct AppState {
    /// 原始日志缓冲
    pub log_entries: VecDeque<LogEntry>,
    /// 最大日志条数
    pub max_log_entries: usize,
    /// 已解析的消息
    pub messages: Vec<MessageData>,
    /// 当前正在组装的消息（跨帧）
    pub current_message: MessageData,
    /// 接收的消息计数
    pub msg_count: usize,
    /// 接收的结果计数
    pub result_count: usize,
    /// 运行开始时间
    pub start_time: Option<chrono::DateTime<Local>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            log_entries: VecDeque::new(),
            max_log_entries: 1000,
            messages: Vec::new(),
            current_message: MessageData::default(),
            msg_count: 0,
            result_count: 0,
            start_time: None,
        }
    }

    pub fn add_log(&mut self, entry: LogEntry) {
        if self.log_entries.len() >= self.max_log_entries {
            self.log_entries.pop_front();
        }
        self.log_entries.push_back(entry);
    }

    pub fn finish_message(&mut self) {
        if self.current_message.header.is_some() {
            self.msg_count += 1;
            self.result_count += self.current_message.results.len();
            self.messages.push(self.current_message.clone());
            self.current_message = MessageData::default();
        }
    }

    pub fn run_time_str(&self) -> String {
        match self.start_time {
            Some(start) => {
                let dur = Local::now().signed_duration_since(start);
                format!("{:02}:{:02}:{:02}", dur.num_hours(), dur.num_minutes() % 60, dur.num_seconds() % 60)
            }
            None => "00:00:00".to_string(),
        }
    }

    pub fn clear_log(&mut self) {
        self.log_entries.clear();
    }

    pub fn clear_messages(&mut self) {
        self.messages.clear();
        self.current_message = MessageData::default();
        self.msg_count = 0;
        self.result_count = 0;
    }
}

/// 格式化控制字符为可读形式
fn format_control_readable(data: &[u8]) -> String {
    let mut result = String::new();
    for &b in data {
        match b {
            0x02 => result.push_str("[STX]"),
            0x03 => result.push_str("[ETX]"),
            0x04 => result.push_str("[EOT]"),
            0x05 => result.push_str("[ENQ]"),
            0x06 => result.push_str("[ACK]"),
            0x15 => result.push_str("[NAK]"),
            0x17 => result.push_str("[ETB]"),
            0x0D => result.push_str("[CR]"),
            0x0A => result.push_str("[LF]"),
            0x20..=0x7e => result.push(b as char),
            _ => result.push_str(&format!("[{:02X}]", b)),
        }
    }
    result
}

/// 转为 HEX 字符串
fn hex_string(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ")
}
