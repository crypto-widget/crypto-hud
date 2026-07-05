use std::{
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex, OnceLock},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use crypto_hud_market as market;
use crypto_hud_runtime::QuoteCache;
use crypto_hud_shell_state as settings;
use settings::{
    clamp_opacity, save_layout_store, AlertCondition, AlertRule, AppSettings, LanguagePreference,
    LayoutStore, ShortcutPreference, ThemePreference, WidgetDefinition, WidgetInstance,
    WidgetKind as WidgetType, WidgetSize,
};
use slint::{ComponentHandle, Model, ModelRc, SharedString, Timer};

use crate::{
    autostart,
    coin_icons::CoinIconRegistry,
    desktop_shell::{install_settings_drag_handler, open_external_url, refresh_tray_text},
    feature_flags, i18n, notifications, plugin, shortcuts, updater, AppTray, SettingsWindow,
    ABOUT_REPOSITORY_URL, WIDGET_REORDER_DOUBLE_CLICK_TIMEOUT,
};
use crate::{
    runtime_bridge::{
        apply_settings_to_widgets, sync_widget_runtimes, update_market_feed_config_from_store,
    },
    state_bridge::{
        move_widget_in_store, normalize_store_with_catalog, select_widget_by_index,
        widget_definitions_from_catalog, widget_reorder_steps,
    },
    widget_host::WidgetRuntime,
    window_manager::{
        apply_tray_hover_display, apply_widget_pinning_for_settings_mode, desktop_size,
        leave_settings_mode, schedule_settings_window_raise,
        schedule_widget_shell_window_configuration, TrayHoverDisplayState,
    },
};

mod settings_actions;
mod settings_models;
mod settings_theme;
use settings_actions::{
    add_plugin_widget_to_store, apply_widget_scale_to_store, apply_widget_settings_to_store,
    delete_widget_from_store_at_index, WidgetSettingsUpdate,
};
pub(crate) use settings_models::widget_type_usage_text;
use settings_models::{
    bool_model, int_model, owned_string_model, plugin_market_items_model, string_model,
    widget_instance_detail_options, widget_instance_options, widget_preview_kind_options,
    widget_scale_max_options, widget_scale_min_options, widget_scale_options,
    widget_visibility_options,
};
use settings_theme::apply_theme_to_settings_window;

const PRIMARY_ALERT_ID: &str = "primary-alert";
const PREVIEW_KIND_GENERIC: i32 = 0;
const PREVIEW_KIND_FOCUS_TICKER: i32 = 1;
const PREVIEW_KIND_MARKET_BOARD: i32 = 2;
const PREVIEW_KIND_TRUST_CARD: i32 = 3;
const PREVIEW_KIND_ORBIT_PULSE: i32 = 4;
const PREVIEW_KIND_MARKET_COMPASS: i32 = 5;
const PREVIEW_KIND_STATUS_STRIP: i32 = 6;
const SYMBOL_PICKER_MODE_WIDGET: i32 = 0;
const SYMBOL_PICKER_MODE_DEFAULT: i32 = 1;
const STATUS_STRIP_PLUGIN_ID: &str = "com.cryptohud.status-strip";
const STATUS_STRIP_VISIBLE_HEIGHT: i32 = 84;
const STATUS_STRIP_PREVIOUS_TIGHT_HEIGHT: i32 = 84;
const POPULAR_SYMBOL_ORDER: &[&str] = &[
    "BTC", "ETH", "SOL", "BNB", "XRP", "DOGE", "ADA", "TRX", "TON", "LINK", "AVAX", "DOT", "LTC",
    "BCH", "SUI", "APT", "NEAR", "ARB", "OP", "ATOM", "FIL", "ETC", "UNI", "AAVE", "MKR", "INJ",
    "PEPE", "SHIB", "WIF", "BONK", "JUP", "SEI", "TIA", "RNDR", "FET", "ONDO", "PYTH", "ICP",
    "HBAR", "XLM", "MATIC", "POL",
];

struct SymbolMetadata {
    symbol: &'static str,
    name: &'static str,
    aliases: &'static [&'static str],
}

const SYMBOL_METADATA: &[SymbolMetadata] = &[
    SymbolMetadata {
        symbol: "BTC",
        name: "Bitcoin",
        aliases: &["比特币", "btc usdt"],
    },
    SymbolMetadata {
        symbol: "ETH",
        name: "Ethereum",
        aliases: &["以太坊", "ether", "eth usdt"],
    },
    SymbolMetadata {
        symbol: "SOL",
        name: "Solana",
        aliases: &["索拉纳"],
    },
    SymbolMetadata {
        symbol: "BNB",
        name: "BNB",
        aliases: &["binance coin", "币安币"],
    },
    SymbolMetadata {
        symbol: "XRP",
        name: "XRP",
        aliases: &["ripple", "瑞波"],
    },
    SymbolMetadata {
        symbol: "DOGE",
        name: "Dogecoin",
        aliases: &["doge coin", "狗狗币"],
    },
    SymbolMetadata {
        symbol: "ADA",
        name: "Cardano",
        aliases: &["卡尔达诺"],
    },
    SymbolMetadata {
        symbol: "TRX",
        name: "TRON",
        aliases: &["波场"],
    },
    SymbolMetadata {
        symbol: "TON",
        name: "Toncoin",
        aliases: &["the open network"],
    },
    SymbolMetadata {
        symbol: "LINK",
        name: "Chainlink",
        aliases: &["chain link"],
    },
    SymbolMetadata {
        symbol: "AVAX",
        name: "Avalanche",
        aliases: &["雪崩"],
    },
    SymbolMetadata {
        symbol: "DOT",
        name: "Polkadot",
        aliases: &["波卡"],
    },
    SymbolMetadata {
        symbol: "LTC",
        name: "Litecoin",
        aliases: &["莱特币"],
    },
    SymbolMetadata {
        symbol: "BCH",
        name: "Bitcoin Cash",
        aliases: &["比特币现金"],
    },
    SymbolMetadata {
        symbol: "SUI",
        name: "Sui",
        aliases: &[],
    },
    SymbolMetadata {
        symbol: "APT",
        name: "Aptos",
        aliases: &[],
    },
    SymbolMetadata {
        symbol: "NEAR",
        name: "NEAR Protocol",
        aliases: &[],
    },
    SymbolMetadata {
        symbol: "ARB",
        name: "Arbitrum",
        aliases: &[],
    },
    SymbolMetadata {
        symbol: "OP",
        name: "Optimism",
        aliases: &[],
    },
    SymbolMetadata {
        symbol: "ATOM",
        name: "Cosmos",
        aliases: &[],
    },
    SymbolMetadata {
        symbol: "UNI",
        name: "Uniswap",
        aliases: &[],
    },
    SymbolMetadata {
        symbol: "AAVE",
        name: "Aave",
        aliases: &[],
    },
    SymbolMetadata {
        symbol: "PEPE",
        name: "Pepe",
        aliases: &["pepe coin"],
    },
    SymbolMetadata {
        symbol: "SHIB",
        name: "Shiba Inu",
        aliases: &["柴犬币"],
    },
];
const STATUS_STRIP_PREVIOUS_BLOCK_WIDTH: i32 = 150;
const STATUS_STRIP_PREVIOUS_TIGHT_BLOCK_WIDTH: i32 = 136;
const STATUS_STRIP_LEGACY_BLOCK_WIDTH: i32 = 168;
const STATUS_STRIP_LEGACY_SIDE_MARGIN: i32 = 160;
const STATUS_STRIP_LEGACY_HEIGHT: i32 = 260;

pub(crate) struct SettingsWindowDeps {
    pub(crate) widgets: Rc<RefCell<Vec<WidgetRuntime>>>,
    pub(crate) layouts: Rc<RefCell<LayoutStore>>,
    pub(crate) state_path: PathBuf,
    pub(crate) settings_status: Rc<RefCell<String>>,
    pub(crate) shortcut_manager: Rc<RefCell<shortcuts::ShortcutManager>>,
    pub(crate) market_feed_config: Arc<Mutex<market::MarketFeedConfig>>,
    pub(crate) widgets_hidden: Rc<RefCell<bool>>,
    pub(crate) settings_mode_active: Rc<RefCell<bool>>,
    pub(crate) tray_handle: Rc<RefCell<Option<slint::Weak<AppTray>>>>,
    pub(crate) tray_hover_state: Rc<RefCell<TrayHoverDisplayState>>,
    pub(crate) quote_cache: Rc<RefCell<QuoteCache>>,
    pub(crate) coin_icons: Rc<CoinIconRegistry>,
    pub(crate) plugin_catalog: Rc<plugin::PluginCatalog>,
}

#[derive(Clone)]
struct SettingsCommitContext {
    widgets: Rc<RefCell<Vec<WidgetRuntime>>>,
    layouts: Rc<RefCell<LayoutStore>>,
    state_path: PathBuf,
    settings_status: Rc<RefCell<String>>,
    market_feed_config: Arc<Mutex<market::MarketFeedConfig>>,
    quote_cache: Rc<RefCell<QuoteCache>>,
    coin_icons: Rc<CoinIconRegistry>,
    settings_mode_active: Rc<RefCell<bool>>,
    plugin_catalog: Rc<plugin::PluginCatalog>,
}

impl SettingsCommitContext {
    fn current_settings(&self) -> AppSettings {
        self.layouts.borrow().settings.clone().normalized()
    }

    fn sync_widget_runtimes(&self, set_positions: bool, error_context: &str) {
        if let Err(error) = sync_widget_runtimes(
            &self.widgets,
            &self.layouts,
            &self.state_path,
            self.quote_cache.clone(),
            self.coin_icons.clone(),
            set_positions,
            self.plugin_catalog.clone(),
        ) {
            eprintln!("{error_context}: {error:#}");
        }
    }

    fn apply_settings_to_widgets(&self) {
        apply_settings_to_widgets(
            &self.widgets,
            &self.layouts,
            &self.quote_cache.borrow(),
            &self.coin_icons,
            &self.plugin_catalog,
        );
    }

    fn apply_widget_pinning_for_settings_mode(&self) {
        apply_widget_pinning_for_settings_mode(
            &self.widgets,
            &self.layouts,
            *self.settings_mode_active.borrow(),
        );
    }

    fn update_market_feed_config_from_store(&self) {
        update_market_feed_config_from_store(
            &self.market_feed_config,
            &self.layouts.borrow(),
            &self.plugin_catalog,
        );
    }

    fn set_saved_status(&self, settings: AppSettings) {
        *self.settings_status.borrow_mut() = status_saved(settings);
    }

    fn refresh_settings_window(&self, weak: &slint::Weak<SettingsWindow>) {
        if let Some(ui) = weak.upgrade() {
            let status = self.settings_status.borrow().clone();
            refresh_settings_window(
                &ui,
                &self.layouts,
                &self.state_path,
                &self.plugin_catalog,
                Some(status.as_str()),
            );
        }
    }

    fn finish_runtime_change(
        &self,
        weak: &slint::Weak<SettingsWindow>,
        status_settings: AppSettings,
        set_positions: bool,
        update_market_feed: bool,
        sync_error_context: &str,
    ) {
        self.sync_widget_runtimes(set_positions, sync_error_context);
        self.apply_widget_pinning_for_settings_mode();
        if update_market_feed {
            self.update_market_feed_config_from_store();
        }
        self.set_saved_status(status_settings);
        self.refresh_settings_window(weak);
        schedule_widget_shell_window_configuration();
    }
}

#[derive(Debug, Default)]
struct WidgetReorderState {
    last_press_index: Option<i32>,
    last_press_at: Option<Instant>,
    active_index: Option<i32>,
    consumed_steps: i32,
}

#[derive(Debug, Clone, Default)]
struct SymbolCatalogState {
    catalog: market::SymbolCatalog,
    fallback_symbols: Vec<String>,
    loading: bool,
    fallback_only: bool,
    last_error: Option<String>,
}

static SYMBOL_CATALOG_STATE: OnceLock<Arc<Mutex<SymbolCatalogState>>> = OnceLock::new();

fn shared_symbol_catalog_state() -> &'static Arc<Mutex<SymbolCatalogState>> {
    SYMBOL_CATALOG_STATE.get_or_init(|| Arc::new(Mutex::new(SymbolCatalogState::default())))
}

pub(crate) fn request_symbol_catalog_refresh_from_store(
    ui: &SettingsWindow,
    store: &LayoutStore,
    plugin_catalog: &plugin::PluginCatalog,
) {
    let settings = store.settings.clone().normalized();
    let locale = i18n::resolve_locale(settings.language);
    request_symbol_catalog_refresh(
        ui.as_weak(),
        settings::effective_network_proxy_url(&settings),
        collect_symbol_fallback_symbols(store, plugin_catalog),
        locale,
    );
}

fn request_symbol_catalog_refresh(
    weak: slint::Weak<SettingsWindow>,
    proxy_url: Option<String>,
    fallback_symbols: Vec<String>,
    locale: i18n::Locale,
) {
    let state = shared_symbol_catalog_state().clone();
    if let Ok(mut state) = state.lock() {
        state.loading = true;
        state.last_error = None;
        state.fallback_symbols = sorted_symbol_values(
            state
                .fallback_symbols
                .iter()
                .cloned()
                .chain(fallback_symbols.clone())
                .collect(),
        );
    }

    thread::spawn(move || {
        let result = market::fetch_symbol_catalog(proxy_url.as_deref());
        let fallback_message = result.is_err();
        if let Ok(mut state) = shared_symbol_catalog_state().lock() {
            state.loading = false;
            match result {
                Ok(catalog) => {
                    state.catalog = catalog;
                    state.fallback_only = false;
                    state.last_error = None;
                }
                Err(error) => {
                    state.catalog = market::SymbolCatalog::default();
                    state.fallback_only = true;
                    state.last_error = Some(error.to_string());
                }
            }
            state.fallback_symbols = sorted_symbol_values(
                state
                    .fallback_symbols
                    .iter()
                    .cloned()
                    .chain(fallback_symbols)
                    .collect(),
            );
        }

        let _ = weak.upgrade_in_event_loop(move |ui| {
            let state = symbol_catalog_snapshot();
            refresh_symbol_selector_models_from_ui(&ui, &state, locale);
            if fallback_message {
                ui.set_status_text(i18n::text(locale).status_symbol_catalog_fallback.into());
            }
        });
    });
}

fn symbol_catalog_snapshot() -> SymbolCatalogState {
    shared_symbol_catalog_state()
        .lock()
        .map(|state| state.clone())
        .unwrap_or_default()
}

