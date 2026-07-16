#![windows_subsystem = "windows"]

mod autostart;
mod coin_icons;
mod desktop_shell;
mod feature_flags;
mod i18n;
mod notifications;
mod plugin;
mod runtime_bridge;
mod settings_window;
mod shortcuts;
mod state_bridge;
mod theme;
mod updater;
mod widget_host;
mod window_manager;

#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::path::PathBuf;
use std::{
    cell::{Cell, RefCell},
    env,
    rc::Rc,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use crypto_hud_market as market;
use crypto_hud_runtime::QuoteCache;
use crypto_hud_shell_state as settings;
use desktop_shell::{
    install_gui_smoke_ready_timer, install_gui_smoke_timer, install_instance_activation_timer,
    install_keepalive_window, install_single_instance_guard, install_tray, parse_launch_options,
    refresh_tray_text,
};
use runtime_bridge::{install_runtime_event_timer, sync_widget_runtimes, RuntimeEventTimerDeps};
#[cfg(test)]
use settings::{
    default_widget_config, AppSettings, LayoutStore, LegacyLayoutStore, WidgetInstance,
    WidgetKind as WidgetType, WidgetLayout,
};
use settings::{save_layout_store, state_dir_for_path, state_path};
use settings_window::{
    apply_dynamic_widget_auto_sizes_to_store, install_preview_carousel_timer,
    install_settings_window, SettingsWindowDeps,
};
use slint::{ComponentHandle, Timer, TimerMode};
use state_bridge::{
    add_plugin_instance, load_layout_store_with_diagnostics, market_subscriptions_from_store,
    normalize_store_with_catalog, widget_definitions_from_catalog,
};
#[cfg(test)]
use state_bridge::{
    add_widget_instance, default_layout_for_index, default_layout_for_size,
    default_layout_for_widget, default_symbols, layout_has_visible_area,
    layout_has_visible_area_for_size, migrate_legacy_store, normalize_store, parse_symbols,
    parse_symbols_for_type,
};
use widget_host::WidgetRuntime;
use window_manager::{
    apply_tray_hover_display, enter_settings_mode, install_hotkey_poll_timer,
    install_tray_hover_display_timer, install_widget_shell_window_maintenance_timer,
    schedule_settings_window_configuration, schedule_widget_shell_window_configuration,
    TrayHoverDisplayState,
};
#[cfg(test)]
use window_manager::{desktop_size, widget_pin_to_top};

slint::include_modules!();

fn shortcut_registration_status(
    settings: &settings::AppSettings,
    error: impl std::fmt::Display,
) -> String {
    let locale = i18n::resolve_locale(settings.language);
    i18n::status_failure_message(locale, i18n::text(locale).status_shortcut_failed, error)
}

#[cfg(test)]
const LEGACY_DEFAULT_POSITION_X: i32 = settings::DEFAULT_WIDGET_POSITION_X;
#[cfg(test)]
const LEGACY_DEFAULT_POSITION_Y: i32 = settings::DEFAULT_WIDGET_POSITION_Y;
#[cfg(test)]
const LEGACY_DEFAULT_POSITION_STEP: i32 = settings::LEGACY_DEFAULT_POSITION_STEP;
#[cfg(test)]
const DEFAULT_LAYOUT_GAP: i32 = settings::DEFAULT_LAYOUT_GAP;
pub(crate) const ABOUT_REPOSITORY_URL: &str = "https://github.com/crypto-widget/crypto-hud";
pub(crate) const RELEASES_URL: &str = "https://github.com/crypto-widget/crypto-hud/releases";
pub(crate) const WIDGET_REORDER_DOUBLE_CLICK_TIMEOUT: Duration = Duration::from_millis(500);

#[cfg(test)]
#[derive(Debug, Clone, Copy)]
struct WidgetRect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[cfg(test)]
fn widget_rect(layout: &WidgetLayout, _widget_type: WidgetType) -> WidgetRect {
    WidgetRect {
        x: layout.x,
        y: layout.y,
        width: layout.width,
        height: layout.height,
    }
}

#[cfg(test)]
impl WidgetRect {
    fn overlaps(self, other: WidgetRect, gap: i32) -> bool {
        self.x < other.x + other.width + gap
            && self.x + self.width + gap > other.x
            && self.y < other.y + other.height + gap
            && self.y + self.height + gap > other.y
    }
}

fn install_gui_smoke_settings_interaction_timer(
    settings_window: slint::Weak<SettingsWindow>,
    interaction_complete: Rc<Cell<bool>>,
) -> Option<Timer> {
    env::var_os("CRYPTO_HUD_GUI_SMOKE_SETTINGS_INTERACTION")?;

    let timer = Timer::default();
    timer.start(
        TimerMode::SingleShot,
        Duration::from_millis(120),
        move || {
            if let Some(ui) = settings_window.upgrade() {
                ui.set_selected_widget_index(0);
                let widget_name = ui.get_widget_name_input_text();
                let always_on_top = ui.get_widgets_always_on_top();
                let layout_locked = ui.get_widget_layout_locked();
                let opacity_percent = ui.get_opacity_percent();
                let scale_percent = ui.get_widget_scale_percent();
                let theme_index = ui.get_widget_theme_index();

                ui.invoke_apply_widget_settings(
                    0,
                    widget_name,
                    always_on_top,
                    layout_locked,
                    opacity_percent,
                    scale_percent,
                    theme_index,
                    false,
                    true,
                    true,
                );
                ui.invoke_remove_widget_symbol(0, 1);
                ui.invoke_open_widget_symbol_picker();
                ui.invoke_confirm_symbol_picker(0, 0);
            }
            let interaction_complete = interaction_complete.clone();
            Timer::single_shot(Duration::from_millis(140), move || {
                interaction_complete.set(true);
            });
        },
    );
    Some(timer)
}

struct PluginReloadWatcher {
    observed: plugin::PluginTreeFingerprint,
    applied: plugin::PluginTreeFingerprint,
    stable_ticks: u8,
}

impl PluginReloadWatcher {
    fn new(initial: plugin::PluginTreeFingerprint) -> Self {
        Self {
            observed: initial.clone(),
            applied: initial,
            stable_ticks: 0,
        }
    }

    fn observe(&mut self, current: plugin::PluginTreeFingerprint) -> bool {
        if current != self.observed {
            self.observed = current;
            self.stable_ticks = 0;
            return false;
        }
        if current == self.applied {
            return false;
        }
        self.stable_ticks = self.stable_ticks.saturating_add(1);
        if self.stable_ticks < 2 {
            return false;
        }
        self.applied = current;
        self.stable_ticks = 0;
        true
    }
}

fn install_plugin_hot_reload_timer(
    state_dir: std::path::PathBuf,
    settings_window: slint::Weak<SettingsWindow>,
) -> Timer {
    let initial = plugin::plugin_tree_fingerprint(&state_dir);
    let mut watcher = PluginReloadWatcher::new(initial);
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_millis(750), move || {
        let current = plugin::plugin_tree_fingerprint(&state_dir);
        if watcher.observe(current) {
            if let Some(ui) = settings_window.upgrade() {
                ui.invoke_reload_custom_components();
            }
        }
    });
    timer
}

