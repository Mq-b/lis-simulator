use anyhow::{Context, Result};
use serialport::{DataBits, FlowControl, Parity, StopBits};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{mpsc, Arc, Mutex};
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
    #[allow(unused_mut)]
    let mut ports: Vec<String> = serialport::available_ports()
        .map(|ports| ports.iter().map(|p| p.port_name.clone()).collect())
        .unwrap_or_default();

    // Linux: 额外扫描 udev 符号链接（指向 /dev/tty* 的设备）
    #[cfg(target_os = "linux")]
    {
        ports.extend(scan_serial_symlinks("/dev/serial/by-id"));
        ports.extend(scan_serial_symlinks("/dev/serial/by-path"));
        ports.sort();
        ports.dedup();
    }

    ports
}

/// 扫描目录下的符号链接，返回解析后的设备路径
#[cfg(target_os = "linux")]
fn scan_serial_symlinks(dir: &str) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return vec![];
    };

    entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let path = e.path();
            // 只处理符号链接
            if !path.is_symlink() {
                return None;
            }
            // 解析符号链接指向的真实路径
            let real = std::fs::canonicalize(&path).ok()?;
            let real_str = real.to_string_lossy().to_string();
            // 只保留 /dev/tty* 设备（排除 /dev/bus/usb 等）
            if real_str.starts_with("/dev/tty") {
                Some(real_str)
            } else {
                None
            }
        })
        .collect()
}

/// 串口事件：发送到主线程
#[derive(Debug, Clone)]
pub enum SerialEvent {
    /// 接收到数据 (原始字节)
    DataReceived(Vec<u8>),
    /// 串口错误
    Error(String),
    /// 串口已关闭（永久断开）
    Closed,
    /// TCP 连接断开（等待重连）
    Disconnected,
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
                Ok(n)
                    if n > 0
                        && event_tx
                            .send(SerialEvent::DataReceived(buf[..n].to_vec()))
                            .is_err() =>
                {
                    break;
                }
                Ok(n) if n > 0 => {}
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

/// 监听 TCP 端口，持续接受连接，断开后自动等待下一个连接
///
/// 用于无物理串口时的集成测试。GUI 启动后即进入监听状态，
/// Python 脚本随时可以连接发送数据，断开后可重新连接。
pub fn listen_tcp(port: u16) -> Result<(mpsc::Receiver<SerialEvent>, SerialHandle)> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
        .with_context(|| format!("无法监听 TCP 端口 {}", port))?;
    listener.set_nonblocking(true)?;

    let (event_tx, event_rx) = mpsc::channel::<SerialEvent>();
    let (write_tx, write_rx) = mpsc::channel::<Vec<u8>>();
    let (stop_tx, stop_rx) = mpsc::channel::<()>();

    let current_stream: Arc<Mutex<Option<TcpStream>>> = Arc::new(Mutex::new(None));
    let stream_for_write = current_stream.clone();

    // 监听线程：持续接受连接，每个连接启一个读取线程
    let event_tx_clone = event_tx.clone();
    thread::spawn(move || {
        let mut active_readers: Vec<thread::JoinHandle<()>> = Vec::new();
        loop {
            if stop_rx.try_recv().is_ok() {
                break;
            }
            match listener.accept() {
                Ok((stream, addr)) => {
                    println!("[TCP] 已连接: {}", addr);

                    let read_stream = match stream.try_clone() {
                        Ok(s) => s,
                        Err(_) => continue,
                    };
                    // 设置当前可写流
                    if let Ok(mut guard) = current_stream.lock() {
                        *guard = Some(stream);
                    }

                    let tx = event_tx_clone.clone();
                    let handle = thread::spawn(move || {
                        run_tcp_reader(read_stream, tx);
                    });
                    active_readers.push(handle);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }
                Err(_) => {
                    thread::sleep(Duration::from_millis(500));
                    continue;
                }
            }
        }
        // 清理
        for h in active_readers {
            let _ = h.join();
        }
    });

    // 写入线程：从 channel 取数据写入当前连接
    thread::spawn(move || {
        while let Ok(data) = write_rx.recv() {
            if let Ok(guard) = stream_for_write.lock() {
                if let Some(ref stream) = *guard {
                    let mut s = stream.try_clone().expect("clone failed");
                    let _ = s.write_all(&data);
                    let _ = s.flush();
                }
            }
        }
    });

    let handle = SerialHandle { write_tx, stop_tx };
    Ok((event_rx, handle))
}

/// 单个 TCP 连接的读取循环
fn run_tcp_reader(mut stream: TcpStream, event_tx: mpsc::Sender<SerialEvent>) {
    stream
        .set_read_timeout(Some(Duration::from_millis(100)))
        .ok();
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(n)
                if n > 0
                    && event_tx
                        .send(SerialEvent::DataReceived(buf[..n].to_vec()))
                        .is_err() =>
            {
                break;
            }
            Ok(n) if n > 0 => {}
            Ok(0) => {
                println!("[TCP] 连接断开，等待重连...");
                let _ = event_tx.send(SerialEvent::Disconnected);
                break;
            }
            Err(ref e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                continue;
            }
            Err(_) => {
                let _ = event_tx.send(SerialEvent::Disconnected);
                break;
            }
            _ => {}
        }
    }
}
