//! Shared protocol and Windows DLL exports for the experimental taskbar market
//! display.
//!
//! The host is the only writer of the market payload. The injected taskbar DLL
//! writes only `status`, `error_code`, `explorer_pid`, and the action sequences.

use std::sync::atomic::{fence, AtomicU32, Ordering};

pub const PROTOCOL_MAGIC: u32 = 0x4D54_4843;
pub const PROTOCOL_VERSION: u16 = 6;
pub const SHARED_STATE_SIZE: u16 = 688;

pub const MAPPING_NAME: &str = r"Local\CryptoHud.TaskbarMarket.v6";
pub const UPDATE_EVENT_NAME: &str = r"Local\CryptoHud.TaskbarMarket.Update.v6";
pub const ACTION_EVENT_NAME: &str = r"Local\CryptoHud.TaskbarMarket.Action.v6";
pub const UPDATE_MESSAGE_NAME: &str = "CryptoHud.TaskbarMarket.Update.v6";

pub const TAP_CLSID: [u8; 16] = [
    0x1e, 0x53, 0x04, 0x23, 0x9e, 0xb5, 0x0e, 0x4f, 0xb3, 0xa7, 0x80, 0x53, 0x50, 0x05, 0x07, 0x6a,
];

pub const SYMBOL_CAPACITY: usize = 64;
pub const PRICE_CAPACITY: usize = 64;
pub const TOOLTIP_CAPACITY: usize = 192;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskbarStatus {
    Disabled = 0,
    Initializing = 1,
    WaitingForVisualTree = 2,
    Attached = 3,
    Detaching = 4,
    Detached = 5,
    Error = 6,
}

impl TaskbarStatus {
    pub fn from_raw(value: u32) -> Option<Self> {
        Some(match value {
            0 => Self::Disabled,
            1 => Self::Initializing,
            2 => Self::WaitingForVisualTree,
            3 => Self::Attached,
            4 => Self::Detaching,
            5 => Self::Detached,
            6 => Self::Error,
            _ => return None,
        })
    }
}

/// Cross-process memory layout shared with the injected taskbar DLL.
///
/// The plain integer fields intentionally mirror their C++ counterparts. The
/// methods below perform atomic operations through aligned pointers where the
/// protocol requires them.
#[repr(C, align(8))]
#[derive(Clone)]
pub struct SharedMarketState {
    pub magic: u32,
    pub version: u16,
    pub size: u16,
    pub sequence: u32,
    pub owner_pid: u32,
    pub heartbeat_ms: u64,
    pub enabled: u32,
    pub status: u32,
    pub error_code: u32,
    pub explorer_pid: u32,
    pub symbol: [u16; SYMBOL_CAPACITY],
    pub price: [u16; PRICE_CAPACITY],
    pub tooltip: [u16; TOOLTIP_CAPACITY],
    pub accent_argb: u32,
    pub action_sequence: u32,
}

impl Default for SharedMarketState {
    fn default() -> Self {
        Self::new(0)
    }
}

impl SharedMarketState {
    pub const fn new(owner_pid: u32) -> Self {
        Self {
            magic: PROTOCOL_MAGIC,
            version: PROTOCOL_VERSION,
            size: SHARED_STATE_SIZE,
            sequence: 0,
            owner_pid,
            heartbeat_ms: 0,
            enabled: 0,
            status: TaskbarStatus::Disabled as u32,
            error_code: 0,
            explorer_pid: 0,
            symbol: [0; SYMBOL_CAPACITY],
            price: [0; PRICE_CAPACITY],
            tooltip: [0; TOOLTIP_CAPACITY],
            accent_argb: 0,
            action_sequence: 0,
        }
    }

    pub fn protocol_is_valid(&self) -> bool {
        self.magic == PROTOCOL_MAGIC
            && self.version == PROTOCOL_VERSION
            && usize::from(self.size) == std::mem::size_of::<Self>()
    }

