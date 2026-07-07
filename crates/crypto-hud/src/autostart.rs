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

#[cfg(windows)]
pub fn refresh_auto_start_registration_if_enabled(
    enabled: bool,
    widget_count: usize,
) -> Result<(), String> {
    if !enabled {
        return Ok(());
    }

    let auto_launch = build_auto_launch(widget_count)?;
    if !auto_launch
        .is_enabled()
        .map_err(|error| error.to_string())?
    {
        return Ok(());
    }

    auto_launch.enable().map_err(|error| error.to_string())
}

fn build_auto_launch(widget_count: usize) -> Result<auto_launch::AutoLaunch, String> {
    build_auto_launch_with_name("Crypto HUD", widget_count)
}

#[cfg(windows)]
fn build_auto_launch_with_name(
    app_name: &str,
    widget_count: usize,
) -> Result<auto_launch::AutoLaunch, String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let exe = exe
        .to_str()
        .ok_or_else(|| "current exe path is not valid UTF-8".to_string())?;
    Ok(build_windows_auto_launch_with_path(
        app_name,
        exe,
        widget_count,
    ))
}

#[cfg(windows)]
fn build_windows_auto_launch_with_path(
    app_name: &str,
    exe: &str,
    widget_count: usize,
) -> auto_launch::AutoLaunch {
    use auto_launch::{AutoLaunch, WindowsEnableMode};

    let exe = quote_windows_auto_launch_path(exe);
    let widget_count = widget_count.to_string();
    let args = ["--widgets", widget_count.as_str()];

    AutoLaunch::new(app_name, &exe, WindowsEnableMode::CurrentUser, &args)
}

#[cfg(windows)]
fn quote_windows_auto_launch_path(exe: &str) -> String {
    format!("\"{exe}\"")
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

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    use super::*;

    #[test]
    #[cfg(windows)]
    fn windows_auto_launch_quotes_paths_with_spaces() {
        let launch = build_windows_auto_launch_with_path(
            "Crypto HUD",
            r"C:\Users\Ada Lovelace\AppData\Local\CryptoHud\crypto-hud.exe",
            3,
        );

        assert_eq!(
            launch.get_app_path(),
            r#""C:\Users\Ada Lovelace\AppData\Local\CryptoHud\crypto-hud.exe""#
        );
        assert_eq!(
            launch.get_args(),
            &["--widgets".to_string(), "3".to_string()]
        );
        assert_eq!(
            format!("{} {}", launch.get_app_path(), launch.get_args().join(" ")),
            r#""C:\Users\Ada Lovelace\AppData\Local\CryptoHud\crypto-hud.exe" --widgets 3"#
        );
    }

    #[test]
    #[cfg(windows)]
    fn windows_auto_launch_quotes_paths_without_spaces_too() {
        assert_eq!(
            quote_windows_auto_launch_path(r"C:\Dev\crypto-hud\target\debug\crypto-hud.exe"),
            r#""C:\Dev\crypto-hud\target\debug\crypto-hud.exe""#
        );
    }
}
