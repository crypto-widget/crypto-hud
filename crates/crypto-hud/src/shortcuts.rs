use global_hotkey::hotkey::{HotKey, HotKeyParseError};

use crypto_hud_shell_state::ShortcutPreference;

#[cfg(not(windows))]
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

#[cfg(windows)]
mod windows_hotkey {
    use std::{
        sync::mpsc::{self, Receiver, Sender},
        thread::{self, JoinHandle},
        time::Duration,
    };

    use windows_sys::Win32::UI::{
        Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_NOREPEAT},
        WindowsAndMessaging::{PeekMessageW, MSG, PM_REMOVE, WM_HOTKEY},
    };

    use crypto_hud_shell_state::ShortcutPreference;

    const HOTKEY_ID: i32 = 0x4357;
    const VK_C: u32 = b'C' as u32;

    #[derive(Debug, Clone, Copy)]
    struct HotKeySpec {
        modifiers: u32,
        virtual_key: u32,
    }

    enum Command {
        Apply(ShortcutPreference, Sender<Result<(), String>>),
        Stop,
    }

    pub struct WindowsShortcutManager {
        command_tx: Sender<Command>,
        event_rx: Receiver<()>,
        worker: Option<JoinHandle<()>>,
    }

    impl WindowsShortcutManager {
        pub fn new() -> Result<Self, String> {
            let (command_tx, command_rx) = mpsc::channel();
            let (event_tx, event_rx) = mpsc::channel();
            let worker = thread::Builder::new()
                .name("crypto-hud-hotkey".to_string())
                .spawn(move || run_hotkey_thread(command_rx, event_tx))
                .map_err(|error| error.to_string())?;

            Ok(Self {
                command_tx,
                event_rx,
                worker: Some(worker),
            })
        }

        pub fn apply(&self, preference: ShortcutPreference) -> Result<(), String> {
            let (reply_tx, reply_rx) = mpsc::channel();
            self.command_tx
                .send(Command::Apply(preference, reply_tx))
                .map_err(|_| "global shortcut worker is unavailable".to_string())?;
            reply_rx
                .recv()
                .map_err(|_| "global shortcut worker stopped before replying".to_string())?
        }

        pub fn poll_toggle_requested(&self) -> bool {
            let mut requested = false;
            while self.event_rx.try_recv().is_ok() {
                requested = true;
            }
            requested
        }
    }

    impl Drop for WindowsShortcutManager {
        fn drop(&mut self) {
            let _ = self.command_tx.send(Command::Stop);
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        }
    }

    fn run_hotkey_thread(command_rx: Receiver<Command>, event_tx: Sender<()>) {
        let mut active = None;

        loop {
            while let Ok(command) = command_rx.try_recv() {
                match command {
                    Command::Apply(preference, reply_tx) => {
                        let result = apply_hotkey_preference(&mut active, preference);
                        let _ = reply_tx.send(result);
                    }
                    Command::Stop => {
                        unregister_active(&mut active);
                        return;
                    }
                }
            }

            pump_hotkey_messages(&event_tx);
            thread::sleep(Duration::from_millis(20));
        }
    }

    fn apply_hotkey_preference(
        active: &mut Option<HotKeySpec>,
        preference: ShortcutPreference,
    ) -> Result<(), String> {
        let previous = *active;
        unregister_active(active);

        let Some(next) = hotkey_spec_for_preference(preference) else {
            return Ok(());
        };

        if let Err(error) = register_hotkey(next) {
            if let Some(previous) = previous {
                if register_hotkey(previous).is_ok() {
                    *active = Some(previous);
                }
            }
            return Err(error);
        }

        *active = Some(next);
        Ok(())
    }

    fn unregister_active(active: &mut Option<HotKeySpec>) {
        if active.take().is_some() {
            unsafe {
                UnregisterHotKey(std::ptr::null_mut(), HOTKEY_ID);
            }
        }
    }

    fn register_hotkey(spec: HotKeySpec) -> Result<(), String> {
        let ok = unsafe {
            RegisterHotKey(
                std::ptr::null_mut(),
                HOTKEY_ID,
                spec.modifiers,
                spec.virtual_key,
            )
        };
        if ok == 0 {
            return Err(format!(
                "failed to register global shortcut: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }

    fn pump_hotkey_messages(event_tx: &Sender<()>) {
        let mut msg = MSG::default();
        loop {
            let has_message =
                unsafe { PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) };
            if has_message == 0 {
                break;
            }
            if msg.message == WM_HOTKEY && msg.wParam as i32 == HOTKEY_ID {
                let _ = event_tx.send(());
            }
        }
    }

    fn hotkey_spec_for_preference(preference: ShortcutPreference) -> Option<HotKeySpec> {
        let no_repeat = MOD_NOREPEAT;
        match preference {
            ShortcutPreference::AltC => Some(HotKeySpec {
                modifiers: MOD_ALT | no_repeat,
                virtual_key: VK_C,
            }),
            ShortcutPreference::CtrlSpace
            | ShortcutPreference::CtrlShiftSpace
            | ShortcutPreference::AltSpace
            | ShortcutPreference::Disabled => None,
        }
    }
}

#[cfg(windows)]
pub struct ShortcutManager {
    manager: Option<windows_hotkey::WindowsShortcutManager>,
}

#[cfg(windows)]
impl ShortcutManager {
    pub fn new() -> Self {
        let manager = windows_hotkey::WindowsShortcutManager::new()
            .map_err(|error| {
                eprintln!("failed to create global shortcut manager: {error}");
            })
            .ok();

        Self { manager }
    }

    pub fn apply(&mut self, preference: ShortcutPreference) -> Result<(), String> {
        let Some(manager) = &self.manager else {
            return if preference == ShortcutPreference::Disabled {
                Ok(())
            } else {
                Err("global shortcut manager is unavailable".to_string())
            };
        };

        manager.apply(preference)
    }

    pub fn poll_toggle_requested(&self) -> bool {
        self.manager
            .as_ref()
            .is_some_and(|manager| manager.poll_toggle_requested())
    }
}

#[cfg(not(windows))]
pub struct ShortcutManager {
    manager: Option<GlobalHotKeyManager>,
    active: Option<HotKey>,
}

#[cfg(not(windows))]
impl ShortcutManager {
    pub fn new() -> Self {
        let manager = GlobalHotKeyManager::new()
            .map_err(|error| {
                eprintln!("failed to create global shortcut manager: {error}");
            })
            .ok();

        Self {
            manager,
            active: None,
        }
    }

    pub fn apply(&mut self, preference: ShortcutPreference) -> Result<(), String> {
        let Some(manager) = &self.manager else {
            return if preference == ShortcutPreference::Disabled {
                self.active = None;
                Ok(())
            } else {
                Err("global shortcut manager is unavailable".to_string())
            };
        };

        let previous = self.active.take();
        if let Some(previous) = previous {
            if let Err(error) = manager.unregister(previous) {
                self.active = Some(previous);
                return Err(error.to_string());
            }
        }

        let Some(next) = hotkey_for_preference(preference).map_err(|error| error.to_string())?
        else {
            self.active = None;
            return Ok(());
        };

        if let Err(error) = manager.register(next) {
            if let Some(previous) = previous {
                let _ = manager.register(previous);
                self.active = Some(previous);
            }
            return Err(error.to_string());
        }

        self.active = Some(next);
        Ok(())
    }

    pub fn poll_toggle_requested(&self) -> bool {
        let Some(active_id) = self.active.map(|hotkey| hotkey.id()) else {
            return false;
        };

        let mut requested = false;
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.id == active_id && event.state == HotKeyState::Pressed {
                requested = true;
            }
        }
        requested
    }
}

#[cfg_attr(windows, allow(dead_code))]
pub fn hotkey_for_preference(
    preference: ShortcutPreference,
) -> Result<Option<HotKey>, HotKeyParseError> {
    match preference {
        ShortcutPreference::AltC => "alt+C".parse().map(Some),
        ShortcutPreference::CtrlSpace
        | ShortcutPreference::CtrlShiftSpace
        | ShortcutPreference::AltSpace
        | ShortcutPreference::Disabled => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_shortcut_preferences_to_hotkeys() {
        assert!(hotkey_for_preference(ShortcutPreference::AltC)
            .unwrap()
            .is_some());
        assert!(hotkey_for_preference(ShortcutPreference::CtrlSpace)
            .unwrap()
            .is_none());
        assert!(hotkey_for_preference(ShortcutPreference::CtrlShiftSpace)
            .unwrap()
            .is_none());
        assert!(hotkey_for_preference(ShortcutPreference::AltSpace)
            .unwrap()
            .is_none());
        assert!(hotkey_for_preference(ShortcutPreference::Disabled)
            .unwrap()
            .is_none());
    }
}