    /// Publishes a complete host-owned payload with a single-writer seqlock.
    pub fn publish(&mut self, update: &MarketUpdate<'_>) {
        let sequence = std::ptr::addr_of!(self.sequence).cast::<AtomicU32>();
        // SAFETY: `sequence` is four-byte aligned and no non-atomic access is
        // performed while the payload publication is in progress.
        let previous = unsafe { (&*sequence).load(Ordering::Relaxed) };
        let writing_sequence = previous.wrapping_add(1) | 1;
        // SAFETY: See the pointer construction above. Publishing an explicit
        // odd value also recovers safely if a prior writer was interrupted.
        unsafe { (&*sequence).store(writing_sequence, Ordering::Release) };

        self.owner_pid = update.owner_pid;
        self.heartbeat_ms = update.heartbeat_ms;
        self.enabled = u32::from(update.enabled);
        self.symbol = encode_utf16_truncated(update.symbol);
        self.price = encode_utf16_truncated(update.price);
        self.tooltip = encode_utf16_truncated(update.tooltip);
        self.accent_argb = update.accent_argb;

        fence(Ordering::Release);
        // SAFETY: See the pointer construction above.
        unsafe { (&*sequence).store(writing_sequence.wrapping_add(1), Ordering::Release) };
    }

    /// Reads a consistent snapshot. `None` means a writer stayed active for
    /// all retry attempts or the protocol header is invalid.
    pub fn read_consistent(&self) -> Option<MarketSnapshot> {
        if !self.protocol_is_valid() {
            return None;
        }

        let sequence = self.sequence_atomic();
        for _ in 0..8 {
            let before = sequence.load(Ordering::Acquire);
            if before & 1 != 0 {
                std::hint::spin_loop();
                continue;
            }

            let snapshot = MarketSnapshot {
                owner_pid: self.owner_pid,
                heartbeat_ms: self.heartbeat_ms,
                enabled: self.enabled != 0,
                symbol: decode_utf16(&self.symbol),
                price: decode_utf16(&self.price),
                tooltip: decode_utf16(&self.tooltip),
                accent_argb: self.accent_argb,
                action_sequence: self.action_sequence_atomic().load(Ordering::Acquire),
            };

            fence(Ordering::Acquire);
            let after = sequence.load(Ordering::Acquire);
            if before == after && after & 1 == 0 {
                return Some(snapshot);
            }
        }

        None
    }

    pub fn taskbar_status(&self) -> Option<TaskbarStatus> {
        TaskbarStatus::from_raw(self.status_atomic().load(Ordering::Acquire))
    }

    pub fn taskbar_error_code(&self) -> u32 {
        self.error_code_atomic().load(Ordering::Acquire)
    }

    pub fn attached_explorer_pid(&self) -> u32 {
        self.explorer_pid_atomic().load(Ordering::Acquire)
    }

    pub fn action_sequence(&self) -> u32 {
        self.action_sequence_atomic().load(Ordering::Acquire)
    }

    fn sequence_atomic(&self) -> &AtomicU32 {
        // SAFETY: `sequence` is four-byte aligned and has the same size as
        // `AtomicU32`. Both processes access this field atomically.
        unsafe { &*std::ptr::addr_of!(self.sequence).cast::<AtomicU32>() }
    }

    fn status_atomic(&self) -> &AtomicU32 {
        // SAFETY: See `sequence_atomic`; this field is written by the DLL.
        unsafe { &*std::ptr::addr_of!(self.status).cast::<AtomicU32>() }
    }

    fn error_code_atomic(&self) -> &AtomicU32 {
        // SAFETY: See `sequence_atomic`; this field is written by the DLL.
        unsafe { &*std::ptr::addr_of!(self.error_code).cast::<AtomicU32>() }
    }

    fn explorer_pid_atomic(&self) -> &AtomicU32 {
        // SAFETY: See `sequence_atomic`; this field is written by the DLL.
        unsafe { &*std::ptr::addr_of!(self.explorer_pid).cast::<AtomicU32>() }
    }

