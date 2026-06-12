//! 排版布局逻辑
//!
//! 定义 A4/A5 纸张的排版参数和坐标计算。

use super::types::PaperSize;

/// 页面边距 (mm)
pub struct Margins {
    pub top: f64,
    pub bottom: f64,
    pub left: f64,
    pub right: f64,
}

impl Margins {
    /// A4 默认边距
    pub fn a4() -> Self {
        Self {
            top: 12.0,
            bottom: 15.0,
            left: 15.0,
            right: 15.0,
        }
    }

    /// A5 默认边距
    pub fn a5() -> Self {
        Self {
            top: 10.0,
            bottom: 12.0,
            left: 12.0,
            right: 12.0,
        }
    }

    /// 根据纸张大小获取边距
    pub fn for_paper(size: PaperSize) -> Self {
        match size {
            PaperSize::A4 => Self::a4(),
            PaperSize::A5 => Self::a5(),
        }
    }
}

/// 表格列定义
pub struct TableColumn {
    /// 列名
    pub header: &'static str,
    /// 列宽比例 (0.0 ~ 1.0)
    pub width_ratio: f64,
    /// 对齐方式
    pub alignment: TextAlignment,
}

/// 文本对齐方式
#[derive(Debug, Clone, Copy)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

/// A4 表格列定义
pub fn a4_columns() -> Vec<TableColumn> {
    vec![
        TableColumn { header: "序号", width_ratio: 0.05, alignment: TextAlignment::Center },
        TableColumn { header: "检验项目", width_ratio: 0.25, alignment: TextAlignment::Left },
        TableColumn { header: "结果", width_ratio: 0.12, alignment: TextAlignment::Right },
        TableColumn { header: "单位", width_ratio: 0.12, alignment: TextAlignment::Center },
        TableColumn { header: "参考范围", width_ratio: 0.18, alignment: TextAlignment::Center },
        TableColumn { header: "提示", width_ratio: 0.13, alignment: TextAlignment::Center },
        TableColumn { header: "备注", width_ratio: 0.15, alignment: TextAlignment::Left },
    ]
}

/// A5 表格列定义（精简版，无备注列）
pub fn a5_columns() -> Vec<TableColumn> {
    vec![
        TableColumn { header: "序号", width_ratio: 0.06, alignment: TextAlignment::Center },
        TableColumn { header: "检验项目", width_ratio: 0.28, alignment: TextAlignment::Left },
        TableColumn { header: "结果", width_ratio: 0.14, alignment: TextAlignment::Right },
        TableColumn { header: "单位", width_ratio: 0.14, alignment: TextAlignment::Center },
        TableColumn { header: "参考范围", width_ratio: 0.22, alignment: TextAlignment::Center },
        TableColumn { header: "提示", width_ratio: 0.16, alignment: TextAlignment::Center },
    ]
}

/// 根据纸张大小获取列定义
pub fn columns_for_paper(size: PaperSize) -> Vec<TableColumn> {
    match size {
        PaperSize::A4 => a4_columns(),
        PaperSize::A5 => a5_columns(),
    }
}

/// 页面布局参数
pub struct PageLayout {
    /// 纸张宽度 (mm)
    pub page_width: f64,
    /// 纸张高度 (mm)
    pub page_height: f64,
    /// 可用区域左边界 (mm)
    pub content_left: f64,
    /// 可用区域右边界 (mm)
    pub content_right: f64,
    /// 可用区域宽度 (mm)
    pub content_width: f64,
    /// 可用区域上边界 (mm，从页面顶部算起)
    pub content_top: f64,
    /// 可用区域下边界 (mm，从页面顶部算起)
    pub content_bottom: f64,
}

impl PageLayout {
    /// 创建页面布局
    pub fn new(paper_size: PaperSize) -> Self {
        let margins = Margins::for_paper(paper_size);
        let page_width = paper_size.width_mm();
        let page_height = paper_size.height_mm();

        Self {
            page_width,
            page_height,
            content_left: margins.left,
            content_right: page_width - margins.right,
            content_width: page_width - margins.left - margins.right,
            content_top: margins.top,
            content_bottom: page_height - margins.bottom,
        }
    }

    /// 获取内容区域高度 (mm)
    pub fn content_height(&self) -> f64 {
        self.content_bottom - self.content_top
    }
}

/// 报告区域高度定义 (mm)
pub struct RegionHeights {
    /// 页眉区域高度
    pub header: f64,
    /// 患者信息区域高度
    pub patient_info: f64,
    /// 表头行高度
    pub table_header: f64,
    /// 表格数据行高度
    pub table_row: f64,
    /// 页脚区域高度
    pub footer: f64,
    /// 区域间距
    pub spacing: f64,
}

impl RegionHeights {
    /// A4 默认高度
    pub fn a4() -> Self {
        Self {
            header: 25.0,
            patient_info: 30.0,
            table_header: 8.0,
            table_row: 7.0,
            footer: 18.0,
            spacing: 5.0,
        }
    }

    /// A5 默认高度
    pub fn a5() -> Self {
        Self {
            header: 20.0,
            patient_info: 25.0,
            table_header: 7.0,
            table_row: 6.0,
            footer: 15.0,
            spacing: 4.0,
        }
    }

    /// 根据纸张大小获取高度
    pub fn for_paper(size: PaperSize) -> Self {
        match size {
            PaperSize::A4 => Self::a4(),
            PaperSize::A5 => Self::a5(),
        }
    }

    /// 计算表格区域可容纳的行数
    pub fn max_table_rows(&self, layout: &PageLayout) -> usize {
        let used_height = self.header
            + self.patient_info
            + self.table_header
            + self.footer
            + self.spacing * 4.0; // 4个间距

        let available = layout.content_height() - used_height;
        (available / self.table_row).floor() as usize
    }
}
