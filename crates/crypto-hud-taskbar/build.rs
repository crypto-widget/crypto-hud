fn main() {
    println!("cargo:rerun-if-changed=native/taskbar_bridge.cpp");
    println!("cargo:rerun-if-changed=native/taskbar_protocol.h");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    assert_eq!(
        target_env, "msvc",
        "crypto-hud-taskbar currently supports only the MSVC Windows target"
    );

    let mut native = cc::Build::new();
    native
        .cpp(true)
        // Explorer must be able to load the companion on a clean Windows
        // installation without a separately installed VC++ Redistributable.
        .static_crt(true)
        .file("native/taskbar_bridge.cpp")
        .define("UNICODE", None)
        .define("_UNICODE", None)
        .define("NOMINMAX", None)
        .define("WIN32_LEAN_AND_MEAN", None)
        .define("WINVER", "0x0A00")
        .define("_WIN32_WINNT", "0x0A00")
        .flag_if_supported("/std:c++20")
        .flag_if_supported("/EHsc")
        .flag_if_supported("/permissive-")
        .flag_if_supported("/Zc:__cplusplus")
        .warnings(true);
    if std::env::var("PROFILE").as_deref() == Ok("debug") {
        native.define("CRYPTO_HUD_TASKBAR_DEBUG", None);
    }
    native.compile("crypto_hud_taskbar_native");

    for library in ["ole32", "oleaut32", "runtimeobject", "user32", "windowsapp"] {
        println!("cargo:rustc-link-lib={library}");
    }
}