    fn action_sequence_atomic(&self) -> &AtomicU32 {
        // SAFETY: See `sequence_atomic`; this field is written by the DLL.
        unsafe { &*std::ptr::addr_of!(self.action_sequence).cast::<AtomicU32>() }
    }
}

pub struct MarketUpdate<'a> {
    pub owner_pid: u32,
    pub heartbeat_ms: u64,
    pub enabled: bool,
    pub symbol: &'a str,
    pub price: &'a str,
    pub tooltip: &'a str,
    /// AARRGGBB, or zero to inherit the taskbar foreground color.
    pub accent_argb: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketSnapshot {
    pub owner_pid: u32,
    pub heartbeat_ms: u64,
    pub enabled: bool,
    pub symbol: String,
    pub price: String,
    pub tooltip: String,
    pub accent_argb: u32,
    pub action_sequence: u32,
}

pub fn encode_utf16_truncated<const N: usize>(value: &str) -> [u16; N] {
    let mut output = [0_u16; N];
    if N == 0 {
        return output;
    }

    let mut written = 0;
    for character in value.chars() {
        let mut encoded = [0_u16; 2];
        let units = character.encode_utf16(&mut encoded);
        if written + units.len() >= N {
            break;
        }
        output[written..written + units.len()].copy_from_slice(units);
        written += units.len();
    }
    output
}

pub fn decode_utf16(value: &[u16]) -> String {
    let length = value
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(value.len());
    String::from_utf16_lossy(&value[..length])
}

#[cfg(windows)]
mod exports {
    use std::ffi::c_void;

    unsafe extern "C" {
        fn crypto_hud_taskbar_hook_impl(code: i32, wparam: usize, lparam: isize) -> isize;
        fn crypto_hud_taskbar_get_class_object_impl(
            class_id: *const c_void,
            interface_id: *const c_void,
            object: *mut *mut c_void,
        ) -> i32;
        fn crypto_hud_taskbar_can_unload_now_impl() -> i32;
    }

    #[no_mangle]
    pub unsafe extern "system" fn CryptoHudTaskbarHook(
        code: i32,
        wparam: usize,
        lparam: isize,
    ) -> isize {
        // SAFETY: Windows supplies the hook callback arguments. The native
        // boundary is noexcept and always chains to the next hook.
        unsafe { crypto_hud_taskbar_hook_impl(code, wparam, lparam) }
    }

    #[no_mangle]
    pub unsafe extern "system" fn DllGetClassObject(
        class_id: *const c_void,
        interface_id: *const c_void,
        object: *mut *mut c_void,
    ) -> i32 {
        // SAFETY: The native implementation validates all COM pointers.
        unsafe { crypto_hud_taskbar_get_class_object_impl(class_id, interface_id, object) }
    }