pub(crate) fn install_settings_window(deps: SettingsWindowDeps) -> Result<SettingsWindow> {
    let SettingsWindowDeps {
        widgets,
        layouts,
        state_path,
        settings_status,
        shortcut_manager,
        market_feed_config,
        widgets_hidden,
        settings_mode_active,
        tray_handle,
        tray_hover_state,
        quote_cache,
        coin_icons,
        plugin_catalog,
    } = deps;
    let ui = SettingsWindow::new().context("failed to create Slint settings window")?;
    install_settings_drag_handler(&ui);
    let widget_reorder_state = Rc::new(RefCell::new(WidgetReorderState::default()));
    refresh_settings_window(
        &ui,
        &layouts,
        &state_path,
        &plugin_catalog,
        Some(settings_status.borrow().as_str()),
    );
    schedule_widget_layout_lock_sync(ui.as_weak(), layouts.clone());
    request_symbol_catalog_refresh_from_store(&ui, &layouts.borrow(), &plugin_catalog);
    let commit_context = SettingsCommitContext {
        widgets: widgets.clone(),
        layouts: layouts.clone(),
        state_path: state_path.clone(),
        settings_status: settings_status.clone(),
        market_feed_config: market_feed_config.clone(),
        quote_cache: quote_cache.clone(),
        coin_icons: coin_icons.clone(),
        settings_mode_active: settings_mode_active.clone(),
        plugin_catalog: plugin_catalog.clone(),
    };

    ui.on_apply_settings({
        let commit = commit_context.clone();
        let shortcut_manager = shortcut_manager.clone();
        let tray_handle = tray_handle.clone();
        let widgets_hidden = widgets_hidden.clone();
        let tray_hover_state = tray_hover_state.clone();
        let weak = ui.as_weak();
        move |red_up_enabled,
              market_binance_enabled,
              market_okx_enabled,
              market_hyperliquid_enabled,
              auto_start_enabled,
              show_main_window_on_startup,
              shortcut_index,
              theme_index,
              language_index,
              default_widgets_always_on_top,
              default_opacity_percent,
              default_widget_scale_percent,
              refresh_interval_seconds,
              market_fallback_enabled,
              tray_icon_enabled,
              tray_hover_display_enabled| {
            let previous = commit.current_settings();
            let mut settings = AppSettings {
                widgets_always_on_top: default_widgets_always_on_top,
                opacity_percent: clamp_opacity(default_opacity_percent),
                widget_scale_percent: settings::clamp_default_widget_scale_percent(
                    default_widget_scale_percent,
                ),
                red_up_enabled,
                market_provider: previous.market_provider,
                market_binance_enabled,
                market_okx_enabled,
                market_hyperliquid_enabled,
                refresh_interval_seconds: settings::clamp_refresh_interval(
                    refresh_interval_seconds,
                ),
                market_default_symbols: previous.market_default_symbols.clone(),
                market_fallback_enabled,
                auto_start_enabled,
                show_main_window_on_startup,
                shortcut: ShortcutPreference::from_index(shortcut_index),
                theme: ThemePreference::from_index(theme_index),
                language: LanguagePreference::from_index(language_index),
                tray_icon_enabled,
                tray_hover_display_enabled,
                network_proxy_enabled: previous.network_proxy_enabled,
                network_proxy_url: previous.network_proxy_url.clone(),
                alert_rules: previous.alert_rules.clone(),
            }
            .normalized();
            let mut status = status_saved(settings.clone());

            if previous.auto_start_enabled != settings.auto_start_enabled {
                let widget_count = commit.layouts.borrow().widgets.len().max(1);
                if let Err(error) =
                    autostart::apply_auto_start(settings.auto_start_enabled, widget_count)
                {
                    settings.auto_start_enabled = previous.auto_start_enabled;
                    status = format!(
                        "{}: {error}",
                        i18n::text(i18n::resolve_locale(settings.language))
                            .status_auto_start_failed
                    );
                }
            }

            if previous.shortcut != settings.shortcut {
                if let Err(error) = shortcut_manager.borrow_mut().apply(settings.shortcut) {
                    settings.shortcut = previous.shortcut;
                    status = format!(
                        "{}: {error}",
                        i18n::text(i18n::resolve_locale(settings.language)).status_shortcut_failed
                    );
                }
            }

            if previous.widget_scale_percent != settings.widget_scale_percent {
                apply_default_widget_scale_to_instances(
                    &commit.layouts,
                    &commit.plugin_catalog,
                    settings.widget_scale_percent,
                );
            }
            apply_settings_to_store(&commit.layouts, &commit.state_path, settings.clone());
            commit.apply_settings_to_widgets();
            commit.apply_widget_pinning_for_settings_mode();
            commit.update_market_feed_config_from_store();
            *commit.settings_status.borrow_mut() = status;
            if let Some(tray) = tray_handle
                .borrow()
                .as_ref()
                .and_then(|tray| tray.upgrade())
            {
                refresh_tray_text(&tray, settings.clone());
            }
            apply_tray_hover_display(
                &commit.widgets,
                &commit.layouts,
                &widgets_hidden,
                &commit.settings_mode_active,
                &tray_hover_state,
                notifications::tray_icon_hovered(),
            );
            commit.refresh_settings_window(&weak);
            schedule_widget_shell_window_configuration();
        }
    });

    ui.on_apply_network_settings({
        let commit = commit_context.clone();
        let weak = ui.as_weak();
        move |network_proxy_enabled, network_proxy_url_input_text| {
            let previous = commit.current_settings();
            let network_proxy_url = match normalized_network_proxy_input(
                network_proxy_enabled,
                network_proxy_url_input_text.as_str(),
            ) {
                Ok(network_proxy_url) => network_proxy_url,
                Err(error) => {
                    let text = i18n::text(i18n::resolve_locale(previous.language));
                    *commit.settings_status.borrow_mut() =
                        format!("{}: {error}", text.status_network_proxy_invalid);
                    if let Some(ui) = weak.upgrade() {
                        let status = commit.settings_status.borrow().clone();
                        ui.set_status_text(status.as_str().into());
                    }
                    return;
                }
            };

            let mut settings = previous;
            settings.network_proxy_enabled = network_proxy_enabled;
            settings.network_proxy_url = network_proxy_url;
            let settings = settings.normalized();
            apply_settings_to_store(&commit.layouts, &commit.state_path, settings.clone());
            commit.update_market_feed_config_from_store();
            commit.set_saved_status(settings);
            commit.refresh_settings_window(&weak);
            if let Some(ui) = weak.upgrade() {
                request_symbol_catalog_refresh_from_store(
                    &ui,
                    &commit.layouts.borrow(),
                    &commit.plugin_catalog,
                );
            }
        }
    });

    ui.on_apply_alert_settings({
        let commit = commit_context.clone();
        let weak = ui.as_weak();
        move |enabled, symbol_input, condition_index, threshold_input| {
            if !feature_flags::ALERT_RULES_ENABLED {
                return;
            }

            let mut settings = commit.current_settings();
            match primary_alert_rule_from_input(
                enabled,
                symbol_input.as_str(),
                condition_index,
                threshold_input.as_str(),
            ) {
                Ok(Some(rule)) => {
                    settings.alert_rules = vec![rule];
                }
                Ok(None) => {
                    settings.alert_rules.clear();
                }
                Err(()) => {
                    let text = i18n::text(i18n::resolve_locale(settings.language));
                    *commit.settings_status.borrow_mut() = text.status_alert_invalid.to_string();
                    commit.refresh_settings_window(&weak);
                    return;
                }
            }

            apply_settings_to_store(&commit.layouts, &commit.state_path, settings.clone());
            commit.update_market_feed_config_from_store();
            commit.set_saved_status(settings);
            commit.refresh_settings_window(&weak);
        }
    });

    ui.on_clear_alert_settings({
        let commit = commit_context.clone();
        let weak = ui.as_weak();
        move || {
            if !feature_flags::ALERT_RULES_ENABLED {
                return;
            }

            let mut settings = commit.current_settings();
            settings.alert_rules.clear();
            apply_settings_to_store(&commit.layouts, &commit.state_path, settings.clone());
            commit.update_market_feed_config_from_store();
            commit.set_saved_status(settings);
            commit.refresh_settings_window(&weak);
        }
    });

    ui.on_open_widget_symbol_picker({
        let weak = ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade() {
                open_symbol_picker(&ui, SYMBOL_PICKER_MODE_WIDGET);
            }
        }
    });

    ui.on_open_default_symbol_picker({
        let weak = ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade() {
                open_symbol_picker(&ui, SYMBOL_PICKER_MODE_DEFAULT);
            }
        }
    });

    ui.on_close_symbol_picker({
        let weak = ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade() {
                close_symbol_picker(&ui);
            }
        }
    });

    ui.on_refresh_symbol_picker_candidates({
        let weak = ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade() {
                let state = symbol_catalog_snapshot();
                refresh_symbol_picker_models_from_ui(&ui, &state, current_ui_locale(&ui));
            }
        }
    });

    ui.on_remove_default_symbol({
        let commit = commit_context.clone();
        let weak = ui.as_weak();
        move |symbol_index| {
            let Some(settings) =
                remove_default_symbol_from_store(&commit.layouts, &commit.state_path, symbol_index)
            else {
                return;
            };
            commit.update_market_feed_config_from_store();
            commit.set_saved_status(settings);
            commit.refresh_settings_window(&weak);
        }
    });

    ui.on_confirm_symbol_picker({
        let commit = commit_context.clone();
        let weak = ui.as_weak();
        move |mode, candidate_index| {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let Some(symbol) =
                model_raw_string_at(ui.get_symbol_picker_candidate_values(), candidate_index)
            else {
                ui.set_symbol_picker_status_text(
                    symbol_picker_empty_status_text(mode, current_ui_locale(&ui)).into(),
                );
                return;
            };

            if mode == SYMBOL_PICKER_MODE_DEFAULT {
                let Some(settings) =
                    add_default_symbol_to_store(&commit.layouts, &commit.state_path, &symbol)
                else {
                    ui.set_symbol_picker_status_text(
                        symbol_picker_empty_status_text(mode, current_ui_locale(&ui)).into(),
                    );
                    return;
                };
                close_symbol_picker(&ui);
                commit.update_market_feed_config_from_store();
                commit.set_saved_status(settings);
                commit.refresh_settings_window(&weak);
            } else {
                let Some(settings) = add_widget_symbol_to_store(
                    &commit.layouts,
                    &commit.state_path,
                    ui.get_selected_widget_index(),
                    &symbol,
                    &commit.plugin_catalog,
                ) else {
                    ui.set_symbol_picker_status_text(
                        symbol_picker_empty_status_text(mode, current_ui_locale(&ui)).into(),
                    );
                    return;
                };
                close_symbol_picker(&ui);
                commit.finish_runtime_change(
                    &weak,
                    settings,
                    false,
                    true,
                    "failed to add widget symbol",
                );
            }
        }
    });

    ui.on_remove_widget_symbol({
        let commit = commit_context.clone();
        let weak = ui.as_weak();
        move |selected_index, symbol_index| {
            if weak.upgrade().is_none() {
                return;
            };
            let Some(settings) = remove_widget_symbol_from_store(
                &commit.layouts,
                &commit.state_path,
                selected_index,
                symbol_index,
                &commit.plugin_catalog,
            ) else {
                return;
            };
            commit.finish_runtime_change(
                &weak,
                settings,
                false,
                true,
                "failed to remove widget symbol",
            );
        }
    });

    ui.on_select_widget({
        let layouts = layouts.clone();
        let state_path = state_path.clone();
        let plugin_catalog = plugin_catalog.clone();
        let weak = ui.as_weak();
        move |selected_index| {
            {
                let mut store = layouts.borrow_mut();
                select_widget_by_index(&mut store, selected_index);
                if let Err(error) = save_layout_store(&state_path, &store) {
                    eprintln!("failed to save selected widget: {error:#}");
                }
            }
            if let Some(ui) = weak.upgrade() {
                refresh_settings_window(&ui, &layouts, &state_path, &plugin_catalog, None);
            }
        }
    });

    ui.on_toggle_widget_visibility({
        let widgets_hidden = widgets_hidden.clone();
        let commit = commit_context.clone();
        let weak = ui.as_weak();
        move |selected_index| {
            let settings = commit.current_settings();
            let mut toggled_visible = None;

            {
                let mut store = commit.layouts.borrow_mut();
                let index = selected_index.max(0) as usize;
                if let Some(instance) = store.widgets.get_mut(index) {
                    instance.visible = !instance.visible;
                    let selected_id = instance.id.clone();
                    toggled_visible = Some(instance.visible);
                    store.selected_widget_id = Some(selected_id);
                    if let Err(error) = save_layout_store(&commit.state_path, &store) {
                        eprintln!("failed to save widget visibility: {error:#}");
                    }
                }
            }

            let Some(is_visible) = toggled_visible else {
                return;
            };

            if is_visible {
                *widgets_hidden.borrow_mut() = false;
            }

            commit.finish_runtime_change(
                &weak,
                settings,
                is_visible,
                false,
                "failed to toggle widget visibility",
            );
        }
    });

    ui.on_reset_widget_positions({
        let commit = commit_context.clone();
        let weak = ui.as_weak();
        move || {
            let status_settings = commit.current_settings();
            let mut changed = false;
            {
                let mut store = commit.layouts.borrow_mut();
                let app_settings = store.settings.clone().normalized();
                for (index, instance) in store.widgets.iter_mut().enumerate() {
                    let current_size = WidgetSize {
                        width: instance.layout.width,
                        height: instance.layout.height,
                    };
                    let layout = settings::default_layout_for_size(
                        index,
                        current_size,
                        app_settings.clone(),
                        desktop_size(),
                    );
                    if instance.layout.x != layout.x || instance.layout.y != layout.y {
                        instance.layout.x = layout.x;
                        instance.layout.y = layout.y;
                        changed = true;
                    }
                }
                if changed {
                    if let Err(error) = save_layout_store(&commit.state_path, &store) {
                        eprintln!("failed to reset widget positions: {error:#}");
                    }
                }
            }

            if changed {
                commit.finish_runtime_change(
                    &weak,
                    status_settings,
                    true,
                    false,
                    "failed to reset widget positions",
                );
            }
        }
    });

    ui.on_hide_all_widgets({
        let widgets_hidden = widgets_hidden.clone();
        let commit = commit_context.clone();
        let weak = ui.as_weak();
        move || {
            let status_settings = commit.current_settings();
            let mut changed = false;
            {
                let mut store = commit.layouts.borrow_mut();
                for instance in &mut store.widgets {
                    if instance.visible {
                        instance.visible = false;
                        changed = true;
                    }
                }
                if changed {
                    if let Err(error) = save_layout_store(&commit.state_path, &store) {
                        eprintln!("failed to hide widgets: {error:#}");
                    }
                }
            }

            if changed {
                *widgets_hidden.borrow_mut() = true;
                commit.finish_runtime_change(
                    &weak,
                    status_settings,
                    false,
                    false,
                    "failed to hide widgets",
                );
            }
        }
    });

    ui.on_widget_list_press({
        let layouts = layouts.clone();
        let state_path = state_path.clone();
        let widget_reorder_state = widget_reorder_state.clone();
        let weak = ui.as_weak();
        move |selected_index| {
            let selected_index = selected_index.max(0);
            let now = Instant::now();
            let mut reorder = widget_reorder_state.borrow_mut();
            let is_double_press = reorder.last_press_index == Some(selected_index)
                && reorder
                    .last_press_at
                    .map(|last_press| {
                        now.duration_since(last_press) <= WIDGET_REORDER_DOUBLE_CLICK_TIMEOUT
                    })
                    .unwrap_or(false);

            reorder.last_press_index = Some(selected_index);
            reorder.last_press_at = Some(now);
            reorder.consumed_steps = 0;

            if !is_double_press {
                return;
            }

            reorder.active_index = Some(selected_index);
            drop(reorder);

            {
                let mut store = layouts.borrow_mut();
                select_widget_by_index(&mut store, selected_index);
                if let Err(error) = save_layout_store(&state_path, &store) {
                    eprintln!("failed to save selected widget: {error:#}");
                }
            }

            if let Some(ui) = weak.upgrade() {
                ui.set_widget_reorder_active(true);
                ui.set_widget_reorder_index(selected_index);
                ui.set_selected_widget_index(selected_index);
            }
        }
    });

    ui.on_widget_list_drag({
        let widget_reorder_state = widget_reorder_state.clone();
        let commit = commit_context.clone();
        let weak = ui.as_weak();
        move |delta_y| {
            let target_steps = widget_reorder_steps(delta_y);
            let mut changed = false;
            let mut active_index_after_move = None;

            {
                let mut reorder = widget_reorder_state.borrow_mut();
                let Some(mut active_index) = reorder.active_index else {
                    return;
                };

                let mut pending_steps = target_steps - reorder.consumed_steps;
                if pending_steps == 0 {
                    return;
                }

                let mut store = commit.layouts.borrow_mut();
                while pending_steps != 0 {
                    let direction = pending_steps.signum();
                    let Some(next_index) =
                        move_widget_in_store(&mut store, active_index, direction)
                    else {
                        break;
                    };

                    active_index = next_index as i32;
                    reorder.active_index = Some(active_index);
                    reorder.consumed_steps += direction;
                    changed = true;
                    pending_steps -= direction;
                }

                if changed {
                    active_index_after_move = Some(active_index);
                    if let Err(error) = save_layout_store(&commit.state_path, &store) {
                        eprintln!("failed to save widget order: {error:#}");
                    }
                }
            }

            if !changed {
                return;
            }

            commit.sync_widget_runtimes(false, "failed to reorder widget windows");
            commit.apply_widget_pinning_for_settings_mode();
            commit.set_saved_status(commit.current_settings());
            commit.refresh_settings_window(&weak);
            if let Some(ui) = weak.upgrade() {
                if let Some(index) = active_index_after_move {
                    ui.set_widget_reorder_active(true);
                    ui.set_widget_reorder_index(index);
                    ui.set_selected_widget_index(index);
                }
            }
            schedule_widget_shell_window_configuration();
        }
    });

    ui.on_widget_list_release({
        let widget_reorder_state = widget_reorder_state.clone();
        let weak = ui.as_weak();
        move || {
            let mut reorder = widget_reorder_state.borrow_mut();
            reorder.active_index = None;
            reorder.consumed_steps = 0;
            drop(reorder);

            if let Some(ui) = weak.upgrade() {
                ui.set_widget_reorder_active(false);
                ui.set_widget_reorder_index(-1);
            }
        }
    });

    ui.on_add_widget({
        let commit = commit_context.clone();
        let weak = ui.as_weak();
        move |plugin_index| {
            let settings = commit.current_settings();
            {
                let Some(plugin) = commit
                    .plugin_catalog
                    .market_plugins()
                    .nth(plugin_index.max(0) as usize)
                else {
                    eprintln!("plugin market index {plugin_index} is out of range");
                    return;
                };
                if !plugin.is_available() {
                    eprintln!("plugin {} is unavailable and cannot be added", plugin.id);
                    return;
                }
                let mut store = commit.layouts.borrow_mut();
                if add_plugin_widget_to_store(&mut store, plugin, &settings).is_none() {
                    eprintln!("plugin {} is unavailable and cannot be added", plugin.id);
                    return;
                }
                if let Err(error) = save_layout_store(&commit.state_path, &store) {
                    eprintln!("failed to save added widget: {error:#}");
                }
            }
            commit.finish_runtime_change(
                &weak,
                settings,
                true,
                true,
                "failed to add widget window",
            );
            schedule_settings_window_raise();
        }
    });

    ui.on_apply_widget_settings({
        let commit = commit_context.clone();
        let weak = ui.as_weak();
        move |selected_index,
              widget_name_text,
              always_on_top,
              layout_locked,
              opacity_percent,
              widget_scale_percent,
              show_coin_logos,
              hide_quote_asset| {
            let settings = commit.current_settings();
            let locale = i18n::resolve_locale(settings.language);
            {
                let mut store = commit.layouts.borrow_mut();
                apply_widget_settings_to_store(
                    &mut store,
                    WidgetSettingsUpdate {
                        selected_index,
                        widget_name: widget_name_text.as_str(),
                        always_on_top,
                        layout_locked,
                        opacity_percent,
                        widget_scale_percent,
                        show_coin_logos,
                        hide_quote_asset,
                        locale,
                        plugin_catalog: &commit.plugin_catalog,
                    },
                );
                if let Err(error) = save_layout_store(&commit.state_path, &store) {
                    eprintln!("failed to save widget settings: {error:#}");
                }
            }
            commit.sync_widget_runtimes(false, "failed to apply widget settings");
            commit.apply_widget_pinning_for_settings_mode();
            commit.update_market_feed_config_from_store();
            commit.set_saved_status(settings.clone());
            if let Some(ui) = weak.upgrade() {
                ui.set_widget_list_options(owned_string_model(widget_instance_options(
                    &commit.layouts.borrow(),
                    locale,
                    &commit.plugin_catalog,
                )));
                ui.set_widget_list_detail_options(owned_string_model(
                    widget_instance_detail_options(
                        &commit.layouts.borrow(),
                        locale,
                        &commit.plugin_catalog,
                    ),
                ));
                ui.set_widget_scale_options(int_model(widget_scale_options(
                    &commit.layouts.borrow(),
                    &commit.plugin_catalog,
                )));
                ui.set_widget_scale_min_options(int_model(widget_scale_min_options(
                    &commit.layouts.borrow(),
                    &commit.plugin_catalog,
                )));
                ui.set_widget_scale_max_options(int_model(widget_scale_max_options(
                    &commit.layouts.borrow(),
                    &commit.plugin_catalog,
                )));
                let status = commit.settings_status.borrow().clone();
                ui.set_status_text(status.as_str().into());
            }
            schedule_widget_shell_window_configuration();
        }
    });

    ui.on_apply_widget_scale({
        let commit = commit_context.clone();
        let weak = ui.as_weak();
        move |selected_index, widget_scale_percent| {
            let settings = commit.current_settings();
            {
                let mut store = commit.layouts.borrow_mut();
                apply_widget_scale_to_store(
                    &mut store,
                    selected_index,
                    widget_scale_percent,
                    &commit.plugin_catalog,
                );
                if let Err(error) = save_layout_store(&commit.state_path, &store) {
                    eprintln!("failed to save widget scale: {error:#}");
                }
            }
            commit.sync_widget_runtimes(false, "failed to apply widget scale");
            commit.apply_widget_pinning_for_settings_mode();
            commit.set_saved_status(settings.clone());
            if let Some(ui) = weak.upgrade() {
                ui.set_widget_scale_percent(widget_scale_percent);
                ui.set_widget_scale_options(int_model(widget_scale_options(
                    &commit.layouts.borrow(),
                    &commit.plugin_catalog,
                )));
                ui.set_widget_scale_min_options(int_model(widget_scale_min_options(
                    &commit.layouts.borrow(),
                    &commit.plugin_catalog,
                )));
                ui.set_widget_scale_max_options(int_model(widget_scale_max_options(
                    &commit.layouts.borrow(),
                    &commit.plugin_catalog,
                )));
                let status = commit.settings_status.borrow().clone();
                ui.set_status_text(status.as_str().into());
            }
            schedule_widget_shell_window_configuration();
        }
    });

    ui.on_delete_widget({
        let commit = commit_context.clone();
        let weak = ui.as_weak();
        move |selected_index| {
            let settings = commit.current_settings();
            let removed = {
                let mut store = commit.layouts.borrow_mut();
                let removed = delete_widget_from_store_at_index(&mut store, selected_index);
                if removed {
                    if let Err(error) = save_layout_store(&commit.state_path, &store) {
                        eprintln!("failed to save deleted widget: {error:#}");
                    }
                }
                removed
            };
            if removed {
                commit.finish_runtime_change(
                    &weak,
                    settings,
                    false,
                    true,
                    "failed to delete widget window",
                );
            }
        }
    });

    ui.on_open_about_link(move || {
        if let Err(error) = open_external_url(ABOUT_REPOSITORY_URL) {
            eprintln!("failed to open about link: {error:#}");
        }
    });

    ui.on_clear_icon_cache({
        let commit = commit_context.clone();
        let weak = ui.as_weak();
        move || {
            let locale = i18n::resolve_locale(commit.current_settings().language);
            let status = match commit.coin_icons.clear_cache() {
                Ok(deleted) => {
                    commit.apply_settings_to_widgets();
                    status_icon_cache_cleared(deleted, locale)
                }
                Err(error) => {
                    format!(
                        "{}: {error}",
                        i18n::text(locale).status_icon_cache_clear_failed
                    )
                }
            };
            *commit.settings_status.borrow_mut() = status;
            commit.refresh_settings_window(&weak);
        }
    });

    ui.on_install_update({
        let layouts = layouts.clone();
        let settings_status = settings_status.clone();
        let weak = ui.as_weak();
        move || {
            let settings = layouts.borrow().settings.clone().normalized();
            let text = i18n::text(i18n::resolve_locale(settings.language));
            match updater::install_latest_update(settings::effective_network_proxy_url(&settings)) {
                Ok(()) => {
                    *settings_status.borrow_mut() = text.status_update_started.to_string();
                    if let Some(ui) = weak.upgrade() {
                        ui.set_status_text(settings_status.borrow().as_str().into());
                    }
                    if let Err(error) = slint::quit_event_loop() {
                        eprintln!("failed to quit Slint event loop for update install: {error:#}");
                    }
                }
                Err(error) => {
                    *settings_status.borrow_mut() =
                        format!("{}: {error:#}", text.status_update_failed);
                    if let Some(ui) = weak.upgrade() {
                        ui.set_status_text(settings_status.borrow().as_str().into());
                    }
                }
            }
        }
    });

    ui.on_minimize_settings({
        let weak = ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade() {
                ui.window().set_minimized(true);
            }
        }
    });

    ui.on_close_settings({
        let widgets = widgets.clone();
        let layouts = layouts.clone();
        let settings_mode_active = settings_mode_active.clone();
        let weak = ui.as_weak();
        move || {
            leave_settings_mode(&widgets, &layouts, &settings_mode_active);
            if let Some(ui) = weak.upgrade() {
                if let Err(error) = ui.hide() {
                    eprintln!("failed to hide settings window: {error:#}");
                }
            }
        }
    });

    Ok(ui)
}

