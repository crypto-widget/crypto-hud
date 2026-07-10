fn main() {
    const DEFAULT_MACOS_BUNDLE_ID: &str = "com.crypto-hud";

    println!("cargo:rerun-if-env-changed=CRYPTO_HUD_MACOS_BUNDLE_ID");
    let macos_bundle_id = std::env::var("CRYPTO_HUD_MACOS_BUNDLE_ID")
        .unwrap_or_else(|_| DEFAULT_MACOS_BUNDLE_ID.to_string());
    assert!(
        !macos_bundle_id.is_empty()
            && macos_bundle_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-')),
        "CRYPTO_HUD_MACOS_BUNDLE_ID must contain only ASCII letters, digits, dots, or hyphens"
    );
    println!("cargo:rustc-env=CRYPTO_HUD_MACOS_BUNDLE_ID={macos_bundle_id}");

    slint_build::compile("ui/price-card.slint").expect("failed to compile Slint UI");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let version = env!("CARGO_PKG_VERSION");
        let file_version = format!("{version}.0");
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon("ui/icon.ico")
            .set_manifest_file("ui/app.manifest")
            .set("CompanyName", "Crypto HUD Contributors")
            .set("ProductName", "Crypto HUD")
            .set("FileDescription", "Crypto HUD native desktop HUD")
            .set("InternalName", "crypto-hud")
            .set("OriginalFilename", "crypto-hud.exe")
            .set("FileVersion", &file_version)
            .set("ProductVersion", &file_version);
        resource
            .compile()
            .expect("failed to compile Windows resources");
    }
}
