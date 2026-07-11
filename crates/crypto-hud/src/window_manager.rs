use std::{
    cell::RefCell,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use crypto_hud_shell_state as settings;
use settings::{AppSettings, DesktopWorkArea, LayoutStore, WidgetInstance};
use slint::{PhysicalPosition, Timer, TimerMode, WindowPosition};

use crate::{
    i18n, notifications, shortcuts, widget_host::request_widget_redraws,
    widget_host::WidgetRuntime, AppTray,
};

const DEFAULT_DESKTOP_WIDTH: i32 = 1920;
const DEFAULT_DESKTOP_HEIGHT: i32 = 1080;
const TRAY_HOVER_DISPLAY_POLL_INTERVAL: Duration = Duration::from_millis(100);
const WIDGET_SHELL_WINDOW_MAINTENANCE_INTERVAL: Duration = Duration::from_millis(250);
static SETTINGS_MODE_ACTIVE_FOR_WINDOW_CONFIGURATION: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TrayHoverDisplayState {
    active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrayHoverDisplayAction {
    None,
    ShowWidgets,
    HideWidgets,
}

pub(crate) fn show_widgets(
    widgets: &Rc<RefCell<Vec<WidgetRuntime>>>,
    layouts: &Rc<RefCell<LayoutStore>>,
    widgets_hidden: &Rc<RefCell<bool>>,
    settings_mode_active: &Rc<RefCell<bool>>,
) {
    let desktop_size = desktop_size();
    let work_areas = desktop_work_areas();
    let store = layouts.borrow();
    for (index, runtime) in widgets.borrow().iter().enumerate() {
        if let Some(instance) = store
            .widgets
            .iter()
            .find(|instance| instance.id == runtime.id)
        {
            if !instance.visible {
                if let Err(error) = runtime.ui.hide() {
                    eprintln!("failed to hide disabled widget {}: {error:#}", runtime.id);
                }
                continue;
            }
            let layout = settings::layout_for_instance_in_work_areas(
                instance,
                index,
                store.settings.clone().normalized(),
                &[],
                desktop_size,
                &work_areas,
            );
            runtime
                .ui
                .window()
                .set_position(WindowPosition::Physical(PhysicalPosition::new(
                    layout.x, layout.y,
                )));
            if let Err(error) = runtime.ui.show() {
                eprintln!("failed to show widget {}: {error:#}", runtime.id);
            }
        }
    }
    drop(store);
    apply_widget_pinning_for_settings_mode(widgets, layouts, *settings_mode_active.borrow());
    request_widget_redraws(widgets);
    *widgets_hidden.borrow_mut() = false;
    schedule_widget_shell_window_configuration();
}

pub(crate) fn hide_widgets(
    widgets: &Rc<RefCell<Vec<WidgetRuntime>>>,
    widgets_hidden: &Rc<RefCell<bool>>,
) {
    for (index, runtime) in widgets.borrow().iter().enumerate() {
        runtime
            .ui
            .window()
            .set_position(WindowPosition::Physical(PhysicalPosition::new(
                settings::PARKED_WIDGET_X - index as i32 * 8,
                settings::PARKED_WIDGET_Y - index as i32 * 8,
            )));
    }
    *widgets_hidden.borrow_mut() = true;
}

pub(crate) fn enter_settings_mode(
    widgets: &Rc<RefCell<Vec<WidgetRuntime>>>,
    layouts: &Rc<RefCell<LayoutStore>>,
    settings_mode_active: &Rc<RefCell<bool>>,
) {
    *settings_mode_active.borrow_mut() = true;
    SETTINGS_MODE_ACTIVE_FOR_WINDOW_CONFIGURATION.store(true, Ordering::Relaxed);
    apply_widget_pinning_for_settings_mode(widgets, layouts, true);
    request_widget_redraws(widgets);
    schedule_widget_shell_window_configuration();
}

pub(crate) fn leave_settings_mode(
    widgets: &Rc<RefCell<Vec<WidgetRuntime>>>,
    layouts: &Rc<RefCell<LayoutStore>>,
    settings_mode_active: &Rc<RefCell<bool>>,
) {
    *settings_mode_active.borrow_mut() = false;
    SETTINGS_MODE_ACTIVE_FOR_WINDOW_CONFIGURATION.store(false, Ordering::Relaxed);
    apply_widget_pinning_for_settings_mode(widgets, layouts, false);
    request_widget_redraws(widgets);
    schedule_widget_shell_window_configuration();
}

pub(crate) fn apply_widget_pinning_for_settings_mode(
    widgets: &Rc<RefCell<Vec<WidgetRuntime>>>,
    layouts: &Rc<RefCell<LayoutStore>>,
    settings_mode_active: bool,
) {
    let store = layouts.borrow();
    for runtime in widgets.borrow().iter() {
        if let Some(instance) = store
            .widgets
            .iter()
            .find(|instance| instance.id == runtime.id)
        {
            runtime
                .ui
                .set_pin_to_top(widget_pin_to_top(instance, settings_mode_active));
        }
    }
}

pub(crate) fn widget_pin_to_top(instance: &WidgetInstance, _settings_mode_active: bool) -> bool {
    instance.layout.always_on_top
}

pub(crate) fn install_tray_hover_display_timer(
    widgets: Rc<RefCell<Vec<WidgetRuntime>>>,
    layouts: Rc<RefCell<LayoutStore>>,
    widgets_hidden: Rc<RefCell<bool>>,
    settings_mode_active: Rc<RefCell<bool>>,
    tray_hover_state: Rc<RefCell<TrayHoverDisplayState>>,
) -> Timer {
    let timer = Timer::default();
    timer.start(
        TimerMode::Repeated,
        TRAY_HOVER_DISPLAY_POLL_INTERVAL,
        move || {
            apply_tray_hover_display(
                &widgets,
                &layouts,
                &widgets_hidden,
                &settings_mode_active,
                &tray_hover_state,
                notifications::tray_icon_hovered(),
            );
        },
    );
    timer
}

pub(crate) fn install_widget_shell_window_maintenance_timer() -> Timer {
    let timer = Timer::default();
    timer.start(
        TimerMode::Repeated,
        WIDGET_SHELL_WINDOW_MAINTENANCE_INTERVAL,
        maintain_shell_window_configuration,
    );
    timer
}

pub(crate) fn apply_tray_hover_display(
    widgets: &Rc<RefCell<Vec<WidgetRuntime>>>,
    layouts: &Rc<RefCell<LayoutStore>>,
    widgets_hidden: &Rc<RefCell<bool>>,
    settings_mode_active: &Rc<RefCell<bool>>,
    tray_hover_state: &Rc<RefCell<TrayHoverDisplayState>>,
    tray_hovered: bool,
) {
    let settings = layouts.borrow().settings.clone().normalized();
    let widgets_are_hidden = *widgets_hidden.borrow();
    let action = {
        let mut state = tray_hover_state.borrow_mut();
        tray_hover_display_action(
            &mut state,
            settings.tray_hover_display_enabled,
            tray_hovered,
            widgets_are_hidden,
        )
    };

    match action {
        TrayHoverDisplayAction::None => {}
        TrayHoverDisplayAction::ShowWidgets => {
            show_widgets(widgets, layouts, widgets_hidden, settings_mode_active);
        }
        TrayHoverDisplayAction::HideWidgets => {
            hide_widgets(widgets, widgets_hidden);
        }
    }
}

pub(crate) fn tray_hover_display_action(
    state: &mut TrayHoverDisplayState,
    enabled: bool,
    tray_hovered: bool,
    widgets_hidden: bool,
) -> TrayHoverDisplayAction {
    if !enabled {
        let action = if state.active && widgets_hidden {
            TrayHoverDisplayAction::ShowWidgets
        } else {
            TrayHoverDisplayAction::None
        };
        state.active = false;
        return action;
    }

    state.active = true;
    if tray_hovered {
        if widgets_hidden {
            TrayHoverDisplayAction::ShowWidgets
        } else {
            TrayHoverDisplayAction::None
        }
    } else if widgets_hidden {
        TrayHoverDisplayAction::None
    } else {
        TrayHoverDisplayAction::HideWidgets
    }
}

pub(crate) fn effective_tray_icon_enabled(settings: &AppSettings) -> bool {
    settings.tray_icon_enabled || settings.tray_hover_display_enabled
}

pub(crate) fn install_hotkey_poll_timer(
    shortcut_manager: Rc<RefCell<shortcuts::ShortcutManager>>,
    widgets: Rc<RefCell<Vec<WidgetRuntime>>>,
    layouts: Rc<RefCell<LayoutStore>>,
    widgets_hidden: Rc<RefCell<bool>>,
    settings_mode_active: Rc<RefCell<bool>>,
    tray: slint::Weak<AppTray>,
) -> Timer {
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_millis(100), move || {
        if shortcut_manager.borrow().poll_toggle_requested() {
            toggle_widgets_from_shortcut(
                &widgets,
                &layouts,
                &widgets_hidden,
                &settings_mode_active,
                &tray,
            );
        }
    });
    timer
}

fn toggle_widgets_from_shortcut(
    widgets: &Rc<RefCell<Vec<WidgetRuntime>>>,
    layouts: &Rc<RefCell<LayoutStore>>,
    widgets_hidden: &Rc<RefCell<bool>>,
    settings_mode_active: &Rc<RefCell<bool>>,
    tray: &slint::Weak<AppTray>,
) {
    let settings = layouts.borrow().settings.clone().normalized();
    if settings.tray_hover_display_enabled {
        if let Some(tray) = tray.upgrade() {
            tray.set_tray_visible(true);
            restore_native_tray_icon(&tray);
        }
        return;
    }

    if *widgets_hidden.borrow() {
        if let Some(tray) = tray.upgrade() {
            let tray_enabled = effective_tray_icon_enabled(&settings);
            tray.set_tray_visible(tray_enabled);
            if tray_enabled {
                restore_native_tray_icon(&tray);
            }
        }
        show_widgets(widgets, layouts, widgets_hidden, settings_mode_active);
    } else {
        hide_widgets(widgets, widgets_hidden);
        if let Some(tray) = tray.upgrade() {
            let tray_enabled = settings.tray_hover_display_enabled;
            tray.set_tray_visible(tray_enabled);
            if tray_enabled {
                restore_native_tray_icon(&tray);
            } else {
                remove_native_tray_icon();
            }
        }
    }
}

pub(crate) fn remove_native_tray_icon() {
    notifications::remove_tray_icon();
}

pub(crate) fn restore_native_tray_icon(tray: &AppTray) {
    notifications::restore_tray_icon(tray.get_tray_tooltip_text().as_str());
}

pub(crate) fn schedule_widget_shell_window_configuration() {
    maintain_shell_window_configuration();
    Timer::single_shot(
        Duration::from_millis(50),
        maintain_shell_window_configuration,
    );
    Timer::single_shot(
        Duration::from_millis(250),
        maintain_shell_window_configuration,
    );
}

pub(crate) fn schedule_settings_window_raise() {
    raise_settings_window();
    Timer::single_shot(Duration::from_millis(50), raise_settings_window);
    Timer::single_shot(Duration::from_millis(250), raise_settings_window);
}

fn maintain_shell_window_configuration() {
    configure_windows_for_widget_shell();
    raise_settings_window();
}

#[cfg(windows)]
fn raise_settings_window() {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowLongPtrW, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
        SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, HWND_TOPMOST, SWP_FRAMECHANGED,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW,
    };

    unsafe extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> i32 {
        let target_pid = lparam as u32;
        let mut pid = 0_u32;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut pid);
        }
        if pid != target_pid {
            return 1;
        }

        let title = read_window_title(hwnd);
        if is_settings_window_title(&title) {
            if unsafe { IsWindowVisible(hwnd) } == 0 || unsafe { IsIconic(hwnd) } != 0 {
                return 0;
            }
            unsafe {
                let style_changed = ensure_taskbar_window_style(hwnd);
                let current_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
                let should_raise =
                    !is_topmost_style(current_style) || visible_topmost_window_above(hwnd, false);
                if should_raise {
                    let mut flags = SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW;
                    if style_changed {
                        flags |= SWP_FRAMECHANGED;
                    }
                    SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, flags);
                }
            }
            return 0;
        }

        1
    }

    unsafe fn ensure_taskbar_window_style(hwnd: HWND) -> bool {
        let style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 };
        let taskbar_style = settings_taskbar_ex_style(style);
        if style == taskbar_style {
            return false;
        }
        unsafe {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, taskbar_style as isize);
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
        true
    }

    unsafe {
        EnumWindows(Some(enum_window), std::process::id() as LPARAM);
    }
}