fn schedule_widget_layout_lock_sync(
    weak: slint::Weak<SettingsWindow>,
    layouts: Rc<RefCell<LayoutStore>>,
) {
    Timer::single_shot(Duration::from_millis(200), move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let selected_index = ui.get_selected_widget_index().max(0) as usize;
        let locked = layouts
            .borrow()
            .widgets
            .get(selected_index)
            .map(|widget| widget.layout.locked)
            .unwrap_or(false);
        if ui.get_widget_layout_locked() != locked {
            ui.set_widget_layout_locked(locked);
        }
        schedule_widget_layout_lock_sync(ui.as_weak(), layouts);
    });
}

fn status_saved(_: AppSettings) -> String {
    String::new()
}

fn status_icon_cache_cleared(deleted: usize, locale: i18n::Locale) -> String {
    match locale {
        i18n::Locale::En => format!("Icon cache cleared ({deleted} files removed)"),
        i18n::Locale::ZhHans => format!("图标缓存已清空（已移除 {deleted} 个文件）"),
    }
}

fn normalized_network_proxy_input(
    enabled: bool,
    proxy_url: &str,
) -> std::result::Result<String, String> {
    let proxy_url = settings::normalize_network_proxy_url(proxy_url.to_string());
    if enabled {
        if proxy_url.is_empty() {
            return Err("empty proxy address".to_string());
        }
        ureq::Proxy::new(&proxy_url).map_err(|error| error.to_string())?;
    }
    Ok(proxy_url)
}

fn apply_settings_to_store(
    layouts: &Rc<RefCell<LayoutStore>>,
    state_path: &std::path::Path,
    settings: AppSettings,
) {
    let mut store = layouts.borrow_mut();
    store.settings = settings;
    if let Err(error) = save_layout_store(state_path, &store) {
        eprintln!("failed to save widget settings: {error:#}");
    }
}

fn apply_default_widget_scale_to_instances(
    layouts: &Rc<RefCell<LayoutStore>>,
    plugin_catalog: &plugin::PluginCatalog,
    next_scale_percent: i32,
) {
    let definitions = widget_definitions_from_catalog(plugin_catalog);
    let mut store = layouts.borrow_mut();
    for instance in &mut store.widgets {
        apply_widget_scale_to_instance(instance, &definitions, next_scale_percent);
    }
    normalize_store_with_catalog(&mut store, 0, Some(plugin_catalog));
}

fn model_symbols(model: ModelRc<SharedString>) -> Vec<String> {
    (0..model.row_count())
        .filter_map(|index| model.row_data(index))
        .filter_map(|symbol| settings::normalize_market_pair_key(symbol.as_str()))
        .fold(Vec::new(), |mut symbols, symbol| {
            if !symbols.contains(&symbol) {
                symbols.push(symbol);
            }
            symbols
        })
}

fn model_raw_string_at(model: ModelRc<SharedString>, index: i32) -> Option<String> {
    if index < 0 {
        return None;
    }
    model
        .row_data(index as usize)
        .map(|value| value.to_string())
}

fn current_ui_locale(ui: &SettingsWindow) -> i18n::Locale {
    i18n::resolve_locale(LanguagePreference::from_index(ui.get_language_index()))
}

fn enabled_market_sources_from_ui(ui: &SettingsWindow) -> Vec<settings::MarketDataSource> {
    let mut sources = Vec::new();
    if ui.get_market_binance_enabled() {
        sources.push(settings::MarketDataSource::Binance);
    }
    if ui.get_market_okx_enabled() {
        sources.push(settings::MarketDataSource::Okx);
    }
    if ui.get_market_hyperliquid_enabled() {
        sources.push(settings::MarketDataSource::Hyperliquid);
    }
    if sources.is_empty() {
        vec![settings::MarketDataSource::Binance]
    } else {
        sources
    }
}

fn add_default_symbol_to_store(
    layouts: &Rc<RefCell<LayoutStore>>,
    state_path: &std::path::Path,
    symbol: &str,
) -> Option<AppSettings> {
    let symbol = settings::normalize_market_pair_key(symbol)?;
    let mut store = layouts.borrow_mut();
    let mut app_settings = store.settings.clone().normalized();
    app_settings.market_default_symbols = add_symbol_with_limit(
        app_settings.market_default_symbols.clone(),
        symbol,
        WidgetType::QuoteBoard.symbol_limit(),
        settings::default_symbols_for_type(WidgetType::QuoteBoard),
    );
    store.settings = app_settings.normalized();
    if let Err(error) = save_layout_store(state_path, &store) {
        eprintln!("failed to save default symbols: {error:#}");
    }
    Some(store.settings.clone())
}

fn remove_default_symbol_from_store(
    layouts: &Rc<RefCell<LayoutStore>>,
    state_path: &std::path::Path,
    symbol_index: i32,
) -> Option<AppSettings> {
    if symbol_index < 0 {
        return None;
    }
    let mut store = layouts.borrow_mut();
    let mut app_settings = store.settings.clone().normalized();
    app_settings.market_default_symbols = remove_symbol_with_limit(
        app_settings.market_default_symbols.clone(),
        symbol_index as usize,
        WidgetType::QuoteBoard.symbol_limit(),
        settings::default_symbols_for_type(WidgetType::QuoteBoard),
    );
    store.settings = app_settings.normalized();
    if let Err(error) = save_layout_store(state_path, &store) {
        eprintln!("failed to save default symbols: {error:#}");
    }
    Some(store.settings.clone())
}

fn add_widget_symbol_to_store(
    layouts: &Rc<RefCell<LayoutStore>>,
    state_path: &std::path::Path,
    selected_index: i32,
    symbol: &str,
    plugin_catalog: &plugin::PluginCatalog,
) -> Option<AppSettings> {
    let symbol = settings::normalize_market_pair_key(symbol)?;
    update_widget_symbols_in_store(
        layouts,
        state_path,
        selected_index,
        plugin_catalog,
        |symbols, min, limit, fallback| {
            add_symbol_with_bounds(symbols, symbol, min, limit, fallback)
        },
    )
}

fn remove_widget_symbol_from_store(
    layouts: &Rc<RefCell<LayoutStore>>,
    state_path: &std::path::Path,
    selected_index: i32,
    symbol_index: i32,
    plugin_catalog: &plugin::PluginCatalog,
) -> Option<AppSettings> {
    if symbol_index < 0 {
        return None;
    }
    update_widget_symbols_in_store(
        layouts,
        state_path,
        selected_index,
        plugin_catalog,
        |symbols, min, limit, fallback| {
            remove_symbol_with_bounds(symbols, symbol_index as usize, min, limit, fallback)
        },
    )
}

fn update_widget_symbols_in_store<F>(
    layouts: &Rc<RefCell<LayoutStore>>,
    state_path: &std::path::Path,
    selected_index: i32,
    plugin_catalog: &plugin::PluginCatalog,
    update: F,
) -> Option<AppSettings>
where
    F: FnOnce(Vec<String>, usize, usize, Vec<String>) -> Vec<String>,
{
    let settings = layouts.borrow().settings.clone().normalized();
    let definitions = widget_definitions_from_catalog(plugin_catalog);
    let mut store = layouts.borrow_mut();
    select_widget_by_index(&mut store, selected_index);
    let selected_id = store.selected_widget_id.clone();
    let instance = selected_id
        .as_deref()
        .and_then(|id| store.widgets.iter_mut().find(|instance| instance.id == id))?;
    let min = settings::symbol_min_for_instance(instance, &definitions);
    let limit = settings::symbol_limit_for_instance(instance, &definitions);
    let fallback =
        settings::default_symbols_for_instance_from_settings(instance, &definitions, &settings);
    let scale_percent = widget_scale_percent_for_definitions(instance, &definitions);
    instance.symbols = update(instance.symbols.clone(), min, limit, fallback);
    apply_widget_scale_to_instance(instance, &definitions, scale_percent);
    normalize_store_with_catalog(&mut store, 0, Some(plugin_catalog));
    if let Err(error) = save_layout_store(state_path, &store) {
        eprintln!("failed to save widget symbols: {error:#}");
    }
    Some(store.settings.clone().normalized())
}

fn add_symbol_with_bounds(
    mut symbols: Vec<String>,
    symbol: String,
    min: usize,
    limit: usize,
    fallback: Vec<String>,
) -> Vec<String> {
    if symbols.len() < limit && !symbols.contains(&symbol) {
        symbols.push(symbol);
    }
    settings::normalized_symbols_with_bounds(symbols, min, limit, fallback)
}

fn remove_symbol_with_bounds(
    mut symbols: Vec<String>,
    symbol_index: usize,
    min: usize,
    limit: usize,
    fallback: Vec<String>,
) -> Vec<String> {
    if symbol_index < symbols.len() {
        symbols.remove(symbol_index);
    }
    settings::normalized_symbols_with_bounds(symbols, min, limit, fallback)
}

fn add_symbol_with_limit(
    mut symbols: Vec<String>,
    symbol: String,
    limit: usize,
    fallback: Vec<String>,
) -> Vec<String> {
    if symbols.len() < limit && !symbols.contains(&symbol) {
        symbols.push(symbol);
    }
    settings::normalized_symbols_with_limit(symbols, limit, fallback)
}

fn remove_symbol_with_limit(
    mut symbols: Vec<String>,
    symbol_index: usize,
    limit: usize,
    fallback: Vec<String>,
) -> Vec<String> {
    if symbol_index < symbols.len() {
        symbols.remove(symbol_index);
    }
    settings::normalized_symbols_with_limit(symbols, limit, fallback)
}

fn collect_symbol_fallback_symbols(
    store: &LayoutStore,
    plugin_catalog: &plugin::PluginCatalog,
) -> Vec<String> {
    let definitions = widget_definitions_from_catalog(plugin_catalog);
    let mut symbols = settings::default_market_symbols();
    symbols.extend(store.settings.clone().normalized().market_default_symbols);
    for widget in &store.widgets {
        symbols.extend(settings::normalized_symbols_for_instance(
            widget,
            &definitions,
        ));
    }
    if feature_flags::ALERT_RULES_ENABLED {
        for rule in &store.settings.alert_rules {
            if let Some(symbol) = settings::normalize_market_pair_key(&rule.symbol) {
                symbols.push(symbol);
            }
        }
    }
    sorted_symbol_values(symbols)
}

fn refresh_symbol_selector_models_from_ui(
    ui: &SettingsWindow,
    state: &SymbolCatalogState,
    locale: i18n::Locale,
) {
    let alert_symbol = feature_flags::ALERT_RULES_ENABLED
        .then(|| current_alert_symbol_from_ui(ui))
        .flatten();
    refresh_alert_symbol_options(
        ui,
        state,
        &enabled_market_sources_from_ui(ui),
        alert_symbol.as_deref(),
    );
    refresh_symbol_picker_models_from_ui(ui, state, locale);
}

fn open_symbol_picker(ui: &SettingsWindow, mode: i32) {
    let locale = current_ui_locale(ui);
    ui.set_symbol_picker_open(true);
    ui.set_symbol_picker_mode(mode);
    ui.set_symbol_picker_search_text("".into());
    ui.set_symbol_picker_candidate_index(0);
    ui.set_symbol_picker_title_text(symbol_picker_title_text(mode, locale).into());
    ui.set_symbol_picker_confirm_text(symbol_picker_confirm_text(locale).into());
    ui.set_symbol_picker_cancel_text(symbol_picker_cancel_text(locale).into());

    let state = symbol_catalog_snapshot();
    refresh_symbol_picker_models_from_ui(ui, &state, locale);
}

fn close_symbol_picker(ui: &SettingsWindow) {
    let locale = current_ui_locale(ui);
    ui.set_symbol_picker_open(false);
    ui.set_symbol_picker_search_text("".into());
    ui.set_symbol_picker_candidate_options(owned_string_model(Vec::new()));
    ui.set_symbol_picker_candidate_values(owned_string_model(Vec::new()));
    ui.set_symbol_picker_candidate_index(0);
    ui.set_symbol_picker_status_text("".into());
    ui.set_symbol_picker_confirm_text(symbol_picker_confirm_text(locale).into());
    ui.set_symbol_picker_cancel_text(symbol_picker_cancel_text(locale).into());
}

fn refresh_symbol_picker_models_from_ui(
    ui: &SettingsWindow,
    state: &SymbolCatalogState,
    locale: i18n::Locale,
) {
    if !ui.get_symbol_picker_open() {
        return;
    }

    let mode = ui.get_symbol_picker_mode();
    let enabled_sources = enabled_market_sources_from_ui(ui);
    let query = ui.get_symbol_picker_search_text().to_string();
    let selected_symbols = if mode == SYMBOL_PICKER_MODE_DEFAULT {
        model_symbols(ui.get_default_symbol_values())
    } else {
        model_symbols(ui.get_widget_symbol_values())
    };
    let max = if mode == SYMBOL_PICKER_MODE_DEFAULT {
        ui.get_default_symbol_max().max(0) as usize
    } else {
        ui.get_widget_symbol_max().max(0) as usize
    };
    let options = if selected_symbols.len() >= max {
        Vec::new()
    } else {
        symbol_add_options_with_query(state, &enabled_sources, &selected_symbols, &query)
    };
    let display_options = options
        .iter()
        .map(|symbol| format_symbol_option(symbol))
        .collect::<Vec<_>>();

    ui.set_symbol_picker_candidate_options(owned_string_model(display_options));
    ui.set_symbol_picker_candidate_values(owned_string_model(options.clone()));
    ui.set_symbol_picker_candidate_index(0);
    ui.set_symbol_picker_title_text(symbol_picker_title_text(mode, locale).into());
    ui.set_symbol_picker_status_text(
        symbol_picker_status_text(
            mode,
            selected_symbols.len(),
            max,
            options.len(),
            &query,
            state.fallback_only,
            locale,
        )
        .into(),
    );
}

