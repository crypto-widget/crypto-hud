use std::{
    path::PathBuf,
    sync::{
        mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
        OnceLock,
    },
    thread,
    time::{Duration, Instant},
};

use crate::{
    i18n,
    theme::{self, ResolvedTheme},
    AppTray, SettingsWindow,
};
use crypto_hud_core::{format_market_pair_symbol, format_pair_change, format_price};
use crypto_hud_runtime::{data_health_for_symbols, QuoteCache};
use crypto_hud_shell_state::AppSettings;

const WORKER_COMMAND_CAPACITY: usize = 1;
const WINDOWS_11_MAJOR_VERSION: u32 = 10;
const WINDOWS_11_MINOR_VERSION: u32 = 0;
const WINDOWS_11_MINIMUM_BUILD: u32 = 22_000;
const WINDOWS_WORKSTATION_PRODUCT_TYPE: u8 = 1;
const WINDOWS_AMD64_PROCESSOR_ARCHITECTURE: u16 = 9;
const TASKBAR_LIGHT_GREEN_ARGB: u32 = 0xFF04_7857;
const TASKBAR_LIGHT_RED_ARGB: u32 = 0xFFDC_2626;
const TASKBAR_DARK_GREEN_ARGB: u32 = 0xFF22_D76F;
const TASKBAR_DARK_RED_ARGB: u32 = 0xFFFF_6B76;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WindowsPlatformFacts {
    major_version: u32,
    minor_version: u32,
    build_number: u32,
    product_type: u8,
    native_processor_architecture: u16,
}

fn windows_11_x64_from_facts(
    target_is_windows_x64: bool,
    facts: Option<WindowsPlatformFacts>,
) -> bool {
    target_is_windows_x64
        && facts.is_some_and(|facts| {
            facts.major_version == WINDOWS_11_MAJOR_VERSION
                && facts.minor_version == WINDOWS_11_MINOR_VERSION
                && facts.build_number >= WINDOWS_11_MINIMUM_BUILD
                && facts.product_type == WINDOWS_WORKSTATION_PRODUCT_TYPE
                && facts.native_processor_architecture == WINDOWS_AMD64_PROCESSOR_ARCHITECTURE
        })
}

