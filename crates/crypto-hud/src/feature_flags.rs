use std::sync::atomic::{AtomicBool, Ordering};

// Keep alert-rule data in settings, but disable all product paths for now.
pub(crate) const ALERT_RULES_ENABLED: bool = false;

static GUI_SMOKE_OFFLINE_NETWORK_DISABLED: AtomicBool = AtomicBool::new(false);

pub(crate) fn set_gui_smoke_offline_network_disabled(disabled: bool) {
    GUI_SMOKE_OFFLINE_NETWORK_DISABLED.store(disabled, Ordering::Relaxed);
}

pub(crate) fn gui_smoke_offline_network_disabled() -> bool {
    GUI_SMOKE_OFFLINE_NETWORK_DISABLED.load(Ordering::Relaxed)
}
