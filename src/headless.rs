//! 无头 TCP 服务器模式（仅用于集成测试）
//!
//! 不启动 GUI，直接在命令行监听 TCP 端口，接收 ASTM 协议数据并解析输出。
//! 支持续连接：客户端断开后自动等待下一个连接。
//!
//! 用法: lis-simulator --headless --tcp 12345

use lis_simulator::astm::control::*;
use lis_simulator::astm::frame::*;
use lis_simulator::astm::record::*;
use lis_simulator::state::*;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

pub fn run(port: u16) -> anyhow::Result<()> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))?;
    listener.set_nonblocking(true)?;
    println!("[HEADLESS] 监听 127.0.0.1:{}", port);

    let mut state = AppState::new();

    loop {
        println!("[HEADLESS] 等待连接...");
        let stream = wait_for_connection(&listener)?;
        println!("[HEADLESS] 已连接");

        handle_client(stream, &mut state);

        println!(
            "[HEADLESS] 累计统计: 消息数={} 结果数={}\n",
            state.msg_count, state.result_count
        );
    }
}

fn wait_for_connection(listener: &TcpListener) -> anyhow::Result<TcpStream> {
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_read_timeout(Some(std::time::Duration::from_millis(100)))?;
                return Ok(stream);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            Err(e) => return Err(e.into()),
        }
    }
}

fn handle_client(mut stream: TcpStream, state: &mut AppState) {
    let mut write_stream = match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    };
    let mut frame_buf: Vec<u8> = Vec::new();
    let mut read_buf = [0u8; 4096];

    loop {
        match stream.read(&mut read_buf) {
            Ok(n) if n > 0 => {
                let data = &read_buf[..n];
                process_data(data, &mut frame_buf, &mut write_stream, state);
            }
            Ok(0) => {
                println!("[HEADLESS] 连接断开");
                return;
            }
            Err(ref e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                continue;
            }
            Err(_) => {
                println!("[HEADLESS] 连接错误");
                return;
            }
            _ => {}
        }
    }
}

fn process_data(
    data: &[u8],
    frame_buf: &mut Vec<u8>,
    write_stream: &mut TcpStream,
    state: &mut AppState,
) {
    if data.len() == 1 && is_control_message(data[0]) {
        let ctrl = identify_control(data[0]);
        let name = match ctrl {
            ControlChar::Enq => {
                println!("[RX] ENQ");
                let _ = write_stream.write_all(&[ACK]);
                let _ = write_stream.flush();
                state.add_log(LogEntry::tx(&[ACK], "ACK"));
                println!("[TX] ACK");
                "ENQ"
            }
            ControlChar::Ack => {
                println!("[RX] ACK");
                "ACK"
            }
            ControlChar::Nak => {
                println!("[RX] NAK");
                "NAK"
            }
            ControlChar::Eot => {
                println!("[RX] EOT");
                state.finish_message();
                println!(
                    "\n[STATE] 消息数: {}, 结果数: {}\n",
                    state.msg_count, state.result_count
                );
                "EOT"
            }
            _ => "?",
        };
        state.add_log(LogEntry::rx(data, name));
    } else {
        frame_buf.extend_from_slice(data);
        state.add_log(LogEntry::rx(data, "DATA"));

        while let Some((frame, consumed)) = try_parse_frame(frame_buf) {
            let cs = if frame.checksum_valid { "OK" } else { "FAIL" };
            println!(
                "[FRAME] 帧#{} 校验:{} 记录数:{}",
                frame.frame_number,
                cs,
                frame.records.len()
            );

            if frame.checksum_valid {
                for record_line in &frame.records {
                    let record = parse_record(record_line);
                    match &record.record_type {
                        RecordType::Header => {
                            let h = extract_header(&record);
                            println!(
                                "  [H] 发送方:{} 类型:{} 版本:{}",
                                h.sender, h.message_type, h.version
                            );
                            state.current_message.header = Some(h);
                        }
                        RecordType::Patient => {
                            let p = extract_patient(&record);
                            println!("  [P] 患者:{} ID:{}", p.name, p.patient_id);
                            state.current_message.patient = Some(p);
                        }
                        RecordType::Result => {
                            let r = extract_result(&record);
                            println!(
                                "  [R] {} {} {} {} [{}]",
                                r.item_code,
                                r.value,
                                r.unit,
                                r.flag_display(),
                                r.ref_range_display()
                            );
                            state.current_message.results.push(r);
                        }
                        RecordType::Terminator => {
                            let t = extract_terminator(&record);
                            println!("  [L] {}", t.code_display());
                            state.current_message.terminator = Some(t);
                            state.finish_message();
                        }
                        _ => {
                            println!("  [?] {}", record_line);
                        }
                    }
                    state.current_message.raw_records.push(record_line.clone());
                }

                let _ = write_stream.write_all(&[ACK]);
                let _ = write_stream.flush();
                state.add_log(LogEntry::tx(&[ACK], "ACK"));
                println!("[TX] ACK");
            }

            frame_buf.drain(..consumed);
        }
    }
}
