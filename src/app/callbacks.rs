//! 回调绑定：将 Slint UI 事件连接到业务逻辑

use slint::{ComponentHandle, SharedString, Timer, TimerMode};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use lis_simulator::astm::control::*;
use lis_simulator::astm::frame::*;
use lis_simulator::astm::record::*;
use lis_simulator::serial::port::*;
use lis_simulator::state::*;

use super::ui_update;
use super::super::LisMainWindow;

/// 绑定所有 UI 回调
pub fn bind_all(
    window: &LisMainWindow,
    app_state: Rc<RefCell<AppState>>,
    serial_rx: Rc<RefCell<Option<mpsc::Receiver<SerialEvent>>>>,
    serial_handle: Rc<RefCell<Option<SerialHandle>>>,
    frame_buffer: Rc<RefCell<Vec<u8>>>,
) {
    bind_refresh_ports(window);
    bind_connect(window, &app_state, &serial_rx, &serial_handle, &frame_buffer);
    bind_disconnect(window, &serial_handle, &frame_buffer);
    bind_poll_timer(window, &app_state, &serial_rx, &serial_handle, &frame_buffer);
    bind_clear_log(window, &app_state);
    bind_clear_results(window, &app_state);
    bind_export_log(window, &app_state);
    bind_send_query(window, &serial_handle, &app_state);
    bind_send_reply(window, &serial_handle, &app_state);
    bind_load_template(window);
}

/// 绑定刷新串口列表回调
fn bind_refresh_ports(window: &LisMainWindow) {
    let weak = window.as_weak();
    window.on_refresh_ports(move || {
        if let Some(win) = weak.upgrade() {
            ui_update::update_port_list(&win);
        }
    });
}

/// 绑定连接串口回调
fn bind_connect(
    window: &LisMainWindow,
    app_state: &Rc<RefCell<AppState>>,
    serial_rx: &Rc<RefCell<Option<mpsc::Receiver<SerialEvent>>>>,
    serial_handle: &Rc<RefCell<Option<SerialHandle>>>,
    frame_buffer: &Rc<RefCell<Vec<u8>>>,
) {
    let weak = window.as_weak();
    let serial_rx = serial_rx.clone();
    let serial_handle = serial_handle.clone();
    let app_state = app_state.clone();
    let _frame_buffer = frame_buffer.clone();

    window.on_connect_clicked(
        move |port_name, baud_idx, db_idx, par_idx, sb_idx, fc_idx, _bidirectional| {
            let Some(win) = weak.upgrade() else { return };

            if port_name.is_empty() {
                win.set_status_text(SharedString::from("请选择串口号"));
                return;
            }

            let config = SerialConfig {
                port_name: port_name.to_string(),
                baud_rate: baud_rate_from_index(baud_idx as usize),
                data_bits: data_bits_from_index(db_idx as usize),
                parity: parity_from_index(par_idx as usize),
                stop_bits: stop_bits_from_index(sb_idx as usize),
                flow_control: flow_control_from_index(fc_idx as usize),
            };

            match open_port(&config) {
                Ok((rx, handle)) => {
                    *serial_rx.borrow_mut() = Some(rx);
                    *serial_handle.borrow_mut() = Some(handle);

                    let mut state = app_state.borrow_mut();
                    state.start_time = Some(chrono::Local::now());
                    state.clear_log();
                    state.clear_messages();

                    win.set_is_connected(true);
                    win.set_status_text(
                        SharedString::from(format!("已连接 {} @ {}bps", config.port_name, config.baud_rate)),
                    );
                }
                Err(e) => {
                    win.set_status_text(SharedString::from(format!("连接失败: {}", e)));
                }
            }
        },
    );
}

/// 绑定断开连接回调
fn bind_disconnect(
    window: &LisMainWindow,
    serial_handle: &Rc<RefCell<Option<SerialHandle>>>,
    frame_buffer: &Rc<RefCell<Vec<u8>>>,
) {
    let weak = window.as_weak();
    let serial_handle = serial_handle.clone();
    let frame_buffer = frame_buffer.clone();

    window.on_disconnect_clicked(move || {
        if let Some(win) = weak.upgrade() {
            *serial_handle.borrow_mut() = None;
            frame_buffer.borrow_mut().clear();
            win.set_is_connected(false);
            win.set_status_text(SharedString::from("已断开连接"));
        }
    });
}

