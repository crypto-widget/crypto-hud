use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

#[derive(Debug, Clone)]
pub struct NotificationThrottle {
    cooldown: Duration,
    records: HashMap<String, NotificationRecord>,
}

#[derive(Debug, Clone)]
struct NotificationRecord {
    body: String,
    shown_at: Instant,
}

impl NotificationThrottle {
    pub fn new(cooldown: Duration) -> Self {
        Self {
            cooldown,
            records: HashMap::new(),
        }
    }

    pub fn should_notify(&mut self, key: &str, body: &str, now: Instant) -> bool {
        if let Some(record) = self.records.get(key) {
            if record.body == body && now.saturating_duration_since(record.shown_at) < self.cooldown
            {
                return false;
            }
        }

        self.records.insert(
            key.to_string(),
            NotificationRecord {
                body: body.to_string(),
                shown_at: now,
            },
        );
        true
    }
}

#[cfg(any(windows, test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrayMenuToggleAction {
    Open,
    Close,
    Ignore,
}

#[cfg(any(windows, test))]
#[derive(Debug, Default)]
struct TrayMenuToggleState {
    menu_active: bool,
    suppress_reopen_until_ms: Option<u64>,
}

#[cfg(any(windows, test))]
impl TrayMenuToggleState {
    fn entered_menu_loop(&mut self) {
        self.menu_active = true;
        self.suppress_reopen_until_ms = None;
    }

    fn exited_menu_loop(&mut self, now_ms: u64, suppression_ms: u64, suppress_reopen: bool) {
        self.menu_active = false;
        self.suppress_reopen_until_ms =
            suppress_reopen.then(|| now_ms.saturating_add(suppression_ms));
    }

    fn toggle_requested(&mut self, now_ms: u64) -> TrayMenuToggleAction {
        if self.menu_active {
            return TrayMenuToggleAction::Close;
        }

        if let Some(deadline_ms) = self.suppress_reopen_until_ms.take() {
            if now_ms <= deadline_ms {
                return TrayMenuToggleAction::Ignore;
            }
        }

        TrayMenuToggleAction::Open
    }
}

#[cfg(windows)]
pub fn install_tray_menu_toggle() -> bool {
    windows::install_tray_menu_toggle()
}

#[cfg(not(windows))]
pub fn install_tray_menu_toggle() -> bool {
    true
}

#[cfg(windows)]
pub fn restore_tray_icon(tooltip: &str) {
    windows::restore_tray_icon(tooltip);
}

#[cfg(not(windows))]
pub fn restore_tray_icon(_tooltip: &str) {}

#[cfg(windows)]
pub fn remove_tray_icon() {
    windows::remove_tray_icon();
}

#[cfg(not(windows))]
pub fn remove_tray_icon() {}

#[cfg(windows)]
pub fn tray_icon_hovered() -> bool {
    windows::tray_icon_hovered()
}

#[cfg(not(windows))]
pub fn tray_icon_hovered() -> bool {
    false
}

#[cfg(windows)]
pub fn show(title: &str, body: &str) {
    windows::show(title, body);
}

#[cfg(not(windows))]
pub fn show(_title: &str, _body: &str) {}

