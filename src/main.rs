//! LIS 模拟器
//!
//! 模拟 LIS（实验室信息系统）端，通过 RS232 串口接收仪器的 ASTM 协议数据。
//! 支持单向（仅接收结果）和双向（支持查询/应答）两种通信模式。
//!
//! 用法:
//!   lis-simulator                        # 正常串口模式 (GUI)
//!   lis-simulator --tcp 12345            # TCP + GUI 模式
//!   lis-simulator --headless --tcp 12345 # 无头 TCP 模式（集成测试用）

mod app;
mod headless;

use std::cell::RefCell;
use std::rc::Rc;

use lis_simulator::serial;
use lis_simulator::state::AppState;

slint::include_modules!();

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let is_headless = args.iter().any(|a| a == "--headless");
    let tcp_port: Option<u16> = args
        .iter()
        .position(|a| a == "--tcp")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok());

    // 无头模式：不启动 GUI，直接用 TCP 测试协议逻辑
    if is_headless {
        let port = tcp_port.unwrap_or(12345);
        return headless::run(port);
    }

    // GUI 模式
    let window = LisMainWindow::new()?;

    let app_state = Rc::new(RefCell::new(AppState::new()));
    let serial_rx: Rc<RefCell<Option<std::sync::mpsc::Receiver<serial::SerialEvent>>>> =
        Rc::new(RefCell::new(None));
    let serial_handle: Rc<RefCell<Option<serial::SerialHandle>>> = Rc::new(RefCell::new(None));
    let frame_buffer: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));

    if let Some(port) = tcp_port {
        // TCP + GUI 模式
        match serial::listen_tcp(port) {
            Ok((rx, handle)) => {
                *serial_rx.borrow_mut() = Some(rx);
                *serial_handle.borrow_mut() = Some(handle);
                app_state.borrow_mut().start_time = Some(chrono::Local::now());
                window.set_is_connected(true);
                window.set_status_text(
                    slint::SharedString::from(format!("TCP 模式: 127.0.0.1:{}", port)),
                );
            }
            Err(e) => {
                eprintln!("TCP 监听失败: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        app::ui_update::update_port_list(&window);
    }

    app::callbacks::bind_all(&window, app_state, serial_rx, serial_handle, frame_buffer);
    window.run()?;
    Ok(())
}