/// 处理接收到的单字节控制消息
fn handle_control_byte(
    data: &[u8],
    app_state: &mut AppState,
    serial_handle: &Option<SerialHandle>,
) {
    if data.len() != 1 || !is_control_message(data[0]) {
        return;
    }

    let ctrl = identify_control(data[0]);
    let name = match ctrl {
        ControlChar::Enq => {
            // 收到 ENQ，自动回复 ACK
            if let Some(ref handle) = serial_handle {
                handle.write_byte(ACK);
                app_state.add_log(LogEntry::tx(&[ACK], "ACK"));
            }
            "ENQ"
        }
        ControlChar::Ack => "ACK",
        ControlChar::Nak => "NAK",
        ControlChar::Eot => {
            // 收到 EOT，消息传输完成
            app_state.finish_message();
            "EOT"
        }
        _ => "?",
    };
    app_state.add_log(LogEntry::rx(data, name));
}

/// 处理接收到的数据帧
fn handle_data_frame(
    data: &[u8],
    buf: &mut Vec<u8>,
    app_state: &mut AppState,
    serial_handle: &Option<SerialHandle>,
) {
    buf.extend_from_slice(data);
    app_state.add_log(LogEntry::rx(data, "DATA"));

    // 尝试从缓冲区中解析完整帧
    while let Some((frame, consumed)) = try_parse_frame(buf) {
        let cs_valid = if frame.checksum_valid { "OK" } else { "FAIL" };
        app_state.add_log(LogEntry::rx(
            &[],
            &format!("帧#{} 校验:{}", frame.frame_number, cs_valid),
        ));

        if frame.checksum_valid {
            process_frame_records(&frame, app_state);
            // 自动回 ACK
            if let Some(ref handle) = serial_handle {
                handle.write_byte(ACK);
                app_state.add_log(LogEntry::tx(&[ACK], "ACK"));
            }
        }

        buf.drain(..consumed);
    }
}

/// 处理一帧中的所有记录
fn process_frame_records(frame: &AstmFrame, app_state: &mut AppState) {
    for record_line in &frame.records {
        let record = parse_record(record_line);
        match &record.record_type {
            RecordType::Header => {
                app_state.current_message.header = Some(extract_header(&record));
            }
            RecordType::Patient => {
                app_state.current_message.patient = Some(extract_patient(&record));
            }
            RecordType::Result => {
                app_state.current_message.results.push(extract_result(&record));
            }
            RecordType::Terminator => {
                app_state.current_message.terminator = Some(extract_terminator(&record));
                app_state.finish_message();
            }
            _ => {}
        }
        app_state.current_message.raw_records.push(record_line.clone());
    }
}

/// 绑定定时器轮询串口数据
fn bind_poll_timer(
    window: &LisMainWindow,
    app_state: &Rc<RefCell<AppState>>,
    serial_rx: &Rc<RefCell<Option<mpsc::Receiver<SerialEvent>>>>,
    serial_handle: &Rc<RefCell<Option<SerialHandle>>>,
    frame_buffer: &Rc<RefCell<Vec<u8>>>,
) {
    let weak = window.as_weak();
    let serial_rx = serial_rx.clone();
    let serial_handle = serial_handle.clone();
    let app_state = app_state.clone();
    let frame_buffer = frame_buffer.clone();

    let poll_timer = Timer::default();
    poll_timer.start(TimerMode::Repeated, Duration::from_millis(50), move || {
        let Some(win) = weak.upgrade() else { return };
        let rx_guard = serial_rx.borrow();
        let rx = match rx_guard.as_ref() {
            Some(rx) => rx,
            None => return,
        };

        while let Ok(event) = rx.try_recv() {
            match event {
                SerialEvent::DataReceived(data) => {
                    let mut state = app_state.borrow_mut();
                    let mut buf = frame_buffer.borrow_mut();
                    let handle_guard = serial_handle.borrow();

                    if data.len() == 1 && is_control_message(data[0]) {
                        handle_control_byte(&data, &mut state, &handle_guard);
                    } else {
                        handle_data_frame(&data, &mut buf, &mut state, &handle_guard);
                    }

                    ui_update::update_log(&win, &state);
                    ui_update::update_results(&win, &state);
                    ui_update::update_status(&win, &state);
                }
                SerialEvent::Error(err) => {
                    let mut state = app_state.borrow_mut();
                    state.add_log(LogEntry::rx(err.as_bytes(), "ERR"));
                    ui_update::update_log(&win, &state);
                    win.set_status_text(SharedString::from(format!("错误: {}", err)));
                }
                SerialEvent::Closed => {
                    win.set_is_connected(false);
                    win.set_status_text(SharedString::from("串口已关闭"));
                    break;
                }
            }
        }
    });
}

