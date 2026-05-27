//! LIS 模拟器
//!
//! 模拟 LIS（实验室信息系统）端，通过 RS232 串口接收仪器的 ASTM 协议数据。
//! 支持单向（仅接收结果）和双向（支持查询/应答）两种通信模式。

mod app;
mod astm;
mod serial;
mod state;

use std::cell::RefCell;
use std::rc::Rc;

use state::AppState;

slint::include_modules!();

fn main() -> anyhow::Result<()> {
    let window = LisMainWindow::new()?;

    // 初始化应用状态
    let app_state = Rc::new(RefCell::new(AppState::new()));

    // 串口通信通道
    let serial_rx: Rc<RefCell<Option<std::sync::mpsc::Receiver<serial::SerialEvent>>>> =
        Rc::new(RefCell::new(None));
    let serial_handle: Rc<RefCell<Option<serial::SerialHandle>>> = Rc::new(RefCell::new(None));

    // 帧数据缓冲区
    let frame_buffer: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));

    // 初始化串口列表
    app::ui_update::update_port_list(&window);

    // 绑定所有 UI 回调
    app::callbacks::bind_all(&window, app_state, serial_rx, serial_handle, frame_buffer);

    window.run()?;
    Ok(())
}