fn refresh_alert_symbol_options(
    ui: &SettingsWindow,
    state: &SymbolCatalogState,
    enabled_sources: &[settings::MarketDataSource],
    alert_symbol: Option<&str>,
) {
    let alert_values = if feature_flags::ALERT_RULES_ENABLED {
        symbol_pick_options(
            state,
            enabled_sources,
            alert_symbol.into_iter().map(str::to_string),
        )
    } else {
        Vec::new()
    };
    let alert_options = alert_values
        .iter()
        .map(|symbol| format_symbol_option(symbol))
        .collect::<Vec<_>>();
    let normalized_alert_symbol = alert_symbol.and_then(settings::normalize_market_pair_key);
    let alert_index = normalized_alert_symbol
        .as_deref()
        .and_then(|symbol| {
            alert_values
                .iter()
                .position(|option| option == symbol)
                .map(|index| index as i32)
        })
        .unwrap_or(0);
    ui.set_alert_symbol_options(owned_string_model(alert_options));
    ui.set_alert_symbol_index(alert_index);
}

fn current_alert_symbol_from_ui(ui: &SettingsWindow) -> Option<String> {
    let options = model_symbols(ui.get_alert_symbol_options());
    options
        .get(ui.get_alert_symbol_index().max(0) as usize)
        .cloned()
        .or_else(|| settings::default_market_symbols().into_iter().next())
}

fn symbol_add_options_with_query(
    state: &SymbolCatalogState,
    enabled_sources: &[settings::MarketDataSource],
    selected_symbols: &[String],
    query: &str,
) -> Vec<String> {
    let selected = selected_symbols
        .iter()
        .filter_map(|symbol| settings::normalize_market_pair_key(symbol))
        .collect::<Vec<_>>();
    symbol_pick_options_with_query(state, enabled_sources, selected.clone(), query)
        .into_iter()
        .filter(|symbol| !selected.contains(symbol))
        .collect()
}

fn symbol_pick_options(
    state: &SymbolCatalogState,
    enabled_sources: &[settings::MarketDataSource],
    extra_symbols: impl IntoIterator<Item = String>,
) -> Vec<String> {
    symbol_pick_options_with_query(state, enabled_sources, extra_symbols, "")
}

fn symbol_pick_options_with_query(
    state: &SymbolCatalogState,
    enabled_sources: &[settings::MarketDataSource],
    extra_symbols: impl IntoIterator<Item = String>,
    query: &str,
) -> Vec<String> {
    let mut symbols = state
        .catalog
        .entries
        .iter()
        .filter(|entry| enabled_sources.contains(&entry.source))
        .map(|entry| entry.key.clone())
        .collect::<Vec<_>>();
    if symbols.is_empty() {
        symbols.extend(settings::default_market_symbols());
    }
    symbols.extend(state.fallback_symbols.iter().cloned());
    symbols.extend(extra_symbols);
    ranked_symbol_values(symbols, query)
}

fn sorted_symbol_values(symbols: Vec<String>) -> Vec<String> {
    ranked_symbol_values(symbols, "")
}

fn ranked_symbol_values(symbols: Vec<String>, query: &str) -> Vec<String> {
    let query = query.trim();
    let symbols = symbols
        .into_iter()
        .filter_map(|symbol| settings::normalize_market_pair_key(&symbol))
        .fold(Vec::new(), |mut unique, symbol| {
            if !unique.contains(&symbol) {
                unique.push(symbol);
            }
            unique
        });

    let mut ranked = symbols
        .into_iter()
        .enumerate()
        .filter(|(_, symbol)| symbol_matches_query(symbol, query))
        .collect::<Vec<_>>();
    ranked.sort_by(|(left_index, left), (right_index, right)| {
        symbol_rank(left, query, *left_index).cmp(&symbol_rank(right, query, *right_index))
    });
    ranked.into_iter().map(|(_, symbol)| symbol).collect()
}

fn symbol_rank(symbol: &str, query: &str, source_index: usize) -> (usize, usize, usize, usize) {
    let base = settings::normalize_symbol_token(symbol).unwrap_or_default();
    if query.trim().is_empty() {
        return (
            popular_symbol_rank(&base).unwrap_or(usize::MAX),
            if popular_symbol_rank(&base).is_some() {
                0
            } else {
                1
            },
            source_rank(symbol),
            source_index,
        );
    }

    (
        symbol_query_rank(symbol, query),
        popular_symbol_rank(&base).unwrap_or(usize::MAX),
        source_rank(symbol),
        source_index,
    )
}

fn source_rank(symbol: &str) -> usize {
    match settings::market_pair_source(symbol) {
        Some(settings::MarketDataSource::Binance) => 0,
        Some(settings::MarketDataSource::Okx) => 1,
        Some(settings::MarketDataSource::Hyperliquid) => 2,
        None => usize::MAX,
    }
}

fn popular_symbol_rank(symbol: &str) -> Option<usize> {
    POPULAR_SYMBOL_ORDER
        .iter()
        .position(|candidate| candidate == &symbol)
}

fn symbol_matches_query(symbol: &str, query: &str) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return true;
    }
    let normalized_query = settings::normalize_symbol_token(query);
    let base = settings::normalize_symbol_token(symbol).unwrap_or_default();
    let query_lower = query.to_ascii_lowercase();
    let haystack = symbol_search_haystack(symbol);
    normalized_query.as_deref().is_some_and(|query| {
        base.contains(query) || settings::format_market_pair_symbol(symbol).contains(query)
    }) || haystack.contains(&query_lower)
}

fn symbol_query_rank(symbol: &str, query: &str) -> usize {
    let normalized_query = settings::normalize_symbol_token(query).unwrap_or_default();
    let query_lower = query.trim().to_ascii_lowercase();
    let base = settings::normalize_symbol_token(symbol).unwrap_or_default();
    let base_lower = base.to_ascii_lowercase();
    let pair_lower = settings::format_market_pair_symbol(symbol).to_ascii_lowercase();
    let key_lower = symbol.to_ascii_lowercase();
    let name_lower = symbol_name(&base)
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    let haystack = symbol_search_haystack(symbol);

    if base == normalized_query
        || base_lower == query_lower
        || pair_lower.replace('/', "") == query_lower
        || pair_lower == query_lower
        || pair_lower.replace('/', "-") == query_lower
    {
        0
    } else if !normalized_query.is_empty() && base.starts_with(&normalized_query) {
        1
    } else if !name_lower.is_empty() && name_lower.starts_with(&query_lower) {
        2
    } else if !normalized_query.is_empty() && base.contains(&normalized_query) {
        3
    } else if haystack.contains(&query_lower) || key_lower.contains(&query_lower) {
        4
    } else {
        5
    }
}

fn symbol_search_haystack(symbol: &str) -> String {
    let base = settings::normalize_symbol_token(symbol).unwrap_or_default();
    let pair = settings::format_market_pair_symbol(symbol);
    let source = settings::format_market_pair_source(symbol);
    let mut values = vec![
        base.to_ascii_lowercase(),
        pair.to_ascii_lowercase(),
        pair.replace('/', "").to_ascii_lowercase(),
        pair.replace('/', "-").to_ascii_lowercase(),
        source.to_ascii_lowercase(),
        symbol.to_ascii_lowercase(),
    ];
    if let Some(metadata) = symbol_metadata(&base) {
        values.push(metadata.name.to_ascii_lowercase());
        values.extend(
            metadata
                .aliases
                .iter()
                .map(|alias| alias.to_ascii_lowercase()),
        );
    }
    values.join(" ")
}

fn symbol_metadata(symbol: &str) -> Option<&'static SymbolMetadata> {
    SYMBOL_METADATA
        .iter()
        .find(|metadata| metadata.symbol == symbol)
}

fn symbol_name(symbol: &str) -> Option<&'static str> {
    symbol_metadata(symbol).map(|metadata| metadata.name)
}

fn format_symbol_option(symbol: &str) -> String {
    let base = settings::normalize_symbol_token(symbol).unwrap_or_default();
    let pair = settings::format_market_pair_symbol(symbol);
    let source = settings::format_market_pair_source(symbol);
    match symbol_name(&base) {
        Some(name) if !source.is_empty() => format!("{pair} · {source} · {name}"),
        Some(name) => format!("{pair} · {name}"),
        None if !source.is_empty() => format!("{pair} · {source}"),
        None => pair,
    }
}

fn format_symbol_chip_label(symbol: &str) -> String {
    settings::format_market_pair_symbol(symbol)
}

const WIDGET_SYMBOL_CHIP_START_X: i32 = 17;
const WIDGET_SYMBOL_CHIP_START_Y: i32 = 315;
const WIDGET_SYMBOL_CHIP_AVAILABLE_WIDTH: i32 = 485;
const WIDGET_SYMBOL_CHIP_ROW_HEIGHT: i32 = 36;
const WIDGET_SYMBOL_CHIP_GAP: i32 = 8;
const WIDGET_SYMBOL_ADD_BUTTON_WIDTH: i32 = 69;
const WIDGET_SYMBOL_MAX_COUNT: usize = 5;

fn widget_symbol_chip_layout(labels: &[String]) -> (Vec<i32>, Vec<i32>, Vec<i32>, i32, i32, i32) {
    let mut x_values = Vec::with_capacity(labels.len());
    let mut y_values = Vec::with_capacity(labels.len());
    let mut widths = Vec::with_capacity(labels.len());
    let mut cursor = 0;
    let mut row = 0;

    for label in labels {
        let width = symbol_chip_width(label);
        if cursor > 0 && cursor + width > WIDGET_SYMBOL_CHIP_AVAILABLE_WIDTH {
            row += 1;
            cursor = 0;
        }

        x_values.push(WIDGET_SYMBOL_CHIP_START_X + cursor);
        y_values.push(WIDGET_SYMBOL_CHIP_START_Y + row * WIDGET_SYMBOL_CHIP_ROW_HEIGHT);
        widths.push(width);
        cursor += width + WIDGET_SYMBOL_CHIP_GAP;
    }

    let can_add_symbol = labels.len() < WIDGET_SYMBOL_MAX_COUNT;
    if can_add_symbol {
        if cursor > 0
            && cursor + WIDGET_SYMBOL_ADD_BUTTON_WIDTH > WIDGET_SYMBOL_CHIP_AVAILABLE_WIDTH
        {
            row += 1;
            cursor = 0;
        }

        let add_x = WIDGET_SYMBOL_CHIP_START_X + cursor;
        let add_y = WIDGET_SYMBOL_CHIP_START_Y + row * WIDGET_SYMBOL_CHIP_ROW_HEIGHT;
        let status_y = add_y + WIDGET_SYMBOL_CHIP_ROW_HEIGHT + 8;
        return (x_values, y_values, widths, add_x, add_y, status_y);
    }

    let add_x = WIDGET_SYMBOL_CHIP_START_X + cursor;
    let add_y = WIDGET_SYMBOL_CHIP_START_Y + row * WIDGET_SYMBOL_CHIP_ROW_HEIGHT;
    let status_y = WIDGET_SYMBOL_CHIP_START_Y
        + row * WIDGET_SYMBOL_CHIP_ROW_HEIGHT
        + WIDGET_SYMBOL_CHIP_ROW_HEIGHT
        + 8;

    (x_values, y_values, widths, add_x, add_y, status_y)
}

fn symbol_chip_width(label: &str) -> i32 {
    let text_width = label.chars().map(symbol_chip_char_width).sum::<i32>();
    (text_width + 32).clamp(76, 84)
}

fn symbol_chip_char_width(ch: char) -> i32 {
    if ch.is_ascii_alphanumeric() {
        7
    } else if ch.is_ascii_whitespace() {
        4
    } else if ch == '/' || ch == '-' {
        5
    } else if ch == '·' {
        8
    } else {
        12
    }
}

fn primary_alert_rule_from_input(
    enabled: bool,
    symbol_input: &str,
    condition_index: i32,
    threshold_input: &str,
) -> Result<Option<AlertRule>, ()> {
    if !enabled && threshold_input.trim().is_empty() {
        return Ok(None);
    }

    let symbol = settings::normalize_market_pair_key(symbol_input).ok_or(())?;
    let threshold = threshold_input.trim().parse::<f64>().map_err(|_| ())?;
    if !threshold.is_finite() {
        return Err(());
    }

    Ok(Some(AlertRule {
        id: PRIMARY_ALERT_ID.to_string(),
        symbol,
        condition: alert_condition_from_index(condition_index),
        threshold,
        enabled,
    }))
}

fn alert_condition_from_index(index: i32) -> AlertCondition {
    match index {
        1 => AlertCondition::PriceBelow,
        2 => AlertCondition::ChangePercentAbove,
        3 => AlertCondition::ChangePercentBelow,
        _ => AlertCondition::PriceAbove,
    }
}

fn alert_condition_index(condition: AlertCondition) -> i32 {
    match condition {
        AlertCondition::PriceAbove => 0,
        AlertCondition::PriceBelow => 1,
        AlertCondition::ChangePercentAbove => 2,
        AlertCondition::ChangePercentBelow => 3,
    }
}

