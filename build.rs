fn main() {
    slint_build::compile_with_config(
        "ui/main.slint",
        slint_build::CompilerConfiguration::new().with_style("fluent".to_string()),
    )
    .unwrap();

    // 设置 Windows 图标
    embed_resource::compile("assets/app.rc", std::iter::empty::<&str>());
}