#[cfg(not(windows))]
fn raise_settings_window() {}

pub(crate) fn desktop_size() -> (i32, i32) {
    platform_desktop_size()
}

pub(crate) fn desktop_work_areas() -> Vec<DesktopWorkArea> {
    let work_areas = platform_desktop_work_areas();
    if work_areas.is_empty() {
        vec![DesktopWorkArea::from_desktop_size(desktop_size())]
    } else {
        work_areas
    }
}

#[cfg(windows)]
fn platform_desktop_size() -> (i32, i32) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

    let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    if width > 0 && height > 0 {
        (width, height)
    } else {
        (DEFAULT_DESKTOP_WIDTH, DEFAULT_DESKTOP_HEIGHT)
    }
}

#[cfg(windows)]
fn platform_desktop_work_areas() -> Vec<DesktopWorkArea> {
    use windows_sys::Win32::Foundation::{LPARAM, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
    };

    unsafe extern "system" fn enum_monitor(
        monitor: HMONITOR,
        _device_context: HDC,
        _monitor_rect: *mut RECT,
        data: LPARAM,
    ) -> i32 {
        let work_areas = unsafe { &mut *(data as *mut Vec<DesktopWorkArea>) };
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..MONITORINFO::default()
        };
        if unsafe { GetMonitorInfoW(monitor, &mut info) } != 0 {
            let rect = info.rcWork;
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
            if width > 0 && height > 0 {
                work_areas.push(DesktopWorkArea {
                    x: rect.left,
                    y: rect.top,
                    width,
                    height,
                });
            }
        }
        1
    }

    let mut work_areas: Vec<DesktopWorkArea> = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(enum_monitor),
            (&mut work_areas as *mut Vec<DesktopWorkArea>) as LPARAM,
        );
    }
    work_areas.sort_by_key(|area| (area.x, area.y, area.width, area.height));
    work_areas.dedup();
    work_areas
}

