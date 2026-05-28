# LIS 模拟器 设计文档

## 项目概述

LIS 模拟器是一个基于 ASTM E1381/E1394 协议的实验室信息系统模拟器，用于测试医疗检验仪器的串口通信功能。程序模拟 LIS 端（接收端），通过 RS232 串口或 TCP 连接接收仪器发送的检验数据，实时解析并结构化展示。

核心特性：

- 完整实现 ASTM E1381 传输层协议（握手、帧解析、校验和）
- 支持 ASTM E1394 全部记录类型（H/P/O/R/Q/C/L）解析
- **协议可配置**：通过 JSON 配置文件适配非标准仪器帧格式，无需修改代码
- 支持物理串口和 TCP 两种连接方式
- 支持无头（Headless）模式，便于自动化测试和 CI 集成

## 架构设计

### 分层架构

```text
┌─────────────────────────────────────────────┐
│               UI 层 (Slint)                 │
│   ui/main.slint + src/app/                  │
│   回调绑定 / UI 更新 / 数据展示              │
├─────────────────────────────────────────────┤
│               二进制层 (bin)                 │
│   src/main.rs                               │
│   CLI 参数解析 / 运行模式选择                │
├─────────────────────────────────────────────┤
│               库层 (lib)                     │
│   astm/    serial/    state/    config/      │
│   协议解析  串口操作   状态管理  协议配置     │
└─────────────────────────────────────────────┘
```

库层不依赖 Slint，可独立使用（集成测试直接调用库 API）。

### 数据流

```text
串口/TCP 线程  ──mpsc channel──▶  主线程 Timer (50ms轮询)
                                      │
                                      ▼
                               帧缓冲区拼接
                               try_parse_frame()
                                      │
                                      ▼
                               记录解析 → AppState
                                      │
                                      ▼
                               UI 更新 (Slint 属性绑定)
```

- 串口读取在独立线程中进行，通过 `mpsc::channel` 发送 `SerialEvent`
- 主线程使用 Slint `Timer` 每 50ms 轮询 channel，每次最多处理 50 个事件
- 帧缓冲区支持跨多次读取拼接，处理 TCP/串口分包情况
- 解析完成后自动回复 ACK（单向模式）

### 运行模式

| 模式 | 启动方式 | 说明 |
| --- | --- | --- |
| GUI + 串口 | `cargo run` | 默认模式，打开 Slint 窗口，手动选择串口 |
| GUI + TCP | `cargo run -- --tcp 12345` | GUI 模式但使用 TCP 连接 |
| Headless TCP | `cargo run -- --headless --tcp 12345` | 无 GUI，TCP 服务器，用于自动化测试 |

## ASTM 协议实现

### E1381 传输层

#### 控制字符

| 字符 | 值 | 含义 |
| --- | --- | --- |
| ENQ | 0x05 | 请求通信 |
| ACK | 0x06 | 确认/准备好 |
| NAK | 0x15 | 否定确认 |
| EOT | 0x04 | 传输结束 |
| STX | 0x02 | 帧数据开始 |
| ETX | 0x03 | 帧数据结束 |
| ETB | 0x17 | 分帧结束 |
| CR | 0x0D | 回车 |
| LF | 0x0A | 换行 |

#### 帧格式

标准 ASTM 帧结构：

```text
STX [帧号] 数据 ETX 校验和(hex) CR LF
```

- **帧号**：单字节 ASCII 数字 (0-9)，可选（见协议可配置设计）
- **数据**：多条记录以 CR 分隔
- **校验和**：从帧号（或 STX）到 ETX（含）所有字节之和，模 256，转 hex 字符串
- **ETB**：用于中间分帧，校验和计算范围到 ETB 而非 ETX

#### 握手流程

单向模式（仪器 → LIS）：

```text
仪器        LIS
  ──ENQ──▶    收到 ENQ 自动回 ACK
  ◀──ACK──
  ──数据帧──▶  校验通过回 ACK
  ◀──ACK──
  ──数据帧──▶
  ◀──ACK──
  ──EOT──▶    收到 EOT，消息完成
```

#### 帧缓冲区

串口/TCP 数据可能分多次到达。程序维护帧缓冲区，每次收到数据时追加到缓冲区，然后循环调用 `try_parse_frame()` 尝试解析。解析成功后移除已消费的字节，剩余数据保留等待下次追加。

### E1394 数据层

#### 记录类型