fn alert_threshold_text(threshold: f64) -> String {
    let text = format!("{threshold:.8}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

pub(crate) fn refresh_settings_window(
    ui: &SettingsWindow,
    layouts: &Rc<RefCell<LayoutStore>>,
    state_path: &std::path::Path,
    plugin_catalog: &plugin::PluginCatalog,
    status_override: Option<&str>,
) {
    let store = layouts.borrow();
    let settings = store.settings.clone().normalized();
    let locale = i18n::resolve_locale(settings.language);
    let text = i18n::text(locale);
    let selected_index = selected_widget_index(&store);
    let selected_widget = store.widgets.get(selected_index);
    let primary_alert = feature_flags::ALERT_RULES_ENABLED
        .then(|| settings.alert_rules.first())
        .flatten();
    let fallback_symbols = collect_symbol_fallback_symbols(&store, plugin_catalog);
    if let Ok(mut state) = shared_symbol_catalog_state().lock() {
        state.fallback_symbols = sorted_symbol_values(
            state
                .fallback_symbols
                .iter()
                .cloned()
                .chain(fallback_symbols)
                .collect(),
        );
    }
    let symbol_catalog_state = symbol_catalog_snapshot();
    ui.set_window_title_text(text.settings_title.into());
    ui.set_tab_widgets_text(text.tab_widgets.into());
    ui.set_tab_plugin_market_text(text.tab_plugin_market.into());
    ui.set_tab_market_data_text(text.tab_market_data.into());
    ui.set_tab_appearance_text(text.tab_appearance.into());
    ui.set_tab_system_text(text.tab_system.into());
    ui.set_always_on_top_text(text.always_on_top.into());
    ui.set_default_always_on_top_text(text.default_always_on_top.into());
    ui.set_opacity_text(text.opacity.into());
    ui.set_default_opacity_text(text.default_opacity.into());
    ui.set_widget_scale_text(text.widget_scale.into());
    ui.set_red_up_color_text(text.red_up_color.into());
    ui.set_market_provider_text(text.market_provider.into());
    ui.set_refresh_interval_text(text.refresh_interval.into());
    ui.set_seconds_unit_text(text.seconds_unit.into());
    ui.set_market_provider_help_text(text.market_provider_help.into());
    ui.set_refresh_interval_help_text(text.refresh_interval_help.into());
    ui.set_default_symbols_text(text.default_symbols.into());
    ui.set_market_fallback_text(text.market_fallback.into());
    ui.set_market_fallback_help_text(text.market_fallback_help.into());
    ui.set_alert_settings_text(text.alert_settings.into());
    ui.set_alert_enabled_text(text.alert_enabled.into());
    ui.set_alert_symbol_text(text.alert_symbol.into());
    ui.set_alert_condition_text(text.alert_condition.into());
    ui.set_alert_threshold_text(text.alert_threshold.into());
    ui.set_alert_clear_text(text.alert_clear.into());
    ui.set_symbols_text(text.symbols.into());
    let symbol_bounds = selected_widget.map(|widget| {
        (
            symbol_min_for_instance(widget, Some(plugin_catalog)),
            symbol_limit_for_instance(widget, Some(plugin_catalog)),
        )
    });
    ui.set_widget_symbol_min(symbol_bounds.map(|(min, _)| min as i32).unwrap_or(1));
    ui.set_widget_symbol_max(symbol_bounds.map(|(_, max)| max as i32).unwrap_or(1));
    ui.set_default_symbol_min(1);
    ui.set_default_symbol_max(WidgetType::QuoteBoard.symbol_limit() as i32);
    ui.set_symbol_search_placeholder_text(symbol_search_placeholder(locale).into());
    ui.set_symbols_help_text(
        symbol_bounds
            .map(|(min, max)| symbols_help_text(min, max, locale))
            .unwrap_or_else(|| text.symbols_help.to_string())
            .into(),
    );
    ui.set_auto_start_text(text.auto_start.into());
    ui.set_show_main_window_on_startup_text(text.show_main_window_on_startup.into());
    ui.set_shortcut_text(text.shortcut.into());
    ui.set_tray_icon_text(text.tray_icon.into());
    ui.set_tray_hover_display_text(text.tray_hover_display.into());
    ui.set_network_proxy_settings_text(text.network_proxy_settings.into());
    ui.set_network_proxy_enabled_text(text.network_proxy_enabled.into());
    ui.set_network_proxy_url_text(text.network_proxy_url.into());
    ui.set_network_proxy_http_example_text(text.network_proxy_http_example.into());
    ui.set_network_proxy_socks_example_text(text.network_proxy_socks_example.into());
    ui.set_network_proxy_help_text(text.network_proxy_help.into());
    ui.set_app_version_label_text(text.app_version.into());
    ui.set_app_version_value_text(env!("CARGO_PKG_VERSION").into());
    ui.set_about_us_text(text.about_us.into());
    ui.set_icon_cache_text(text.icon_cache.into());
    ui.set_icon_cache_help_text(text.icon_cache_help.into());
    ui.set_clear_icon_cache_text(text.clear_icon_cache.into());
    ui.set_install_update_text(text.install_update.into());
    ui.set_theme_text(text.theme.into());
    ui.set_language_text(text.language.into());
    ui.set_appearance_interface_text(text.appearance_interface.into());
    ui.set_appearance_widget_defaults_text(text.appearance_widget_defaults.into());
    ui.set_theme_help_text(text.theme_help.into());
    ui.set_language_help_text(text.language_help.into());
    ui.set_red_up_color_help_text(text.red_up_color_help.into());
    ui.set_default_opacity_help_text(text.default_opacity_help.into());
    ui.set_default_always_on_top_help_text(text.default_always_on_top_help.into());
    ui.set_system_startup_text(text.system_startup.into());
    ui.set_system_tray_text(text.system_tray.into());
    ui.set_system_app_info_text(text.system_app_info.into());
    ui.set_auto_start_help_text(text.auto_start_help.into());
    ui.set_show_main_window_on_startup_help_text(text.show_main_window_on_startup_help.into());
    ui.set_shortcut_help_text(text.shortcut_help.into());
    ui.set_tray_icon_help_text(text.tray_icon_help.into());
    ui.set_tray_hover_display_help_text(text.tray_hover_display_help.into());
    ui.set_apply_text(text.apply.into());
    ui.set_close_text(text.close.into());
    ui.set_widget_library_text(text.widget_library.into());
    ui.set_my_widgets_text(text.my_widgets.into());
    ui.set_selected_widget_text(text.selected_widget.into());
    ui.set_selected_widget_description_text(text.selected_widget_description.into());
    ui.set_widget_name_text(text.widget_name.into());
    ui.set_lock_position_help_text(text.lock_position_help.into());
    ui.set_widget_scale_help_text(text.widget_scale_help.into());
    ui.set_opacity_help_text(text.opacity_help.into());
    ui.set_widget_show_coin_logos_text(text.widget_show_coin_logos.into());
    ui.set_widget_show_coin_logos_help_text(text.widget_show_coin_logos_help.into());
    ui.set_widget_hide_quote_asset_text(text.widget_hide_quote_asset.into());
    ui.set_widget_hide_quote_asset_help_text(text.widget_hide_quote_asset_help.into());
    ui.set_widget_topmost_text(text.widget_topmost.into());
    ui.set_widget_topmost_help_text(text.widget_topmost_help.into());
    ui.set_advanced_options_text(text.advanced_options.into());
    ui.set_reset_widget_positions_text(text.reset_widget_positions.into());
    ui.set_hide_all_widgets_text(text.hide_all_widgets.into());
    ui.set_reset_text(text.reset.into());
    ui.set_delete_widget_text(text.delete_widget.into());
    ui.set_widget_visible_text(text.widget_visible.into());
    ui.set_widget_hidden_text(text.widget_hidden.into());
    ui.set_preview_text(text.preview.into());
    ui.set_preview_pairs_text(text.preview_pairs.into());
    ui.set_preview_updated_text(text.preview_updated.into());
    ui.set_preview_source_ok_text(text.preview_source_ok.into());
    ui.set_app_settings_text(text.app_settings.into());
    ui.set_add_widget_text(text.add_widget.into());
    ui.set_apply_widget_text(text.apply_widget.into());
    ui.set_no_widgets_text(text.no_widgets.into());
    ui.set_plugin_market_description_text(text.plugin_market_description.into());
    ui.set_my_widgets_description_text(text.my_widgets_description.into());
    ui.set_market_settings_description_text(text.market_settings_description.into());
    ui.set_appearance_settings_description_text(text.appearance_settings_description.into());
    ui.set_system_settings_description_text(text.system_settings_description.into());
    ui.set_quote_board_title_text(widget_type_title(WidgetType::QuoteBoard, locale).into());
    ui.set_quote_board_description_text(
        widget_type_description(WidgetType::QuoteBoard, locale).into(),
    );
    ui.set_mini_ticker_title_text(widget_type_title(WidgetType::MiniTicker, locale).into());
    ui.set_mini_ticker_description_text(
        widget_type_description(WidgetType::MiniTicker, locale).into(),
    );
    ui.set_market_settings_text(text.market_settings.into());
    ui.set_appearance_settings_text(text.appearance_settings.into());
    ui.set_system_settings_text(text.system_settings.into());
    ui.set_settings_path_label_text(text.settings_path_label.into());
    ui.set_provider_options(string_model(i18n::provider_options(locale)));
    ui.set_shortcut_options(string_model(i18n::shortcut_options(locale)));
    ui.set_theme_options(string_model(i18n::theme_options(locale)));
    ui.set_language_options(string_model(i18n::language_options(locale)));
    ui.set_alert_condition_options(string_model(i18n::alert_condition_options(locale)));
    ui.set_plugin_market_items(plugin_market_items_model(plugin_catalog, &store, locale));
    ui.set_widget_list_options(owned_string_model(widget_instance_options(
        &store,
        locale,
        plugin_catalog,
    )));
    ui.set_widget_list_detail_options(owned_string_model(widget_instance_detail_options(
        &store,
        locale,
        plugin_catalog,
    )));
    ui.set_widget_visible_options(bool_model(widget_visibility_options(&store)));
    ui.set_widget_preview_kinds(int_model(widget_preview_kind_options(&store)));
    ui.set_widget_scale_options(int_model(widget_scale_options(&store, plugin_catalog)));
    ui.set_widget_scale_min_options(int_model(widget_scale_min_options(&store, plugin_catalog)));
    ui.set_widget_scale_max_options(int_model(widget_scale_max_options(&store, plugin_catalog)));
    ui.set_quote_board_used_text(
        widget_type_usage_text(&store, WidgetType::QuoteBoard, locale).into(),
    );
    ui.set_mini_ticker_used_text(
        widget_type_usage_text(&store, WidgetType::MiniTicker, locale).into(),
    );
    ui.set_selected_widget_index(selected_widget.map(|_| selected_index as i32).unwrap_or(-1));
    ui.set_widget_name_input_text(
        selected_widget
            .map(|widget| widget_display_name(widget, selected_index, locale))
            .unwrap_or_default()
            .into(),
    );
    ui.set_widgets_always_on_top(
        selected_widget
            .map(|widget| widget.layout.always_on_top)
            .unwrap_or(settings.widgets_always_on_top),
    );
    ui.set_widget_layout_locked(
        selected_widget
            .map(|widget| widget.layout.locked)
            .unwrap_or(false),
    );
    ui.set_opacity_percent(
        selected_widget
            .map(|widget| widget.layout.opacity_percent)
            .unwrap_or(settings.opacity_percent),
    );
    let (scale_min, scale_max) = selected_widget
        .map(|widget| widget_scale_percent_bounds(widget, plugin_catalog))
        .unwrap_or((
            settings::MIN_WIDGET_SCALE_PERCENT,
            settings::MAX_WIDGET_SCALE_PERCENT,
        ));
    ui.set_widget_scale_min_percent(scale_min);
    ui.set_widget_scale_max_percent(scale_max);
    ui.set_widget_scale_percent(
        selected_widget
            .map(|widget| widget_scale_percent(widget, plugin_catalog))
            .unwrap_or(settings.widget_scale_percent),
    );
    ui.set_widget_show_coin_logos(
        selected_widget
            .map(settings::widget_show_coin_logos)
            .unwrap_or(true),
    );
    ui.set_widget_hide_quote_asset(
        selected_widget
            .map(settings::widget_hide_quote_asset)
            .unwrap_or(false),
    );
    ui.set_default_widgets_always_on_top(settings.widgets_always_on_top);
    ui.set_default_opacity_percent(settings.opacity_percent);
    ui.set_default_widget_scale_percent(settings.widget_scale_percent);
    ui.set_red_up_enabled(settings.red_up_enabled);
    ui.set_provider_index(settings.market_provider.index());
    ui.set_market_binance_enabled(settings.market_binance_enabled);
    ui.set_market_okx_enabled(settings.market_okx_enabled);
    ui.set_market_hyperliquid_enabled(settings.market_hyperliquid_enabled);
    ui.set_refresh_interval_seconds(settings.refresh_interval_seconds);
    ui.set_market_fallback_enabled(settings.market_fallback_enabled);
    ui.set_alert_enabled(primary_alert.map(|rule| rule.enabled).unwrap_or(false));
    let default_symbols = settings.market_default_symbols.clone();
    let widget_symbols = selected_widget
        .map(|widget| widget.symbols.clone())
        .unwrap_or_default();
    let alert_symbol = primary_alert
        .map(|rule| rule.symbol.clone())
        .or_else(|| settings.market_default_symbols.first().cloned())
        .unwrap_or_else(|| "BTC".to_string());
    ui.set_default_symbol_values(owned_string_model(
        default_symbols
            .iter()
            .map(|symbol| format_symbol_option(symbol))
            .collect(),
    ));
    let widget_symbol_values = widget_symbols
        .iter()
        .map(|symbol| format_symbol_option(symbol))
        .collect::<Vec<_>>();
    let widget_symbol_chip_values = widget_symbols
        .iter()
        .map(|symbol| format_symbol_chip_label(symbol))
        .collect::<Vec<_>>();
    let (chip_x, chip_y, chip_width, add_x, add_y, status_y) =
        widget_symbol_chip_layout(&widget_symbol_chip_values);
    ui.set_widget_symbol_values(owned_string_model(widget_symbol_values));
    ui.set_widget_symbol_chip_values(owned_string_model(widget_symbol_chip_values));
    ui.set_widget_symbol_chip_x(int_model(chip_x));
    ui.set_widget_symbol_chip_y(int_model(chip_y));
    ui.set_widget_symbol_chip_width(int_model(chip_width));
    ui.set_widget_symbol_add_x(add_x);
    ui.set_widget_symbol_add_y(add_y);
    ui.set_widget_symbol_status_y(status_y);
    ui.set_symbol_picker_open(false);
    ui.set_symbol_picker_mode(SYMBOL_PICKER_MODE_WIDGET);
    ui.set_symbol_picker_search_text("".into());
    ui.set_symbol_picker_candidate_options(owned_string_model(Vec::new()));
    ui.set_symbol_picker_candidate_values(owned_string_model(Vec::new()));
    ui.set_symbol_picker_candidate_index(0);
    ui.set_symbol_picker_title_text(
        symbol_picker_title_text(SYMBOL_PICKER_MODE_WIDGET, locale).into(),
    );
    ui.set_symbol_picker_status_text("".into());
    ui.set_symbol_picker_confirm_text(symbol_picker_confirm_text(locale).into());
    ui.set_symbol_picker_cancel_text(symbol_picker_cancel_text(locale).into());
    ui.set_default_symbol_status_text(
        default_symbol_status_text(
            default_symbols.len(),
            WidgetType::QuoteBoard.symbol_limit(),
            0,
            "",
            locale,
        )
        .into(),
    );
    ui.set_widget_symbol_status_text(
        widget_symbol_status_text(
            widget_symbols.len(),
            ui.get_widget_symbol_min().max(0) as usize,
            ui.get_widget_symbol_max().max(0) as usize,
            0,
            "",
            locale,
        )
        .into(),
    );
    refresh_alert_symbol_options(
        ui,
        &symbol_catalog_state,
        &settings::enabled_market_sources(&settings),
        Some(alert_symbol.as_str()),
    );
    ui.set_alert_condition_index(
        primary_alert
            .map(|rule| alert_condition_index(rule.condition))
            .unwrap_or(0),
    );
    ui.set_alert_threshold_input_text(
        primary_alert
            .map(|rule| alert_threshold_text(rule.threshold))
            .unwrap_or_default()
            .into(),
    );
    ui.set_auto_start_enabled(settings.auto_start_enabled);
    ui.set_show_main_window_on_startup(settings.show_main_window_on_startup);
    ui.set_shortcut_index(settings.shortcut.index());
    ui.set_theme_index(settings.theme.index());
    ui.set_language_index(settings.language.index());
    ui.set_tray_icon_enabled(settings.tray_icon_enabled);
    ui.set_tray_hover_display_enabled(settings.tray_hover_display_enabled);
    ui.set_network_proxy_enabled(settings.network_proxy_enabled);
    ui.set_network_proxy_url_input_text(settings.network_proxy_url.clone().into());
    ui.set_settings_path_text(state_path.display().to_string().into());
    let status = status_override
        .filter(|status| !status.trim().is_empty())
        .unwrap_or(if symbol_catalog_state.fallback_only {
            text.status_symbol_catalog_fallback
        } else {
            ""
        });
    ui.set_status_text(status.into());
    apply_theme_to_settings_window(ui, settings);
}

pub(crate) fn widget_type_title(widget_type: WidgetType, locale: i18n::Locale) -> &'static str {
    i18n::widget_title(locale, widget_text(widget_type))
}

fn widget_type_description(widget_type: WidgetType, locale: i18n::Locale) -> &'static str {
    i18n::widget_description(locale, widget_text(widget_type))
}

fn widget_text(widget_type: WidgetType) -> i18n::WidgetText {
    match widget_type {
        WidgetType::QuoteBoard => i18n::WidgetText::QuoteBoard,
        WidgetType::MiniTicker => i18n::WidgetText::MiniTicker,
    }
}

pub(crate) fn default_widget_name(widget_type: WidgetType, number: u64) -> String {
    i18n::default_widget_name(i18n::Locale::En, widget_text(widget_type), number)
}

pub(crate) fn widget_display_name(
    widget: &WidgetInstance,
    fallback_index: usize,
    locale: i18n::Locale,
) -> String {
    let name = widget.name.trim();
    let fallback_number = widget_default_number(widget, fallback_index);
    if name.is_empty() || parse_default_widget_name(name, widget.widget_type()).is_some() {
        localized_default_widget_name(widget.widget_type(), fallback_number, locale)
    } else {
        widget.name.clone()
    }
}

pub(crate) fn widget_default_number(widget: &WidgetInstance, fallback_index: usize) -> u64 {
    parse_default_widget_name(&widget.name, widget.widget_type())
        .or_else(|| widget_id_suffix(&widget.id))
        .unwrap_or(fallback_index as u64 + 1)
}

fn widget_id_suffix(id: &str) -> Option<u64> {
    settings::widget_id_suffix(id)
}

fn localized_default_widget_name(
    widget_type: WidgetType,
    number: u64,
    locale: i18n::Locale,
) -> String {
    i18n::default_widget_name(locale, widget_text(widget_type), number)
}

fn parse_default_widget_name(raw: &str, widget_type: WidgetType) -> Option<u64> {
    let prefix = i18n::widget_title(i18n::Locale::En, widget_text(widget_type));
    raw.trim().strip_prefix(&format!("{prefix} "))?.parse().ok()
}

pub(crate) fn normalize_widget_name(
    raw: &str,
    widget_type: WidgetType,
    fallback_number: u64,
    locale: i18n::Locale,
) -> String {
    let name = raw.trim();
    if name.is_empty() {
        default_widget_name(widget_type, fallback_number)
    } else if let Some(number) = parse_default_widget_name(name, widget_type) {
        default_widget_name(widget_type, number)
    } else if name == localized_default_widget_name(widget_type, fallback_number, locale) {
        default_widget_name(widget_type, fallback_number)
    } else {
        name.chars().take(48).collect()
    }
}

fn selected_widget_index(store: &LayoutStore) -> usize {
    store
        .selected_widget_id
        .as_deref()
        .and_then(|selected_id| {
            store
                .widgets
                .iter()
                .position(|widget| widget.id == selected_id)
        })
        .unwrap_or(0)
}

fn widget_scale_percent(instance: &WidgetInstance, plugin_catalog: &plugin::PluginCatalog) -> i32 {
    let definitions = widget_definitions_from_catalog(plugin_catalog);
    widget_scale_percent_for_definitions(instance, &definitions)
}

fn widget_scale_percent_for_definitions(
    instance: &WidgetInstance,
    definitions: &[WidgetDefinition],
) -> i32 {
    settings::widget_scale_percent_for_instance(
        instance,
        definitions,
        settings::DEFAULT_WIDGET_SCALE_PERCENT,
    )
}

fn widget_scale_percent_bounds(
    instance: &WidgetInstance,
    plugin_catalog: &plugin::PluginCatalog,
) -> (i32, i32) {
    let definitions = widget_definitions_from_catalog(plugin_catalog);
    let default_size = settings::default_widget_size_for_instance(instance, &definitions);
    settings::widget_scale_percent_bounds(default_size)
}

fn apply_widget_scale_to_instance(
    instance: &mut WidgetInstance,
    definitions: &[WidgetDefinition],
    scale_percent: i32,
) {
    settings::resize_widget_to_content(instance, definitions, scale_percent);
}

pub(crate) fn apply_dynamic_widget_auto_sizes_to_store(
    store: &mut LayoutStore,
    plugin_catalog: &plugin::PluginCatalog,
) -> bool {
    let definitions = widget_definitions_from_catalog(plugin_catalog);
    apply_dynamic_widget_auto_sizes_for_definitions(store, &definitions)
}

fn apply_dynamic_widget_auto_sizes_for_definitions(
    store: &mut LayoutStore,
    definitions: &[WidgetDefinition],
) -> bool {
    let mut changed = false;
    for instance in &mut store.widgets {
        let Some(scale_percent) = dynamic_widget_existing_scale_percent(instance, definitions)
        else {
            continue;
        };
        let previous_layout = (
            instance.layout.width,
            instance.layout.height,
            instance.layout.scale_percent,
        );
        apply_dynamic_widget_auto_size(instance, definitions, scale_percent);
        changed |= previous_layout
            != (
                instance.layout.width,
                instance.layout.height,
                instance.layout.scale_percent,
            );
    }
    changed
}

fn dynamic_widget_base_size(
    instance: &WidgetInstance,
    definitions: &[WidgetDefinition],
) -> Option<WidgetSize> {
    let definition = definitions
        .iter()
        .find(|definition| definition.id == instance.plugin_id)?;
    if definition.size_policy == settings::WidgetSizePolicy::Fixed {
        return None;
    }
    Some(settings::default_widget_size_for_instance(
        instance,
        definitions,
    ))
}

fn status_strip_legacy_base_size(instance: &WidgetInstance) -> Option<WidgetSize> {
    let mut base_size = status_strip_legacy_block_base_size(instance)?;
    base_size.width += STATUS_STRIP_LEGACY_SIDE_MARGIN * 2;
    base_size.height = STATUS_STRIP_LEGACY_HEIGHT;
    Some(base_size)
}

