//! 导出回调绑定

use slint::{ComponentHandle, SharedString};
use std::cell::RefCell;
use std::rc::Rc;

use lis_simulator::astm::record::PatientInfo;
use lis_simulator::print::{
    self,
    types::{PaperSize, PrintResultRow, ReportData},
};
use lis_simulator::state::AppState;

/// 绑定导出回调
pub fn bind_print_callbacks(
    window: &crate::LisMainWindow,
    app_state: Rc<RefCell<AppState>>,
) {
    let weak = window.as_weak();
    let app_state = app_state.clone();

    window.on_export_clicked(move || {
        let Some(win) = weak.upgrade() else { return };

        let state = app_state.borrow();
        if state.messages.is_empty() {
            win.set_status_text(SharedString::from("没有可导出的数据"));
            return;
        }

        // 收集所有结果
        let mut all_results = Vec::new();
        let mut patient_info: Option<PatientInfo> = None;
        let mut msg_time = String::new();

        for msg in &state.messages {
            if patient_info.is_none() {
                patient_info = msg.patient.clone();
            }
            for result in &msg.results {
                all_results.push(PrintResultRow::from_result_info(
                    (all_results.len() + 1) as u32,
                    result,
                ));
            }
            if msg_time.is_empty() {
                msg_time = msg.header.as_ref().map(|h| h.timestamp.clone()).unwrap_or_default();
            }
        }

        if all_results.is_empty() {
            win.set_status_text(SharedString::from("没有可导出的结果"));
            return;
        }

        let report = ReportData {
            title: "LIS 模拟器".to_string(),
            patient: patient_info,
            sample_id: String::new(),
            sample_type: String::new(),
            results: all_results,
            print_time: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            msg_time,
        };

        // 生成 PDF
        match print::pdf::generate_report_pdf(&report, PaperSize::A4) {
            Ok(pdf_data) => {
                let default_name = format!("检验报告_{}.pdf",
                    chrono::Local::now().format("%Y%m%d_%H%M%S"));

                // 弹出保存对话框
                let path = rfd::FileDialog::new()
                    .set_title("导出 PDF")
                    .set_file_name(&default_name)
                    .add_filter("PDF 文件", &["pdf"])
                    .save_file();

                if let Some(path) = path {
                    match std::fs::write(&path, &pdf_data) {
                        Ok(_) => {
                            win.set_status_text(SharedString::from(
                                format!("已导出: {}", path.file_name().unwrap_or_default().to_string_lossy())
                            ));
                        }
                        Err(e) => {
                            win.set_status_text(SharedString::from(format!("导出失败: {}", e)));
                        }
                    }
                }
            }
            Err(e) => {
                win.set_status_text(SharedString::from(format!("生成失败: {}", e)));
            }
        }
    });
}