| 记录 | 标识 | 说明 |
| --- | --- | --- |
| Header | `H\|` | 消息头：发送方、消息类型、版本、时间戳 |
| Patient | `P\|` | 患者信息：ID、姓名、年龄、性别、就诊类型、医生、科室 |
| Order | `O\|` | 检验申请：样本号、项目编码、样本类型 |
| Result | `R\|` | 检验结果：项目编码、结果值、单位、标志、参考范围 |
| Request | `Q\|` | 查询请求（双向模式） |
| Comment | `C\|` | 备注 |
| Terminator | `L\|` | 结束标记：N=正常结束，I=无信息 |

#### 字段分隔符

| 符号 | 含义 |
| --- | --- |
| `\|` | 字段分隔 |
| `^` | 组件分隔 |
| `\\` | 重复分隔 |
| `&` | 转义字符 |

#### 结果标志映射

| 标志 | 显示 |
| --- | --- |
| M | 正常 |
| H | 偏高 |
| L | 偏低 |
| P | 阳性 |
| W | 警告 |
| N | 正常(仪器) |
| E | 异常 |
| U | 不确定 |

## 协议可配置设计

现实中许多医疗仪器并不严格遵循 ASTM 标准，本项目通过 JSON 配置文件实现协议行为的灵活适配，**无需修改代码或重新编译**。

### 配置项

配置文件 `settings/protocol.json`：

```json
{
  "astm": {
    "has_frame_number": false,
    "checksum_includes_stx": true,
    "checksum_zero_padded": false
  }
}
```

| 配置项 | 类型 | 说明 |
| --- | --- | --- |
| `has_frame_number` | bool | STX 后是否有帧号字节。标准 ASTM 为 `true` |
| `checksum_includes_stx` | bool | 校验和计算是否包含 STX 字节。标准 ASTM 为 `false` |
| `checksum_zero_padded` | bool | 校验和 hex 是否补零到 2 位。标准 ASTM 为 `true` |

### 标准 vs 非标准对照

| 配置项 | 标准 ASTM | 非标准仪器（当前配置） |
| --- | --- | --- |
| `has_frame_number` | `true` — STX 后紧跟帧号 | `false` — 无帧号，数据直接跟在 STX 后 |
| `checksum_includes_stx` | `false` — 从帧号开始求和 | `true` — 从 STX 开始求和 |
| `checksum_zero_padded` | `true` — `0a` 补零为 2 位 | `false` — `a` 不补零 |

#### 帧结构对比

标准模式：

```text
STX [帧号] 记录1 CR 记录2 CR ... ETX XX(hex2) CR LF
     └──校验和范围──────────────────┘
```

非标准模式（当前配置）：

```text
STX 记录1 CR 记录2 CR ... ETX X(hex) CR LF
└───校验和范围───────────────────────┘
```

### 配置加载机制

- 使用 `OnceLock<ProtocolConfig>` 实现全局单例，启动时加载一次
- 搜索路径：`settings/protocol.json` → `protocol.json`（当前目录）
- 未找到配置文件时使用标准 ASTM 默认值
- 加载失败不中断程序，打印警告并使用默认配置

### Rust/Python 双端共享

测试脚本（Python）读取同一份 `settings/protocol.json`，确保测试端与应用端的帧格式完全一致：

```text
settings/protocol.json
    ├── src/config.rs (Rust 应用加载)
    └── tests/protocol_config.py (Python 测试加载)
```

Python 端的 `build_frame()` 函数与 Rust 端实现相同的帧构建逻辑，包括帧号、校验和计算范围、hex 补零等。

### 扩展方式

如需支持更多协议变体，在 `AstmConfig` 结构体中添加新字段，并同步更新 `settings/protocol.json` 的 schema 和 Python 端的配置加载。配置项设计为向后兼容——新增字段使用 `#[serde(default)]` 提供默认值。

## 测试体系

### 四层测试

| 层级 | 工具 | 说明 |
| --- | --- | --- |
| 单元测试 | `cargo test --lib` | 协议模块内部逻辑，22 项 |
| 集成测试 | `cargo test --test astm_e2e` | 完整通信流程模拟，8 项 |
| TCP 测试 | `python tests/test_tcp.py` | 无头模式端到端测试，3 项 |
| 串口测试 | `python tests/instrument_simulator.py` | 物理串口/com0com 测试 |

### CI 流程

GitHub Actions 在 Windows 环境运行：

```text
cargo check → cargo clippy -D warnings → cargo test → cargo build --release
```

构建产物自动上传为 artifact。