#[cfg(not(windows))]
fn platform_desktop_size() -> (i32, i32) {
    (DEFAULT_DESKTOP_WIDTH, DEFAULT_DESKTOP_HEIGHT)
}

#[cfg(not(windows))]
fn platform_desktop_work_areas() -> Vec<DesktopWorkArea> {
    Vec::new()
}

#[cfg(windows)]
fn configure_windows_for_widget_shell() {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowLongPtrW, GetWindowThreadProcessId, IsIconic, SetWindowLongPtrW,
        SetWindowPos, ShowWindow, GWL_EXSTYLE, HWND_TOPMOST, SWP_FRAMECHANGED, SWP_NOACTIVATE,
        SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_HIDE, SW_SHOWNOACTIVATE,
    };

    unsafe extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> i32 {
        let target_pid = lparam as u32;
        let mut pid = 0_u32;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut pid);
        }
        if pid != target_pid {
            return 1;
        }

        let class_name = read_window_class(hwnd);
        if class_name == "Winit Thread Event Target" {
            unsafe {
                ShowWindow(hwnd, SW_HIDE);
            }
            return 1;
        }

        let title = read_window_title(hwnd);
        if is_widget_shell_window_title(&title) {
            unsafe {
                let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
                let settings_mode_active =
                    SETTINGS_MODE_ACTIVE_FOR_WINDOW_CONFIGURATION.load(Ordering::Relaxed);
                let widget_style = widget_shell_ex_style(style, settings_mode_active);
                if IsIconic(hwnd) != 0 {
                    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                }
                let style_changed = style != widget_style;
                if style_changed {
                    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, widget_style as isize);
                }
                let topmost_window_above =
                    !settings_mode_active && visible_topmost_window_above(hwnd, true);
                let should_change_z_order = widget_shell_should_change_z_order(
                    widget_style,
                    settings_mode_active,
                    topmost_window_above,
                );
                if style_changed || should_change_z_order {
                    let mut flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE;
                    if style_changed {
                        flags |= SWP_FRAMECHANGED;
                    }
                    if !should_change_z_order {
                        flags |= SWP_NOZORDER;
                    }
                    SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, flags);
                }
            }
        }

        1
    }

    unsafe {
        EnumWindows(Some(enum_window), std::process::id() as LPARAM);
    }
}

