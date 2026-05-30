# 测试指南

## 测试方式概览

| 方式 | 平台 | 需要物理串口 | 说明 |
|------|------|--------------|------|
| TCP 测试 | 全平台 | 否 | 推荐，最简单 |
| 虚拟串口 | Linux | 否 | 使用 socat 创建 |
| 物理串口 | 全平台 | 是 | 需要真实串口设备 |
| 集成测试 | Windows | 否 | 使用 com0com |

---

## 1. TCP 测试（推荐）

无需串口设备，通过 TCP 连接测试协议逻辑。

```bash
# 终端 1：启动 LIS 模拟器（无头模式）
cargo run -- --headless --tcp 12345

# 终端 2：运行测试脚本
python tests/test_tcp.py --port 12345
```

带 GUI 的 TCP 模式：
```bash
# 终端 1：启动 LIS 模拟器（GUI + TCP）
cargo run -- --tcp 12345

# 终端 2：运行测试脚本
python tests/test_tcp.py --port 12345
```

---

## 2. Linux 虚拟串口测试

使用 socat 创建虚拟串口对，模拟串口通信。

```bash
# 1. 创建虚拟串口（需要 sudo）
sudo ./tests/setup_vserial.sh start

# 2. 启动 LIS 模拟器
cargo run
# 在 GUI 中选择 /dev/ttyV0，点击"开始监听"

# 3. 发送测试数据（另一个终端）
./tests/test_serial_linux.sh

# 4. 测试完成后清理
sudo ./tests/setup_vserial.sh stop
```

自定义端口和波特率：
```bash
./tests/test_serial_linux.sh /dev/ttyV1 115200
```

---

## 3. Windows 测试

使用 com0com 虚拟串口：
```powershell
# 运行集成测试脚本
powershell -File tests/test_integration.ps1
```

手动测试：
```powershell
# 终端 1：启动 LIS 模拟器，选择 COM10

# 终端 2：运行仪器模拟器
python tests/instrument_simulator.py --port COM11 --baud 9600
```

---

## 4. Rust 单元测试

```bash
# 运行所有测试
cargo test

# 仅单元测试
cargo test --lib

# 仅集成测试
cargo test --test astm_e2e
```

---

## 文件说明

| 文件 | 用途 |
|------|------|
| `test_tcp.py` | TCP 模式集成测试 |
| `setup_vserial.sh` | Linux 虚拟串口管理 |
| `test_serial_linux.sh` | Linux 串口测试脚本 |
| `instrument_simulator.py` | 仪器模拟器（通用） |
| `protocol_config.py` | ASTM 协议配置 |
| `test_integration.ps1` | Windows 集成测试 |
| `astm_e2e.rs` | Rust 端到端测试 |
