fn main() {
    slint_build::compile("ui/price-card.slint").expect("failed to compile Slint UI");

    #[cfg(windows)]
    {
        let version = env!("CARGO_PKG_VERSION");
        let release_version = version
            .split_once('-')
            .map_or(version, |(release, _)| release);
        let file_version = format!("{release_version}.0");
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