fn status_strip_previous_base_size(instance: &WidgetInstance) -> Option<WidgetSize> {
    if instance.plugin_id != STATUS_STRIP_PLUGIN_ID {
        return None;
    }
    let block_count = instance.symbols.len().clamp(1, 5) as i32;
    Some(WidgetSize {
        width: STATUS_STRIP_PREVIOUS_BLOCK_WIDTH * block_count,
        height: STATUS_STRIP_VISIBLE_HEIGHT,
    })
}

fn status_strip_previous_tight_base_size(instance: &WidgetInstance) -> Option<WidgetSize> {
    if instance.plugin_id != STATUS_STRIP_PLUGIN_ID {
        return None;
    }
    let block_count = instance.symbols.len().clamp(1, 5) as i32;
    Some(WidgetSize {
        width: STATUS_STRIP_PREVIOUS_TIGHT_BLOCK_WIDTH * block_count,
        height: STATUS_STRIP_PREVIOUS_TIGHT_HEIGHT,
    })
}

fn status_strip_legacy_block_base_size(instance: &WidgetInstance) -> Option<WidgetSize> {
    if instance.plugin_id != STATUS_STRIP_PLUGIN_ID {
        return None;
    }
    let block_count = instance.symbols.len().clamp(1, 5) as i32;
    Some(WidgetSize {
        width: STATUS_STRIP_LEGACY_BLOCK_WIDTH * block_count,
        height: STATUS_STRIP_VISIBLE_HEIGHT,
    })
}

fn dynamic_widget_existing_scale_percent(
    instance: &WidgetInstance,
    definitions: &[WidgetDefinition],
) -> Option<i32> {
    let current_base = dynamic_widget_base_size(instance, definitions)?;
    if instance.layout.scale_percent > 0 {
        return Some(settings::widget_scale_percent_for_instance(
            instance,
            definitions,
            settings::DEFAULT_WIDGET_SCALE_PERCENT,
        ));
    }
    if instance.plugin_id != STATUS_STRIP_PLUGIN_ID {
        let current_size = settings::clamp_widget_size(WidgetSize {
            width: instance.layout.width,
            height: instance.layout.height,
        });
        return Some(settings::widget_content_scale_percent_for_size(
            current_size,
            current_base,
        ));
    }
    status_strip_existing_scale_percent(instance, current_base)
}

fn status_strip_existing_scale_percent(
    instance: &WidgetInstance,
    current_base: WidgetSize,
) -> Option<i32> {
    let previous_base = status_strip_previous_base_size(instance)?;
    let previous_tight_base = status_strip_previous_tight_base_size(instance)?;
    let legacy_block_base = status_strip_legacy_block_base_size(instance)?;
    let legacy_base = status_strip_legacy_base_size(instance)?;
    let current_size = settings::clamp_widget_size(WidgetSize {
        width: instance.layout.width,
        height: instance.layout.height,
    });
    let current_scale = settings::widget_scale_percent_for_size(current_size, current_base);
    let previous_scale = settings::widget_scale_percent_for_size(current_size, previous_base);
    let previous_tight_scale =
        settings::widget_scale_percent_for_size(current_size, previous_tight_base);
    let legacy_block_scale =
        settings::widget_scale_percent_for_size(current_size, legacy_block_base);
    let legacy_scale = settings::widget_scale_percent_for_size(current_size, legacy_base);
    let candidates = [
        (
            current_scale,
            scale_reconstruction_error(current_size, current_base, current_scale),
        ),
        (
            previous_scale,
            scale_reconstruction_error(current_size, previous_base, previous_scale),
        ),
        (
            previous_tight_scale,
            scale_reconstruction_error(current_size, previous_tight_base, previous_tight_scale),
        ),
        (
            legacy_block_scale,
            scale_reconstruction_error(current_size, legacy_block_base, legacy_block_scale),
        ),
        (
            legacy_scale,
            scale_reconstruction_error(current_size, legacy_base, legacy_scale),
        ),
    ];

    candidates
        .into_iter()
        .min_by_key(|(_, error)| *error)
        .map(|(scale, _)| scale)
}

fn scale_reconstruction_error(
    target_size: WidgetSize,
    base_size: WidgetSize,
    scale_percent: i32,
) -> i32 {
    let reconstructed = settings::widget_size_from_scale_percent(base_size, scale_percent);
    (target_size.width - reconstructed.width).abs()
        + (target_size.height - reconstructed.height).abs()
}

fn apply_dynamic_widget_auto_size(
    instance: &mut WidgetInstance,
    definitions: &[WidgetDefinition],
    scale_percent: i32,
) {
    if dynamic_widget_base_size(instance, definitions).is_none() {
        return;
    }
    settings::resize_widget_to_content(instance, definitions, scale_percent);
}

pub(crate) fn symbol_limit_for_instance(
    instance: &WidgetInstance,
    plugin_catalog: Option<&plugin::PluginCatalog>,
) -> usize {
    let definitions = widget_definitions_from_optional_catalog(plugin_catalog);
    settings::symbol_limit_for_instance(instance, &definitions)
}

pub(crate) fn symbol_min_for_instance(
    instance: &WidgetInstance,
    plugin_catalog: Option<&plugin::PluginCatalog>,
) -> usize {
    let definitions = widget_definitions_from_optional_catalog(plugin_catalog);
    settings::symbol_min_for_instance(instance, &definitions)
}

fn widget_definitions_from_optional_catalog(
    catalog: Option<&plugin::PluginCatalog>,
) -> Vec<WidgetDefinition> {
    catalog
        .map(widget_definitions_from_catalog)
        .unwrap_or_default()
}

pub(crate) fn symbols_help_text(min: usize, max: usize, locale: i18n::Locale) -> String {
    match locale {
        i18n::Locale::ZhHans if min == max => {
            format!("必须选择 {max} 个交易对")
        }
        i18n::Locale::ZhHans => format!("最多选择 {max} 个交易对"),
        i18n::Locale::En if min == max => {
            format!("Exactly {max} pairs")
        }
        i18n::Locale::En => format!("Up to {max} pairs"),
    }
}

fn symbol_search_placeholder(locale: i18n::Locale) -> &'static str {
    match locale {
        i18n::Locale::ZhHans => "搜索代码、名称或 BTCUSDT",
        i18n::Locale::En => "Search pair, name, or BTCUSDT",
    }
}

fn symbol_picker_title_text(mode: i32, locale: i18n::Locale) -> &'static str {
    match (mode, locale) {
        (SYMBOL_PICKER_MODE_DEFAULT, i18n::Locale::ZhHans) => "添加新建默认交易对",
        (SYMBOL_PICKER_MODE_DEFAULT, i18n::Locale::En) => "Add new-widget default pair",
        (_, i18n::Locale::ZhHans) => "添加当前组件交易对",
        (_, i18n::Locale::En) => "Add current widget pair",
    }
}

fn symbol_picker_confirm_text(locale: i18n::Locale) -> &'static str {
    match locale {
        i18n::Locale::ZhHans => "确定",
        i18n::Locale::En => "Confirm",
    }
}

fn symbol_picker_cancel_text(locale: i18n::Locale) -> &'static str {
    match locale {
        i18n::Locale::ZhHans => "取消",
        i18n::Locale::En => "Cancel",
    }
}

fn symbol_picker_empty_status_text(mode: i32, locale: i18n::Locale) -> &'static str {
    match (mode, locale) {
        (SYMBOL_PICKER_MODE_DEFAULT, i18n::Locale::ZhHans) => "没有可添加的新建默认交易对",
        (SYMBOL_PICKER_MODE_DEFAULT, i18n::Locale::En) => {
            "No new-widget default pairs are available"
        }
        (_, i18n::Locale::ZhHans) => "没有可添加的当前组件交易对",
        (_, i18n::Locale::En) => "No current widget pairs are available",
    }
}

fn symbol_picker_status_text(
    mode: i32,
    selected_count: usize,
    max: usize,
    candidate_count: usize,
    query: &str,
    fallback_only: bool,
    locale: i18n::Locale,
) -> String {
    match (mode, locale) {
        (_, i18n::Locale::ZhHans) if selected_count >= max => {
            format!("已选 {selected_count}/{max}，已达上限，先移除一个交易对")
        }
        (_, i18n::Locale::ZhHans) if !query.trim().is_empty() && candidate_count == 0 => {
            format!("已选 {selected_count}/{max}，没有匹配交易对")
        }
        (_, i18n::Locale::ZhHans) if candidate_count == 0 => {
            format!("已选 {selected_count}/{max}，没有可添加的交易对")
        }
        (_, i18n::Locale::ZhHans) if fallback_only => {
            format!("候选目录暂不可用，已使用本地候选，找到 {candidate_count} 个")
        }
        (SYMBOL_PICKER_MODE_DEFAULT, i18n::Locale::ZhHans) => {
            format!("找到 {candidate_count} 个可添加交易对，只影响以后新建的小组件")
        }
        (_, i18n::Locale::ZhHans) => {
            format!("找到 {candidate_count} 个可添加交易对，会立即影响当前小组件")
        }
        (_, i18n::Locale::En) if selected_count >= max => {
            format!("Selected {selected_count}/{max}. Limit reached; remove one pair first")
        }
        (_, i18n::Locale::En) if !query.trim().is_empty() && candidate_count == 0 => {
            format!("Selected {selected_count}/{max}. No matching pairs")
        }
        (_, i18n::Locale::En) if candidate_count == 0 => {
            format!("Selected {selected_count}/{max}. No pairs available to add")
        }
        (_, i18n::Locale::En) if fallback_only => {
            format!("Catalog is unavailable; using local candidates. Found {candidate_count}")
        }
        (SYMBOL_PICKER_MODE_DEFAULT, i18n::Locale::En) => {
            format!("Found {candidate_count} pairs. Only affects newly created widgets")
        }
        (_, i18n::Locale::En) => {
            format!("Found {candidate_count} pairs. Applies to this widget immediately")
        }
    }
}

fn widget_symbol_status_text(
    selected_count: usize,
    min: usize,
    max: usize,
    candidate_count: usize,
    query: &str,
    locale: i18n::Locale,
) -> String {
    match locale {
        i18n::Locale::ZhHans if selected_count >= max => {
            format!("已选择 {selected_count}/{max}")
        }
        i18n::Locale::ZhHans if !query.trim().is_empty() && candidate_count == 0 => {
            format!("已选择 {selected_count}/{max}")
        }
        i18n::Locale::ZhHans if selected_count <= min => {
            format!("已选择 {selected_count}/{max}")
        }
        i18n::Locale::ZhHans => {
            format!("已选择 {selected_count}/{max}")
        }
        i18n::Locale::En if selected_count >= max => {
            format!("Selected {selected_count}/{max}")
        }
        i18n::Locale::En if !query.trim().is_empty() && candidate_count == 0 => {
            format!("Selected {selected_count}/{max}")
        }
        i18n::Locale::En if selected_count <= min => {
            format!("Selected {selected_count}/{max}")
        }
        i18n::Locale::En => {
            format!("Selected {selected_count}/{max}")
        }
    }
}

fn default_symbol_status_text(
    selected_count: usize,
    max: usize,
    candidate_count: usize,
    query: &str,
    locale: i18n::Locale,
) -> String {
    match locale {
        i18n::Locale::ZhHans if selected_count >= max => {
            format!("已选 {selected_count}/{max}，新建小组件默认交易对已满")
        }
        i18n::Locale::ZhHans if !query.trim().is_empty() && candidate_count == 0 => {
            format!("已选 {selected_count}/{max}，没有匹配交易对")
        }
        i18n::Locale::ZhHans => {
            format!("已选 {selected_count}/{max}，只影响以后新建的小组件")
        }
        i18n::Locale::En if selected_count >= max => {
            format!("Selected {selected_count}/{max}. New-widget defaults are full")
        }
        i18n::Locale::En if !query.trim().is_empty() && candidate_count == 0 => {
            format!("Selected {selected_count}/{max}. No matching pairs")
        }
        i18n::Locale::En => {
            format!("Selected {selected_count}/{max}. Only affects newly created widgets")
        }
    }
}

fn format_symbols_input(symbols: &[String]) -> String {
    symbols
        .iter()
        .map(|symbol| settings::format_market_pair_symbol(symbol))
        .collect::<Vec<_>>()
        .join(" · ")
}

