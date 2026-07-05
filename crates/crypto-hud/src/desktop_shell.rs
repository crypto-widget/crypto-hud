use std::{cell::RefCell, env, fs, path::PathBuf, rc::Rc, time::Duration};

use anyhow::{Context, Result};
use crypto_hud_shell_state as settings;
use settings::{AppSettings, LayoutStore};
use single_instance::SingleInstance;
use slint::{ComponentHandle, PhysicalPosition, Timer, TimerMode, WindowPosition};

use crate::{
    i18n, plugin,
    settings_window::{refresh_settings_window, request_symbol_catalog_refresh_from_store},
    widget_host::WidgetRuntime,
    window_manager::{
        effective_tray_icon_enabled, enter_settings_mode, remove_native_tray_icon,
        restore_native_tray_icon, schedule_settings_window_raise,
        schedule_widget_shell_window_configuration, show_widgets,
    },
    AppTray, KeepAliveWindow, SettingsWindow,
};

const DEFAULT_WIDGET_COUNT: usize = 1;
const DEFAULT_SINGLE_INSTANCE_ID: &str = "com.crypto-hud";

#[derive(Debug, Clone)]
pub(crate) struct LaunchOptions {
    pub(crate) widget_count: usize,
    pub(crate) each_widget: bool,
    pub(crate) show_settings: bool,
    pub(crate) gui_smoke_exit_after: Option<Duration>,
}

pub(crate) fn install_single_instance_guard() -> Result<SingleInstance> {
    let instance_id = env_var_with_legacy(
        "CRYPTO_HUD_INSTANCE_ID",
        &["CRYPTO_WIDGET_SLINT_INSTANCE_ID"],
    )
    .unwrap_or_else(|_| DEFAULT_SINGLE_INSTANCE_ID.to_string());
    SingleInstance::new(&instance_id).context("failed to create single-instance guard")
}

pub(crate) fn parse_launch_options() -> LaunchOptions {
    let mut options = LaunchOptions {
        widget_count: DEFAULT_WIDGET_COUNT,
        each_widget: false,
        show_settings: false,
        gui_smoke_exit_after: None,
    };
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--widgets" => {
                if let Some(value) = args.next().and_then(|value| value.parse::<usize>().ok()) {
                    options.widget_count = value.max(1);
                    options.each_widget = false;
                }
            }
            "--each-widget" => {
                options.each_widget = true;
                options.widget_count = 0;
            }
            "--show-settings" => {
                options.show_settings = true;
            }
            "--gui-smoke-ms" => {
                if let Some(value) = args.next().and_then(|value| value.parse::<u64>().ok()) {
                    options.gui_smoke_exit_after = Some(Duration::from_millis(value.max(100)));
                }
            }
            _ => {}
        }
    }

    options
}

pub(crate) fn install_gui_smoke_timer(exit_after: Option<Duration>) -> Option<Timer> {
    let exit_after = exit_after?;
    let timer = Timer::default();
    timer.start(TimerMode::SingleShot, exit_after, move || {
        if let Err(error) = slint::quit_event_loop() {
            eprintln!("failed to quit Slint event loop from GUI smoke timer: {error:#}");
        }
    });
    Some(timer)
}

pub(crate) fn write_gui_smoke_ready_file(
    widgets: &Rc<RefCell<Vec<WidgetRuntime>>>,
    layouts: &Rc<RefCell<LayoutStore>>,
    settings_window_requested: bool,
) {
    let Some(path) = env_os_with_legacy(
        "CRYPTO_HUD_GUI_SMOKE_READY_FILE",
        &["CRYPTO_WIDGET_SLINT_GUI_SMOKE_READY_FILE"],
    ) else {
        return;
    };
    let widgets_ref = widgets.borrow();
    let store = layouts.borrow();
    let widget_count = widgets_ref.len();
    let widget_states = widgets_ref
        .iter()
        .filter_map(|runtime| {
            let instance = store
                .widgets
                .iter()
                .find(|instance| instance.id == runtime.id)?;
            let runtime_size = runtime.ui.window().size();
            Some(serde_json::json!({
                "id": runtime.id,
                "pluginId": runtime.plugin_id,
                "locked": instance.layout.locked,
                "layoutWidth": instance.layout.width,
                "layoutHeight": instance.layout.height,
                "scalePercent": instance.layout.scale_percent,
                "runtimeWidth": runtime_size.width,
                "runtimeHeight": runtime_size.height,
                "symbolCount": runtime.symbols.len(),
                "widgetScale": runtime.widget_scale,
            }))
        })
        .collect::<Vec<_>>();
    let marker = serde_json::json!({
        "ready": true,
        "widgetCount": widget_count,
        "widgets": widget_states,
        "settingsWindowRequested": settings_window_requested,
    });
    let path = PathBuf::from(path);
    if let Some(parent) = path.parent() {
        if let Err(error) = fs::create_dir_all(parent) {
            eprintln!("failed to create GUI smoke marker directory: {error:#}");
            return;
        }
    }
    if let Err(error) = fs::write(&path, format!("{marker}\n")) {
        eprintln!(
            "failed to write GUI smoke marker {}: {error:#}",
            path.display()
        );
    }
}

