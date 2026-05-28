//! 协议配置加载

use serde::Deserialize;
use std::path::Path;
use std::sync::OnceLock;

/// ASTM 协议配置
#[derive(Debug, Clone, Deserialize)]
pub struct AstmConfig {
    /// 是否有帧号字节（STX 后第一个字节）
    pub has_frame_number: bool,
    /// 校验和计算是否包含 STX
    pub checksum_includes_stx: bool,
    /// 校验和 hex 是否补零（2 位）
    pub checksum_zero_padded: bool,
}

/// 协议配置
#[derive(Debug, Clone, Deserialize)]
pub struct ProtocolConfig {
    pub astm: AstmConfig,
}

static CONFIG: OnceLock<ProtocolConfig> = OnceLock::new();

/// 获取全局配置
pub fn get_config() -> &'static ProtocolConfig {
    CONFIG.get_or_init(|| {
        load_config().unwrap_or_else(|e| {
            eprintln!("加载协议配置失败: {}，使用默认配置", e);
            default_config()
        })
    })
}

/// 默认配置（标准 ASTM）
fn default_config() -> ProtocolConfig {
    ProtocolConfig {
        astm: AstmConfig {
            has_frame_number: true,
            checksum_includes_stx: false,
            checksum_zero_padded: true,
        },
    }
}

/// 从文件加载配置
fn load_config() -> Result<ProtocolConfig, Box<dyn std::error::Error>> {
    // 尝试多个路径
    let paths = [
        "settings/protocol.json",
        "protocol.json",
    ];

    for path in &paths {
        if Path::new(path).exists() {
            let content = std::fs::read_to_string(path)?;
            let config: ProtocolConfig = serde_json::from_str(&content)?;
            return Ok(config);
        }
    }

    Err("未找到 protocol.json 配置文件".into())
}