#[cfg(windows)]
mod windows {
    use std::{cell::RefCell, ffi::OsStr, os::windows::ffi::OsStrExt};

    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
        System::SystemInformation::GetTickCount64,
        UI::{
            Input::KeyboardAndMouse::{GetAsyncKeyState, GetDoubleClickTime, VK_RBUTTON},
            Shell::{
                DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass, Shell_NotifyIconGetRect,
                Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_INFO, NIM_ADD,
                NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW, NOTIFYICONIDENTIFIER,
            },
            WindowsAndMessaging::{
                ChangeWindowMessageFilterEx, DestroyIcon, EndMenu, FindWindowExW, GetCursorPos,
                GetWindowThreadProcessId, LoadImageW, HICON, HWND_MESSAGE, IMAGE_ICON,
                LR_LOADFROMFILE, MSGFLT_ALLOW, WM_APP, WM_CONTEXTMENU, WM_ENTERMENULOOP,
                WM_EXITMENULOOP, WM_NCDESTROY, WM_RBUTTONUP,
            },
        },
    };

    use super::{TrayMenuToggleAction, TrayMenuToggleState};

    const SLINT_TRAY_CLASS: &str = "SlintSystemTrayWindow";
    const TRAY_MENU_SUBCLASS_ID: usize = 0x4352_5950;
    const TRAY_UID: u32 = 1;
    const WM_TRAYICON: u32 = WM_APP + 1;

    thread_local! {
        static TRAY_MENU_TOGGLE_STATE: RefCell<TrayMenuToggleState> =
            RefCell::new(TrayMenuToggleState::default());
    }

    pub fn install_tray_menu_toggle() -> bool {
        let Some(hwnd) = find_slint_tray_window() else {
            return false;
        };
        allow_taskbar_context_menu_message(hwnd);
        (unsafe {
            SetWindowSubclass(
                hwnd,
                Some(tray_menu_subclass_proc),
                TRAY_MENU_SUBCLASS_ID,
                0,
            )
        }) != 0
    }

    unsafe extern "system" fn tray_menu_subclass_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        subclass_id: usize,
        _refdata: usize,
    ) -> LRESULT {
        if message == WM_ENTERMENULOOP && wparam != 0 {
            TRAY_MENU_TOGGLE_STATE.with(|state| state.borrow_mut().entered_menu_loop());
        } else if message == WM_EXITMENULOOP && wparam != 0 {
            let now_ms = unsafe { GetTickCount64() };
            let suppression_ms = u64::from(unsafe { GetDoubleClickTime() });
            let right_button_down = unsafe { GetAsyncKeyState(VK_RBUTTON.into()) } < 0;
            TRAY_MENU_TOGGLE_STATE.with(|state| {
                state
                    .borrow_mut()
                    .exited_menu_loop(now_ms, suppression_ms, right_button_down)
            });
        }

        let tray_callback = message == WM_TRAYICON
            && wparam == TRAY_UID as usize
            && matches!((lparam as u32) & 0xFFFF, WM_CONTEXTMENU | WM_RBUTTONUP);
        if tray_callback {
            let action = TRAY_MENU_TOGGLE_STATE.with(|state| {
                state
                    .borrow_mut()
                    .toggle_requested(unsafe { GetTickCount64() })
            });
            match action {
                TrayMenuToggleAction::Close => {
                    unsafe {
                        EndMenu();
                    }
                    return 0;
                }
                TrayMenuToggleAction::Ignore => return 0,
                TrayMenuToggleAction::Open => {}
            }
        }

        if message == WM_NCDESTROY {
            TRAY_MENU_TOGGLE_STATE
                .with(|state| *state.borrow_mut() = TrayMenuToggleState::default());
            unsafe {
                RemoveWindowSubclass(hwnd, Some(tray_menu_subclass_proc), subclass_id);
            }
        }
        unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
    }

    pub fn remove_tray_icon() {
        let Some(hwnd) = find_slint_tray_window() else {
            return;
        };
        allow_taskbar_context_menu_message(hwnd);
        let data = tray_icon_data(hwnd, std::ptr::null_mut(), "");
        unsafe {
            Shell_NotifyIconW(NIM_DELETE, &data);
        }
    }

    fn allow_taskbar_context_menu_message(hwnd: HWND) {
        // The taskbar extension runs inside Explorer. If Crypto HUD was started
        // elevated, UIPI otherwise rejects Explorer's tray callback message.
        // Scope the exception to the existing Slint tray HWND and message only.
        unsafe {
            ChangeWindowMessageFilterEx(hwnd, WM_TRAYICON, MSGFLT_ALLOW, std::ptr::null_mut());
        }
    }

    pub fn restore_tray_icon(tooltip: &str) {
        let Some(hwnd) = find_slint_tray_window() else {
            return;
        };
        let Some(icon) = load_tray_icon() else {
            return;
        };
        let data = tray_icon_data(hwnd, icon, tooltip);
        unsafe {
            Shell_NotifyIconW(NIM_ADD, &data);
            DestroyIcon(icon);
        }
    }

    pub fn show(title: &str, body: &str) {
        let Some(hwnd) = find_slint_tray_window() else {
            return;
        };
        let data = notification_data(hwnd, title, body);
        unsafe {
            Shell_NotifyIconW(NIM_MODIFY, &data);
        }
    }

    pub fn tray_icon_hovered() -> bool {
        let Some(hwnd) = find_slint_tray_window() else {
            return false;
        };
        let Some(rect) = tray_icon_rect(hwnd) else {
            return false;
        };
        let mut cursor = POINT { x: 0, y: 0 };
        if unsafe { GetCursorPos(&mut cursor) } == 0 {
            return false;
        }

        cursor.x >= rect.left
            && cursor.x < rect.right
            && cursor.y >= rect.top
            && cursor.y < rect.bottom
    }

    fn find_slint_tray_window() -> Option<HWND> {
        let class_name = wide_null(SLINT_TRAY_CLASS);
        let target_pid = std::process::id();
        let mut previous = std::ptr::null_mut();

        loop {
            let hwnd = unsafe {
                FindWindowExW(
                    HWND_MESSAGE,
                    previous,
                    class_name.as_ptr(),
                    std::ptr::null(),
                )
            };
            if hwnd.is_null() {
                return None;
            }

            let mut pid = 0_u32;
            unsafe {
                GetWindowThreadProcessId(hwnd, &mut pid);
            }
            if pid == target_pid {
                return Some(hwnd);
            }

            previous = hwnd;
        }
    }

    fn tray_icon_rect(hwnd: HWND) -> Option<RECT> {
        let identifier = NOTIFYICONIDENTIFIER {
            cbSize: std::mem::size_of::<NOTIFYICONIDENTIFIER>() as u32,
            hWnd: hwnd,
            uID: TRAY_UID,
            ..Default::default()
        };
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if unsafe { Shell_NotifyIconGetRect(&identifier, &mut rect) } == 0 {
            Some(rect)
        } else {
            None
        }
    }

    fn load_tray_icon() -> Option<HICON> {
        let path = crate::plugin::bundled_resource_path("icon.ico");
        let path = wide_null(path.as_os_str());
        let handle = unsafe {
            LoadImageW(
                std::ptr::null_mut(),
                path.as_ptr(),
                IMAGE_ICON,
                0,
                0,
                LR_LOADFROMFILE,
            )
        };
        if handle.is_null() {
            None
        } else {
            Some(handle as HICON)
        }
    }

    fn tray_icon_data(hwnd: HWND, icon: HICON, tooltip: &str) -> NOTIFYICONDATAW {
        let mut data = notify_icon_data(hwnd, NIF_MESSAGE | NIF_ICON | NIF_TIP);
        data.uCallbackMessage = WM_TRAYICON;
        data.hIcon = icon;
        copy_wide(&mut data.szTip, tooltip);
        data
    }

    fn notification_data(hwnd: HWND, title: &str, body: &str) -> NOTIFYICONDATAW {
        let mut data = notify_icon_data(hwnd, NIF_INFO);
        copy_wide(&mut data.szInfoTitle, title);
        copy_wide(&mut data.szInfo, body);
        data.dwInfoFlags = NIIF_INFO;
        data
    }

    fn notify_icon_data(hwnd: HWND, flags: u32) -> NOTIFYICONDATAW {
        NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_UID,
            uFlags: flags,
            ..Default::default()
        }
    }

    fn copy_wide<const N: usize>(target: &mut [u16; N], value: &str) {
        let encoded = OsStr::new(value).encode_wide().collect::<Vec<_>>();
        let count = encoded.len().min(target.len().saturating_sub(1));
        target[..count].copy_from_slice(&encoded[..count]);
    }

    fn wide_null(value: impl AsRef<OsStr>) -> Vec<u16> {
        value
            .as_ref()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_throttle_suppresses_duplicate_messages_inside_cooldown() {
        let now = Instant::now();
        let mut throttle = NotificationThrottle::new(Duration::from_secs(60));

        assert!(throttle.should_notify("market", "Binance failed", now));
        assert!(!throttle.should_notify("market", "Binance failed", now + Duration::from_secs(30)));
        assert!(throttle.should_notify("market", "OKX failed", now + Duration::from_secs(31)));
        assert!(throttle.should_notify("market", "OKX failed", now + Duration::from_secs(91)));
    }

    #[test]
    fn tray_menu_toggle_closes_an_active_menu_without_reopening_it() {
        let mut state = TrayMenuToggleState::default();

        assert_eq!(state.toggle_requested(1_000), TrayMenuToggleAction::Open);
        state.entered_menu_loop();
        assert_eq!(state.toggle_requested(1_100), TrayMenuToggleAction::Close);

        state.exited_menu_loop(1_120, 500, false);
        assert_eq!(state.toggle_requested(1_150), TrayMenuToggleAction::Open);
    }

    #[test]
    fn tray_menu_toggle_suppresses_the_reopen_after_right_click_exits_the_menu_first() {
        let mut state = TrayMenuToggleState::default();
        state.entered_menu_loop();

        state.exited_menu_loop(1_120, 500, true);
        assert_eq!(state.toggle_requested(1_150), TrayMenuToggleAction::Ignore);
        assert_eq!(state.toggle_requested(1_160), TrayMenuToggleAction::Open);
    }

    #[test]
    fn tray_menu_toggle_reopens_normally_after_the_exit_suppression_expires() {
        let mut state = TrayMenuToggleState::default();
        state.entered_menu_loop();
        state.exited_menu_loop(2_000, 500, true);

        assert_eq!(state.toggle_requested(2_501), TrayMenuToggleAction::Open);
    }

    #[test]
    fn tray_menu_toggle_does_not_suppress_reopen_after_a_non_right_click_exit() {
        let mut state = TrayMenuToggleState::default();
        state.entered_menu_loop();
        state.exited_menu_loop(3_000, 500, false);

        assert_eq!(state.toggle_requested(3_001), TrayMenuToggleAction::Open);
    }
}
