use anyhow::{Context, Result};
use serialport::{DataBits, FlowControl, Parity, StopBits};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// 串口配置参数
#[derive(Debug, Clone)]
pub struct SerialConfig {
    pub port_name: String,
    pub baud_rate: u32,
    pub data_bits: DataBits,
    pub parity: Parity,
    pub stop_bits: StopBits,
    pub flow_control: FlowControl,
}

impl Default for SerialConfig {
    fn default() -> Self {
        Self {
            port_name: String::new(),
            baud_rate: 9600,
            data_bits: DataBits::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            flow_control: FlowControl::None,
        }
    }
}

/// 从 ComboBox 索引转换参数
pub fn data_bits_from_index(idx: usize) -> DataBits {
    match idx {
        0 => DataBits::Five,
        1 => DataBits::Six,
        2 => DataBits::Seven,
        _ => DataBits::Eight,
    }
}

pub fn parity_from_index(idx: usize) -> Parity {
    match idx {
        1 => Parity::Odd,
        2 => Parity::Even,
        _ => Parity::None,
    }
}

pub fn stop_bits_from_index(idx: usize) -> StopBits {
    match idx {
        1 => StopBits::Two,
        _ => StopBits::One,
    }
}

pub fn flow_control_from_index(idx: usize) -> FlowControl {
    match idx {
        1 => FlowControl::Hardware,
        2 => FlowControl::Software,
        _ => FlowControl::None,
    }
}

pub fn baud_rate_from_index(idx: usize) -> u32 {
    match idx {
        0 => 1200,
        1 => 2400,
        2 => 4800,
        3 => 9600,
        4 => 19200,
        5 => 38400,
        6 => 57600,
        7 => 115200,
        _ => 9600,
    }
}

/// 列举系统可用串口
pub fn list_ports() -> Vec<String> {
    serialport::available_ports()
        .map(|ports| ports.iter().map(|p| p.port_name.clone()).collect())
        .unwrap_or_default()
}

/// 串口事件：发送到主线程
#[derive(Debug, Clone)]
pub enum SerialEvent {
    /// 接收到数据 (原始字节)
    DataReceived(Vec<u8>),
    /// 串口错误
    Error(String),
    /// 串口已关闭
    Closed,
}

/// 串口读写句柄
pub struct SerialHandle {
    /// 发送数据到串口的通道
    write_tx: mpsc::Sender<Vec<u8>>,
    /// 停止信号
    stop_tx: mpsc::Sender<()>,
}

impl SerialHandle {
    /// 向串口写入数据
    pub fn write(&self, data: &[u8]) {
        let _ = self.write_tx.send(data.to_vec());
    }

    /// 发送单个控制字符
    pub fn write_byte(&self, byte: u8) {
        self.write(&[byte]);
    }
}

impl Drop for SerialHandle {
    fn drop(&mut self) {
        let _ = self.stop_tx.send(());
    }
}

/// 打开串口并在后台线程中读取
///
/// 返回 (事件接收通道, 串口写入句柄)
pub fn open_port(config: &SerialConfig) -> Result<(mpsc::Receiver<SerialEvent>, SerialHandle)> {
    let port = serialport::new(&config.port_name, config.baud_rate)
        .data_bits(config.data_bits)
        .parity(config.parity)
        .stop_bits(config.stop_bits)
        .flow_control(config.flow_control)
        .timeout(Duration::from_millis(100))
        .open()
        .with_context(|| format!("无法打开串口 {}", config.port_name))?;

    let (event_tx, event_rx) = mpsc::channel::<SerialEvent>();
    let (write_tx, write_rx) = mpsc::channel::<Vec<u8>>();
    let (stop_tx, stop_rx) = mpsc::channel::<()>();

    // 读取端口（需要 try_clone）
    let mut read_port = port
        .try_clone()
        .context("无法克隆串口用于读取")?;
    let mut write_port = port;

    // 读取线程
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            // 检查停止信号
            if stop_rx.try_recv().is_ok() {
                let _ = event_tx.send(SerialEvent::Closed);
                break;
            }

            match read_port.read(&mut buf) {
                Ok(n) if n > 0 => {
                    if event_tx
                        .send(SerialEvent::DataReceived(buf[..n].to_vec()))
                        .is_err()
                    {
                        break;
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    // 超时是正常的，继续读取
                    continue;
                }
                Err(e) => {
                    let _ = event_tx.send(SerialEvent::Error(e.to_string()));
                    break;
                }
                _ => {}
            }
        }
    });

    // 写入线程
    thread::spawn(move || {
        while let Ok(data) = write_rx.recv() {
            if write_port.write_all(&data).is_err() {
                break;
            }
            let _ = write_port.flush();
        }
    });

    let handle = SerialHandle { write_tx, stop_tx };

    Ok((event_rx, handle))
}