#[cfg(not(windows))]
fn configure_windows_for_widget_shell() {}

#[cfg(windows)]
fn is_widget_shell_window_title(title: &str) -> bool {
    title.starts_with("price-card-")
        || title.starts_with("quote-board-")
        || title.starts_with("mini-ticker-")
        || title.starts_with("plugin-")
        || title == "crypto-hud-keepalive"
}

#[cfg(windows)]
fn is_settings_window_title(title: &str) -> bool {
    let normalized_title = i18n::strip_bidi_isolate_marks(title);
    matches!(
        normalized_title.as_str(),
        "Crypto HUD Settings" | "Crypto HUD 设置"
    ) || i18n::Locale::ALL.iter().any(|locale| {
        i18n::strip_bidi_isolate_marks(i18n::text(*locale).settings_title) == normalized_title
    })
}

#[cfg(windows)]
fn widget_shell_ex_style(style: u32, _settings_mode_active: bool) -> u32 {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };

    (style & !WS_EX_APPWINDOW) | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE
}

#[cfg(windows)]
fn settings_taskbar_ex_style(style: u32) -> u32 {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };

    (style & !(WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE)) | WS_EX_APPWINDOW
}

#[cfg(windows)]
fn is_topmost_style(style: u32) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::WS_EX_TOPMOST;

    style & WS_EX_TOPMOST != 0
}

