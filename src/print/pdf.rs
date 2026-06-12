//! PDF 生成模块
//!
//! 使用 printpdf 生成检验报告 PDF。

use anyhow::{Context, Result};
use printpdf::*;

use super::layout::{PageLayout, RegionHeights, TextAlignment, columns_for_paper};
use super::types::{PaperSize, PrintResultRow, ReportData};

/// PDF 字体集合
struct PdfFonts {
    /// 标题字体（黑体）
    title_font: IndirectFontRef,
    /// 正文字体（宋体）
    body_font: IndirectFontRef,
    /// 等宽字体（数值列）
    mono_font: IndirectFontRef,
}

/// 系统字体数据
struct SystemFontData {
    title: Vec<u8>,
    body: Vec<u8>,
}

/// 加载系统字体数据
fn load_system_font_data() -> Result<SystemFontData> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    // 查找微软雅黑（标题用）
    let title_data = find_system_font(&db, "Microsoft YaHei")
        .or_else(|| find_system_font(&db, "SimHei"))
        .context("未找到标题字体（Microsoft YaHei 或 SimHei）")?;

    // 查找宋体（正文用）
    let body_data = find_system_font(&db, "SimSun")
        .or_else(|| find_system_font(&db, "Microsoft YaHei"))
        .context("未找到正文字体（SimSun 或 Microsoft YaHei）")?;

    Ok(SystemFontData {
        title: title_data,
        body: body_data,
    })
}

/// 查找系统字体数据
fn find_system_font(db: &fontdb::Database, name: &str) -> Option<Vec<u8>> {
    let query = fontdb::Query {
        families: &[fontdb::Family::Name(name)],
        ..Default::default()
    };

    let face_id = db.query(&query)?;
    let (source, _index) = db.face_source(face_id)?;
    match source {
        fontdb::Source::File(path) => std::fs::read(path).ok(),
        fontdb::Source::Binary(_data) => {
            // Arc<dyn AsRef<[u8]>> 需要特殊处理
            // 暂时返回 None，优先使用文件路径
            None
        }
        fontdb::Source::SharedFile(_path, _data) => {
            // 共享文件源，暂不支持
            None
        }
    }
}

/// 生成检验报告 PDF
pub fn generate_report_pdf(report: &ReportData, paper_size: PaperSize) -> Result<Vec<u8>> {
    // 创建 PDF 文档
    let (doc, page_idx, layer_idx) = create_pdf_document(paper_size)?;

    // 加载字体
    let fonts = load_fonts(&doc)?;

    // 获取页面和图层引用
    let page = doc.get_page(page_idx);
    let layer = page.get_layer(layer_idx);

    // 创建布局参数
    let layout = PageLayout::new(paper_size);
    let heights = RegionHeights::for_paper(paper_size);
    let columns = columns_for_paper(paper_size);

    // 当前 Y 坐标（从页面顶部开始，向下递减）
    let mut current_y = layout.page_height - layout.content_top;

    // 绘制页眉
    current_y = draw_header(&layer, &fonts, &layout, current_y, report)?;

    // 绘制患者信息
    current_y = draw_patient_info(&layer, &fonts, &layout, current_y, report)?;

    // 绘制分隔线
    draw_separator_line(&layer, &layout, current_y);
    current_y -= heights.spacing;

    // 绘制检验结果表格
    current_y = draw_results_table(
        &layer,
        &fonts,
        &layout,
        current_y,
        &report.results,
        &columns,
        &heights,
        paper_size,
    )?;

    // 绘制页脚
    draw_footer(&layer, &fonts, &layout, current_y, report)?;

    // 保存为字节
    let buf = doc.save_to_bytes()?;

    Ok(buf)
}

/// 创建 PDF 文档
fn create_pdf_document(paper_size: PaperSize) -> Result<(PdfDocumentReference, PdfPageIndex, PdfLayerIndex)> {
    let page_width = Mm(paper_size.width_mm() as f32);
    let page_height = Mm(paper_size.height_mm() as f32);

    let (doc, page_idx, layer_idx) = PdfDocument::new(
        "临床检验报告",
        page_width,
        page_height,
        "Layer 0",
    );

    Ok((doc, page_idx, layer_idx))
}

/// 加载字体到文档
fn load_fonts(doc: &PdfDocumentReference) -> Result<PdfFonts> {
    let font_data = load_system_font_data()?;

    let title_font = doc.add_external_font(font_data.title.as_slice())?;
    let body_font = doc.add_external_font(font_data.body.as_slice())?;
    let mono_font = doc.add_builtin_font(BuiltinFont::Courier)?;

    Ok(PdfFonts {
        title_font,
        body_font,
        mono_font,
    })
}

