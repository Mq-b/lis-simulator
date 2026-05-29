//! 协议配置加载

use serde::Deserialize;
use std::collections::HashMap;
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

/// 单条查询应答配置（样本 ID → 固定应答）
#[derive(Debug, Clone, Deserialize)]
pub struct QueryResponseEntry {
    pub patient_id: String,
    pub name: String,
    pub age: String,
    pub sex: String,
    pub visit_type: String,
    pub bed: String,
    pub doctor: String,
    pub department: String,
    pub item_code: String,
    pub result_value: String,
    pub unit: String,
    pub ref_low: String,
    pub ref_high: String,
    pub sample_type: String,
}

/// 查询应答配置：样本 ID → 应答
pub type QueryResponseConfig = HashMap<String, QueryResponseEntry>;

static CONFIG: OnceLock<ProtocolConfig> = OnceLock::new();
static QUERY_RESPONSES: OnceLock<QueryResponseConfig> = OnceLock::new();

/// 获取全局配置
pub fn get_config() -> &'static ProtocolConfig {
    CONFIG.get_or_init(|| {
        load_config().unwrap_or_else(|e| {
            eprintln!("加载协议配置失败: {}，使用默认配置", e);
            default_config()
        })
    })
}

/// 获取查询应答配置
pub fn get_query_responses() -> &'static QueryResponseConfig {
    QUERY_RESPONSES.get_or_init(|| {
        load_query_responses().unwrap_or_else(|e| {
            eprintln!("加载查询应答配置失败: {}，使用空配置", e);
            QueryResponseConfig::new()
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

/// 从文件加载查询应答配置
fn load_query_responses() -> Result<QueryResponseConfig, Box<dyn std::error::Error>> {
    let paths = [
        "settings/query_responses.json",
        "query_responses.json",
    ];

    for path in &paths {
        if Path::new(path).exists() {
            let content = std::fs::read_to_string(path)?;
            let config: QueryResponseConfig = serde_json::from_str(&content)?;
            return Ok(config);
        }
    }

    Err("未找到 query_responses.json 配置文件".into())
}

/// 构建查询应答记录列表
///
/// 根据样本 ID 从配置中查找对应数据，构建 ASTM 应答记录。
/// - 已配置的样本 → 返回 H + P + O + L(N)
/// - 未配置的样本 → 返回 H + L(I)
///
/// 注意：R3M 的 getSample() 只解析 H/P/O/L，不支持 R 记录。
///
/// # 返回
/// `(records, is_found)` — 记录列表和是否找到配置
pub fn build_query_response_records(sample_id: &str, timestamp: &str) -> (Vec<String>, bool) {
    let responses = get_query_responses();

    let Some(entry) = responses.get(sample_id) else {
        return (
            vec![
                format!("H|\\^&|LIS_SIM|QA|V1.0|{}", timestamp),
                "L|1|I".to_string(),
            ],
            false,
        );
    };

    (
        vec![
            format!("H|\\^&|LIS_SIM|QA|V1.0|{}", timestamp),
            format!("P|1|{}|{}|{}|{}|{}|{}|{}|{}",
                entry.patient_id, entry.name, entry.age, entry.sex,
                entry.visit_type, entry.bed, entry.doctor, entry.department),
            format!("O|1|{}|{}||1|{}|{}|Q",
                sample_id, entry.item_code, timestamp, entry.sample_type),
            "L|1|N".to_string(),
        ],
        true,
    )
}
