# LIS 模拟器 - 项目文档

## 项目概述

基于 Rust + Slint 的 ASTM E1381/E1394 协议 LIS 模拟器，模拟实验室信息系统端接收医疗检验仪器的串口数据。

## 技术栈

- **语言**: Rust (edition 2021)
- **UI**: Slint 1.15 (fluent 风格)
- **串口**: serialport 4.x
- **测试**: Python 3 + pyserial (集成测试脚本)

## 项目结构

```text
├── Cargo.toml              # 项目配置 (lib + bin)
├── build.rs                # Slint 编译配置
├── settings/
│   ├── protocol.json       # 协议配置 (帧格式、校验和)
│   └── query_responses.json # 查询应答配置
├── images/
│   ├── astm_handshake.svg  # 握手流程图
│   ├── astm_frame_format.svg # 帧格式图
│   └── astm_record_types.svg # 数据区记录格式图
├── ui/
│   └── main.slint          # UI 界面定义
├── src/
│   ├── lib.rs              # 库入口 (导出 astm, serial, state, config)
│   ├── main.rs             # 二进制入口
│   ├── config.rs           # 协议配置加载
│   ├── headless.rs         # 无头模式 (TCP 服务)
│   ├── app/
│   │   ├── mod.rs          # app 模块
│   │   ├── callbacks.rs    # UI 回调绑定
│   │   └── ui_update.rs    # UI 更新函数
│   ├── astm/
│   │   ├── mod.rs          # ASTM 协议模块
│   │   ├── control.rs      # 控制字符 (ENQ/ACK/NAK/EOT)
│   │   ├── frame.rs        # 帧解析/构建/校验和
│   │   └── record.rs       # 记录解析 (H/P/O/R/Q/C/L)
│   ├── serial/
│   │   ├── mod.rs          # 串口模块
│   │   └── port.rs         # 串口操作 (列举/打开/读写)
│   └── state.rs            # 应用状态管理
├── tests/
│   ├── astm_e2e.rs         # Rust 端到端集成测试
│   ├── instrument_simulator.py  # Python 仪器模拟器
│   └── test_integration.ps1     # PowerShell 集成测试脚本
└── docs/
    └── TEST_PLAN.md        # 测试计划文档
```

## 架构要点

### 分层架构

- **lib 层** (`astm`, `serial`, `state`): 纯协议和状态逻辑，不依赖 Slint
- **bin 层** (`app`): UI 回调绑定，依赖 Slint 和 lib 层
- 集成测试通过 `lis_simulator` crate 访问 lib 层

### ASTM 协议实现

- 帧格式（R3M）: `STX + 数据 + ETX + checksum(hex) + CR + LF`（无帧号字节）
- 帧格式（标准）: `STX + 帧号 + 数据 + ETX + checksum(hex) + CR + LF`
- 校验和: 所有字节模256，转hex字符串（通过 `protocol.json` 配置）
- 握手: ENQ → ACK → 数据帧 → ACK → EOT
- 自动应答: 收到 ENQ 自动回 ACK，收到数据帧校验通过后自动回 ACK

### 协议配置

通过 `settings/protocol.json` 适配不同仪器的帧格式差异：

| 配置项 | 说明 | R3M 默认值 |
| ------ | ---- | ---------- |
| `has_frame_number` | STX 后是否有帧号字节 | `false` |
| `checksum_includes_stx` | 校验和计算是否包含 STX | `true` |
| `checksum_zero_padded` | 校验和 hex 是否补零（2位） | `false` |

### 关键设计决策

- 串口读取使用独立线程 + mpsc channel，避免阻塞 UI
- UI 更新使用 Timer 50ms 轮询，避免 Slint 线程安全问题
- 帧缓冲区支持跨多次读取拼接，处理分包情况
- 配置文件使用 JSON 格式，便于后期扩展协议字段映射

## 测试

```bash
# 单元测试 + 集成测试
cargo test

# 仅单元测试
cargo test --lib

# 仅集成测试
cargo test --test astm_e2e

# TCP 集成测试 (无需物理串口)
cargo run -- --headless --tcp 12345
python tests/test_tcp.py --port 12345

# 串口集成测试 (需要 com0com 虚拟串口)
python tests/instrument_simulator.py --port COM11 --baud 9600
powershell -File tests/test_integration.ps1
```

## 构建

```bash
cargo build --release
```

产物: `target/release/lis-simulator.exe`

## 更新检查清单

当修改 ASTM 协议实现时，确保:

- [ ] `astm/control.rs` 中的单元测试通过
- [ ] `astm/frame.rs` 中的帧解析/构建测试通过
- [ ] `astm/record.rs` 中的记录解析测试通过
- [ ] `tests/astm_e2e.rs` 中的端到端测试通过
- [ ] 更新 `docs/TEST_PLAN.md` 中的测试状态
