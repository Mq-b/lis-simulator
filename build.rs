fn main() {
    slint_build::compile_with_config(
        "ui/main.slint",
        slint_build::CompilerConfiguration::new().with_style("fluent".to_string()),
    )
    .unwrap();

    // 仅在 Windows 上设置图标
    #[cfg(target_os = "windows")]
    embed_resource::compile("assets/app.rc", std::iter::empty::<&str>());
}