    #[no_mangle]
    pub extern "system" fn DllCanUnloadNow() -> i32 {
        // SAFETY: The native implementation reads only the C++/WinRT module
        // reference count.
        unsafe { crypto_hud_taskbar_can_unload_now_impl() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, offset_of, size_of};

    #[cfg(windows)]
    #[repr(C)]
    struct TestGuid {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    #[cfg(windows)]
    #[repr(C)]
    struct UnknownVtable {
        query_interface: usize,
        add_ref: usize,
        release: unsafe extern "system" fn(*mut std::ffi::c_void) -> u32,
    }

    #[test]
    fn shared_layout_matches_the_native_protocol() {
        assert_eq!(PROTOCOL_VERSION, 6);
        assert!(MAPPING_NAME.ends_with(".v6"));
        assert!(UPDATE_EVENT_NAME.ends_with(".v6"));
        assert!(ACTION_EVENT_NAME.ends_with(".v6"));
        assert!(UPDATE_MESSAGE_NAME.ends_with(".v6"));
        assert_eq!(align_of::<SharedMarketState>(), 8);
        assert_eq!(
            size_of::<SharedMarketState>(),
            usize::from(SHARED_STATE_SIZE)
        );
        assert_eq!(offset_of!(SharedMarketState, magic), 0);
        assert_eq!(offset_of!(SharedMarketState, version), 4);
        assert_eq!(offset_of!(SharedMarketState, size), 6);
        assert_eq!(offset_of!(SharedMarketState, sequence), 8);
        assert_eq!(offset_of!(SharedMarketState, owner_pid), 12);
        assert_eq!(offset_of!(SharedMarketState, heartbeat_ms), 16);
        assert_eq!(offset_of!(SharedMarketState, enabled), 24);
        assert_eq!(offset_of!(SharedMarketState, status), 28);
        assert_eq!(offset_of!(SharedMarketState, error_code), 32);
        assert_eq!(offset_of!(SharedMarketState, explorer_pid), 36);
        assert_eq!(offset_of!(SharedMarketState, symbol), 40);
        assert_eq!(offset_of!(SharedMarketState, price), 168);
        assert_eq!(offset_of!(SharedMarketState, tooltip), 296);
        assert_eq!(offset_of!(SharedMarketState, accent_argb), 680);
        assert_eq!(offset_of!(SharedMarketState, action_sequence), 684);
    }

    #[test]
    fn native_bridge_applies_the_accent_to_the_change_segment_only() {
        let source = include_str!("../native/taskbar_bridge.cpp");
        assert!(source.contains("CryptoHudTaskbarMarketV6"));
        assert!(source.contains("attachment.price_row.Children().Append(attachment.change);"));
        assert!(source.contains("attachment.change.Foreground(SolidColorBrush{color});"));
        assert!(!source.contains("attachment.price.Foreground(SolidColorBrush{color});"));
        assert!(source.contains("watcher\\towner-stale"));
        assert!(source.contains("Release the diagnostics callback when the host disappears."));
        assert!(source.contains("CoGetObjectContext("));
        assert!(source.contains("advise_context_->ContextCallback("));
        assert!(source.contains("stop_requested_.exchange(true"));
        assert!(source.contains("SUCCEEDED(TryCompleteStop())"));
        assert!(source.contains("owner_present ||"));
        assert!(source.contains("ReadOwnerPidBestEffort()"));
        assert!(source.contains("active_diagnostics_users_"));
        assert!(source.contains("last_owner_pid_"));
    }

    #[test]
    fn native_bridge_routes_right_tap_to_the_existing_host_tray_menu() {
        let source = include_str!("../native/taskbar_bridge.cpp");
        assert!(source.contains("SlintSystemTrayWindow"));
        assert!(source.contains("ShowHostTrayContextMenu()"));
        assert!(source.contains("attachment.root.RightTapped("));
        assert!(source.contains("static_cast<LPARAM>(WM_CONTEXTMENU)"));
        assert!(source.contains("args.Handled(true);"));
        assert!(source.contains("attachment.root.RightTapped(attachment.right_tapped_token);"));
    }

    #[test]
    fn utf16_encoding_is_null_terminated_and_does_not_split_surrogates() {
        let exact = encode_utf16_truncated::<5>("BTC!");
        assert_eq!(
            exact,
            [b'B' as u16, b'T' as u16, b'C' as u16, b'!' as u16, 0]
        );

        let truncated = encode_utf16_truncated::<4>("A🚀B");
        assert_eq!(decode_utf16(&truncated), "A🚀");
        assert_eq!(truncated[3], 0);

        let no_room_for_pair = encode_utf16_truncated::<3>("A🚀");
        assert_eq!(decode_utf16(&no_room_for_pair), "A");
    }

    #[test]
    fn publish_and_read_round_trip_a_consistent_snapshot() {
        let mut state = SharedMarketState::new(17);
        state.publish(&MarketUpdate {
            owner_pid: 42,
            heartbeat_ms: 1_234,
            enabled: true,
            symbol: "BTC/USDT",
            price: "62,000.00 +1.25%",
            tooltip: "BTC/USDT\n62,000.00 · +1.25%",
            accent_argb: 0xFF22_C55E,
        });

        assert_eq!(state.sequence & 1, 0);
        assert_eq!(
            state.read_consistent(),
            Some(MarketSnapshot {
                owner_pid: 42,
                heartbeat_ms: 1_234,
                enabled: true,
                symbol: "BTC/USDT".to_owned(),
                price: "62,000.00 +1.25%".to_owned(),
                tooltip: "BTC/USDT\n62,000.00 · +1.25%".to_owned(),
                accent_argb: 0xFF22_C55E,
                action_sequence: 0,
            })
        );
    }

    #[test]
    fn publisher_recovers_an_interrupted_odd_sequence() {
        let mut state = SharedMarketState::new(1);
        state.sequence = 7;
        state.publish(&MarketUpdate {
            owner_pid: 2,
            heartbeat_ms: 99,
            enabled: true,
            symbol: "ETH/USDT",
            price: "3,500",
            tooltip: "",
            accent_argb: 0,
        });

        assert_eq!(state.sequence & 1, 0);
        assert_eq!(
            state.read_consistent().map(|snapshot| snapshot.symbol),
            Some("ETH/USDT".to_owned())
        );
    }

    #[test]
    fn reader_rejects_invalid_or_permanently_busy_state() {
        let mut state = SharedMarketState::new(1);
        state.magic = 0;
        assert!(state.read_consistent().is_none());

        state.magic = PROTOCOL_MAGIC;
        state.sequence = 3;
        assert!(state.read_consistent().is_none());
    }

    #[test]
    fn status_values_are_stable() {
        for (raw, expected) in [
            (0, TaskbarStatus::Disabled),
            (1, TaskbarStatus::Initializing),
            (2, TaskbarStatus::WaitingForVisualTree),
            (3, TaskbarStatus::Attached),
            (4, TaskbarStatus::Detaching),
            (5, TaskbarStatus::Detached),
            (6, TaskbarStatus::Error),
        ] {
            assert_eq!(TaskbarStatus::from_raw(raw), Some(expected));
        }
        assert_eq!(TaskbarStatus::from_raw(7), None);
    }

    #[cfg(windows)]
    #[test]
    fn native_tap_class_factory_implements_the_frozen_clsid() {
        unsafe extern "C" {
            fn crypto_hud_taskbar_get_class_object_impl(
                class_id: *const std::ffi::c_void,
                interface_id: *const std::ffi::c_void,
                object: *mut *mut std::ffi::c_void,
            ) -> i32;
        }

        let tap_class = TestGuid {
            data1: 0x2304_531e,
            data2: 0xb59e,
            data3: 0x4f0e,
            data4: [0xb3, 0xa7, 0x80, 0x53, 0x50, 0x05, 0x07, 0x6a],
        };
        let class_factory_interface = TestGuid {
            data1: 1,
            data2: 0,
            data3: 0,
            data4: [0xc0, 0, 0, 0, 0, 0, 0, 0x46],
        };
        let mut object = std::ptr::null_mut();
        // SAFETY: Both GUIDs have the native GUID layout and `object` is a
        // writable COM out pointer.
        let result = unsafe {
            crypto_hud_taskbar_get_class_object_impl(
                std::ptr::addr_of!(tap_class).cast(),
                std::ptr::addr_of!(class_factory_interface).cast(),
                &mut object,
            )
        };
        assert_eq!(result, 0);
        assert!(!object.is_null());

        // SAFETY: A successful IClassFactory query returned an IUnknown-based
        // COM pointer whose first three vtable entries are stable.
        let remaining = unsafe {
            let vtable = *object.cast::<*const UnknownVtable>();
            ((*vtable).release)(object)
        };
        assert_eq!(remaining, 0);
    }
}
