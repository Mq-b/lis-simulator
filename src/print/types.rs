//! 打印功能数据类型定义

use crate::astm::record::{PatientInfo, ResultInfo};

/// 纸张大小
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaperSize {
    /// A4 纸张 (210mm x 297mm)
    A4,
    /// A5 纸张 (148mm x 210mm)
    A5,
}

impl PaperSize {
    /// 获取纸张宽度 (mm)
    pub fn width_mm(&self) -> f64 {
        match self {
            PaperSize::A4 => 210.0,
            PaperSize::A5 => 148.0,
        }
    }

    /// 获取纸张高度 (mm)
    pub fn height_mm(&self) -> f64 {
        match self {
            PaperSize::A4 => 297.0,
            PaperSize::A5 => 210.0,
        }
    }

    /// 获取纸张显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            PaperSize::A4 => "A4 (210×297mm)",
            PaperSize::A5 => "A5 (148×210mm)",
        }
    }
}

/// 打印配置
pub struct PrintConfig {
    /// 打印机名称
    pub printer_name: String,
    /// 纸张大小
    pub paper_size: PaperSize,
    /// 打印份数
    pub copies: u32,
}

/// 报告数据（从 MessageData 转换）
pub struct ReportData {
    /// 报告标题
    pub title: String,
    /// 患者信息
    pub patient: Option<PatientInfo>,
    /// 样本编号
    pub sample_id: String,
    /// 样本类型
    pub sample_type: String,
    /// 检验结果
    pub results: Vec<PrintResultRow>,
    /// 打印时间
    pub print_time: String,
    /// 消息时间
    pub msg_time: String,
}

/// 打印结果行
pub struct PrintResultRow {
    /// 序号
    pub seq: u32,
    /// 项目代码/名称
    pub item_code: String,
    /// 结果值
    pub value: String,
    /// 单位
    pub unit: String,
    /// 参考范围
    pub ref_range: String,
    /// 异常标志
    pub flag: AbnormalFlag,
    /// 备注
    pub comment: String,
}

/// 异常标志
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbnormalFlag {
    /// 正常
    Normal,
    /// 偏高
    High,
    /// 偏低
    Low,
    /// 阳性
    Positive,
    /// 弱阳性
    WeakPositive,
    /// 阴性
    Negative,
    /// 异常/错误
    Error,
}

impl AbnormalFlag {
    /// 从 ASTM 标志字符创建
    pub fn from_flag_str(flag: &str) -> Self {
        match flag {
            "H" => AbnormalFlag::High,
            "L" => AbnormalFlag::Low,
            "P" => AbnormalFlag::Positive,
            "W" => AbnormalFlag::WeakPositive,
            "N" => AbnormalFlag::Negative,
            "E" | "U" => AbnormalFlag::Error,
            _ => AbnormalFlag::Normal,
        }
    }

    /// 是否异常（需要标记）
    pub fn is_abnormal(&self) -> bool {
        matches!(
            self,
            AbnormalFlag::High
                | AbnormalFlag::Low
                | AbnormalFlag::Positive
                | AbnormalFlag::WeakPositive
                | AbnormalFlag::Error
        )
    }

    /// 获取箭头符号
    pub fn arrow(&self) -> &'static str {
        match self {
            AbnormalFlag::High => "↑",
            AbnormalFlag::Low => "↓",
            AbnormalFlag::Positive => "+",
            AbnormalFlag::WeakPositive => "±",
            AbnormalFlag::Negative => "-",
            _ => "",
        }
    }

    /// 获取中文标签
    pub fn label(&self) -> &'static str {
        match self {
            AbnormalFlag::High => "偏高",
            AbnormalFlag::Low => "偏低",
            AbnormalFlag::Positive => "阳性",
            AbnormalFlag::WeakPositive => "弱阳性",
            AbnormalFlag::Negative => "阴性",
            AbnormalFlag::Error => "异常",
            AbnormalFlag::Normal => "",
        }
    }
}

impl PrintResultRow {
    /// 从 ResultInfo 创建
    pub fn from_result_info(seq: u32, info: &ResultInfo) -> Self {
        Self {
            seq,
            item_code: info.item_code.clone(),
            value: info.value.clone(),
            unit: info.unit.clone(),
            ref_range: info.ref_range_display(),
            flag: AbnormalFlag::from_flag_str(&info.flag),
            comment: info.comment.clone(),
        }
    }

    /// 获取带箭头的结果值
    pub fn value_with_arrow(&self) -> String {
        if self.flag.is_abnormal() && !self.flag.arrow().is_empty() {
            format!("{} {}", self.value, self.flag.arrow())
        } else {
            self.value.clone()
        }
    }
}