fn env_var_with_legacy(
    primary: &str,
    legacy: &[&str],
) -> std::result::Result<String, env::VarError> {
    env::var(primary).or_else(|primary_error| {
        legacy
            .iter()
            .find_map(|name| env::var(name).ok())
            .ok_or(primary_error)
    })
}

fn env_os_with_legacy(primary: &str, legacy: &[&str]) -> Option<std::ffi::OsString> {
    env::var_os(primary).or_else(|| legacy.iter().find_map(env::var_os))
}

pub(crate) fn install_settings_drag_handler(ui: &SettingsWindow) {
    let weak = ui.as_weak();
    ui.on_drag_settings(move |dx, dy| {
        let Some(ui) = weak.upgrade() else {
            return;
        };

        let scale = ui.window().scale_factor();
        let current = ui.window().position();
        let x = current.x + (dx * scale).round() as i32;
        let y = current.y + (dy * scale).round() as i32;
        ui.window()
            .set_position(WindowPosition::Physical(PhysicalPosition::new(x, y)));
    });
}

#[cfg(windows)]
pub(crate) fn open_external_url(url: &str) -> Result<()> {
    use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};

    let operation = wide_null_str("open");
    let target = wide_null_str(url);
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };

    if result as isize <= 32 {
        anyhow::bail!("ShellExecuteW failed with code {}", result as isize);
    }

    Ok(())
}

#[cfg(not(windows))]
pub(crate) fn open_external_url(url: &str) -> Result<()> {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(opener)
        .arg(url)
        .spawn()
        .with_context(|| format!("failed to open {url} with {opener}"))?;
    Ok(())
}

#[cfg(windows)]
fn wide_null_str(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

pub(crate) fn install_tray(
    widgets: Rc<RefCell<Vec<WidgetRuntime>>>,
    settings_window: slint::Weak<SettingsWindow>,
    layouts: Rc<RefCell<LayoutStore>>,
    state_path: PathBuf,
    widgets_hidden: Rc<RefCell<bool>>,
    settings_mode_active: Rc<RefCell<bool>>,
    plugin_catalog: Rc<plugin::PluginCatalog>,
) -> Result<AppTray> {
    let tray = AppTray::new().context("failed to create Slint tray icon")?;

    tray.on_open_settings({
        let settings_window = settings_window.clone();
        let widgets = widgets.clone();
        let layouts = layouts.clone();
        let state_path = state_path.clone();
        let settings_mode_active = settings_mode_active.clone();
        let plugin_catalog = plugin_catalog.clone();
        move || {
            if let Some(ui) = settings_window.upgrade() {
                refresh_settings_window(&ui, &layouts, &state_path, &plugin_catalog, None);
                request_symbol_catalog_refresh_from_store(&ui, &layouts.borrow(), &plugin_catalog);
                enter_settings_mode(&widgets, &layouts, &settings_mode_active);
                if let Err(error) = ui.show() {
                    eprintln!("failed to show settings window from tray: {error:#}");
                }
                schedule_settings_window_raise();
                schedule_widget_shell_window_configuration();
            }
        }
    });

    tray.on_show_widgets({
        let widgets = widgets.clone();
        let layouts = layouts.clone();
        let widgets_hidden = widgets_hidden.clone();
        let settings_mode_active = settings_mode_active.clone();
        move || {
            show_widgets(&widgets, &layouts, &widgets_hidden, &settings_mode_active);
        }
    });

    tray.on_quit(|| {
        if let Err(error) = slint::quit_event_loop() {
            eprintln!("failed to quit Slint event loop: {error:#}");
        }
    });

    refresh_tray_text(&tray, layouts.borrow().settings.clone().normalized());
    tray.show().context("failed to show Slint tray icon")?;
    Ok(tray)
}

pub(crate) fn install_keepalive_window() -> Result<KeepAliveWindow> {
    let ui = KeepAliveWindow::new().context("failed to create Slint keepalive window")?;
    ui.window()
        .set_position(WindowPosition::Physical(PhysicalPosition::new(
            settings::PARKED_WIDGET_X,
            settings::PARKED_WIDGET_Y,
        )));
    ui.show().context("failed to show Slint keepalive window")?;
    schedule_widget_shell_window_configuration();
    Ok(ui)
}

pub(crate) fn refresh_tray_text(tray: &AppTray, settings: AppSettings) {
    let text = i18n::text(i18n::resolve_locale(settings.language));
    tray.set_tray_tooltip_text(text.tray_tooltip.into());
    tray.set_tray_settings_text(text.tray_settings.into());
    tray.set_tray_quit_text(text.tray_quit.into());
    let tray_enabled = effective_tray_icon_enabled(&settings);
    tray.set_tray_visible(tray_enabled);
    if tray_enabled {
        restore_native_tray_icon(tray);
    } else {
        remove_native_tray_icon();
    }
}
