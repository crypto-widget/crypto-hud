pub fn apply_auto_start(enabled: bool, widget_count: usize) -> Result<(), String> {
    let auto_launch = build_auto_launch(widget_count)?;
    let legacy_auto_launch = build_auto_launch_with_name("Crypto Widget Slint", widget_count)?;
    if enabled {
        let _ = legacy_auto_launch.disable();
        auto_launch.enable().map_err(|error| error.to_string())
    } else {
        auto_launch
            .disable()
            .and_then(|_| legacy_auto_launch.disable())
            .map_err(|error| error.to_string())
    }
}

fn build_auto_launch(widget_count: usize) -> Result<auto_launch::AutoLaunch, String> {
    build_auto_launch_with_name("Crypto HUD", widget_count)
}

#[cfg(windows)]
fn build_auto_launch_with_name(
    app_name: &str,
    widget_count: usize,
) -> Result<auto_launch::AutoLaunch, String> {
    use auto_launch::{AutoLaunch, WindowsEnableMode};

    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let exe = exe
        .to_str()
        .ok_or_else(|| "current exe path is not valid UTF-8".to_string())?;
    let widget_count = widget_count.to_string();
    let args = ["--widgets", widget_count.as_str()];

    Ok(AutoLaunch::new(
        app_name,
        exe,
        WindowsEnableMode::CurrentUser,
        &args,
    ))
}

#[cfg(not(windows))]
fn build_auto_launch_with_name(
    app_name: &str,
    widget_count: usize,
) -> Result<auto_launch::AutoLaunch, String> {
    let widget_count = widget_count.to_string();
    auto_launch::AutoLaunchBuilder::new()
        .set_app_name(app_name)
        .set_app_path(
            std::env::current_exe()
                .map_err(|error| error.to_string())?
                .to_string_lossy()
                .as_ref(),
        )
        .set_args(&["--widgets", widget_count.as_str()])
        .build()
        .map_err(|error| error.to_string())
}
