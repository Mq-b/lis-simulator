//! 导出模块
//!
//! 提供检验报告导出为 PDF 功能。

pub mod types;
pub mod pdf;
pub mod layout;

// 重新导出常用类型
pub use types::{PaperSize, ReportData, PrintResultRow};
