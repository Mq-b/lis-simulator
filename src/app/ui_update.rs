//! UI 更新函数：将应用状态同步到 Slint 界面

use slint::{ModelRc, SharedString, VecModel};
use crate::state::*;
use super::super::LisMainWindow;

/// 将日志条目转换为 Slint 类型
fn to_slint_log(entry: &LogEntry) -> crate::LogEntry {
    crate::LogEntry {
        timestamp: SharedString::from(&entry.timestamp),
        direction: SharedString::from(&entry.direction),
        ctrl_type: SharedString::from(&entry.ctrl_type),
        raw_data: SharedString::from(&entry.raw_data),
        hex_data: SharedString::from(&entry.hex_data),
    }
}

/// 将解析后的消息转换为 Slint 类型
fn to_slint_message(idx: usize, msg: &MessageData) -> crate::ParsedMessage {
    let h = msg.header.as_ref();
    let p = msg.patient.as_ref();
    let l = msg.terminator.as_ref();

    crate::ParsedMessage {
        msg_index: SharedString::from(format!("#{}", idx + 1)),
        h_sender: SharedString::from(h.map(|h| h.sender.as_str()).unwrap_or("")),
        h_msg_type: SharedString::from(h.map(|h| h.message_type_display()).unwrap_or("")),
        h_version: SharedString::from(h.map(|h| h.version.as_str()).unwrap_or("")),
        h_time: SharedString::from(h.map(|h| h.timestamp.as_str()).unwrap_or("")),
        p_patient_id: SharedString::from(p.map(|p| p.patient_id.as_str()).unwrap_or("")),
        p_name: SharedString::from(p.map(|p| p.name.as_str()).unwrap_or("")),
        p_age: SharedString::from(p.map(|p| p.age.as_str()).unwrap_or("")),
        p_sex: SharedString::from(p.map(|p| p.sex_display()).unwrap_or("")),
        p_visit_type: SharedString::from(p.map(|p| p.visit_type_display()).unwrap_or("")),
        p_bed: SharedString::from(p.map(|p| p.bed.as_str()).unwrap_or("")),
        p_doctor: SharedString::from(p.map(|p| p.doctor.as_str()).unwrap_or("")),
        p_department: SharedString::from(p.map(|p| p.department.as_str()).unwrap_or("")),
        has_patient: p.is_some(),
        l_terminator: SharedString::from(l.map(|l| l.code_display()).unwrap_or("")),
    }
}

/// 将结果行转换为 Slint Model
fn to_slint_result_rows(msg: &MessageData) -> ModelRc<crate::ResultRow> {
    let rows: Vec<crate::ResultRow> = msg
        .results
        .iter()
        .map(|r| crate::ResultRow {
            item_code: SharedString::from(&r.item_code),
            value: SharedString::from(&r.value),
            unit: SharedString::from(&r.unit),
            flag: SharedString::from(r.flag_display()),
            ref_range: SharedString::from(&r.ref_range_display()),
            test_time: SharedString::from(&r.test_time),
        })
        .collect();
    ModelRc::new(VecModel::from(rows))
}

/// 更新日志面板
pub fn update_log(win: &LisMainWindow, state: &AppState) {
    let entries: Vec<crate::LogEntry> =
        state.log_entries.iter().map(|e| to_slint_log(e)).collect();
    win.set_log_entries(ModelRc::new(VecModel::from(entries)));
}

/// 更新解析结果面板
pub fn update_results(win: &LisMainWindow, state: &AppState) {
    let messages: Vec<crate::ParsedMessage> = state
        .messages
        .iter()
        .enumerate()
        .map(|(i, m)| to_slint_message(i, m))
        .collect();
    win.set_messages(ModelRc::new(VecModel::from(messages)));

    let result_models: Vec<ModelRc<crate::ResultRow>> = state
        .messages
        .iter()
        .map(|m| to_slint_result_rows(m))
        .collect();
    win.set_result_rows(ModelRc::new(VecModel::from(result_models)));
}

/// 更新状态栏
pub fn update_status(win: &LisMainWindow, state: &AppState) {
    win.set_msg_count(state.msg_count as i32);
    win.set_result_count(state.result_count as i32);
    win.set_run_time(SharedString::from(state.run_time_str()).into());
}

/// 更新串口列表
pub fn update_port_list(win: &LisMainWindow) {
    let ports = crate::serial::list_ports();
    let slint_ports: Vec<SharedString> = ports.iter().map(|p| SharedString::from(p.as_str())).collect();
    win.set_port_list(ModelRc::new(VecModel::from(slint_ports)));
}