#[cfg(windows)]
fn widget_shell_should_change_z_order(
    widget_style: u32,
    settings_mode_active: bool,
    visible_topmost_above: bool,
) -> bool {
    if settings_mode_active {
        return false;
    }

    !is_topmost_style(widget_style) || visible_topmost_above
}

#[cfg(windows)]
fn visible_topmost_window_above(
    hwnd: windows_sys::Win32::Foundation::HWND,
    ignore_own_shell: bool,
) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetTopWindow, GetWindow, GetWindowLongPtrW, GetWindowThreadProcessId, IsWindowVisible,
        GWL_EXSTYLE, GW_HWNDNEXT,
    };

    let mut current = unsafe { GetTopWindow(std::ptr::null_mut()) };
    while !current.is_null() && current != hwnd {
        let title = read_window_title(current);
        let mut pid = 0_u32;
        unsafe {
            GetWindowThreadProcessId(current, &mut pid);
        }
        if !title.trim().is_empty()
            && unsafe { IsWindowVisible(current) } != 0
            && (!ignore_own_shell || pid != std::process::id() || !is_shell_window_title(&title))
        {
            let style = unsafe { GetWindowLongPtrW(current, GWL_EXSTYLE) as u32 };
            if is_topmost_style(style) {
                return true;
            }
        }
        current = unsafe { GetWindow(current, GW_HWNDNEXT) };
    }

    false
}

#[cfg(windows)]
fn is_shell_window_title(title: &str) -> bool {
    is_settings_window_title(title) || is_widget_shell_window_title(title)
}

#[cfg(windows)]
fn read_window_class(hwnd: windows_sys::Win32::Foundation::HWND) -> String {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetClassNameW;

    let mut class_name = [0_u16; 256];
    let len = unsafe { GetClassNameW(hwnd, class_name.as_mut_ptr(), class_name.len() as i32) };
    if len <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&class_name[..len as usize])
}