/// 绑定清空日志回调
fn bind_clear_log(window: &LisMainWindow, app_state: &Rc<RefCell<AppState>>) {
    let app_state = app_state.clone();
    let weak = window.as_weak();
    window.on_clear_log(move || {
        let mut state = app_state.borrow_mut();
        state.clear_log();
        if let Some(win) = weak.upgrade() {
            ui_update::update_log(&win, &state);
        }
    });
}

/// 绑定清空结果回调
fn bind_clear_results(window: &LisMainWindow, app_state: &Rc<RefCell<AppState>>) {
    let app_state = app_state.clone();
    let weak = window.as_weak();
    window.on_clear_results(move || {
        let mut state = app_state.borrow_mut();
        state.clear_messages();
        if let Some(win) = weak.upgrade() {
            ui_update::update_results(&win, &state);
            ui_update::update_status(&win, &state);
        }
    });
}

/// 绑定导出日志回调
fn bind_export_log(window: &LisMainWindow, app_state: &Rc<RefCell<AppState>>) {
    let app_state = app_state.clone();
    window.on_export_log(move || {
        let state = app_state.borrow();
        let path = chrono::Local::now().format("lis_log_%Y%m%d_%H%M%S.txt").to_string();
        let mut content = String::new();
        for entry in &state.log_entries {
            content.push_str(&format!(
                "{}\t{}\t{}\t{}\n",
                entry.timestamp, entry.direction, entry.ctrl_type, entry.raw_data
            ));
        }
        let _ = std::fs::write(&path, content);
    });
}

/// 绑定发送查询回调（双向模式）
fn bind_send_query(
    window: &LisMainWindow,
    serial_handle: &Rc<RefCell<Option<SerialHandle>>>,
    app_state: &Rc<RefCell<AppState>>,
) {
    let serial_handle = serial_handle.clone();
    let app_state = app_state.clone();
    window.on_send_query(move |sample_id, _start, _end| {
        let handle_guard = serial_handle.borrow();
        if let Some(ref handle) = *handle_guard {
            let mut state = app_state.borrow_mut();
            let now = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
            let q_record = format!("Q|1||{}||{}|O", sample_id, now);
            let frame = build_astm_frame(&[&q_record]);

            handle.write_byte(ENQ);
            state.add_log(LogEntry::tx(&[ENQ], "ENQ"));
            handle.write(&frame);
            state.add_log(LogEntry::tx(&frame, "DATA"));
            handle.write_byte(EOT);
            state.add_log(LogEntry::tx(&[EOT], "EOT"));
        }
    });
}

/// 绑定发送应答回调（双向模式）
fn bind_send_reply(
    window: &LisMainWindow,
    serial_handle: &Rc<RefCell<Option<SerialHandle>>>,
    app_state: &Rc<RefCell<AppState>>,
) {
    let serial_handle = serial_handle.clone();
    let app_state = app_state.clone();
    window.on_send_reply(move || {
        let handle_guard = serial_handle.borrow();
        if let Some(ref handle) = *handle_guard {
            let mut state = app_state.borrow_mut();
            let now = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
            let records = [
                format!("H|\\^&|LIS_SIM|QA|V1.0|{}", now),
                "L|1|N".to_string(),
            ];
            let frame = build_astm_frame(&records.iter().map(|s| s.as_str()).collect::<Vec<_>>());
            handle.write(&frame);
            state.add_log(LogEntry::tx(&frame, "REPLY"));
        }
    });
}

/// 绑定加载模板回调
fn bind_load_template(window: &LisMainWindow) {
    window.on_load_template(move || {
        // 暂时为空，后期可从文件加载应答模板
    });
}

/// 构建一个 ASTM 数据帧
///
/// 帧格式: STX + 帧号 + 记录(CR分隔) + ETX + 校验和 + CR + LF
fn build_astm_frame(records: &[&str]) -> Vec<u8> {
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