#[cfg(test)]
pub(crate) fn plugin_market_item(
    definition: &plugin::PluginDefinition,
    store: &LayoutStore,
    locale: i18n::Locale,
) -> crate::PluginMarketItem {
    settings_models::plugin_market_item(definition, store, locale)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_widget(id: &str, plugin_id: &str, symbols: Vec<&str>) -> WidgetInstance {
        WidgetInstance {
            id: id.to_string(),
            plugin_id: plugin_id.to_string(),
            legacy_widget_type: None,
            name: id.to_string(),
            visible: true,
            layout: settings::WidgetLayout::default(),
            symbols: symbols.into_iter().map(str::to_string).collect(),
            config: settings::default_widget_config(),
        }
    }

    fn temp_state_path(label: &str) -> std::path::PathBuf {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "crypto-hud-settings-window-{label}-{}-{suffix}.json",
            std::process::id()
        ))
    }

    fn catalog_entry(
        source: settings::MarketDataSource,
        market_type: settings::MarketType,
        base: &str,
        quote: &str,
    ) -> market::SymbolCatalogEntry {
        let pair = settings::MarketPair::new(source, market_type, base, quote).unwrap();
        market::SymbolCatalogEntry {
            key: pair.key(),
            base: pair.base,
            quote: pair.quote,
            source: pair.source,
            market_type: pair.market_type,
        }
    }

    fn enabled_sources(sources: &[settings::MarketDataSource]) -> Vec<settings::MarketDataSource> {
        sources.to_vec()
    }

    fn settings_window_ui_source() -> String {
        std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("ui")
                .join("settings-window.slint"),
        )
        .unwrap()
    }

    fn block_before_anchor<'a>(source: &'a str, block_start: &str, anchor: &str) -> &'a str {
        let anchor_index = source.find(anchor).unwrap();
        let block_index = source[..anchor_index].rfind(block_start).unwrap();
        &source[block_index..]
    }

    fn block_after_anchor<'a>(source: &'a str, anchor: &str, block_start: &str) -> &'a str {
        let anchor_index = source.find(anchor).unwrap();
        let rest = &source[anchor_index..];
        let block_index = rest.find(block_start).unwrap();
        &rest[block_index..]
    }

    fn slint_px_assignment(block: &str, property: &str) -> i32 {
        let assignment = format!("{property}:");
        let start = block.find(&assignment).unwrap() + assignment.len();
        let value = block[start..].trim_start();
        let end = value.find("px").unwrap();
        value[..end].trim().parse().unwrap()
    }

    fn slint_x_assignment(block: &str, parent_width: i32) -> i32 {
        let start = block.find("x:").unwrap() + "x:".len();
        let value = block[start..].trim_start();
        if let Some(rest) = value.strip_prefix("parent.width -") {
            let end = rest.find("px").unwrap();
            parent_width - rest[..end].trim().parse::<i32>().unwrap()
        } else {
            let end = value.find("px").unwrap();
            value[..end].trim().parse().unwrap()
        }
    }

    #[test]
    fn settings_widget_scale_controls_have_clear_hit_areas() {
        let source = settings_window_ui_source();
        let selected_panel =
            block_after_anchor(&source, "selected_widget_panel := Rectangle {", "");
        let scrollbar_gutter = slint_px_assignment(
            &source,
            "property <length> widget-settings-scrollbar-gutter",
        );
        let panel_width = slint_px_assignment(selected_panel, "width") - scrollbar_gutter;

        let show_logos_touch = block_before_anchor(
            &source,
            "TouchArea {",
            "root.widget-show-coin-logos = !root.widget-show-coin-logos;",
        );
        let hide_quote_touch = block_before_anchor(
            &source,
            "TouchArea {",
            "root.widget-hide-quote-asset = !root.widget-hide-quote-asset;",
        );
        let scale_stepper = block_after_anchor(
            &source,
            "label: root.widget-scale-text;",
            "PercentStepper {",
        );
        let percent_label = block_after_anchor(scale_stepper, "edited(value) =>", "Text {");
        let reset_button = block_after_anchor(
            scale_stepper,
            "SettingsSmallButton {",
            "SettingsSmallButton {",
        );

        let show_logos_bottom = slint_px_assignment(show_logos_touch, "y")
            + slint_px_assignment(show_logos_touch, "height");
        let hide_quote_bottom = slint_px_assignment(hide_quote_touch, "y")
            + slint_px_assignment(hide_quote_touch, "height");
        let scale_top = slint_px_assignment(scale_stepper, "y");
        assert!(
            show_logos_bottom <= scale_top,
            "show logos touch area overlaps widget scale stepper"
        );
        assert!(
            hide_quote_bottom <= scale_top,
            "hide quote touch area overlaps widget scale stepper"
        );

        let stepper_left = slint_x_assignment(scale_stepper, panel_width);
        let stepper_right = stepper_left + slint_px_assignment(scale_stepper, "width");
        let percent_left = slint_x_assignment(percent_label, panel_width);
        let percent_right = percent_left + slint_px_assignment(percent_label, "width");
        let reset_left = slint_x_assignment(reset_button, panel_width);
        let reset_right = reset_left + slint_px_assignment(reset_button, "width");
        let switch_right = panel_width - 17;

        assert!(stepper_right <= percent_left);
        assert!(
            percent_left - stepper_right <= 12,
            "scale stepper and percent label should stay visually grouped"
        );
        assert!(percent_right <= reset_left);
        assert!(
            reset_left - percent_right <= 20,
            "scale percent label and reset button should stay visually grouped"
        );
        assert_eq!(
            reset_right, switch_right,
            "scale reset button should right-align with setting switches"
        );
    }

    #[test]
    fn selected_widget_settings_scroll_view_reserves_scrollbar_gutter() {
        let source = settings_window_ui_source();
        let selected_panel =
            block_after_anchor(&source, "selected_widget_panel := Rectangle {", "");

        assert!(
            selected_panel.contains(
                "viewport-width: selected_widget_panel.width - root.widget-settings-scrollbar-gutter;"
            ),
            "selected widget settings scroll view should not let the scrollbar overlay content"
        );
        assert!(
            selected_panel.contains(
                "width: selected_widget_panel.width - root.widget-settings-scrollbar-gutter;"
            ),
            "selected widget settings content should reserve the same scrollbar gutter"
        );
    }

    #[test]
    fn widget_symbol_chip_layout_keeps_max_symbols_in_one_row() {
        let labels = vec![
            "BTC/USDT · Binance · Bitcoin".to_string(),
            "ETH/USDT · Binance · Ethereum".to_string(),
            "SOL/USDT · Binance · Solana".to_string(),
            "XRP/USDT · Binance · Ripple".to_string(),
            "BNB/USDT · Binance · BNB".to_string(),
        ];

        let (x_values, y_values, widths, add_x, add_y, status_y) =
            widget_symbol_chip_layout(&labels);

        assert_eq!(x_values[0], WIDGET_SYMBOL_CHIP_START_X);
        assert_eq!(y_values[0], WIDGET_SYMBOL_CHIP_START_Y);
        assert!(widths.iter().all(|width| *width >= 76 && *width <= 84));
        assert!(y_values.iter().all(|y| *y == WIDGET_SYMBOL_CHIP_START_Y));
        assert!(add_x >= WIDGET_SYMBOL_CHIP_START_X);
        assert_eq!(add_y, WIDGET_SYMBOL_CHIP_START_Y);
        assert!(status_y > add_y);
    }

    #[test]
    fn widget_symbol_chip_layout_does_not_reserve_add_row_when_full() {
        let labels = vec![
            "BTC/USDT".to_string(),
            "ETH/USDT".to_string(),
            "SOL/USDT".to_string(),
            "BNB/USDT".to_string(),
            "XRP/USDT".to_string(),
        ];

        let (_x_values, y_values, _widths, _add_x, add_y, status_y) =
            widget_symbol_chip_layout(&labels);

        let last_chip_y = *y_values.last().unwrap();
        assert_eq!(add_y, last_chip_y);
        assert_eq!(status_y, last_chip_y + WIDGET_SYMBOL_CHIP_ROW_HEIGHT + 8);
    }

    #[test]
    fn widget_symbol_chip_label_uses_pair_display() {
        assert_eq!(format_symbol_chip_label("BTC"), "BTC/USDT");
        assert_eq!(format_symbol_chip_label("eth/usdt"), "ETH/USDT");
    }

    #[test]
    fn primary_alert_input_builds_normalized_rule() {
        let rule = primary_alert_rule_from_input(true, "eth/usdt", 2, "3.5")
            .unwrap()
            .unwrap();

        assert_eq!(rule.id, PRIMARY_ALERT_ID);
        assert_eq!(rule.symbol, "binance:spot:ETH/USDT");
        assert_eq!(rule.condition, AlertCondition::ChangePercentAbove);
        assert_eq!(rule.threshold, 3.5);
        assert!(rule.enabled);
    }

    #[test]
    fn disabled_blank_alert_input_clears_rule() {
        assert!(primary_alert_rule_from_input(false, "BTC", 0, "")
            .unwrap()
            .is_none());
    }

    #[test]
    fn primary_alert_input_rejects_invalid_threshold() {
        assert!(primary_alert_rule_from_input(true, "BTC", 0, "not-a-number").is_err());
    }

    #[test]
    fn network_proxy_input_trims_and_validates_when_enabled() {
        assert_eq!(
            normalized_network_proxy_input(false, "  ftp://127.0.0.1:21  ").unwrap(),
            "ftp://127.0.0.1:21"
        );
        assert_eq!(
            normalized_network_proxy_input(true, "  http://127.0.0.1:7890  ").unwrap(),
            "http://127.0.0.1:7890"
        );
        assert_eq!(
            normalized_network_proxy_input(true, "socks5://127.0.0.1:1080").unwrap(),
            "socks5://127.0.0.1:1080"
        );
        assert!(normalized_network_proxy_input(true, "  ").is_err());
        assert!(normalized_network_proxy_input(true, "ftp://127.0.0.1:21").is_err());
    }

    #[test]
    fn add_plugin_widget_action_uses_plugin_defaults_and_selects_new_widget() {
        let catalog = plugin::PluginCatalog::builtins();
        let plugin = catalog.find(WidgetType::QuoteBoard.plugin_id()).unwrap();
        let settings = AppSettings {
            market_default_symbols: vec!["ETH".to_string(), "SOL".to_string()],
            widget_scale_percent: 125,
            ..AppSettings::default()
        }
        .normalized();
        let mut store = LayoutStore {
            settings: settings.clone(),
            ..LayoutStore::default()
        };

        let id = add_plugin_widget_to_store(&mut store, plugin, &settings).unwrap();
        let widget = store.widgets.first().unwrap();
        let definitions = widget_definitions_from_catalog(&catalog);
        let expected_size = settings::widget_size_from_scale_percent(
            settings::default_widget_size_for_instance(widget, &definitions),
            125,
        );

        assert_eq!(store.selected_widget_id.as_deref(), Some(id.as_str()));
        assert_eq!(widget.plugin_id, WidgetType::QuoteBoard.plugin_id());
        assert_eq!(
            widget.symbols,
            vec!["binance:spot:ETH/USDT", "binance:spot:SOL/USDT"]
        );
        assert_eq!(widget.layout.width, expected_size.width);
        assert_eq!(widget.layout.height, expected_size.height);
    }

    #[test]
    fn apply_widget_settings_action_normalizes_name_opacity_and_size() {
        let catalog = plugin::PluginCatalog::builtins();
        let mut store = LayoutStore {
            selected_widget_id: Some("quote-board-1".to_string()),
            widgets: vec![test_widget(
                "quote-board-1",
                WidgetType::QuoteBoard.plugin_id(),
                vec!["BTC"],
            )],
            ..LayoutStore::default()
        };
        let definitions = widget_definitions_from_catalog(&catalog);
        settings::resize_widget_to_content(&mut store.widgets[0], &definitions, 150);

        assert!(apply_widget_settings_to_store(
            &mut store,
            WidgetSettingsUpdate {
                selected_index: 0,
                widget_name: "  Main Board  ",
                always_on_top: true,
                layout_locked: true,
                opacity_percent: 42,
                widget_scale_percent: 150,
                show_coin_logos: false,
                hide_quote_asset: true,
                locale: i18n::Locale::En,
                plugin_catalog: &catalog,
            },
        ));

        let widget = &store.widgets[0];
        let expected_size = settings::widget_size_from_scale_percent(
            settings::default_widget_size_for_instance(widget, &definitions),
            150,
        );
        assert_eq!(widget.name, "Main Board");
        assert!(widget.layout.always_on_top);
        assert!(widget.layout.locked);
        assert_eq!(widget.layout.opacity_percent, 42);
        assert_eq!(widget.layout.width, expected_size.width);
        assert_eq!(widget.layout.height, expected_size.height);
        assert!(!settings::widget_show_coin_logos(widget));
        assert!(settings::widget_hide_quote_asset(widget));
    }

    #[test]
    fn quote_board_scale_uses_effective_content_scale_for_extra_height() {
        let catalog = plugin::PluginCatalog::builtins();
        let definitions = widget_definitions_from_catalog(&catalog);
        let mut widget = test_widget(
            "quote-board-1",
            WidgetType::QuoteBoard.plugin_id(),
            vec!["BTC"],
        );
        widget.layout.width = settings::QUOTE_BOARD_WIDTH;
        widget.layout.height = settings::QUOTE_BOARD_HEIGHT;

        assert_eq!(
            widget_scale_percent_for_definitions(&widget, &definitions),
            100
        );
    }

    #[test]
    fn apply_widget_settings_preserves_content_scale_when_toggling_coin_logos() {
        let catalog = plugin::PluginCatalog::builtins();
        let mut store = LayoutStore {
            selected_widget_id: Some("quote-board-1".to_string()),
            widgets: vec![test_widget(
                "quote-board-1",
                WidgetType::QuoteBoard.plugin_id(),
                vec!["BTC"],
            )],
            ..LayoutStore::default()
        };
        store.widgets[0].layout.width = settings::QUOTE_BOARD_WIDTH;
        store.widgets[0].layout.height = settings::QUOTE_BOARD_HEIGHT;

        assert!(apply_widget_settings_to_store(
            &mut store,
            WidgetSettingsUpdate {
                selected_index: 0,
                widget_name: "Quote Board 1",
                always_on_top: false,
                layout_locked: false,
                opacity_percent: 96,
                widget_scale_percent: 172,
                show_coin_logos: false,
                hide_quote_asset: false,
                locale: i18n::Locale::En,
                plugin_catalog: &catalog,
            },
        ));

        let widget = &store.widgets[0];
        assert_eq!(widget.layout.width, 274);
        assert_eq!(widget.layout.height, 80);
        assert!(!settings::widget_show_coin_logos(widget));
    }

    #[test]
    fn add_widget_symbol_preserves_quote_board_scale_while_resizing_to_content() {
        let catalog = plugin::PluginCatalog::builtins();
        let definitions = widget_definitions_from_catalog(&catalog);
        let mut widget = test_widget(
            "quote-board-1",
            WidgetType::QuoteBoard.plugin_id(),
            vec!["BTC"],
        );
        settings::resize_widget_to_content(&mut widget, &definitions, 150);
        let state_path = temp_state_path("quote-board-symbol-add");
        let layouts = Rc::new(RefCell::new(LayoutStore {
            selected_widget_id: Some("quote-board-1".to_string()),
            widgets: vec![widget],
            ..LayoutStore::default()
        }));

        add_widget_symbol_to_store(&layouts, &state_path, 0, "ETH", &catalog).unwrap();

        let store = layouts.borrow();
        let widget = &store.widgets[0];
        let expected_size = settings::widget_size_from_scale_percent(
            settings::default_widget_size_for_instance(widget, &definitions),
            150,
        );
        assert_eq!(
            widget.symbols,
            vec!["binance:spot:BTC/USDT", "binance:spot:ETH/USDT"]
        );
        assert_eq!(widget.layout.width, expected_size.width);
        assert_eq!(widget.layout.height, expected_size.height);
        assert_eq!(
            widget_scale_percent_for_definitions(widget, &definitions),
            150
        );
        let _ = std::fs::remove_file(state_path);
    }

    #[test]
    fn apply_widget_scale_action_resizes_selected_widget() {
        let catalog = plugin::PluginCatalog::builtins();
        let plugin = catalog.find(WidgetType::MiniTicker.plugin_id()).unwrap();
        let mut store = LayoutStore {
            selected_widget_id: Some("mini-ticker-1".to_string()),
            widgets: vec![test_widget(
                "mini-ticker-1",
                WidgetType::MiniTicker.plugin_id(),
                vec!["BTC"],
            )],
            ..LayoutStore::default()
        };

        assert!(apply_widget_scale_to_store(&mut store, 0, 175, &catalog));

        let widget = &store.widgets[0];
        let expected_size = settings::widget_size_from_scale_percent(
            WidgetSize {
                width: plugin.default_size.width,
                height: plugin.default_size.height,
            },
            175,
        );
        assert_eq!(widget.layout.width, expected_size.width);
        assert_eq!(widget.layout.height, expected_size.height);
    }

    #[test]
    fn delete_widget_action_removes_selected_index_and_keeps_selection_valid() {
        let mut store = LayoutStore {
            selected_widget_id: Some("quote-board-1".to_string()),
            widgets: vec![
                test_widget(
                    "quote-board-1",
                    WidgetType::QuoteBoard.plugin_id(),
                    vec!["BTC"],
                ),
                test_widget(
                    "mini-ticker-2",
                    WidgetType::MiniTicker.plugin_id(),
                    vec!["ETH"],
                ),
            ],
            ..LayoutStore::default()
        };

        assert!(delete_widget_from_store_at_index(&mut store, 0));

        assert_eq!(store.widgets.len(), 1);
        assert_eq!(store.widgets[0].id, "mini-ticker-2");
        assert_eq!(store.selected_widget_id.as_deref(), Some("mini-ticker-2"));
    }

    #[test]
    fn symbol_options_follow_enabled_sources() {
        let state = SymbolCatalogState {
            catalog: market::SymbolCatalog {
                entries: vec![
                    catalog_entry(
                        settings::MarketDataSource::Binance,
                        settings::MarketType::Spot,
                        "BTC",
                        "USDT",
                    ),
                    catalog_entry(
                        settings::MarketDataSource::Okx,
                        settings::MarketType::Spot,
                        "BTC",
                        "USDT",
                    ),
                    catalog_entry(
                        settings::MarketDataSource::Hyperliquid,
                        settings::MarketType::Perp,
                        "BTC",
                        "USDC",
                    ),
                    catalog_entry(
                        settings::MarketDataSource::Binance,
                        settings::MarketType::Spot,
                        "BNB",
                        "USDT",
                    ),
                    catalog_entry(
                        settings::MarketDataSource::Okx,
                        settings::MarketType::Spot,
                        "OKB",
                        "USDT",
                    ),
                ],
            },
            ..SymbolCatalogState::default()
        };

        assert_eq!(
            symbol_pick_options(
                &state,
                &enabled_sources(&[
                    settings::MarketDataSource::Binance,
                    settings::MarketDataSource::Okx,
                    settings::MarketDataSource::Hyperliquid,
                ]),
                Vec::new()
            ),
            vec![
                "binance:spot:BTC/USDT",
                "okx:spot:BTC/USDT",
                "hyperliquid:perp:BTC/USDC",
                "binance:spot:BNB/USDT",
                "okx:spot:OKB/USDT"
            ]
        );
        assert_eq!(
            symbol_pick_options(
                &state,
                &enabled_sources(&[settings::MarketDataSource::Binance]),
                Vec::new()
            ),
            vec!["binance:spot:BTC/USDT", "binance:spot:BNB/USDT"]
        );
        assert_eq!(
            symbol_pick_options(
                &state,
                &enabled_sources(&[settings::MarketDataSource::Okx]),
                Vec::new()
            ),
            vec!["okx:spot:BTC/USDT", "okx:spot:OKB/USDT"]
        );
    }

    #[test]
    fn symbol_add_options_exclude_selected_and_keep_saved_fallbacks() {
        let state = SymbolCatalogState {
            catalog: market::SymbolCatalog {
                entries: vec![catalog_entry(
                    settings::MarketDataSource::Binance,
                    settings::MarketType::Spot,
                    "BTC",
                    "USDT",
                )],
            },
            fallback_symbols: vec!["binance:spot:DELISTED/USDT".to_string()],
            ..SymbolCatalogState::default()
        };

        assert_eq!(
            symbol_add_options_with_query(
                &state,
                &enabled_sources(&[settings::MarketDataSource::Binance]),
                &["binance:spot:BTC/USDT".to_string()],
                "",
            ),
            vec!["binance:spot:DELISTED/USDT"]
        );
    }

    #[test]
    fn symbol_options_prioritize_popular_markets_over_character_order() {
        let state = SymbolCatalogState {
            catalog: market::SymbolCatalog {
                entries: vec![
                    catalog_entry(
                        settings::MarketDataSource::Binance,
                        settings::MarketType::Spot,
                        "1INCH",
                        "USDT",
                    ),
                    catalog_entry(
                        settings::MarketDataSource::Binance,
                        settings::MarketType::Spot,
                        "AAVE",
                        "USDT",
                    ),
                    catalog_entry(
                        settings::MarketDataSource::Binance,
                        settings::MarketType::Spot,
                        "DOGE",
                        "USDT",
                    ),
                    catalog_entry(
                        settings::MarketDataSource::Binance,
                        settings::MarketType::Spot,
                        "ADA",
                        "USDT",
                    ),
                ],
            },
            ..SymbolCatalogState::default()
        };

        assert_eq!(
            symbol_pick_options(
                &state,
                &enabled_sources(&[settings::MarketDataSource::Binance]),
                Vec::new()
            ),
            vec![
                "binance:spot:DOGE/USDT",
                "binance:spot:ADA/USDT",
                "binance:spot:AAVE/USDT",
                "binance:spot:1INCH/USDT"
            ]
        );
    }

    #[test]
    fn symbol_options_filter_by_pair_name_and_alias() {
        let state = SymbolCatalogState {
            catalog: market::SymbolCatalog {
                entries: vec![
                    catalog_entry(
                        settings::MarketDataSource::Binance,
                        settings::MarketType::Spot,
                        "BTC",
                        "USDT",
                    ),
                    catalog_entry(
                        settings::MarketDataSource::Binance,
                        settings::MarketType::Spot,
                        "BCH",
                        "USDT",
                    ),
                    catalog_entry(
                        settings::MarketDataSource::Binance,
                        settings::MarketType::Spot,
                        "DOGE",
                        "USDT",
                    ),
                    catalog_entry(
                        settings::MarketDataSource::Binance,
                        settings::MarketType::Spot,
                        "ETH",
                        "USDT",
                    ),
                ],
            },
            ..SymbolCatalogState::default()
        };
        let sources = enabled_sources(&[settings::MarketDataSource::Binance]);

        assert_eq!(
            symbol_pick_options_with_query(&state, &sources, Vec::new(), "btcusdt"),
            vec!["binance:spot:BTC/USDT"]
        );
        assert_eq!(
            symbol_pick_options_with_query(&state, &sources, Vec::new(), "bitcoin"),
            vec!["binance:spot:BTC/USDT", "binance:spot:BCH/USDT"]
        );
        assert_eq!(
            symbol_pick_options_with_query(&state, &sources, Vec::new(), "doge"),
            vec!["binance:spot:DOGE/USDT"]
        );
        assert_eq!(
            symbol_pick_options_with_query(&state, &sources, Vec::new(), "Dogecoin"),
            vec!["binance:spot:DOGE/USDT"]
        );
        assert_eq!(
            symbol_pick_options_with_query(&state, &sources, Vec::new(), "狗"),
            vec!["binance:spot:DOGE/USDT"]
        );
    }

    #[test]
    fn symbol_picker_status_reports_limits_and_fallback_catalog() {
        assert_eq!(
            symbol_picker_status_text(
                SYMBOL_PICKER_MODE_WIDGET,
                5,
                5,
                0,
                "",
                false,
                i18n::Locale::ZhHans,
            ),
            "已选 5/5，已达上限，先移除一个交易对"
        );
        assert_eq!(
            symbol_picker_status_text(
                SYMBOL_PICKER_MODE_DEFAULT,
                2,
                5,
                3,
                "",
                true,
                i18n::Locale::ZhHans,
            ),
            "候选目录暂不可用，已使用本地候选，找到 3 个"
        );
        assert_eq!(
            symbol_picker_status_text(
                SYMBOL_PICKER_MODE_WIDGET,
                2,
                5,
                0,
                "not-a-pair",
                false,
                i18n::Locale::En,
            ),
            "Selected 2/5. No matching pairs"
        );
    }

    #[test]
    fn empty_widget_list_models_are_empty() {
        let store = LayoutStore::default();
        let catalog = plugin::PluginCatalog::builtins();

        assert!(widget_instance_options(&store, i18n::Locale::En, &catalog).is_empty());
        assert!(widget_visibility_options(&store).is_empty());
        assert!(widget_preview_kind_options(&store).is_empty());
        assert!(widget_scale_options(&store, &catalog).is_empty());
    }

    #[test]
    fn widget_scale_options_follow_each_instance_size() {
        let catalog = plugin::PluginCatalog::builtins();
        let definitions = widget_definitions_from_catalog(&catalog);
        let quote_base = settings::default_widget_size_for_instance(
            &test_widget(
                "quote-board-base",
                WidgetType::QuoteBoard.plugin_id(),
                vec!["BTC"],
            ),
            &definitions,
        );
        let quote_scale_125 = settings::widget_size_from_scale_percent(quote_base, 125);
        let mini_scale_150 =
            settings::widget_size_from_scale_percent(WidgetType::MiniTicker.default_size(), 150);
        let store = LayoutStore {
            widgets: vec![
                WidgetInstance {
                    id: "quote-board-1".to_string(),
                    plugin_id: WidgetType::QuoteBoard.plugin_id().to_string(),
                    legacy_widget_type: None,
                    name: "Quote Board 1".to_string(),
                    visible: true,
                    layout: settings::WidgetLayout {
                        width: quote_base.width,
                        height: quote_base.height,
                        ..settings::WidgetLayout::default()
                    },
                    symbols: vec!["BTC".to_string()],
                    config: settings::default_widget_config(),
                },
                WidgetInstance {
                    id: "quote-board-2".to_string(),
                    plugin_id: WidgetType::QuoteBoard.plugin_id().to_string(),
                    legacy_widget_type: None,
                    name: "Quote Board 2".to_string(),
                    visible: true,
                    layout: settings::WidgetLayout {
                        width: quote_scale_125.width,
                        height: quote_scale_125.height,
                        scale_percent: 125,
                        ..settings::WidgetLayout::default()
                    },
                    symbols: vec!["ETH".to_string()],
                    config: settings::default_widget_config(),
                },
                WidgetInstance {
                    id: "mini-ticker-3".to_string(),
                    plugin_id: WidgetType::MiniTicker.plugin_id().to_string(),
                    legacy_widget_type: None,
                    name: "Mini Ticker 3".to_string(),
                    visible: true,
                    layout: settings::WidgetLayout {
                        width: mini_scale_150.width,
                        height: mini_scale_150.height,
                        scale_percent: 150,
                        ..settings::WidgetLayout::default()
                    },
                    symbols: vec!["SOL".to_string()],
                    config: settings::default_widget_config(),
                },
            ],
            ..LayoutStore::default()
        };

        assert_eq!(widget_scale_options(&store, &catalog), vec![100, 125, 150]);
        assert_eq!(
            widget_scale_min_options(&store, &catalog),
            vec![
                settings::MIN_WIDGET_SCALE_PERCENT,
                settings::MIN_WIDGET_SCALE_PERCENT,
                settings::MIN_WIDGET_SCALE_PERCENT,
            ]
        );
    }

    #[test]
    fn default_widget_scale_change_updates_existing_instances() {
        let catalog = plugin::PluginCatalog::builtins();
        let definitions = widget_definitions_from_catalog(&catalog);
        let quote_scale_base = settings::default_widget_size_for_instance(
            &test_widget(
                "quote-board-1",
                WidgetType::QuoteBoard.plugin_id(),
                vec!["BTC"],
            ),
            &definitions,
        );
        let quote_scale_150 = settings::widget_size_from_scale_percent(quote_scale_base, 150);
        let mini_scale_200 =
            settings::widget_size_from_scale_percent(WidgetType::MiniTicker.default_size(), 200);
        let mini_scale_150 =
            settings::widget_size_from_scale_percent(WidgetType::MiniTicker.default_size(), 150);
        let layouts = Rc::new(RefCell::new(LayoutStore {
            widgets: vec![
                WidgetInstance {
                    id: "quote-board-1".to_string(),
                    plugin_id: WidgetType::QuoteBoard.plugin_id().to_string(),
                    legacy_widget_type: None,
                    name: "Quote Board 1".to_string(),
                    visible: true,
                    layout: settings::WidgetLayout::default(),
                    symbols: vec!["BTC".to_string()],
                    config: settings::default_widget_config(),
                },
                WidgetInstance {
                    id: "mini-ticker-2".to_string(),
                    plugin_id: WidgetType::MiniTicker.plugin_id().to_string(),
                    legacy_widget_type: None,
                    name: "Mini Ticker 2".to_string(),
                    visible: true,
                    layout: settings::WidgetLayout {
                        width: mini_scale_200.width,
                        height: mini_scale_200.height,
                        ..settings::WidgetLayout::default()
                    },
                    symbols: vec!["ETH".to_string()],
                    config: settings::default_widget_config(),
                },
            ],
            ..LayoutStore::default()
        }));

        apply_default_widget_scale_to_instances(&layouts, &catalog, 150);
        let store = layouts.borrow();

        assert_eq!(store.widgets[0].layout.width, quote_scale_150.width);
        assert_eq!(store.widgets[0].layout.height, quote_scale_150.height);
        assert_eq!(store.widgets[1].layout.width, mini_scale_150.width);
        assert_eq!(store.widgets[1].layout.height, mini_scale_150.height);
    }

    #[test]
    fn widget_preview_kinds_follow_instance_plugins() {
        let store = LayoutStore {
            widgets: vec![
                WidgetInstance {
                    id: "focus-ticker-1".to_string(),
                    plugin_id: "com.cryptohud.focus-ticker".to_string(),
                    legacy_widget_type: None,
                    name: "Focus Ticker 1".to_string(),
                    visible: true,
                    layout: settings::WidgetLayout::default(),
                    symbols: vec!["BTC".to_string()],
                    config: settings::default_widget_config(),
                },
                WidgetInstance {
                    id: "market-board-1".to_string(),
                    plugin_id: "com.cryptohud.market-board".to_string(),
                    legacy_widget_type: None,
                    name: "Market Board 1".to_string(),
                    visible: true,
                    layout: settings::WidgetLayout::default(),
                    symbols: vec!["BTC".to_string(), "ETH".to_string()],
                    config: settings::default_widget_config(),
                },
                WidgetInstance {
                    id: "trust-card-1".to_string(),
                    plugin_id: "com.cryptohud.trust-card".to_string(),
                    legacy_widget_type: None,
                    name: "Trust Card 1".to_string(),
                    visible: true,
                    layout: settings::WidgetLayout::default(),
                    symbols: vec!["BTC".to_string()],
                    config: settings::default_widget_config(),
                },
                WidgetInstance {
                    id: "stage3-1".to_string(),
                    plugin_id: "com.example.stage3-price-card".to_string(),
                    legacy_widget_type: None,
                    name: "Stage 3 Price Card 1".to_string(),
                    visible: true,
                    layout: settings::WidgetLayout::default(),
                    symbols: vec!["BTC".to_string()],
                    config: settings::default_widget_config(),
                },
                WidgetInstance {
                    id: "plugin-pulse-1".to_string(),
                    plugin_id: "com.cryptohud.orbit-pulse".to_string(),
                    legacy_widget_type: None,
                    name: "Orbit Pulse 1".to_string(),
                    visible: true,
                    layout: settings::WidgetLayout::default(),
                    symbols: vec!["BTC".to_string()],
                    config: settings::default_widget_config(),
                },
                WidgetInstance {
                    id: "plugin-compass-1".to_string(),
                    plugin_id: "com.cryptohud.market-compass".to_string(),
                    legacy_widget_type: None,
                    name: "Market Compass 1".to_string(),
                    visible: true,
                    layout: settings::WidgetLayout::default(),
                    symbols: vec![
                        "BTC".to_string(),
                        "ETH".to_string(),
                        "SOL".to_string(),
                        "BNB".to_string(),
                    ],
                    config: settings::default_widget_config(),
                },
                WidgetInstance {
                    id: "plugin-strip-1".to_string(),
                    plugin_id: STATUS_STRIP_PLUGIN_ID.to_string(),
                    legacy_widget_type: None,
                    name: "Status Strip 1".to_string(),
                    visible: true,
                    layout: settings::WidgetLayout::default(),
                    symbols: vec!["BTC".to_string(), "ETH".to_string(), "SOL".to_string()],
                    config: settings::default_widget_config(),
                },
            ],
            ..LayoutStore::default()
        };

        assert_eq!(
            widget_preview_kind_options(&store),
            vec![1, 2, 3, 0, 4, 5, 6]
        );
    }

    fn status_strip_definitions() -> Vec<WidgetDefinition> {
        vec![WidgetDefinition {
            id: STATUS_STRIP_PLUGIN_ID.to_string(),
            name: "Status Strip".to_string(),
            default_size: settings::WidgetSize {
                width: 688,
                height: 92,
            },
            size_policy: settings::WidgetSizePolicy::SymbolGrid {
                cell_width: 136,
                cell_height: 84,
                content_padding_width: 8,
                content_padding_height: 8,
                columns: Some(5),
                rows: None,
            },
            min_symbol_limit: 1,
            symbol_limit: 5,
        }]
    }

    fn status_strip_catalog() -> plugin::PluginCatalog {
        plugin::PluginCatalog::from_plugins_for_tests(vec![plugin::PluginDefinition {
            id: STATUS_STRIP_PLUGIN_ID.to_string(),
            name: "Status Strip".to_string(),
            version: semver::Version::new(0, 1, 0),
            source: plugin::PluginSource::LocalUnsigned,
            renderer: plugin::PluginRendererDefinition::Builtin(
                plugin::BuiltinRenderer::QuoteBoard,
            ),
            default_size: plugin::PluginSize {
                width: 688,
                height: 92,
            },
            size_policy: plugin::PluginSizePolicy::SymbolGrid {
                cell_size: plugin::PluginSize {
                    width: 136,
                    height: 84,
                },
                content_padding: plugin::PluginSize {
                    width: 8,
                    height: 8,
                },
                columns: Some(5),
                rows: None,
            },
            min_symbol_limit: 1,
            symbol_limit: 5,
            data_requirements: Vec::new(),
            status: plugin::PluginStatus::Available,
        }])
    }

    #[test]
    fn remove_widget_symbol_preserves_symbol_block_scale_while_resizing_to_content() {
        let catalog = status_strip_catalog();
        let definitions = widget_definitions_from_catalog(&catalog);
        let mut widget = test_widget(
            "plugin-strip-1",
            STATUS_STRIP_PLUGIN_ID,
            vec!["BTC", "ETH", "SOL"],
        );
        settings::resize_widget_to_content(&mut widget, &definitions, 200);
        let state_path = temp_state_path("status-strip-symbol-remove");
        let layouts = Rc::new(RefCell::new(LayoutStore {
            selected_widget_id: Some("plugin-strip-1".to_string()),
            widgets: vec![widget],
            ..LayoutStore::default()
        }));

        remove_widget_symbol_from_store(&layouts, &state_path, 0, 2, &catalog).unwrap();

        let store = layouts.borrow();
        let widget = &store.widgets[0];
        let expected_size = settings::widget_size_from_scale_percent(
            settings::default_widget_size_for_instance(widget, &definitions),
            200,
        );
        assert_eq!(
            widget.symbols,
            vec!["binance:spot:BTC/USDT", "binance:spot:ETH/USDT"]
        );
        assert_eq!(widget.layout.width, expected_size.width);
        assert_eq!(widget.layout.height, expected_size.height);
        assert_eq!(
            widget_scale_percent_for_definitions(widget, &definitions),
            200
        );
        let _ = std::fs::remove_file(state_path);
    }

    #[test]
    fn status_strip_auto_size_tracks_symbol_blocks() {
        let mut instance = WidgetInstance {
            id: "plugin-strip-1".to_string(),
            plugin_id: STATUS_STRIP_PLUGIN_ID.to_string(),
            legacy_widget_type: None,
            name: "Status Strip 1".to_string(),
            visible: true,
            layout: settings::WidgetLayout::default(),
            symbols: vec!["BTC".to_string()],
            config: settings::default_widget_config(),
        };
        let definitions = status_strip_definitions();

        apply_dynamic_widget_auto_size(&mut instance, &definitions, 100);

        assert_eq!(instance.layout.width, 144);
        assert_eq!(instance.layout.height, 92);

        instance.symbols = vec!["BTC".to_string(), "ETH".to_string(), "SOL".to_string()];
        apply_dynamic_widget_auto_size(&mut instance, &definitions, 100);

        assert_eq!(instance.layout.width, 416);
        assert_eq!(instance.layout.height, 92);
    }

    #[test]
    fn status_strip_auto_size_migrates_legacy_canvas_size() {
        let mut store = LayoutStore {
            widgets: vec![WidgetInstance {
                id: "plugin-strip-1".to_string(),
                plugin_id: STATUS_STRIP_PLUGIN_ID.to_string(),
                legacy_widget_type: None,
                name: "Status Strip 1".to_string(),
                visible: true,
                layout: settings::WidgetLayout {
                    width: 824,
                    height: 260,
                    scale_percent: 0,
                    ..settings::WidgetLayout::default()
                },
                symbols: vec!["BTC".to_string(), "ETH".to_string(), "SOL".to_string()],
                config: settings::default_widget_config(),
            }],
            ..LayoutStore::default()
        };

        assert!(apply_dynamic_widget_auto_sizes_for_definitions(
            &mut store,
            &status_strip_definitions()
        ));
        assert_eq!(store.widgets[0].layout.width, 416);
        assert_eq!(store.widgets[0].layout.height, 92);
    }

    #[test]
    fn status_strip_auto_size_migrates_legacy_block_scaled_size() {
        let mut store = LayoutStore {
            widgets: vec![WidgetInstance {
                id: "plugin-strip-1".to_string(),
                plugin_id: STATUS_STRIP_PLUGIN_ID.to_string(),
                legacy_widget_type: None,
                name: "Status Strip 1".to_string(),
                visible: true,
                layout: settings::WidgetLayout {
                    width: 1008,
                    height: 168,
                    scale_percent: 0,
                    ..settings::WidgetLayout::default()
                },
                symbols: vec!["BTC".to_string(), "ETH".to_string(), "SOL".to_string()],
                config: settings::default_widget_config(),
            }],
            ..LayoutStore::default()
        };

        assert!(apply_dynamic_widget_auto_sizes_for_definitions(
            &mut store,
            &status_strip_definitions()
        ));
        assert_eq!(store.widgets[0].layout.width, 832);
        assert_eq!(store.widgets[0].layout.height, 184);
    }

    #[test]
    fn status_strip_auto_size_migrates_previous_default_size() {
        let mut store = LayoutStore {
            widgets: vec![WidgetInstance {
                id: "plugin-strip-1".to_string(),
                plugin_id: STATUS_STRIP_PLUGIN_ID.to_string(),
                legacy_widget_type: None,
                name: "Status Strip 1".to_string(),
                visible: true,
                layout: settings::WidgetLayout {
                    width: 450,
                    height: 84,
                    scale_percent: 0,
                    ..settings::WidgetLayout::default()
                },
                symbols: vec!["BTC".to_string(), "ETH".to_string(), "SOL".to_string()],
                config: settings::default_widget_config(),
            }],
            ..LayoutStore::default()
        };

        assert!(apply_dynamic_widget_auto_sizes_for_definitions(
            &mut store,
            &status_strip_definitions()
        ));
        assert_eq!(store.widgets[0].layout.width, 416);
        assert_eq!(store.widgets[0].layout.height, 92);
    }

    #[test]
    fn status_strip_auto_size_migrates_previous_tight_size() {
        let mut store = LayoutStore {
            widgets: vec![WidgetInstance {
                id: "plugin-strip-1".to_string(),
                plugin_id: STATUS_STRIP_PLUGIN_ID.to_string(),
                legacy_widget_type: None,
                name: "Status Strip 1".to_string(),
                visible: true,
                layout: settings::WidgetLayout {
                    width: 408,
                    height: 84,
                    scale_percent: 0,
                    ..settings::WidgetLayout::default()
                },
                symbols: vec!["BTC".to_string(), "ETH".to_string(), "SOL".to_string()],
                config: settings::default_widget_config(),
            }],
            ..LayoutStore::default()
        };

        assert!(apply_dynamic_widget_auto_sizes_for_definitions(
            &mut store,
            &status_strip_definitions()
        ));
        assert_eq!(store.widgets[0].layout.width, 416);
        assert_eq!(store.widgets[0].layout.height, 92);
    }

    #[test]
    fn status_strip_auto_size_preserves_current_scaled_size() {
        let mut store = LayoutStore {
            widgets: vec![WidgetInstance {
                id: "plugin-strip-1".to_string(),
                plugin_id: STATUS_STRIP_PLUGIN_ID.to_string(),
                legacy_widget_type: None,
                name: "Status Strip 1".to_string(),
                visible: true,
                layout: settings::WidgetLayout {
                    width: 832,
                    height: 184,
                    scale_percent: 200,
                    ..settings::WidgetLayout::default()
                },
                symbols: vec!["BTC".to_string(), "ETH".to_string(), "SOL".to_string()],
                config: settings::default_widget_config(),
            }],
            ..LayoutStore::default()
        };

        assert!(!apply_dynamic_widget_auto_sizes_for_definitions(
            &mut store,
            &status_strip_definitions()
        ));
        assert_eq!(store.widgets[0].layout.width, 832);
        assert_eq!(store.widgets[0].layout.height, 184);
    }
}