/// 绘制页眉
fn draw_header(
    layer: &PdfLayerReference,
    fonts: &PdfFonts,
    layout: &PageLayout,
    y: f64,
    report: &ReportData,
) -> Result<f64> {
    let center_x = layout.page_width / 2.0;
    let mut current_y = y;

    // 医院名称（居中，大字）
    layer.use_text(&report.title, 16.0, Mm(center_x as f32), Mm(current_y as f32), &fonts.title_font);
    current_y -= 8.0;

    // 报告标题（居中）
    layer.use_text("临床检验报告", 14.0, Mm(center_x as f32), Mm(current_y as f32), &fonts.title_font);
    current_y -= 6.0;

    // 报告时间和消息时间
    let time_text = format!("报告时间: {}  消息时间: {}", report.print_time, report.msg_time);
    layer.use_text(&time_text, 9.0, Mm(layout.content_left as f32), Mm(current_y as f32), &fonts.body_font);
    current_y -= 5.0;

    // 分隔线
    draw_line(layer, layout.content_left, current_y, layout.content_right, current_y);
    current_y -= 5.0;

    Ok(current_y)
}

/// 绘制患者信息
fn draw_patient_info(
    layer: &PdfLayerReference,
    fonts: &PdfFonts,
    layout: &PageLayout,
    y: f64,
    report: &ReportData,
) -> Result<f64> {
    let mut current_y = y;
    let label_x = layout.content_left;

    if let Some(patient) = &report.patient {
        // 第一行：姓名、性别、年龄、门诊号/住院号
        draw_field(layer, fonts, label_x, current_y, "姓名:", &patient.name);
        draw_field(layer, fonts, label_x + 50.0, current_y, "性别:", &patient.sex_display());
        draw_field(layer, fonts, label_x + 90.0, current_y, "年龄:", &format!("{}岁", patient.age));
        draw_field(layer, fonts, label_x + 130.0, current_y, "门诊号:", &patient.patient_id);
        current_y -= 8.0;

        // 第二行：科室、床号、送检医生
        draw_field(layer, fonts, label_x, current_y, "科室:", &patient.department);
        draw_field(layer, fonts, label_x + 50.0, current_y, "床号:", &patient.bed);
        draw_field(layer, fonts, label_x + 90.0, current_y, "送检医生:", &patient.doctor);
        current_y -= 8.0;

        // 第三行：样本编号、样本类型
        draw_field(layer, fonts, label_x, current_y, "样本编号:", &report.sample_id);
        draw_field(layer, fonts, label_x + 80.0, current_y, "样本类型:", &report.sample_type);
    } else {
        // 无患者信息时显示样本信息
        draw_field(layer, fonts, label_x, current_y, "样本编号:", &report.sample_id);
        draw_field(layer, fonts, label_x + 80.0, current_y, "样本类型:", &report.sample_type);
    }

    Ok(current_y - 5.0)
}

/// 绘制字段（标签 + 值）
fn draw_field(
    layer: &PdfLayerReference,
    fonts: &PdfFonts,
    x: f64,
    y: f64,
    label: &str,
    value: &str,
) {
    // 标签
    layer.use_text(label, 9.0, Mm(x as f32), Mm(y as f32), &fonts.body_font);
    // 值（稍大字号）
    let value_x = x + label.len() as f64 * 2.5;
    layer.use_text(value, 10.0, Mm(value_x as f32), Mm(y as f32), &fonts.body_font);
}

/// 绘制直线
fn draw_line(layer: &PdfLayerReference, x1: f64, y: f64, x2: f64, _y2: f64) {
    let points = vec![
        (Point::new(Mm(x1 as f32), Mm(y as f32)), false),
        (Point::new(Mm(x2 as f32), Mm(y as f32)), false),
    ];
    let line = Line {
        points,
        is_closed: false,
    };
    layer.add_line(line);
}

/// 绘制分隔线
fn draw_separator_line(layer: &PdfLayerReference, layout: &PageLayout, y: f64) {
    draw_line(layer, layout.content_left, y, layout.content_right, y);
}