fn offline_gui_smoke_enabled(gui_smoke_requested: bool, flag: Option<&str>) -> bool {
    gui_smoke_requested && flag == Some("1")
}

fn deterministic_gui_smoke_market_events(
    subscriptions: &[market::MarketSubscription],
) -> Vec<market::MarketEvent> {
    subscriptions
        .iter()
        .enumerate()
        .map(|(index, subscription)| {
            let price = 100.0 + index as f64 * 25.0;
            let timestamp = 1_700_000_000_000 + index as u64 * 1_000_000;
            let chart_candles_24h = if subscription.needs_candles {
                (0_u32..4)
                    .map(|offset| {
                        let open = price + f64::from(offset) * 0.5;
                        let close = open + 0.25;
                        market::MarketCandle {
                            open_time_millis: timestamp + u64::from(offset) * 300_000,
                            open,
                            high: close + 0.25,
                            low: open - 0.25,
                            close,
                        }
                    })
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let chart_closes_24h = chart_candles_24h
                .iter()
                .map(|candle| candle.close)
                .collect();
            market::MarketEvent::Snapshot(market::MarketSnapshot {
                symbol: subscription.symbol.clone(),
                price,
                change_percent_24h: 1.0 + index as f64 * 0.1,
                chart_closes_24h,
                chart_candles_24h,
                chart_updated_at: subscription.needs_candles.then(Instant::now),
                chart_error: None,
                source: crypto_hud_core::market_pair_source(&subscription.symbol)
                    .unwrap_or_default(),
            })
        })
        .collect()
}

fn main() -> Result<()> {
    if env::var_os("SLINT_BACKEND").is_none() {
        env::set_var("SLINT_BACKEND", "software");
    }

    let (single_instance, instance_activation) = install_single_instance_guard()?;
    if !single_instance.is_single() {
        instance_activation.request_activation()?;
        return Ok(());
    }

    let launch_options = parse_launch_options();
    let offline_gui_smoke = offline_gui_smoke_enabled(
        launch_options.gui_smoke_exit_after.is_some(),
        env::var("CRYPTO_HUD_GUI_SMOKE_OFFLINE").ok().as_deref(),
    );
    feature_flags::set_gui_smoke_offline_network_disabled(offline_gui_smoke);
    let requested_widget_count = if launch_options.each_widget {
        0
    } else {
        launch_options.widget_count
    };
    let state_path = state_path()?;
    let state_dir = state_dir_for_path(&state_path);
    if let Err(error) = plugin::sync_user_plugin_development_guide(&state_dir) {
        eprintln!("failed to sync custom plugin development guide: {error:#}");
    }
    let plugin_catalog = Rc::new(plugin::PluginCatalog::load(&state_dir));
    for error in plugin_catalog.errors() {
        eprintln!("plugin catalog warning: {error}");
    }
    let plugin_definitions = widget_definitions_from_catalog(&plugin_catalog);
    let loaded_layout_store = load_layout_store_with_diagnostics(
        &state_path,
        requested_widget_count,
        &plugin_definitions,
    );
    let state_load_warning = loaded_layout_store
        .warning
        .as_ref()
        .map(ToString::to_string);
    if let Some(warning) = &state_load_warning {
        eprintln!("layout state recovery warning: {warning}");
    }
    let mut layout_store = loaded_layout_store.store;
    seed_each_widget_on_empty_start(
        &mut layout_store,
        launch_options.each_widget,
        &plugin_catalog,
    );
    if apply_dynamic_widget_auto_sizes_to_store(&mut layout_store, &plugin_catalog) {
        if let Err(error) = save_layout_store(&state_path, &layout_store) {
            eprintln!("failed to save migrated dynamic widget layout: {error:#}");
        }
    }
    let layouts = Rc::new(RefCell::new(layout_store));
    let app_settings = layouts.borrow().settings.clone().normalized();
    #[cfg(windows)]
    if let Err(error) = autostart::refresh_auto_start_registration_if_enabled(
        app_settings.auto_start_enabled,
        layouts.borrow().widgets.len().max(1),
    ) {
        eprintln!("failed to refresh auto-start registration: {error}");
    }
    let settings_status = Rc::new(RefCell::new(state_load_warning.unwrap_or_default()));
    let shortcut_manager = Rc::new(RefCell::new(shortcuts::ShortcutManager::new()));
    let market_subscriptions = market_subscriptions_from_store(&layouts.borrow(), &plugin_catalog);
    let market_feed_config = Arc::new(Mutex::new(market::MarketFeedConfig {
        subscriptions: market_subscriptions.clone(),
        provider: app_settings.market_provider,
        refresh_interval_seconds: app_settings.refresh_interval_seconds,
        enabled_sources: settings::enabled_market_sources(&app_settings),
        proxy_url: settings::effective_network_proxy_url(&app_settings),
    }));
    if let Err(error) = shortcut_manager.borrow_mut().apply(app_settings.shortcut) {
        let shortcut_status = shortcut_registration_status(&app_settings, error);
        let mut status = settings_status.borrow_mut();
        if !status.is_empty() {
            status.push_str(" | ");
        }
        status.push_str(&shortcut_status);
    }

    let quote_cache = Rc::new(RefCell::new(QuoteCache::new()));
    let coin_icons = Rc::new(coin_icons::CoinIconRegistry::new(
        state_dir.join("coin-icons"),
    ));
    let widgets = Rc::new(RefCell::new(Vec::<WidgetRuntime>::new()));
    sync_widget_runtimes(
        &widgets,
        &layouts,
        &state_path,
        quote_cache.clone(),
        coin_icons.clone(),
        true,
        plugin_catalog.clone(),
    )?;
    schedule_widget_shell_window_configuration();

    let widgets_hidden = Rc::new(RefCell::new(false));
    let settings_mode_active = Rc::new(RefCell::new(false));
    let tray_handle = Rc::new(RefCell::new(None));
    let tray_hover_state = Rc::new(RefCell::new(TrayHoverDisplayState::default()));
    let settings_window = install_settings_window(SettingsWindowDeps {
        widgets: widgets.clone(),
        layouts: layouts.clone(),
        state_path: state_path.clone(),
        settings_status: settings_status.clone(),
        shortcut_manager: shortcut_manager.clone(),
        market_feed_config: market_feed_config.clone(),
        widgets_hidden: widgets_hidden.clone(),
        settings_mode_active: settings_mode_active.clone(),
        tray_handle: tray_handle.clone(),
        tray_hover_state: tray_hover_state.clone(),
        quote_cache: quote_cache.clone(),
        coin_icons: coin_icons.clone(),
        plugin_catalog: plugin_catalog.clone(),
    })?;
    let plugin_hot_reload_timer =
        install_plugin_hot_reload_timer(state_dir.clone(), settings_window.as_weak());
    let preview_carousel_timer = install_preview_carousel_timer(settings_window.as_weak());
    let tray = install_tray(
        widgets.clone(),
        settings_window.as_weak(),
        layouts.clone(),
        state_path.clone(),
        settings_mode_active.clone(),
        plugin_catalog.clone(),
    )?;
    *tray_handle.borrow_mut() = Some(tray.as_weak());
    let show_main_window_on_startup =
        app_settings.show_main_window_on_startup || launch_options.show_settings;
    refresh_tray_text(&tray, app_settings);
    apply_tray_hover_display(
        &widgets,
        &layouts,
        &widgets_hidden,
        &tray_hover_state,
        notifications::tray_icon_hovered(),
    );
    if show_main_window_on_startup {
        enter_settings_mode(&widgets, &layouts, &settings_mode_active);
        settings_window
            .show()
            .context("failed to show settings window on startup")?;
        schedule_settings_window_configuration();
    }
    let gui_smoke_settings_interaction_complete = Rc::new(Cell::new(false));
    let gui_smoke_settings_interaction_timer = install_gui_smoke_settings_interaction_timer(
        settings_window.as_weak(),
        gui_smoke_settings_interaction_complete.clone(),
    );
    if gui_smoke_settings_interaction_timer.is_none() {
        gui_smoke_settings_interaction_complete.set(true);
    }
    let keepalive_window = install_keepalive_window()?;
    let gui_smoke_timer = install_gui_smoke_timer(launch_options.gui_smoke_exit_after);
    let hotkey_timer = install_hotkey_poll_timer(
        shortcut_manager.clone(),
        widgets.clone(),
        layouts.clone(),
        widgets_hidden.clone(),
        tray.as_weak(),
    );
    let tray_hover_timer = install_tray_hover_display_timer(
        widgets.clone(),
        layouts.clone(),
        widgets_hidden.clone(),
        tray_hover_state.clone(),
    );
    let widget_shell_window_maintenance_timer = install_widget_shell_window_maintenance_timer();
    let instance_activation_timer = install_instance_activation_timer(
        instance_activation,
        settings_window.as_weak(),
        widgets.clone(),
        layouts.clone(),
        state_path.clone(),
        settings_mode_active.clone(),
        plugin_catalog.clone(),
    );

    let market_updates = if offline_gui_smoke {
        market::MarketFeed::from_events(deterministic_gui_smoke_market_events(
            &market_subscriptions,
        ))
    } else {
        market::spawn_market_feed(market_feed_config)
    };
    let update_events = if launch_options.gui_smoke_exit_after.is_some() {
        None
    } else {
        updater::UpdateCheckConfig::from_env().map(|mut config| {
            config.proxy_url = settings::effective_network_proxy_url(
                &layouts.borrow().settings.clone().normalized(),
            );
            updater::spawn_update_check(config)
        })
    };
    let runtime_event_timer = install_runtime_event_timer(RuntimeEventTimerDeps {
        widgets: widgets.clone(),
        layouts: layouts.clone(),
        quote_cache: quote_cache.clone(),
        coin_icons: coin_icons.clone(),
        plugin_catalog: plugin_catalog.clone(),
        settings_window: settings_window.as_weak(),
        market_updates,
        update_events,
    });
    let gui_smoke_ready_timer = install_gui_smoke_ready_timer(
        launch_options.gui_smoke_exit_after,
        widgets.clone(),
        layouts.clone(),
        quote_cache.clone(),
        plugin_catalog.clone(),
        show_main_window_on_startup,
        gui_smoke_settings_interaction_complete,
    );

    let event_loop_result = slint::run_event_loop_until_quit().context("Slint event loop failed");
    drop(gui_smoke_ready_timer);
    drop(runtime_event_timer);
    drop(instance_activation_timer);
    drop(widget_shell_window_maintenance_timer);
    drop(gui_smoke_settings_interaction_timer);
    drop(plugin_hot_reload_timer);
    drop(preview_carousel_timer);
    drop(tray_hover_timer);
    drop(hotkey_timer);
    drop(gui_smoke_timer);
    drop(keepalive_window);
    drop(tray);
    drop(settings_window);
    drop(single_instance);
    event_loop_result
}

fn seed_each_widget_on_empty_start(
    store: &mut settings::LayoutStore,
    each_widget: bool,
    plugin_catalog: &plugin::PluginCatalog,
) -> bool {
    if !each_widget || !store.widgets.is_empty() {
        return false;
    }

    let app_settings = store.settings.clone().normalized();
    let mut slot = 0;
    for plugin in plugin_catalog
        .market_plugins()
        .filter(|plugin| plugin.is_available())
    {
        add_plugin_instance(store, &plugin, &app_settings);
        slot += 1;
    }

    if slot == 0 {
        return false;
    }

    store.selected_widget_id = store.widgets.first().map(|widget| widget.id.clone());
    normalize_store_with_catalog(store, 0, Some(plugin_catalog));
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_gui_smoke_requires_both_launch_option_and_exact_flag() {
        assert!(offline_gui_smoke_enabled(true, Some("1")));
        assert!(!offline_gui_smoke_enabled(false, Some("1")));
        assert!(!offline_gui_smoke_enabled(true, None));
        assert!(!offline_gui_smoke_enabled(true, Some("true")));
        assert!(!offline_gui_smoke_enabled(true, Some("0")));
    }

    #[test]
    fn plugin_reload_watcher_waits_for_two_stable_observations() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let state_dir = std::env::temp_dir().join(format!(
            "crypto-hud-plugin-reload-watcher-{}-{unique}",
            std::process::id()
        ));
        let initial = plugin::plugin_tree_fingerprint(&state_dir);
        let mut watcher = PluginReloadWatcher::new(initial);
        let plugin_dir = plugin::user_plugin_root(&state_dir).join("com.example.changed");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        std::fs::write(plugin_dir.join(plugin::MANIFEST_FILE_NAME), "{}").unwrap();
        let changed = plugin::plugin_tree_fingerprint(&state_dir);

        assert!(!watcher.observe(changed.clone()));
        assert!(!watcher.observe(changed.clone()));
        assert!(watcher.observe(changed.clone()));
        assert!(!watcher.observe(changed));

        let _ = std::fs::remove_dir_all(state_dir);
    }

    #[test]
    fn deterministic_gui_smoke_events_respect_candle_requirements() {
        let subscriptions = vec![
            market::MarketSubscription {
                symbol: "binance:spot:BTC/USDT".to_string(),
                needs_candles: false,
            },
            market::MarketSubscription {
                symbol: "coinbase:spot:ETH/USD".to_string(),
                needs_candles: true,
            },
        ];
        let events = deterministic_gui_smoke_market_events(&subscriptions);

        assert_eq!(events.len(), subscriptions.len());
        for (event, subscription) in events.iter().zip(&subscriptions) {
            let market::MarketEvent::Snapshot(snapshot) = event else {
                panic!("offline GUI smoke should only emit snapshots");
            };
            assert_eq!(snapshot.symbol, subscription.symbol);
            assert!(snapshot.price.is_finite() && snapshot.price > 0.0);
            assert!(snapshot.change_percent_24h.is_finite());
            let expected_candles = if subscription.needs_candles { 4 } else { 0 };
            assert_eq!(snapshot.chart_closes_24h.len(), expected_candles);
            assert_eq!(snapshot.chart_candles_24h.len(), expected_candles);
            assert_eq!(
                snapshot.chart_updated_at.is_some(),
                subscription.needs_candles
            );
            assert!(snapshot.chart_error.is_none());
            assert!(snapshot.chart_candles_24h.iter().all(|candle| {
                [candle.open, candle.high, candle.low, candle.close]
                    .into_iter()
                    .all(|value| value.is_finite() && value > 0.0)
                    && candle.low <= candle.open
                    && candle.open <= candle.high
                    && candle.low <= candle.close
                    && candle.close <= candle.high
            }));
        }
    }

    fn local_test_plugin_definition() -> plugin::PluginDefinition {
        plugin::PluginDefinition {
            id: "com.example.stage3-price-card".to_string(),
            name: "Stage 3 Price Card".to_string(),
            version: semver::Version::new(1, 0, 0),
            schema_version: plugin::PLUGIN_MANIFEST_SCHEMA_VERSION,
            host_api_version: semver::VersionReq::STAR,
            source: plugin::PluginSource::LocalUnsigned,
            renderer: plugin::PluginRendererDefinition::Slint {
                root_dir: PathBuf::from("plugins/com.example.stage3-price-card"),
                entry: PathBuf::from("plugins/com.example.stage3-price-card/ui/main.slint"),
                component: "Stage3PriceCard".to_string(),
                definition: None,
            },
            default_size: plugin::PluginSize {
                width: 260,
                height: 170,
            },
            size_policy: plugin::PluginSizePolicy::Fixed,
            min_symbol_limit: 1,
            symbol_limit: 2,
            default_symbols: Vec::new(),
            preview_images: Vec::new(),
            themes: plugin::single_default_theme(),
            data_requirements: vec![plugin::PluginDataRequirement {
                capability: "market.price".to_string(),
            }],
            parameters: Vec::new(),
            status: plugin::PluginStatus::Available,
        }
    }

    #[test]
    fn app_settings_normalize_opacity_bounds() {
        assert_eq!(settings::MIN_OPACITY_PERCENT, 20);
        assert_eq!(settings::clamp_opacity(12), settings::MIN_OPACITY_PERCENT);
        assert_eq!(settings::clamp_opacity(70), 70);
        assert_eq!(settings::clamp_opacity(140), settings::MAX_OPACITY_PERCENT);
    }

    #[test]
    fn startup_shortcut_failure_status_uses_selected_language() {
        let settings = AppSettings {
            language: settings::LanguagePreference::Ar,
            ..AppSettings::default()
        };

        assert_eq!(
            shortcut_registration_status(&settings, "denied"),
            "فشل تسجيل الاختصار: \u{2066}denied\u{2069}"
        );
    }

    #[test]
    fn default_layout_uses_global_widget_settings() {
        let settings = AppSettings {
            widgets_always_on_top: false,
            opacity_percent: 64,
            widget_scale_percent: 125,
            ..AppSettings::default()
        };

        let layout = default_layout_for_index(2, settings);

        assert!(!layout.always_on_top);
        assert_eq!(layout.opacity_percent, 64);
        assert_eq!(layout.width, 358);
        assert_eq!(layout.height, 243);
    }

    #[test]
    fn default_layout_avoids_overlap_between_quote_boards() {
        let settings = AppSettings::default();
        let first = default_layout_for_widget(0, WidgetType::QuoteBoard, settings.clone());
        let second = default_layout_for_widget(1, WidgetType::QuoteBoard, settings);

        assert!(!widget_rect(&first, WidgetType::QuoteBoard).overlaps(
            widget_rect(&second, WidgetType::QuoteBoard),
            DEFAULT_LAYOUT_GAP,
        ));
    }

    #[test]
    fn add_widget_instance_uses_first_available_non_overlapping_layout() {
        let settings = AppSettings::default();
        let mut store = LayoutStore::default();

        add_widget_instance(&mut store, WidgetType::QuoteBoard, &settings);
        add_widget_instance(&mut store, WidgetType::QuoteBoard, &settings);

        assert_eq!(store.widgets.len(), 2);
        assert!(
            !widget_rect(&store.widgets[0].layout, WidgetType::QuoteBoard).overlaps(
                widget_rect(&store.widgets[1].layout, WidgetType::QuoteBoard),
                DEFAULT_LAYOUT_GAP,
            )
        );
        assert!(store.widgets.iter().all(|widget| {
            layout_has_visible_area_for_size(
                &widget.layout,
                (widget.layout.width, widget.layout.height),
            )
        }));
    }

    #[test]
    fn add_plugin_instance_uses_local_manifest_metadata() {
        let settings = AppSettings {
            market_default_symbols: vec!["BTC".to_string(), "ETH".to_string(), "SOL".to_string()],
            ..AppSettings::default()
        };
        let mut store = LayoutStore::default();
        let plugin = local_test_plugin_definition();

        let id = add_plugin_instance(&mut store, &plugin, &settings);

        assert_eq!(id, "plugin-card-1");
        assert_eq!(store.widgets.len(), 1);
        assert_eq!(store.widgets[0].plugin_id, "com.example.stage3-price-card");
        assert_eq!(store.widgets[0].name, "Stage 3 Price Card 1");
        assert_eq!(
            store.widgets[0].symbols,
            vec!["binance:spot:BTC/USDT", "binance:spot:ETH/USDT"]
        );
        assert_eq!(
            store.widgets[0].layout.x,
            default_layout_for_size(0, (260, 170), settings).x
        );
    }

    #[test]
    fn normalizing_multi_widget_store_migrates_legacy_default_cascade() {
        let mut store = LayoutStore {
            settings: AppSettings::default(),
            widgets: vec![
                WidgetInstance {
                    id: "quote-board-1".to_string(),
                    plugin_id: WidgetType::QuoteBoard.plugin_id().to_string(),
                    legacy_widget_type: None,
                    name: String::new(),
                    visible: true,
                    layout: WidgetLayout {
                        x: LEGACY_DEFAULT_POSITION_X,
                        y: LEGACY_DEFAULT_POSITION_Y,
                        always_on_top: true,
                        opacity_percent: 92,
                        ..WidgetLayout::default()
                    },
                    symbols: default_symbols(),
                    config: default_widget_config(),
                },
                WidgetInstance {
                    id: "quote-board-2".to_string(),
                    plugin_id: WidgetType::QuoteBoard.plugin_id().to_string(),
                    legacy_widget_type: None,
                    name: String::new(),
                    visible: true,
                    layout: WidgetLayout {
                        x: LEGACY_DEFAULT_POSITION_X + LEGACY_DEFAULT_POSITION_STEP,
                        y: LEGACY_DEFAULT_POSITION_Y + LEGACY_DEFAULT_POSITION_STEP,
                        always_on_top: true,
                        opacity_percent: 92,
                        ..WidgetLayout::default()
                    },
                    symbols: default_symbols(),
                    config: default_widget_config(),
                },
            ],
            ..LayoutStore::default()
        };

        normalize_store(&mut store, 0);

        assert!(
            !widget_rect(&store.widgets[0].layout, WidgetType::QuoteBoard).overlaps(
                widget_rect(&store.widgets[1].layout, WidgetType::QuoteBoard),
                DEFAULT_LAYOUT_GAP,
            )
        );
        assert!(store.widgets.iter().all(|widget| {
            layout_has_visible_area_for_size(
                &widget.layout,
                (widget.layout.width, widget.layout.height),
            )
        }));
    }

    #[test]
    fn normalizing_store_recovers_offscreen_widget_positions() {
        let (desktop_width, desktop_height) = desktop_size();
        let mut store = LayoutStore {
            settings: AppSettings::default(),
            widgets: vec![WidgetInstance {
                id: "quote-board-1".to_string(),
                plugin_id: WidgetType::QuoteBoard.plugin_id().to_string(),
                legacy_widget_type: None,
                name: String::new(),
                visible: true,
                layout: WidgetLayout {
                    x: desktop_width + 10,
                    y: desktop_height + 10,
                    always_on_top: true,
                    opacity_percent: 92,
                    ..WidgetLayout::default()
                },
                symbols: default_symbols(),
                config: default_widget_config(),
            }],
            ..LayoutStore::default()
        };

        normalize_store(&mut store, 0);

        assert!(layout_has_visible_area(
            &store.widgets[0].layout,
            WidgetType::QuoteBoard
        ));
        assert_ne!(store.widgets[0].layout.x, desktop_width + 10);
        assert_ne!(store.widgets[0].layout.y, desktop_height + 10);
    }

    #[test]
    fn settings_mode_preserves_pinned_widgets_topmost() {
        let instance = WidgetInstance {
            id: "quote-board-1".to_string(),
            plugin_id: WidgetType::QuoteBoard.plugin_id().to_string(),
            legacy_widget_type: None,
            name: String::new(),
            visible: true,
            layout: WidgetLayout {
                always_on_top: true,
                ..WidgetLayout::default()
            },
            symbols: default_symbols(),
            config: default_widget_config(),
        };

        assert!(widget_pin_to_top(&instance));
    }

    #[test]
    fn parses_pair_input_to_unique_base_symbols() {
        assert_eq!(
            parse_symbols("btc, ETHUSDT sol/usdt SOL-USDT"),
            vec![
                "binance:spot:BTC/USDT",
                "binance:spot:ETH/USDT",
                "binance:spot:SOL/USDT"
            ]
        );
    }

    #[test]
    fn empty_pair_input_uses_default_symbols() {
        assert_eq!(parse_symbols("  , ; "), default_symbols());
    }

    #[test]
    fn pair_input_is_capped_to_widget_capacity() {
        let input = (0..25)
            .map(|index| format!("COIN{index}"))
            .collect::<Vec<_>>()
            .join(", ");
        let expected = (0..20)
            .map(|index| format!("binance:spot:COIN{index}/USDT"))
            .collect::<Vec<_>>();

        assert_eq!(parse_symbols(&input), expected);
    }

    #[test]
    fn mini_ticker_pair_input_is_capped_to_one_symbol() {
        assert_eq!(
            parse_symbols_for_type("BTC, ETH, SOL", WidgetType::MiniTicker),
            vec!["binance:spot:BTC/USDT"]
        );
    }

    #[test]
    fn market_usage_text_follows_locale() {
        let store = LayoutStore {
            widgets: vec![
                WidgetInstance {
                    id: "quote-board-1".to_string(),
                    plugin_id: WidgetType::QuoteBoard.plugin_id().to_string(),
                    legacy_widget_type: None,
                    name: String::new(),
                    visible: true,
                    layout: WidgetLayout::default(),
                    symbols: vec!["BTC".to_string()],
                    config: default_widget_config(),
                },
                WidgetInstance {
                    id: "quote-board-2".to_string(),
                    plugin_id: WidgetType::QuoteBoard.plugin_id().to_string(),
                    legacy_widget_type: None,
                    name: String::new(),
                    visible: true,
                    layout: WidgetLayout::default(),
                    symbols: vec!["ETH".to_string()],
                    config: default_widget_config(),
                },
            ],
            ..Default::default()
        };

        assert_eq!(
            settings_window::widget_type_usage_text(
                &store,
                WidgetType::QuoteBoard,
                i18n::Locale::En
            ),
            "Used 2"
        );
        assert_eq!(
            settings_window::widget_type_usage_text(
                &store,
                WidgetType::QuoteBoard,
                i18n::Locale::ZhHans
            ),
            "已使用 2 个"
        );
    }

    #[test]
    fn plugin_market_item_preserves_local_plugin_metadata_in_every_locale() {
        let store = LayoutStore {
            widgets: vec![WidgetInstance {
                id: "plugin-card-1".to_string(),
                plugin_id: "com.example.stage3-price-card".to_string(),
                legacy_widget_type: None,
                name: "Stage 3 Price Card 1".to_string(),
                visible: true,
                layout: WidgetLayout::default(),
                symbols: vec!["BTC".to_string(), "ETH".to_string()],
                config: default_widget_config(),
            }],
            ..Default::default()
        };
        let plugin = local_test_plugin_definition();

        for locale in i18n::Locale::ALL {
            let item = settings_window::plugin_market_item(&plugin, &store, locale);

            assert_eq!(
                item.title.as_str(),
                "Stage 3 Price Card",
                "local plugin manifest names should stay exact for {locale:?}"
            );
            assert_eq!(item.status.as_str(), "");
            assert!(item.available);
            assert!(!item.builtin);
            assert_eq!(item.symbol_limit, 2);
            assert_eq!(item.preview_kind, 0);
            assert_eq!(item.preview_image_count, 0);
            assert!(item.description.as_str().contains("260x170"));
            assert!(!item.description.as_str().contains("market.price"));
        }

        let item = settings_window::plugin_market_item(&plugin, &store, i18n::Locale::En);
        assert_eq!(item.usage.as_str(), "Used 1");
        assert!(item.description.as_str().contains("prices"));

        let ar_item = settings_window::plugin_market_item(&plugin, &store, i18n::Locale::Ar);
        assert!(ar_item.description.as_str().contains("الأسعار"));
        assert!(!ar_item.description.as_str().contains("market.price"));
    }

    #[test]
    fn plugin_market_item_maps_design_preview_kinds() {
        let store = LayoutStore::default();
        for (plugin_id, expected_preview_kind) in [
            ("com.cryptohud.focus-ticker", 1),
            ("com.cryptohud.market-board", 2),
            ("com.cryptohud.trust-card", 3),
            ("com.cryptohud.market-compass", 5),
            ("com.cryptohud.status-strip", 6),
        ] {
            let mut plugin = local_test_plugin_definition();
            plugin.id = plugin_id.to_string();

            let item = settings_window::plugin_market_item(&plugin, &store, i18n::Locale::En);

            assert_eq!(item.preview_kind, expected_preview_kind);
        }
    }

    #[test]
    fn builtin_slint_plugin_market_titles_follow_locale() {
        let store = LayoutStore::default();
        let mut plugin = local_test_plugin_definition();
        plugin.id = "com.cryptohud.market-compass".to_string();
        plugin.name = "Market Compass".to_string();
        plugin.source = plugin::PluginSource::Builtin;

        let zh_item = settings_window::plugin_market_item(&plugin, &store, i18n::Locale::ZhHans);
        assert_eq!(zh_item.title.as_str(), "市场罗盘");
        assert_eq!(
            zh_item.description.as_str(),
            "环形多交易对视图，方便快速观察市场轮动。"
        );

        let ar_item = settings_window::plugin_market_item(&plugin, &store, i18n::Locale::Ar);
        assert_eq!(ar_item.title.as_str(), "بوصلة السوق");
        assert_eq!(
            ar_item.description.as_str(),
            "عرض دائري لعدة أزواج لرصد دوران السوق بسرعة."
        );

        plugin.source = plugin::PluginSource::LocalUnsigned;
        let local_item = settings_window::plugin_market_item(&plugin, &store, i18n::Locale::ZhHans);
        assert_eq!(local_item.title.as_str(), "Market Compass");
        assert!(local_item.description.as_str().contains("260x170"));
    }

    #[test]
    fn local_plugin_symbols_are_capped_by_plugin_definition() {
        let plugin = local_test_plugin_definition();
        let catalog = plugin::PluginCatalog::from_plugins_for_tests(vec![plugin]);
        let mut store = LayoutStore {
            widgets: vec![WidgetInstance {
                id: "plugin-card-1".to_string(),
                plugin_id: "com.example.stage3-price-card".to_string(),
                legacy_widget_type: None,
                name: "Stage 3 Price Card 1".to_string(),
                visible: true,
                layout: WidgetLayout::default(),
                symbols: vec![
                    "BTC".to_string(),
                    "ETH".to_string(),
                    "SOL".to_string(),
                    "BNB".to_string(),
                ],
                config: default_widget_config(),
            }],
            ..LayoutStore::default()
        };

        normalize_store_with_catalog(&mut store, 0, Some(&catalog));

        assert_eq!(
            store.widgets[0].symbols,
            vec!["binance:spot:BTC/USDT", "binance:spot:ETH/USDT"]
        );
        assert_eq!(
            market_subscriptions_from_store(&store, &catalog),
            vec![
                market::MarketSubscription {
                    symbol: "binance:spot:BTC/USDT".to_string(),
                    needs_candles: false,
                },
                market::MarketSubscription {
                    symbol: "binance:spot:ETH/USDT".to_string(),
                    needs_candles: false,
                },
            ]
        );
        assert_eq!(
            settings_window::symbols_help_text(
                settings_window::symbol_min_for_instance(&store.widgets[0], Some(&catalog)),
                settings_window::symbol_limit_for_instance(&store.widgets[0], Some(&catalog)),
                i18n::Locale::En
            ),
            "Up to 2 pairs"
        );
    }

    #[test]
    fn market_subscriptions_merge_candle_requirements_for_shared_pairs() {
        let price_plugin = local_test_plugin_definition();
        let mut candle_plugin = local_test_plugin_definition();
        candle_plugin.id = "com.example.stage3-chart-card".to_string();
        candle_plugin.name = "Stage 3 Chart Card".to_string();
        candle_plugin
            .data_requirements
            .push(plugin::PluginDataRequirement {
                capability: "market.candles".to_string(),
            });
        let catalog = plugin::PluginCatalog::from_plugins_for_tests(vec![
            price_plugin.clone(),
            candle_plugin.clone(),
        ]);
        let store = LayoutStore {
            widgets: vec![
                WidgetInstance {
                    id: "price-card-1".to_string(),
                    plugin_id: price_plugin.id,
                    legacy_widget_type: None,
                    name: "Price Card 1".to_string(),
                    visible: true,
                    layout: WidgetLayout::default(),
                    symbols: vec!["BTC".to_string(), "ETH".to_string()],
                    config: default_widget_config(),
                },
                WidgetInstance {
                    id: "chart-card-2".to_string(),
                    plugin_id: candle_plugin.id,
                    legacy_widget_type: None,
                    name: "Chart Card 2".to_string(),
                    visible: true,
                    layout: WidgetLayout::default(),
                    symbols: vec!["binance:spot:BTC/USDT".to_string()],
                    config: default_widget_config(),
                },
            ],
            ..LayoutStore::default()
        };

        assert_eq!(
            market_subscriptions_from_store(&store, &catalog),
            vec![
                market::MarketSubscription {
                    symbol: "binance:spot:BTC/USDT".to_string(),
                    needs_candles: true,
                },
                market::MarketSubscription {
                    symbol: "binance:spot:ETH/USDT".to_string(),
                    needs_candles: false,
                },
            ]
        );
    }

    #[test]
    fn unavailable_plugins_cannot_enable_candle_requests() {
        let mut unavailable_plugin = local_test_plugin_definition();
        unavailable_plugin
            .data_requirements
            .push(plugin::PluginDataRequirement {
                capability: "market.candles".to_string(),
            });
        unavailable_plugin.status =
            plugin::PluginStatus::Unavailable("renderer unavailable".to_string());
        let plugin_id = unavailable_plugin.id.clone();
        let catalog = plugin::PluginCatalog::from_plugins_for_tests(vec![unavailable_plugin]);
        let store = LayoutStore {
            widgets: vec![WidgetInstance {
                id: "unavailable-chart-card-1".to_string(),
                plugin_id,
                legacy_widget_type: None,
                name: "Unavailable Chart Card 1".to_string(),
                visible: true,
                layout: WidgetLayout::default(),
                symbols: vec!["binance:spot:BTC/USDT".to_string()],
                config: default_widget_config(),
            }],
            ..LayoutStore::default()
        };

        assert_eq!(
            market_subscriptions_from_store(&store, &catalog),
            vec![market::MarketSubscription {
                symbol: "binance:spot:BTC/USDT".to_string(),
                needs_candles: false,
            }]
        );
    }

    #[test]
    fn default_widget_names_are_localized_only_for_display() {
        let widget = WidgetInstance {
            id: "quote-board-7".to_string(),
            plugin_id: WidgetType::QuoteBoard.plugin_id().to_string(),
            legacy_widget_type: None,
            name: settings_window::default_widget_name(WidgetType::QuoteBoard, 7),
            visible: true,
            layout: WidgetLayout::default(),
            symbols: vec!["BTC".to_string()],
            config: default_widget_config(),
        };

        assert_eq!(
            settings_window::widget_display_name(&widget, 0, i18n::Locale::En),
            "Quote Board 7"
        );
        assert_eq!(
            settings_window::widget_display_name(&widget, 0, i18n::Locale::ZhHans),
            "行情面板 7"
        );
        assert_eq!(
            settings_window::normalize_widget_name(
                "行情面板 7",
                WidgetType::QuoteBoard,
                7,
                i18n::Locale::ZhHans
            ),
            "Quote Board 7"
        );
        assert_eq!(
            settings_window::normalize_widget_name(
                "行情面板 9",
                WidgetType::QuoteBoard,
                7,
                i18n::Locale::En
            ),
            "Quote Board 9"
        );
        assert_eq!(
            settings_window::normalize_widget_name(
                "لوحة الأسعار \u{2066}7\u{2069}",
                WidgetType::QuoteBoard,
                7,
                i18n::Locale::Ar
            ),
            "Quote Board 7"
        );

        let custom = WidgetInstance {
            name: "Trading Desk".to_string(),
            ..widget
        };
        assert_eq!(
            settings_window::widget_display_name(&custom, 0, i18n::Locale::ZhHans),
            "Trading Desk"
        );
    }

    #[test]
    fn normalizing_empty_store_creates_requested_default_instances() {
        let mut store = LayoutStore::default();

        normalize_store(&mut store, 4);

        assert_eq!(store.widgets.len(), 4);
        assert_eq!(store.widgets[0].id, "quote-board-1");
        assert_eq!(store.widgets[1].id, "quote-board-2");
        assert_eq!(store.widgets[2].id, "quote-board-3");
        assert_eq!(store.widgets[3].id, "quote-board-4");
        assert_eq!(store.selected_widget_id.as_deref(), Some("quote-board-1"));
        assert_eq!(store.widgets[0].symbols, vec!["binance:spot:BTC/USDT"]);
        assert_eq!(store.widgets[1].symbols, vec!["binance:spot:ETH/USDT"]);
        assert_eq!(store.widgets[2].symbols, vec!["binance:spot:SOL/USDT"]);
        assert_eq!(store.widgets[3].symbols, vec!["binance:spot:BTC/USDT"]);
    }

    #[test]
    fn normalizing_empty_store_creates_single_quote_board_with_default_symbols() {
        let mut store = LayoutStore::default();

        normalize_store(&mut store, 1);

        assert_eq!(store.widgets.len(), 1);
        assert_eq!(store.widgets[0].id, "quote-board-1");
        assert_eq!(store.selected_widget_id.as_deref(), Some("quote-board-1"));
        assert_eq!(store.widgets[0].symbols, default_symbols());
    }

    #[test]
    fn each_widget_launch_seeds_one_instance_per_available_plugin() {
        let catalog = plugin::PluginCatalog::builtins();
        let mut store = LayoutStore::default();

        assert!(seed_each_widget_on_empty_start(&mut store, true, &catalog));

        assert_eq!(store.widgets.len(), 1);
        assert_eq!(
            store.widgets[0].plugin_id,
            plugin::BUILTIN_QUOTE_BOARD_PLUGIN_ID
        );
        assert_eq!(store.selected_widget_id.as_deref(), Some("quote-board-1"));
        assert_eq!(store.widgets[0].symbols, default_symbols());
    }

    #[test]
    fn legacy_layout_store_migrates_to_widget_instances() {
        let mut layouts = HashMap::new();
        layouts.insert(
            "price-card-1".to_string(),
            WidgetLayout {
                x: 24,
                y: 48,
                always_on_top: false,
                opacity_percent: 64,
                ..WidgetLayout::default()
            },
        );
        let legacy = LegacyLayoutStore {
            settings: AppSettings::default(),
            symbols: vec!["eth".to_string(), "ETHUSDT".to_string()],
            widgets: layouts,
        };

        let store = migrate_legacy_store(legacy);

        assert_eq!(store.widgets.len(), 1);
        assert_eq!(store.widgets[0].id, "price-card-1");
        assert_eq!(store.widgets[0].widget_type(), WidgetType::QuoteBoard);
        assert_eq!(store.widgets[0].layout.x, 24);
        assert_eq!(store.widgets[0].layout.y, 48);
        assert!(!store.widgets[0].layout.always_on_top);
        assert_eq!(store.widgets[0].layout.opacity_percent, 64);
        assert_eq!(store.widgets[0].symbols, vec!["binance:spot:ETH/USDT"]);
    }

    #[test]
    fn legacy_widget_type_field_migrates_to_plugin_id() {
        let mut store = serde_json::from_str::<LayoutStore>(
            r#"{
              "widgets": [
                {
                  "id": "mini-ticker-1",
                  "widget_type": "mini_ticker",
                  "name": "Mini Ticker 1",
                  "symbols": ["ETH", "BTC"]
                }
              ]
            }"#,
        )
        .unwrap();

        normalize_store(&mut store, 0);

        assert_eq!(
            store.widgets[0].plugin_id,
            plugin::BUILTIN_MINI_TICKER_PLUGIN_ID
        );
        assert_eq!(store.widgets[0].widget_type(), WidgetType::MiniTicker);
        assert_eq!(store.widgets[0].symbols, vec!["binance:spot:ETH/USDT"]);
        assert_eq!(store.widgets[0].config, default_widget_config());

        let serialized = serde_json::to_string(&store).unwrap();
        assert!(serialized.contains(r#""plugin_id":"builtin.mini-ticker""#));
        assert!(!serialized.contains("widget_type"));
    }

    #[test]
    fn market_subscriptions_ignore_alert_rules_while_disabled() {
        let store = LayoutStore {
            widgets: vec![
                WidgetInstance {
                    id: "quote-board-1".to_string(),
                    plugin_id: WidgetType::QuoteBoard.plugin_id().to_string(),
                    legacy_widget_type: None,
                    name: String::new(),
                    visible: true,
                    layout: WidgetLayout::default(),
                    symbols: vec!["BTC".to_string(), "ETH".to_string()],
                    config: default_widget_config(),
                },
                WidgetInstance {
                    id: "mini-ticker-2".to_string(),
                    plugin_id: WidgetType::MiniTicker.plugin_id().to_string(),
                    legacy_widget_type: None,
                    name: String::new(),
                    visible: true,
                    layout: WidgetLayout::default(),
                    symbols: vec!["ETH".to_string()],
                    config: default_widget_config(),
                },
            ],
            settings: AppSettings {
                alert_rules: vec![
                    settings::AlertRule {
                        id: "sol-breakout".to_string(),
                        symbol: "SOL".to_string(),
                        condition: settings::AlertCondition::PriceAbove,
                        threshold: 150.0,
                        enabled: true,
                    },
                    settings::AlertRule {
                        id: "bnb-disabled".to_string(),
                        symbol: "BNB".to_string(),
                        condition: settings::AlertCondition::PriceBelow,
                        threshold: 400.0,
                        enabled: false,
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        };

        let catalog = plugin::PluginCatalog::builtins();
        assert_eq!(
            market_subscriptions_from_store(&store, &catalog),
            vec![
                market::MarketSubscription {
                    symbol: "binance:spot:BTC/USDT".to_string(),
                    needs_candles: false,
                },
                market::MarketSubscription {
                    symbol: "binance:spot:ETH/USDT".to_string(),
                    needs_candles: false,
                },
            ]
        );
    }
}