#[cfg(windows)]
fn read_window_title(hwnd: windows_sys::Win32::Foundation::HWND) -> String {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowTextW;

    let mut title = [0_u16; 256];
    let len = unsafe { GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32) };
    if len <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&title[..len as usize])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_hover_display_hides_when_enabled_and_not_hovered() {
        let mut state = TrayHoverDisplayState::default();

        assert_eq!(
            tray_hover_display_action(&mut state, true, false, false),
            TrayHoverDisplayAction::HideWidgets
        );
        assert!(state.active);
    }

    #[test]
    fn tray_hover_display_shows_only_while_hovered() {
        let mut state = TrayHoverDisplayState::default();

        assert_eq!(
            tray_hover_display_action(&mut state, true, false, false),
            TrayHoverDisplayAction::HideWidgets
        );
        assert_eq!(
            tray_hover_display_action(&mut state, true, true, true),
            TrayHoverDisplayAction::ShowWidgets
        );
        assert_eq!(
            tray_hover_display_action(&mut state, true, true, false),
            TrayHoverDisplayAction::None
        );
        assert_eq!(
            tray_hover_display_action(&mut state, true, false, false),
            TrayHoverDisplayAction::HideWidgets
        );
    }

    #[test]
    fn tray_hover_display_restores_widgets_when_disabled_after_hiding() {
        let mut state = TrayHoverDisplayState::default();

        assert_eq!(
            tray_hover_display_action(&mut state, true, false, false),
            TrayHoverDisplayAction::HideWidgets
        );
        assert_eq!(
            tray_hover_display_action(&mut state, false, false, true),
            TrayHoverDisplayAction::ShowWidgets
        );
        assert!(!state.active);
    }

    #[test]
    fn tray_hover_display_forces_tray_icon_available() {
        let settings = AppSettings {
            tray_icon_enabled: false,
            tray_hover_display_enabled: true,
            ..AppSettings::default()
        };

        assert!(effective_tray_icon_enabled(&settings));
    }

    #[cfg(windows)]
    #[test]
    fn settings_window_is_not_treated_as_widget_shell_window() {
        assert!(is_settings_window_title("Crypto HUD Settings"));
        assert!(is_settings_window_title("Crypto HUD 设置"));
        for locale in i18n::Locale::ALL {
            assert!(
                is_settings_window_title(i18n::text(locale).settings_title),
                "{locale:?} settings title should be recognized"
            );
        }
        assert!(!is_widget_shell_window_title("Crypto HUD"));
        assert!(!is_widget_shell_window_title("\u{2066}Crypto HUD\u{2069}"));
        assert!(!is_widget_shell_window_title("Crypto HUD Settings"));
        assert!(!is_widget_shell_window_title("Crypto HUD 设置"));
        assert!(is_widget_shell_window_title("price-card-1"));
        assert!(is_widget_shell_window_title("quote-board-1"));
        assert!(is_widget_shell_window_title("crypto-hud-keepalive"));
    }

    #[cfg(windows)]
    #[test]
    fn widget_shell_style_is_non_activating_tool_window() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
        };

        let style = widget_shell_ex_style(WS_EX_APPWINDOW, false);
        assert_eq!(style & WS_EX_APPWINDOW, 0);
        assert_ne!(style & WS_EX_TOOLWINDOW, 0);
        assert_ne!(style & WS_EX_NOACTIVATE, 0);
    }

    #[cfg(windows)]
    #[test]
    fn widget_shell_style_preserves_topmost_during_settings_mode() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{WS_EX_APPWINDOW, WS_EX_TOPMOST};

        let normal_style = widget_shell_ex_style(WS_EX_APPWINDOW | WS_EX_TOPMOST, false);
        assert_ne!(normal_style & WS_EX_TOPMOST, 0);

        let settings_mode_style = widget_shell_ex_style(WS_EX_APPWINDOW | WS_EX_TOPMOST, true);
        assert_ne!(settings_mode_style & WS_EX_TOPMOST, 0);
    }

    #[cfg(windows)]
    #[test]
    fn widget_shell_z_order_is_not_changed_during_settings_mode() {
        use windows_sys::Win32::UI::WindowsAndMessaging::WS_EX_TOPMOST;

        assert!(!widget_shell_should_change_z_order(
            WS_EX_TOPMOST,
            true,
            true
        ));
        assert!(widget_shell_should_change_z_order(0, false, false));
        assert!(widget_shell_should_change_z_order(
            WS_EX_TOPMOST,
            false,
            true
        ));
        assert!(!widget_shell_should_change_z_order(
            WS_EX_TOPMOST,
            false,
            false
        ));
    }

    #[cfg(windows)]
    #[test]
    fn settings_window_style_uses_taskbar_app_window() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
        };

        let style = settings_taskbar_ex_style(WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE);
        assert_ne!(style & WS_EX_APPWINDOW, 0);
        assert_eq!(style & WS_EX_TOOLWINDOW, 0);
        assert_eq!(style & WS_EX_NOACTIVATE, 0);
    }
}