pub(crate) fn is_windows_11_x64() -> bool {
    #[cfg(debug_assertions)]
    if crate::feature_flags::gui_smoke_offline_network_disabled()
        && std::env::var("CRYPTO_HUD_GUI_SMOKE_FORCE_TASKBAR_UNSUPPORTED")
            .ok()
            .as_deref()
            == Some("1")
    {
        return false;
    }

    static SUPPORTED: OnceLock<bool> = OnceLock::new();
    *SUPPORTED.get_or_init(|| {
        windows_11_x64_from_facts(
            cfg!(all(windows, target_arch = "x86_64")),
            platform::windows_platform_facts(),
        )
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TaskbarMarketFrame {
    symbol: String,
    price: String,
    tooltip: String,
    accent_argb: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum TaskbarMarketStatus {
    #[default]
    Disabled,
    Initializing,
    WaitingForTaskbar,
    Attached,
    Detaching,
    Unsupported,
    CompanionMissing,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkerCommand {
    enabled: bool,
    frame: TaskbarMarketFrame,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkerEvent {
    Status(TaskbarMarketStatus),
    OpenSettingsRequested,
}

#[derive(Debug)]
struct TaskbarWorker {
    commands: SyncSender<WorkerCommand>,
    events: Receiver<WorkerEvent>,
}

impl TaskbarWorker {
    fn spawn(runtime_root: PathBuf) -> Self {
        let (command_tx, command_rx) = mpsc::sync_channel(WORKER_COMMAND_CAPACITY);
        let (event_tx, event_rx) = mpsc::channel();
        let spawn_result = thread::Builder::new()
            .name("crypto-hud-taskbar".to_string())
            .spawn(move || platform::run_worker(command_rx, event_tx, runtime_root));
        if let Err(error) = spawn_result {
            eprintln!("failed to start taskbar market worker: {error}");
        }
        Self {
            commands: command_tx,
            events: event_rx,
        }
    }
}

#[derive(Debug)]
pub(crate) struct TaskbarMarketController {
    runtime_root: PathBuf,
    worker: Option<TaskbarWorker>,
    observed_symbols: Vec<String>,
    observed_interval_seconds: i32,
    current_index: usize,
    next_switch_at: Option<Instant>,
    last_frame: Option<TaskbarMarketFrame>,
    last_command_enabled: Option<bool>,
    status: TaskbarMarketStatus,
}

impl TaskbarMarketController {
    pub(crate) fn new(runtime_root: PathBuf) -> Self {
        Self {
            runtime_root,
            worker: None,
            observed_symbols: Vec::new(),
            observed_interval_seconds: 0,
            current_index: 0,
            next_switch_at: None,
            last_frame: None,
            last_command_enabled: None,
            status: TaskbarMarketStatus::Disabled,
        }
    }

    pub(crate) fn refresh(
        &mut self,
        tray: &AppTray,
        settings_window: &slint::Weak<SettingsWindow>,
        settings: &AppSettings,
        quote_cache: &QuoteCache,
        has_market_error: bool,
        now: Instant,
    ) -> bool {
        let requested = settings.tray_market_enabled && !settings.tray_market_symbols.is_empty();
        let platform_supported = is_windows_11_x64();
        let enabled = requested && platform_supported;
        let frame = if enabled {
            let index = self.rotation_index(
                &settings.tray_market_symbols,
                settings.tray_market_switch_interval_seconds,
                now,
            );
            taskbar_market_frame(
                &settings.tray_market_symbols[index],
                settings,
                quote_cache,
                has_market_error,
                now,
            )
        } else {
            self.reset_rotation();
            disabled_frame()
        };

        if self.worker.is_none() && enabled {
            self.worker = Some(TaskbarWorker::spawn(self.runtime_root.clone()));
            self.status = TaskbarMarketStatus::Initializing;
            self.last_frame = None;
            self.last_command_enabled = None;
        }
        if let Some(worker) = &self.worker {
            let should_send = enabled
                || self.last_command_enabled != Some(enabled)
                || self.last_frame.as_ref() != Some(&frame);
            if should_send {
                match worker.commands.try_send(WorkerCommand {
                    enabled,
                    frame: frame.clone(),
                }) {
                    Ok(()) => {
                        self.last_command_enabled = Some(enabled);
                        self.last_frame = Some(frame.clone());
                    }
                    Err(TrySendError::Full(_)) => {}
                    Err(TrySendError::Disconnected(_)) => {
                        self.status = TaskbarMarketStatus::Failed;
                        self.worker = None;
                        self.last_command_enabled = None;
                    }
                }
            }
        } else if !enabled {
            self.status = if requested && !platform_supported {
                TaskbarMarketStatus::Unsupported
            } else {
                TaskbarMarketStatus::Disabled
            };
        }

        let mut open_settings_requested = false;
        if let Some(worker) = &self.worker {
            loop {
                match worker.events.try_recv() {
                    Ok(WorkerEvent::Status(status)) => self.status = status,
                    Ok(WorkerEvent::OpenSettingsRequested) => open_settings_requested = true,
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        self.status = TaskbarMarketStatus::Failed;
                        break;
                    }
                }
            }
        }
        // The injected text replaces the ordinary app icon only after the TAP reports that
        // the taskbar element exists. During startup, Explorer restarts, and every failure mode,
        // the normal tray icon remains available as a recovery path.
        let taskbar_owns_tray_area = matches!(
            self.status,
            TaskbarMarketStatus::Attached | TaskbarMarketStatus::Detaching
        );
        crate::window_manager::set_taskbar_market_attached(taskbar_owns_tray_area);
        if taskbar_owns_tray_area {
            crate::window_manager::reconcile_native_tray_icon(tray, false);
        } else {
            let fallback_visible = crate::window_manager::effective_tray_icon_enabled(settings);
            // Shell_NotifyIcon state can be lost after an explicit NIM_DELETE or recreated by
            // Explorer's TaskbarCreated broadcast. Reconcile the native registration on every
            // controller pass instead of trusting Slint's unchanged `visible` property.
            crate::window_manager::reconcile_native_tray_icon(tray, fallback_visible);
        }
        if let Some(ui) = settings_window.upgrade() {
            let locale = i18n::resolve_locale(settings.language);
            ui.set_tray_market_status_text(localized_status(locale, self.status).into());
        }
        open_settings_requested
    }

    fn rotation_index(&mut self, symbols: &[String], interval_seconds: i32, now: Instant) -> usize {
        let interval_seconds = interval_seconds.max(1);
        let configuration_changed =
            self.observed_symbols != symbols || self.observed_interval_seconds != interval_seconds;
        let interval = Duration::from_secs(interval_seconds as u64);

        if configuration_changed {
            self.observed_symbols = symbols.to_vec();
            self.observed_interval_seconds = interval_seconds;
            self.current_index = 0;
            self.next_switch_at = now.checked_add(interval);
            self.last_frame = None;
            return 0;
        }
        if symbols.len() <= 1 {
            self.current_index = 0;
            self.next_switch_at = now.checked_add(interval);
            return 0;
        }
        let Some(next_switch_at) = self.next_switch_at else {
            self.next_switch_at = now.checked_add(interval);
            return self.current_index.min(symbols.len() - 1);
        };
        if now < next_switch_at {
            return self.current_index.min(symbols.len() - 1);
        }

        let elapsed_periods =
            now.saturating_duration_since(next_switch_at).as_secs() / interval.as_secs();
        let steps = elapsed_periods.saturating_add(1);
        self.current_index = (self.current_index + steps as usize % symbols.len()) % symbols.len();
        self.next_switch_at = interval
            .as_secs()
            .checked_mul(steps)
            .and_then(|seconds| next_switch_at.checked_add(Duration::from_secs(seconds)))
            .or_else(|| now.checked_add(interval));
        self.last_frame = None;
        self.current_index
    }

    fn reset_rotation(&mut self) {
        self.observed_symbols.clear();
        self.observed_interval_seconds = 0;
        self.current_index = 0;
        self.next_switch_at = None;
    }
}

fn disabled_frame() -> TaskbarMarketFrame {
    TaskbarMarketFrame {
        symbol: String::new(),
        price: String::new(),
        tooltip: String::new(),
        accent_argb: 0,
    }
}

fn taskbar_market_worker_enabled(configured_enabled: bool, shortcut_surfaces_hidden: bool) -> bool {
    configured_enabled && !shortcut_surfaces_hidden
}

fn taskbar_change_accent_argb(
    formatted_change: &str,
    red_up_enabled: bool,
    taskbar_theme: ResolvedTheme,
) -> u32 {
    let Some(displayed_change) = formatted_change
        .strip_suffix('%')
        .and_then(|value| value.parse::<f64>().ok())
    else {
        return 0;
    };
    if !displayed_change.is_finite() || displayed_change == 0.0 {
        return 0;
    }

    let is_gain = displayed_change > 0.0;
    let use_red = is_gain == red_up_enabled;
    match (taskbar_theme, use_red) {
        (ResolvedTheme::Light, false) => TASKBAR_LIGHT_GREEN_ARGB,
        (ResolvedTheme::Light, true) => TASKBAR_LIGHT_RED_ARGB,
        (ResolvedTheme::Dark, false) => TASKBAR_DARK_GREEN_ARGB,
        (ResolvedTheme::Dark, true) => TASKBAR_DARK_RED_ARGB,
    }
}

fn taskbar_market_frame(
    symbol: &str,
    settings: &AppSettings,
    quote_cache: &QuoteCache,
    has_market_error: bool,
    now: Instant,
) -> TaskbarMarketFrame {
    let locale = i18n::resolve_locale(settings.language);
    let text = i18n::text(locale);
    let pair = format_market_pair_symbol(symbol);
    let isolated_pair = i18n::ltr_isolate_for_locale(locale, &pair);

    let Some(quote) = quote_cache.get(symbol) else {
        let status = if has_market_error {
            text.runtime_connection_error
        } else {
            text.runtime_connecting
        };
        return TaskbarMarketFrame {
            symbol: pair,
            price: "---".to_string(),
            tooltip: format!("{isolated_pair}\n{status}"),
            accent_argb: 0,
        };
    };

    let price = format_price(quote.price);
    let change = format_pair_change(quote.change_percent_24h);
    let price_line = format!("{price} {change}");
    let accent_argb = taskbar_change_accent_argb(
        &change,
        settings.red_up_enabled,
        theme::resolve_taskbar_theme(),
    );
    let isolated_price = i18n::ltr_isolate_for_locale(locale, &price);
    let isolated_change = i18n::ltr_isolate_for_locale(locale, &change);
    let stale = data_health_for_symbols(&[symbol.to_string()], quote_cache, now).stale > 0;
    let tooltip = if stale && has_market_error {
        format!(
            "{isolated_pair}\n{isolated_price} · {isolated_change}\n{} · {}",
            text.runtime_stale, text.runtime_source_error
        )
    } else if stale {
        format!(
            "{isolated_pair}\n{isolated_price} · {isolated_change}\n{}",
            text.runtime_stale
        )
    } else {
        format!("{isolated_pair}\n{isolated_price} · {isolated_change}")
    };

    TaskbarMarketFrame {
        symbol: pair,
        price: price_line,
        tooltip,
        accent_argb,
    }
}

fn localized_status(locale: i18n::Locale, status: TaskbarMarketStatus) -> &'static str {
    let text = i18n::text(locale);
    match status {
        TaskbarMarketStatus::Disabled => "",
        TaskbarMarketStatus::Initializing => text.tray_market_status_initializing,
        TaskbarMarketStatus::WaitingForTaskbar => text.tray_market_status_waiting,
        TaskbarMarketStatus::Attached => text.tray_market_status_attached,
        TaskbarMarketStatus::Detaching => text.tray_market_status_detaching,
        TaskbarMarketStatus::Unsupported => text.tray_market_status_unsupported,
        TaskbarMarketStatus::CompanionMissing => text.tray_market_status_companion_missing,
        TaskbarMarketStatus::Failed => text.tray_market_status_failed,
    }
}

#[cfg(windows)]
mod platform {
    use std::{
        ffi::OsStr,
        fs, mem,
        os::windows::ffi::OsStrExt,
        path::{Path, PathBuf},
        ptr,
        sync::{
            atomic::{AtomicU32, Ordering},
            mpsc::{Receiver, RecvTimeoutError, Sender},
        },
        time::{Duration, Instant},
    };

    use windows_sys::{
        core::{GUID, HRESULT},
        Wdk::System::SystemServices::RtlGetVersion,
        Win32::{
            Foundation::{
                CloseHandle, FreeLibrary, GetLastError, ERROR_NOT_FOUND, HANDLE, HMODULE,
                INVALID_HANDLE_VALUE,
            },
            System::{
                LibraryLoader::{GetProcAddress, LoadLibraryW},
                Memory::{
                    CreateFileMappingW, MapViewOfFile, UnmapViewOfFile, FILE_MAP_ALL_ACCESS,
                    MEMORY_MAPPED_VIEW_ADDRESS, PAGE_READWRITE,
                },
                SystemInformation::{
                    GetNativeSystemInfo, GetTickCount64, OSVERSIONINFOEXW, OSVERSIONINFOW,
                    SYSTEM_INFO,
                },
                Threading::{CreateEventW, SetEvent},
            },
            UI::WindowsAndMessaging::{FindWindowW, GetWindowThreadProcessId},
        },
    };

    use super::{TaskbarMarketStatus, WindowsPlatformFacts, WorkerCommand, WorkerEvent};

    const MAPPING_NAME: &str = "Local\\CryptoHud.TaskbarMarket.v6";
    const UPDATE_EVENT_NAME: &str = "Local\\CryptoHud.TaskbarMarket.Update.v6";
    const ACTION_EVENT_NAME: &str = "Local\\CryptoHud.TaskbarMarket.Action.v6";
    const SHARED_MAGIC: u32 = 0x4D54_4843;
    const SHARED_VERSION: u16 = 6;
    const TASKBAR_TAP_CLSID: GUID = GUID::from_u128(0x2304531e_b59e_4f0e_b3a7_80535005076a);
    const RETRY_INTERVAL: Duration = Duration::from_secs(10);
    const EXISTING_TAP_DISCOVERY_GRACE: Duration = Duration::from_millis(750);
    const DETACHED_TAP_RECOVERY_GRACE: Duration = Duration::from_secs(1);
    const DLL_FILE_NAME: &str = "crypto_hud_taskbar.dll";

    const STATUS_DISABLED: u32 = 0;
    const STATUS_INITIALIZING: u32 = 1;
    const STATUS_WAITING_FOR_VISUAL_TREE: u32 = 2;
    const STATUS_ATTACHED: u32 = 3;
    const STATUS_DETACHING: u32 = 4;
    const STATUS_DETACHED: u32 = 5;
    const STATUS_ERROR: u32 = 6;

    pub(super) fn windows_platform_facts() -> Option<WindowsPlatformFacts> {
        let mut version = OSVERSIONINFOEXW {
            dwOSVersionInfoSize: mem::size_of::<OSVERSIONINFOEXW>() as u32,
            ..Default::default()
        };
        let status = unsafe { RtlGetVersion(ptr::from_mut(&mut version).cast::<OSVERSIONINFOW>()) };
        if status < 0 {
            return None;
        }

        let mut system_info = SYSTEM_INFO::default();
        unsafe {
            GetNativeSystemInfo(ptr::from_mut(&mut system_info));
        }
        let native_processor_architecture =
            unsafe { system_info.Anonymous.Anonymous.wProcessorArchitecture };

        Some(WindowsPlatformFacts {
            major_version: version.dwMajorVersion,
            minor_version: version.dwMinorVersion,
            build_number: version.dwBuildNumber,
            product_type: version.wProductType,
            native_processor_architecture,
        })
    }

    #[repr(C, align(8))]
    struct SharedState {
        magic: u32,
        version: u16,
        size: u16,
        seq: u32,
        owner_pid: u32,
        heartbeat_ms: u64,
        enabled: u32,
        status: u32,
        error_code: u32,
        explorer_pid: u32,
        symbol: [u16; 64],
        price: [u16; 64],
        tooltip: [u16; 192],
        accent_argb: u32,
        action_seq: u32,
    }

    const _: () = assert!(mem::size_of::<SharedState>() == 688);
    const _: () = assert!(mem::align_of::<SharedState>() == 8);

    struct SharedMapping {
        mapping: HANDLE,
        view: MEMORY_MAPPED_VIEW_ADDRESS,
        update_event: HANDLE,
        action_event: HANDLE,
    }

    impl SharedMapping {
        fn create() -> Result<Self, u32> {
            let mapping_name = wide_null(MAPPING_NAME);
            let mapping = unsafe {
                CreateFileMappingW(
                    INVALID_HANDLE_VALUE,
                    ptr::null(),
                    PAGE_READWRITE,
                    0,
                    mem::size_of::<SharedState>() as u32,
                    mapping_name.as_ptr(),
                )
            };
            if mapping.is_null() {
                return Err(unsafe { GetLastError() });
            }
            let view = unsafe {
                MapViewOfFile(
                    mapping,
                    FILE_MAP_ALL_ACCESS,
                    0,
                    0,
                    mem::size_of::<SharedState>(),
                )
            };
            if view.Value.is_null() {
                let error = unsafe { GetLastError() };
                unsafe {
                    CloseHandle(mapping);
                }
                return Err(error);
            }
            let update_event_name = wide_null(UPDATE_EVENT_NAME);
            let update_event =
                unsafe { CreateEventW(ptr::null(), 0, 0, update_event_name.as_ptr()) };
            if update_event.is_null() {
                let error = unsafe { GetLastError() };
                unsafe {
                    UnmapViewOfFile(view);
                    CloseHandle(mapping);
                }
                return Err(error);
            }
            let action_event_name = wide_null(ACTION_EVENT_NAME);
            let action_event =
                unsafe { CreateEventW(ptr::null(), 0, 0, action_event_name.as_ptr()) };
            if action_event.is_null() {
                let error = unsafe { GetLastError() };
                unsafe {
                    CloseHandle(update_event);
                    UnmapViewOfFile(view);
                    CloseHandle(mapping);
                }
                return Err(error);
            }

            let result = Self {
                mapping,
                view,
                update_event,
                action_event,
            };
            result.initialize_header();
            Ok(result)
        }

        fn state(&self) -> *mut SharedState {
            self.view.Value.cast()
        }

        fn initialize_header(&self) {
            let state = self.state();
            unsafe {
                ptr::write_bytes(state.cast::<u8>(), 0, mem::size_of::<SharedState>());
                ptr::write_volatile(ptr::addr_of_mut!((*state).magic), SHARED_MAGIC);
                ptr::write_volatile(ptr::addr_of_mut!((*state).version), SHARED_VERSION);
                ptr::write_volatile(
                    ptr::addr_of_mut!((*state).size),
                    mem::size_of::<SharedState>() as u16,
                );
                ptr::write_volatile(ptr::addr_of_mut!((*state).owner_pid), std::process::id());
            }
        }

        fn write(&self, command: &WorkerCommand) {
            let state = self.state();
            unsafe {
                let seq = &*ptr::addr_of!((*state).seq).cast::<AtomicU32>();
                let start = seq.load(Ordering::Relaxed).wrapping_add(1) | 1;
                seq.store(start, Ordering::Release);
                ptr::write_volatile(ptr::addr_of_mut!((*state).owner_pid), std::process::id());
                ptr::write_volatile(ptr::addr_of_mut!((*state).heartbeat_ms), GetTickCount64());
                ptr::write_volatile(
                    ptr::addr_of_mut!((*state).enabled),
                    u32::from(command.enabled),
                );
                write_utf16_fixed(ptr::addr_of_mut!((*state).symbol), &command.frame.symbol);
                write_utf16_fixed(ptr::addr_of_mut!((*state).price), &command.frame.price);
                write_utf16_fixed(ptr::addr_of_mut!((*state).tooltip), &command.frame.tooltip);
                ptr::write_volatile(
                    ptr::addr_of_mut!((*state).accent_argb),
                    command.frame.accent_argb,
                );
                seq.store(start.wrapping_add(1), Ordering::Release);
                SetEvent(self.update_event);
            }
        }

        fn status(&self) -> (u32, u32, u32) {
            let state = self.state();
            unsafe {
                let status =
                    (&*ptr::addr_of!((*state).status).cast::<AtomicU32>()).load(Ordering::Acquire);
                let error = (&*ptr::addr_of!((*state).error_code).cast::<AtomicU32>())
                    .load(Ordering::Acquire);
                let explorer = (&*ptr::addr_of!((*state).explorer_pid).cast::<AtomicU32>())
                    .load(Ordering::Acquire);
                (status, error, explorer)
            }
        }

        fn action_seq(&self) -> u32 {
            let state = self.state();
            unsafe {
                (&*ptr::addr_of!((*state).action_seq).cast::<AtomicU32>()).load(Ordering::Acquire)
            }
        }
    }

    impl Drop for SharedMapping {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.action_event);
                CloseHandle(self.update_event);
                UnmapViewOfFile(self.view);
                CloseHandle(self.mapping);
            }
        }
    }

    pub(super) fn run_worker(
        commands: Receiver<WorkerCommand>,
        events: Sender<WorkerEvent>,
        runtime_root: PathBuf,
    ) {
        if !super::is_windows_11_x64() {
            let _ = events.send(WorkerEvent::Status(TaskbarMarketStatus::Unsupported));
            return;
        }
        let mapping = match SharedMapping::create() {
            Ok(mapping) => mapping,
            Err(error) => {
                eprintln!("failed to create taskbar market IPC: Win32 error {error}");
                let _ = events.send(WorkerEvent::Status(TaskbarMarketStatus::Failed));
                return;
            }
        };

        let disabled = WorkerCommand {
            enabled: false,
            frame: super::disabled_frame(),
        };
        let mut current = disabled.clone();
        mapping.write(&disabled);
        let mut published_enabled = false;
        let mut shortcut_surfaces_hidden = crate::window_manager::shortcut_surfaces_hidden();
        let mut last_status = TaskbarMarketStatus::Disabled;
        let mut last_action_seq = mapping.action_seq();
        let mut initialized_explorer = 0_u32;
        let mut last_attempt: Option<Instant> = None;
        let mut existing_tap_deadline: Option<Instant> = None;
        let mut detached_tap_since: Option<Instant> = None;
        let mut host_error_status: Option<TaskbarMarketStatus> = None;

        loop {
            let mut command_updated = false;
            match commands.recv_timeout(Duration::from_millis(250)) {
                Ok(command) => {
                    current = command;
                    command_updated = true;
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    mapping.write(&disabled);
                    break;
                }
            }

            let next_shortcut_surfaces_hidden = crate::window_manager::shortcut_surfaces_hidden();
            let effective_enabled = super::taskbar_market_worker_enabled(
                current.enabled,
                next_shortcut_surfaces_hidden,
            );
            if command_updated
                || next_shortcut_surfaces_hidden != shortcut_surfaces_hidden
                || effective_enabled != published_enabled
            {
                let was_enabled = published_enabled;
                if effective_enabled {
                    mapping.write(&current);
                } else {
                    mapping.write(&disabled);
                }
                published_enabled = effective_enabled;
                shortcut_surfaces_hidden = next_shortcut_surfaces_hidden;
                if published_enabled && !was_enabled && initialized_explorer == 0 {
                    existing_tap_deadline =
                        Instant::now().checked_add(EXISTING_TAP_DISCOVERY_GRACE);
                }
                if !published_enabled {
                    host_error_status = None;
                    existing_tap_deadline = None;
                    detached_tap_since = None;
                }
            }

            let shell_pid = explorer_process_id();
            let (tap_status, tap_error, tap_explorer) = mapping.status();
            if tap_status == STATUS_ERROR && tap_error != 0 {
                eprintln!("taskbar TAP reported HRESULT 0x{tap_error:08X}");
            }

            if initialized_explorer != 0 && shell_pid != initialized_explorer {
                // Explorer restarts should recover immediately instead of
                // inheriting the previous process's retry delay.
                initialized_explorer = 0;
                host_error_status = None;
                last_attempt = None;
                existing_tap_deadline = None;
                detached_tap_since = None;
            }
            if published_enabled
                && initialized_explorer == shell_pid
                && tap_explorer == shell_pid
                && tap_status == STATUS_ERROR
                && last_attempt.is_none_or(|attempt| attempt.elapsed() >= RETRY_INTERVAL)
            {
                // Advise/dispatcher failures are asynchronous to the initial
                // diagnostics call. Allow a clean, rate-limited reinitialize
                // in the same Explorer session.
                initialized_explorer = 0;
                host_error_status = None;
                last_attempt = None;
                existing_tap_deadline = None;
            }
            if current_tap_is_detached(
                published_enabled,
                shell_pid,
                initialized_explorer,
                tap_explorer,
                tap_status,
            ) {
                let detached_since = detached_tap_since.get_or_insert_with(Instant::now);
                if detached_since.elapsed() >= DETACHED_TAP_RECOVERY_GRACE {
                    // A watcher that intentionally stops publishes Detached.
                    // Give a replacement watcher a dispatcher turn to take
                    // over, then reinitialize if the state remains orphaned.
                    initialized_explorer = 0;
                    host_error_status = None;
                    last_attempt = None;
                    existing_tap_deadline = None;
                    detached_tap_since = None;
                }
            } else {
                detached_tap_since = None;
            }
            if published_enabled
                && initialized_explorer == 0
                && tap_explorer == shell_pid
                && matches!(
                    tap_status,
                    STATUS_INITIALIZING | STATUS_WAITING_FOR_VISUAL_TREE | STATUS_ATTACHED
                )
            {
                // A compatible TAP can outlive the previous app process in
                // Explorer. Reuse it instead of accumulating a new diagnostics
                // watcher on every normal app restart.
                initialized_explorer = shell_pid;
                existing_tap_deadline = None;
            }

            let discovering_existing_tap =
                existing_tap_deadline.is_some_and(|deadline| Instant::now() < deadline);
            let should_attempt = published_enabled
                && shell_pid != 0
                && initialized_explorer != shell_pid
                && !discovering_existing_tap
                && last_attempt.is_none_or(|attempt| attempt.elapsed() >= RETRY_INTERVAL);
            if should_attempt {
                existing_tap_deadline = None;
                last_attempt = Some(Instant::now());
                host_error_status = None;
                report_status(&events, &mut last_status, TaskbarMarketStatus::Initializing);
                match prepare_runtime_dll(&runtime_root).and_then(|dll| {
                    initialize_xaml_diagnostics(shell_pid, &dll).map_err(InitializeError::Hresult)
                }) {
                    Ok(()) => {
                        initialized_explorer = shell_pid;
                    }
                    Err(InitializeError::MissingCompanion(path)) => {
                        eprintln!("taskbar companion DLL was not found at {}", path.display());
                        host_error_status = Some(TaskbarMarketStatus::CompanionMissing);
                    }
                    Err(InitializeError::Io(error)) => {
                        eprintln!("failed to prepare taskbar companion DLL: {error}");
                        host_error_status = Some(TaskbarMarketStatus::Failed);
                    }
                    Err(InitializeError::Hresult(hr)) => {
                        eprintln!("failed to initialize taskbar XAML diagnostics: 0x{hr:08X}");
                        host_error_status = Some(if is_unsupported_hresult(hr) {
                            TaskbarMarketStatus::Unsupported
                        } else {
                            TaskbarMarketStatus::Failed
                        });
                    }
                }
            }
            let status = resolve_worker_status(
                published_enabled,
                shell_pid,
                tap_explorer,
                tap_status,
                host_error_status,
            );
            report_status(&events, &mut last_status, status);

            let action_seq = mapping.action_seq();
            if action_seq != last_action_seq {
                last_action_seq = action_seq;
                let _ = events.send(WorkerEvent::OpenSettingsRequested);
            }
        }
    }

    fn report_status(
        events: &Sender<WorkerEvent>,
        previous: &mut TaskbarMarketStatus,
        next: TaskbarMarketStatus,
    ) {
        if *previous != next {
            *previous = next;
            let _ = events.send(WorkerEvent::Status(next));
        }
    }

    fn resolve_worker_status(
        enabled: bool,
        shell_pid: u32,
        tap_explorer: u32,
        tap_status: u32,
        host_error_status: Option<TaskbarMarketStatus>,
    ) -> TaskbarMarketStatus {
        let tap_belongs_to_current_explorer = shell_pid != 0 && tap_explorer == shell_pid;
        if !enabled {
            return if tap_belongs_to_current_explorer
                && matches!(tap_status, STATUS_ATTACHED | STATUS_DETACHING)
            {
                TaskbarMarketStatus::Detaching
            } else {
                // A stale Attached value can survive in the shared mapping
                // after Explorer restarts. It no longer owns any taskbar UI,
                // so the fallback tray icon must be restored immediately.
                TaskbarMarketStatus::Disabled
            };
        }
        if let Some(status) = host_error_status {
            return status;
        }
        if shell_pid == 0 || !tap_belongs_to_current_explorer {
            return TaskbarMarketStatus::WaitingForTaskbar;
        }
        match tap_status {
            STATUS_ATTACHED => TaskbarMarketStatus::Attached,
            STATUS_INITIALIZING => TaskbarMarketStatus::Initializing,
            STATUS_WAITING_FOR_VISUAL_TREE | STATUS_DISABLED | STATUS_DETACHED => {
                TaskbarMarketStatus::WaitingForTaskbar
            }
            STATUS_DETACHING => TaskbarMarketStatus::Detaching,
            STATUS_ERROR => TaskbarMarketStatus::Failed,
            _ => TaskbarMarketStatus::WaitingForTaskbar,
        }
    }

    fn current_tap_is_detached(
        enabled: bool,
        shell_pid: u32,
        initialized_explorer: u32,
        tap_explorer: u32,
        tap_status: u32,
    ) -> bool {
        enabled
            && shell_pid != 0
            && initialized_explorer == shell_pid
            && tap_explorer == shell_pid
            && tap_status == STATUS_DETACHED
    }

    #[derive(Debug)]
    enum InitializeError {
        MissingCompanion(PathBuf),
        Io(std::io::Error),
        Hresult(u32),
    }

    fn prepare_runtime_dll(runtime_root: &Path) -> Result<PathBuf, InitializeError> {
        let source = companion_dll_path();
        if !source.is_file() {
            return Err(InitializeError::MissingCompanion(source));
        }
        let bytes = fs::read(&source).map_err(InitializeError::Io)?;
        let hash = fnv1a64(&bytes);
        let version_root = runtime_root.join(format!("{hash:016x}"));
        fs::create_dir_all(&version_root).map_err(InitializeError::Io)?;
        let target = version_root.join(DLL_FILE_NAME);
        if !target.is_file() {
            let temporary =
                version_root.join(format!("{DLL_FILE_NAME}.tmp-{}", std::process::id()));
            fs::write(&temporary, bytes).map_err(InitializeError::Io)?;
            match fs::rename(&temporary, &target) {
                Ok(()) => {}
                Err(error) if target.is_file() => {
                    let _ = fs::remove_file(&temporary);
                    let _ = error;
                }
                Err(error) => return Err(InitializeError::Io(error)),
            }
        }
        Ok(target)
    }

    fn companion_dll_path() -> PathBuf {
        if let Some(path) = std::env::var_os("CRYPTO_HUD_TASKBAR_DLL") {
            return PathBuf::from(path);
        }
        if let Ok(executable) = std::env::current_exe() {
            if let Some(directory) = executable.parent() {
                let installed = directory
                    .join("resources")
                    .join("taskbar")
                    .join(DLL_FILE_NAME);
                if installed.is_file() {
                    return installed;
                }
            }
        }
        let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let debug = workspace.join("target").join("debug").join(DLL_FILE_NAME);
        if debug.is_file() {
            return debug;
        }
        workspace.join("target").join("release").join(DLL_FILE_NAME)
    }

    fn explorer_process_id() -> u32 {
        let class_name = wide_null("Shell_TrayWnd");
        let hwnd = unsafe { FindWindowW(class_name.as_ptr(), ptr::null()) };
        if hwnd.is_null() {
            return 0;
        }
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut pid);
        }
        pid
    }

    type InitializeXamlDiagnosticsEx = unsafe extern "system" fn(
        *const u16,
        u32,
        *const u16,
        *const u16,
        GUID,
        *const u16,
    ) -> HRESULT;

    fn initialize_xaml_diagnostics(pid: u32, dll_path: &Path) -> Result<(), u32> {
        let system_dll = std::env::var_os("WINDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
            .join("System32")
            .join("Windows.UI.Xaml.dll");
        let system_dll = wide_null(system_dll.as_os_str());
        let module = unsafe { LoadLibraryW(system_dll.as_ptr()) };
        if module.is_null() {
            return Err(hresult_from_win32(unsafe { GetLastError() }));
        }
        let result = initialize_with_module(module, pid, dll_path);
        unsafe {
            FreeLibrary(module);
        }
        result
    }

    fn initialize_with_module(module: HMODULE, pid: u32, dll_path: &Path) -> Result<(), u32> {
        let procedure =
            unsafe { GetProcAddress(module, c"InitializeXamlDiagnosticsEx".as_ptr().cast()) };
        let Some(procedure) = procedure else {
            return Err(hresult_from_win32(unsafe { GetLastError() }));
        };
        let initialize: InitializeXamlDiagnosticsEx = unsafe { mem::transmute(procedure) };
        let dll_path = wide_null(dll_path.as_os_str());
        let empty = [0_u16];
        let not_found = hresult_from_win32(ERROR_NOT_FOUND);
        let mut last = not_found;
        for index in 1..=10_000 {
            let endpoint = wide_null(format!("VisualDiagConnection{index}"));
            let hr = unsafe {
                initialize(
                    endpoint.as_ptr(),
                    pid,
                    empty.as_ptr(),
                    dll_path.as_ptr(),
                    TASKBAR_TAP_CLSID,
                    ptr::null(),
                )
            } as u32;
            last = hr;
            if hr != not_found {
                return if (hr as i32) >= 0 { Ok(()) } else { Err(hr) };
            }
        }
        Err(last)
    }

    fn is_unsupported_hresult(hr: u32) -> bool {
        matches!(hr, 0x8007_007E | 0x8007_007F | 0x8004_0154)
    }

    fn hresult_from_win32(error: u32) -> u32 {
        if error == 0 {
            0
        } else {
            (error & 0xFFFF) | 0x8007_0000
        }
    }

    fn wide_null(value: impl AsRef<OsStr>) -> Vec<u16> {
        value.as_ref().encode_wide().chain(Some(0)).collect()
    }

    unsafe fn write_utf16_fixed<const N: usize>(target: *mut [u16; N], value: &str) {
        let target = &mut *target;
        target.fill(0);
        let encoded = truncate_utf16(value, N.saturating_sub(1));
        target[..encoded.len()].copy_from_slice(&encoded);
    }

    fn truncate_utf16(value: &str, maximum_units: usize) -> Vec<u16> {
        let mut result = Vec::with_capacity(maximum_units.min(value.len()));
        for ch in value.chars() {
            let mut units = [0_u16; 2];
            let encoded = ch.encode_utf16(&mut units);
            if result.len() + encoded.len() > maximum_units {
                break;
            }
            result.extend_from_slice(encoded);
        }
        result
    }

    fn fnv1a64(bytes: &[u8]) -> u64 {
        bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn shared_protocol_layout_matches_the_native_tap_contract() {
            assert_eq!(SHARED_VERSION, 6);
            assert!(MAPPING_NAME.ends_with(".v6"));
            assert!(UPDATE_EVENT_NAME.ends_with(".v6"));
            assert!(ACTION_EVENT_NAME.ends_with(".v6"));
            assert_eq!(mem::size_of::<SharedState>(), 688);
            assert_eq!(mem::align_of::<SharedState>(), 8);
            assert_eq!(mem::offset_of!(SharedState, heartbeat_ms), 16);
            assert_eq!(mem::offset_of!(SharedState, symbol), 40);
            assert_eq!(mem::offset_of!(SharedState, price), 168);
            assert_eq!(mem::offset_of!(SharedState, tooltip), 296);
            assert_eq!(mem::offset_of!(SharedState, action_seq), 684);
        }

        #[test]
        fn utf16_truncation_never_splits_a_surrogate_pair() {
            assert_eq!(
                truncate_utf16("AB😀C", 4),
                "AB😀".encode_utf16().collect::<Vec<_>>()
            );
            assert_eq!(
                truncate_utf16("AB😀C", 3),
                "AB".encode_utf16().collect::<Vec<_>>()
            );
        }

        #[test]
        fn content_hash_is_stable_and_sensitive_to_bytes() {
            assert_eq!(fnv1a64(b"crypto-hud"), fnv1a64(b"crypto-hud"));
            assert_ne!(fnv1a64(b"crypto-hud"), fnv1a64(b"crypto-hud!"));
        }

        #[test]
        fn stale_explorer_status_never_claims_or_detaches_the_taskbar() {
            let current_explorer = 200;
            let old_explorer = 100;

            for tap_status in [STATUS_ATTACHED, STATUS_DETACHING, STATUS_ERROR] {
                assert_eq!(
                    resolve_worker_status(false, current_explorer, old_explorer, tap_status, None,),
                    TaskbarMarketStatus::Disabled
                );
                assert_eq!(
                    resolve_worker_status(true, current_explorer, old_explorer, tap_status, None,),
                    TaskbarMarketStatus::WaitingForTaskbar
                );
            }
        }

        #[test]
        fn only_the_current_explorer_can_report_live_tap_states() {
            let explorer = 200;
            assert_eq!(
                resolve_worker_status(false, explorer, explorer, STATUS_ATTACHED, None),
                TaskbarMarketStatus::Detaching
            );
            assert_eq!(
                resolve_worker_status(true, explorer, explorer, STATUS_ATTACHED, None),
                TaskbarMarketStatus::Attached
            );
            assert_eq!(
                resolve_worker_status(true, explorer, explorer, STATUS_DETACHING, None),
                TaskbarMarketStatus::Detaching
            );
            assert_eq!(
                resolve_worker_status(true, explorer, explorer, STATUS_ERROR, None),
                TaskbarMarketStatus::Failed
            );
            assert!(current_tap_is_detached(
                true,
                explorer,
                explorer,
                explorer,
                STATUS_DETACHED
            ));
            assert!(!current_tap_is_detached(
                true,
                explorer,
                explorer,
                explorer - 1,
                STATUS_DETACHED
            ));
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use std::{
        path::PathBuf,
        sync::mpsc::{Receiver, Sender},
    };

    use super::{TaskbarMarketStatus, WindowsPlatformFacts, WorkerCommand, WorkerEvent};

    pub(super) fn windows_platform_facts() -> Option<WindowsPlatformFacts> {
        None
    }

    pub(super) fn run_worker(
        commands: Receiver<WorkerCommand>,
        events: Sender<WorkerEvent>,
        _runtime_root: PathBuf,
    ) {
        while let Ok(command) = commands.recv() {
            let status = if command.enabled {
                TaskbarMarketStatus::Unsupported
            } else {
                TaskbarMarketStatus::Disabled
            };
            let _ = events.send(WorkerEvent::Status(status));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn windows_facts(build_number: u32) -> WindowsPlatformFacts {
        WindowsPlatformFacts {
            major_version: 10,
            minor_version: 0,
            build_number,
            product_type: WINDOWS_WORKSTATION_PRODUCT_TYPE,
            native_processor_architecture: WINDOWS_AMD64_PROCESSOR_ARCHITECTURE,
        }
    }

    #[test]
    fn platform_support_requires_windows_11_client_on_native_amd64() {
        assert!(windows_11_x64_from_facts(true, Some(windows_facts(22_000))));
        assert!(windows_11_x64_from_facts(true, Some(windows_facts(26_100))));
        assert!(!windows_11_x64_from_facts(
            true,
            Some(windows_facts(19_045))
        ));
        assert!(!windows_11_x64_from_facts(
            true,
            Some(windows_facts(21_999))
        ));

        let mut unexpected_version = windows_facts(26_100);
        unexpected_version.major_version = 11;
        assert!(!windows_11_x64_from_facts(true, Some(unexpected_version)));

        let mut arm64 = windows_facts(26_100);
        arm64.native_processor_architecture = 12;
        assert!(!windows_11_x64_from_facts(true, Some(arm64)));

        let mut server = windows_facts(26_100);
        server.product_type = 3;
        assert!(!windows_11_x64_from_facts(true, Some(server)));

        assert!(!windows_11_x64_from_facts(
            false,
            Some(windows_facts(26_100))
        ));
        assert!(!windows_11_x64_from_facts(true, None));
    }

    #[test]
    fn disabled_status_has_no_visible_settings_copy() {
        for locale in i18n::Locale::ALL {
            assert_eq!(localized_status(locale, TaskbarMarketStatus::Disabled), "");
        }
    }

    #[test]
    fn shortcut_surface_suppression_overrides_the_configured_worker_state() {
        assert!(taskbar_market_worker_enabled(true, false));
        assert!(!taskbar_market_worker_enabled(true, true));
        assert!(!taskbar_market_worker_enabled(false, false));
        assert!(!taskbar_market_worker_enabled(false, true));
    }

    #[test]
    fn taskbar_change_colors_follow_the_displayed_direction_and_red_up_preference() {
        for (taskbar_theme, gain, loss) in [
            (
                ResolvedTheme::Light,
                TASKBAR_LIGHT_GREEN_ARGB,
                TASKBAR_LIGHT_RED_ARGB,
            ),
            (
                ResolvedTheme::Dark,
                TASKBAR_DARK_GREEN_ARGB,
                TASKBAR_DARK_RED_ARGB,
            ),
        ] {
            assert_eq!(
                taskbar_change_accent_argb("+1.25%", false, taskbar_theme),
                gain
            );
            assert_eq!(
                taskbar_change_accent_argb("-2.06%", false, taskbar_theme),
                loss
            );
            assert_eq!(
                taskbar_change_accent_argb("+1.25%", true, taskbar_theme),
                loss
            );
            assert_eq!(
                taskbar_change_accent_argb("-2.06%", true, taskbar_theme),
                gain
            );
        }
    }

    #[test]
    fn taskbar_change_colors_keep_displayed_zero_and_invalid_values_neutral() {
        for change in [0.0, -0.0, 0.004, -0.004] {
            let formatted = format_pair_change(change);
            assert_eq!(formatted, "+0.00%");
            assert_eq!(
                taskbar_change_accent_argb(&formatted, false, ResolvedTheme::Dark),
                0
            );
        }
        assert_eq!(
            taskbar_change_accent_argb(&format_pair_change(0.005), false, ResolvedTheme::Dark),
            TASKBAR_DARK_GREEN_ARGB
        );
        assert_eq!(
            taskbar_change_accent_argb(&format_pair_change(-0.005), false, ResolvedTheme::Dark),
            TASKBAR_DARK_RED_ARGB
        );
        assert_eq!(
            taskbar_change_accent_argb("not-a-change", false, ResolvedTheme::Dark),
            0
        );
        assert_eq!(
            taskbar_change_accent_argb("NaN%", false, ResolvedTheme::Dark),
            0
        );
    }

    #[test]
    fn taskbar_frame_uses_full_pair_price_and_change_text() {
        let settings = AppSettings::default();
        let symbol = "binance:spot:BTC/USDT";
        let now = Instant::now();
        let mut quotes = QuoteCache::new();
        quotes.insert(
            symbol.to_string(),
            crypto_hud_runtime::QuoteState::new(
                620_000.0,
                1.25,
                Vec::new(),
                crypto_hud_core::MarketDataSource::Binance,
                now,
            ),
        );

        let frame = taskbar_market_frame(symbol, &settings, &quotes, false, now);
        assert_eq!(frame.symbol, "BTC/USDT");
        assert_eq!(frame.price, "620000 +1.25%");
        assert!(frame.tooltip.contains("+1.25%"));
        assert_ne!(frame.accent_argb, 0);

        let mut red_up_settings = settings;
        red_up_settings.red_up_enabled = true;
        let red_up_frame = taskbar_market_frame(symbol, &red_up_settings, &quotes, false, now);
        assert_ne!(red_up_frame.accent_argb, 0);
        assert_ne!(red_up_frame.accent_argb, frame.accent_argb);
    }

    #[test]
    fn taskbar_frame_places_negative_change_after_price() {
        let settings = AppSettings::default();
        let symbol = "binance:spot:ETH/USDT";
        let now = Instant::now();
        let mut quotes = QuoteCache::new();
        quotes.insert(
            symbol.to_string(),
            crypto_hud_runtime::QuoteState::new(
                1_921.25,
                -2.06,
                Vec::new(),
                crypto_hud_core::MarketDataSource::Binance,
                now,
            ),
        );

        let frame = taskbar_market_frame(symbol, &settings, &quotes, false, now);
        assert_eq!(frame.price, "1921 -2.06%");
        assert_ne!(frame.accent_argb, 0);
    }

    #[test]
    fn missing_quote_distinguishes_connecting_from_market_failure() {
        let settings = AppSettings::default();
        let quotes = QuoteCache::new();
        let now = Instant::now();
        let connecting = taskbar_market_frame("BTC", &settings, &quotes, false, now);
        let failed = taskbar_market_frame("BTC", &settings, &quotes, true, now);
        let text = i18n::text(i18n::Locale::En);

        assert_eq!(connecting.price, "---");
        assert_eq!(connecting.accent_argb, 0);
        assert_eq!(failed.accent_argb, 0);
        assert!(!connecting.price.contains('%'));
        assert!(connecting.tooltip.contains(text.runtime_connecting));
        assert!(failed.tooltip.contains(text.runtime_connection_error));
    }

    #[test]
    fn rotation_advances_wraps_and_resets_on_configuration_changes() {
        let symbols = vec!["BTC".to_string(), "ETH".to_string(), "SOL".to_string()];
        let changed = vec!["SOL".to_string(), "BTC".to_string()];
        let started = Instant::now();
        let mut controller = TaskbarMarketController::new(PathBuf::new());

        assert_eq!(controller.rotation_index(&symbols, 5, started), 0);
        assert_eq!(
            controller.rotation_index(&symbols, 5, started + Duration::from_secs(5)),
            1
        );
        assert_eq!(
            controller.rotation_index(&symbols, 5, started + Duration::from_secs(15)),
            0
        );
        assert_eq!(
            controller.rotation_index(&changed, 5, started + Duration::from_secs(16)),
            0
        );
        assert_eq!(
            controller.rotation_index(&changed, 10, started + Duration::from_secs(17)),
            0
        );
    }

    #[test]
    fn fallback_statuses_do_not_claim_the_taskbar_is_attached() {
        for status in [
            TaskbarMarketStatus::Initializing,
            TaskbarMarketStatus::WaitingForTaskbar,
            TaskbarMarketStatus::Unsupported,
            TaskbarMarketStatus::CompanionMissing,
            TaskbarMarketStatus::Failed,
        ] {
            assert_ne!(status, TaskbarMarketStatus::Attached);
        }
    }
}