/// 绘制检验结果表格
fn draw_results_table(
    layer: &PdfLayerReference,
    fonts: &PdfFonts,
    layout: &PageLayout,
    y: f64,
    results: &[PrintResultRow],
    columns: &[super::layout::TableColumn],
    heights: &RegionHeights,
    paper_size: PaperSize,
) -> Result<f64> {
    let mut current_y = y;
    let table_width = layout.content_width;

    // 计算列宽
    let col_widths: Vec<f64> = columns.iter().map(|c| table_width * c.width_ratio).collect();

    // 计算列起始位置
    let mut col_x: Vec<f64> = Vec::new();
    let mut x = layout.content_left;
    for width in &col_widths {
        col_x.push(x);
        x += width;
    }

    // 绘制表头背景
    let header_rect = Rect::new(
        Mm(layout.content_left as f32),
        Mm((current_y - heights.table_header) as f32),
        Mm(layout.content_right as f32),
        Mm(current_y as f32),
    );
    layer.set_fill_color(Color::Rgb(Rgb::new(0.9, 0.9, 0.9, None)));
    layer.add_rect(header_rect);
    layer.set_fill_color(Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));

    // 绘制表头文字
    for (i, col) in columns.iter().enumerate() {
        let text_x = match col.alignment {
            TextAlignment::Left => col_x[i] + 2.0,
            TextAlignment::Center => col_x[i] + col_widths[i] / 2.0,
            TextAlignment::Right => col_x[i] + col_widths[i] - 2.0,
        };
        layer.use_text(col.header, 9.0, Mm(text_x as f32), Mm((current_y - 5.0) as f32), &fonts.title_font);
    }

    current_y -= heights.table_header;

    // 绘制表头下边框
    draw_line(layer, layout.content_left, current_y, layout.content_right, current_y);

    // 绘制数据行
    for (row_idx, result) in results.iter().enumerate() {
        // 交替行背景色
        if row_idx % 2 == 1 {
            let row_rect = Rect::new(
                Mm(layout.content_left as f32),
                Mm((current_y - heights.table_row) as f32),
                Mm(layout.content_right as f32),
                Mm(current_y as f32),
            );
            layer.set_fill_color(Color::Rgb(Rgb::new(0.95, 0.95, 0.95, None)));
            layer.add_rect(row_rect);
            layer.set_fill_color(Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
        }

        // 准备行数据
        let row_data = prepare_row_data(result, paper_size);

        // 绘制单元格文字
        for (i, text) in row_data.iter().enumerate() {
            let font = if i == 2 { &fonts.mono_font } else { &fonts.body_font }; // 数值列用等宽字体

            // 异常值用红色
            if result.flag.is_abnormal() && (i == 2 || i == 5) {
                layer.set_fill_color(Color::Rgb(Rgb::new(0.8, 0.0, 0.0, None)));
            }

            let text_x = match columns[i].alignment {
                TextAlignment::Left => col_x[i] + 2.0,
                TextAlignment::Center => col_x[i] + col_widths[i] / 2.0,
                TextAlignment::Right => col_x[i] + col_widths[i] - 2.0,
            };

            layer.use_text(text, 9.0, Mm(text_x as f32), Mm((current_y - 4.5) as f32), font);

            // 重置颜色
            if result.flag.is_abnormal() && (i == 2 || i == 5) {
                layer.set_fill_color(Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
            }
        }

        current_y -= heights.table_row;

        // 行下边框
        draw_line(layer, layout.content_left, current_y, layout.content_right, current_y);
    }

    Ok(current_y)
}

/// 准备行数据
fn prepare_row_data(result: &PrintResultRow, paper_size: PaperSize) -> Vec<String> {
    let value_text = result.value_with_arrow();
    let flag_text = if result.flag.is_abnormal() {
        format!("{} {}", result.flag.label(), result.flag.arrow())
    } else {
        String::new()
    };

    match paper_size {
        PaperSize::A4 => vec![
            result.seq.to_string(),
            result.item_code.clone(),
            value_text,
            result.unit.clone(),
            result.ref_range.clone(),
            flag_text,
            result.comment.clone(),
        ],
        PaperSize::A5 => vec![
            result.seq.to_string(),
            result.item_code.clone(),
            value_text,
            result.unit.clone(),
            result.ref_range.clone(),
            flag_text,
        ],
    }
}

/// 绘制页脚
fn draw_footer(
    layer: &PdfLayerReference,
    fonts: &PdfFonts,
    layout: &PageLayout,
    y: f64,
    report: &ReportData,
) -> Result<f64> {
    let mut current_y = y - 10.0;

    // 分隔线
    draw_line(layer, layout.content_left, current_y, layout.content_right, current_y);
    current_y -= 5.0;

    // 备注
    layer.use_text("注: 本报告仅供临床参考", 8.0, Mm(layout.content_left as f32), Mm(current_y as f32), &fonts.body_font);
    current_y -= 4.0;

    // 打印时间
    let print_info = format!("打印时间: {}", report.print_time);
    layer.use_text(&print_info, 8.0, Mm(layout.content_left as f32), Mm(current_y as f32), &fonts.body_font);

    Ok(current_y)
}
