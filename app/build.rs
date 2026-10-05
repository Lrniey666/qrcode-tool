//! Build script: compiles the Slint UI (with all translations bundled into the binary) and,
//! on Windows, embeds the application icon, version info and DPI-aware manifest.

fn main() {
    let config = slint_build::CompilerConfiguration::new()
        .with_style("fluent".into())
        .with_bundled_translations("translations");
    slint_build::compile_with_config("ui/main.slint", config).expect("Slint UI failed to compile");

    println!("cargo:rerun-if-changed=translations");
    println!("cargo:rerun-if-changed=windows/app.manifest");
    println!("cargo:rerun-if-changed=../assets/icons/app.ico");

    // Windows resources: rc.exe on Windows hosts, windres when cross-compiling with
    // mingw-w64. A missing resource compiler only produces a warning.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../assets/icons/app.ico");
        res.set_manifest_file("windows/app.manifest");
        if let Err(e) = res.compile() {
            println!("cargo:warning=failed to embed Windows resources: {e}");
        }
    }
}
