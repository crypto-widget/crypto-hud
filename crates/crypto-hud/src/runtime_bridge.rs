use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{mpsc::Receiver, Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use crypto_hud_market as market;
use crypto_hud_runtime as widget_runtime;
use crypto_hud_runtime::{ProviderLabels, QuoteCache, QuoteState, RuntimeTextLabels};
use crypto_hud_shell_state as settings;
use settings::{
    save_layout_store, AppSettings, LayoutStore, WidgetInstance, WidgetKind as WidgetType,
};
use slint::{ComponentHandle, LogicalSize, PhysicalPosition, Timer, TimerMode, WindowPosition};
use slint_interpreter::{ComponentInstance, Value};

use crate::{
    coin_icons::CoinIconRegistry,
    feature_flags, i18n, notifications, plugin,
    settings_window::widget_type_title,
    state_bridge::{layout_for_instance, normalized_symbols_for_instance, symbols_from_store},
    theme, updater,
    widget_host::{apply_runtime_view_to_widget, WidgetRuntime, WidgetUi},
    window_manager::schedule_widget_shell_window_configuration,
};

const MARKET_ERROR_NOTIFICATION_COOLDOWN: Duration = Duration::from_secs(300);
const DRAG_LAYOUT_SAVE_DEBOUNCE: Duration = Duration::from_millis(300);

struct WidgetMoveRequest<'a> {
    window: &'a slint::Window,
    dx: f32,
    dy: f32,
    id: &'a str,
    always_on_top: bool,
    opacity_percent: i32,
}

pub(crate) struct RuntimeEventTimerDeps {
    pub(crate) widgets: Rc<RefCell<Vec<WidgetRuntime>>>,
    pub(crate) layouts: Rc<RefCell<LayoutStore>>,
    pub(crate) quote_cache: Rc<RefCell<QuoteCache>>,
    pub(crate) coin_icons: Rc<CoinIconRegistry>,
    pub(crate) plugin_catalog: Rc<plugin::PluginCatalog>,
    pub(crate) market_updates: Receiver<market::MarketEvent>,
    pub(crate) update_events: Option<Receiver<updater::UpdateEvent>>,
}

pub(crate) fn install_runtime_event_timer(deps: RuntimeEventTimerDeps) -> Timer {
    let RuntimeEventTimerDeps {
        widgets,
        layouts,
        quote_cache,
        coin_icons,
        plugin_catalog,
        market_updates,
        update_events,
    } = deps;
    let notification_throttle = Rc::new(RefCell::new(notifications::NotificationThrottle::new(
        MARKET_ERROR_NOTIFICATION_COOLDOWN,
    )));
    let timer = Timer::default();
    {
        let widgets = widgets.clone();
        let layouts = layouts.clone();
        let quote_cache = quote_cache.clone();
        let coin_icons = coin_icons.clone();
        let notification_throttle = notification_throttle.clone();
        let market_error_active = Rc::new(RefCell::new(false));
        timer.start(TimerMode::Repeated, Duration::from_secs(1), move || {
            refresh_runtime_symbols_from_store(
                &widgets,
                &layouts,
                &quote_cache.borrow(),
                &coin_icons,
                &plugin_catalog,
            );

            while let Ok(event) = market_updates.try_recv() {
                match event {
                    market::MarketEvent::Snapshot(snapshot) => {
                        *market_error_active.borrow_mut() = false;
                        let settings = layouts.borrow().settings.clone().normalized();
                        let updated_at = Instant::now();
                        {
                            let mut cache = quote_cache.borrow_mut();
                            cache.insert(
                                snapshot.symbol.clone(),
                                QuoteState::new(
                                    snapshot.price,
                                    snapshot.change_percent_24h,
                                    snapshot.chart_closes_24h.clone(),
                                    snapshot.source,
                                    updated_at,
                                ),
                            );
                        }

                        let cache = quote_cache.borrow();
                        if feature_flags::ALERT_RULES_ENABLED {
                            notify_alerts(&notification_throttle, &settings.alert_rules, &cache);
                        }
                    }
                    market::MarketEvent::Error(error) => {
                        *market_error_active.borrow_mut() = true;
                        eprintln!("market data update failed: {error}");
                        let settings = layouts.borrow().settings.clone().normalized();
                        notify_market_error(
                            &notification_throttle,
                            i18n::resolve_locale(settings.language),
                            &error,
                        );
                    }
                }
            }

            if let Some(update_events) = &update_events {
                while let Ok(event) = update_events.try_recv() {
                    match event {
                        updater::UpdateEvent::Available(update) => {
                            let settings = layouts.borrow().settings.clone().normalized();
                            notify_update_available(
                                &notification_throttle,
                                i18n::resolve_locale(settings.language),
                                &update,
                            );
                        }
                        updater::UpdateEvent::UpToDate => {}
                        updater::UpdateEvent::Error(error) => {
                            eprintln!("update check failed: {error}");
                        }
                    }
                }
            }

            let now = Instant::now();
            let cache = quote_cache.borrow();
            let settings = layouts.borrow().settings.clone().normalized();
            let proxy_url = settings::effective_network_proxy_url(&settings);
            let has_market_error = *market_error_active.borrow();
            for widget in widgets.borrow().iter() {
                apply_runtime_view_with_icons(
                    &widget.ui,
                    &widget_runtime_view(
                        &widget.id,
                        &widget.symbols,
                        &cache,
                        &settings,
                        now,
                        has_market_error,
                    ),
                    &widget.symbols,
                    &coin_icons,
                    proxy_url.as_deref(),
                );
            }
        });
    }
    timer
}

pub(crate) fn sync_widget_runtimes(
    widgets: &Rc<RefCell<Vec<WidgetRuntime>>>,
    layouts: &Rc<RefCell<LayoutStore>>,
    state_path: &Path,
    quote_cache: Rc<RefCell<QuoteCache>>,
    coin_icons: Rc<CoinIconRegistry>,
    set_positions: bool,
    plugin_catalog: Rc<plugin::PluginCatalog>,
) -> Result<()> {
    let store = layouts.borrow();
    let settings = store.settings.clone().normalized();
    let instances = store.widgets.clone();
    drop(store);
    let quote_cache_ref = quote_cache.borrow();

    let desired_plugins = instances
        .iter()
        .map(|instance| (instance.id.clone(), instance.plugin_id.clone()))
        .collect::<HashMap<_, _>>();
    let mut runtimes = widgets.borrow_mut();
    runtimes.retain(|runtime| {
        let keep = desired_plugins
            .get(&runtime.id)
            .map(|plugin_id| plugin_id == &runtime.plugin_id)
            .unwrap_or(false);
        if !keep {
            if let Err(error) = runtime.ui.hide() {
                eprintln!("failed to hide removed widget {}: {error:#}", runtime.id);
            }
        }
        keep
    });

    for (index, instance) in instances.iter().enumerate() {
        if let Some(runtime) = runtimes
            .iter_mut()
            .find(|runtime| runtime.id == instance.id)
        {
            apply_instance_to_widget(
                &runtime.ui,
                instance,
                index,
                settings.clone(),
                &quote_cache_ref,
                &coin_icons,
                set_positions,
                &plugin_catalog,
            );
            apply_widget_visibility(&runtime.ui, instance.visible)?;
            runtime.symbols = normalized_symbols_for_instance(instance, Some(&plugin_catalog));
            continue;
        }

        let Some(plugin) = plugin_catalog.find(&instance.plugin_id) else {
            eprintln!(
                "widget {} references unknown plugin {}",
                instance.id, instance.plugin_id
            );
            continue;
        };
        if let plugin::PluginStatus::Unavailable(reason) = &plugin.status {
            eprintln!(
                "widget {} plugin {} is unavailable: {reason}",
                instance.id, instance.plugin_id
            );
            continue;
        }

        let ui = WidgetUi::from_plugin(plugin)
            .with_context(|| format!("failed to create widget plugin {}", instance.plugin_id))?;
        apply_instance_to_widget(
            &ui,
            instance,
            index,
            settings.clone(),
            &quote_cache_ref,
            &coin_icons,
            true,
            &plugin_catalog,
        );
        install_drag_handler(
            &ui,
            layouts.clone(),
            state_path.to_path_buf(),
            instance.id.clone(),
        );
        install_layout_lock_handler(
            &ui,
            layouts.clone(),
            state_path.to_path_buf(),
            instance.id.clone(),
        );
        apply_widget_visibility(&ui, instance.visible)?;
        let symbols = normalized_symbols_for_instance(instance, Some(&plugin_catalog));
        runtimes.push(WidgetRuntime {
            id: instance.id.clone(),
            plugin_id: instance.plugin_id.clone(),
            ui,
            symbols,
        });

        if index == instances.len().saturating_sub(1) {
            schedule_widget_shell_window_configuration();
        }
    }

    let order = instances
        .iter()
        .enumerate()
        .map(|(index, instance)| (instance.id.clone(), index))
        .collect::<HashMap<_, _>>();
    runtimes.sort_by_key(|runtime| order.get(&runtime.id).copied().unwrap_or(usize::MAX));
    Ok(())
}

fn install_drag_handler(
    ui: &WidgetUi,
    layouts: Rc<RefCell<LayoutStore>>,
    state_path: PathBuf,
    id: String,
) {
    match ui {
        WidgetUi::BuiltinPriceCard(ui) => {
            let weak = ui.as_weak();
            let save_timer = Timer::default();
            ui.on_drag_move(move |dx, dy| {
                let Some(ui) = weak.upgrade() else {
                    return;
                };

                move_widget_window_and_schedule_save(
                    WidgetMoveRequest {
                        window: ui.window(),
                        dx,
                        dy,
                        id: &id,
                        always_on_top: ui.get_pin_to_top(),
                        opacity_percent: ui.get_content_opacity(),
                    },
                    &layouts,
                    &state_path,
                    &save_timer,
                );
            });
        }
        WidgetUi::DynamicSlint(ui) => {
            let weak = ui.instance.as_weak();
            let save_timer = Timer::default();
            ui.instance
                .set_callback("drag-move", move |args| {
                    let Some(ui) = weak.upgrade() else {
                        return Value::Void;
                    };
                    let dx = callback_number_arg(args, 0).unwrap_or(0.0) as f32;
                    let dy = callback_number_arg(args, 1).unwrap_or(0.0) as f32;
                    let pin_to_top = dynamic_bool_property(&ui, "pin-to-top").unwrap_or(false);
                    let opacity_percent =
                        dynamic_number_property(&ui, "content-opacity").unwrap_or(100);

                    move_widget_window_and_schedule_save(
                        WidgetMoveRequest {
                            window: ui.window(),
                            dx,
                            dy,
                            id: &id,
                            always_on_top: pin_to_top,
                            opacity_percent,
                        },
                        &layouts,
                        &state_path,
                        &save_timer,
                    );
                    Value::Void
                })
                .unwrap_or_else(|error| {
                    eprintln!("failed to install dynamic drag callback: {error:?}");
                });
        }
    }
}

fn install_layout_lock_handler(
    ui: &WidgetUi,
    layouts: Rc<RefCell<LayoutStore>>,
    state_path: PathBuf,
    id: String,
) {
    match ui {
        WidgetUi::BuiltinPriceCard(ui) => {
            let weak = ui.as_weak();
            ui.on_toggle_layout_lock(move || {
                let Some(ui) = weak.upgrade() else {
                    return;
                };
                if let Some(locked) = toggle_widget_layout_lock_and_save(&layouts, &state_path, &id)
                {
                    ui.set_layout_locked(locked);
                }
            });
        }
        WidgetUi::DynamicSlint(ui) => {
            let weak = ui.instance.as_weak();
            ui.instance
                .set_callback("toggle-layout-lock", move |_| {
                    let Some(ui) = weak.upgrade() else {
                        return Value::Void;
                    };
                    if let Some(locked) =
                        toggle_widget_layout_lock_and_save(&layouts, &state_path, &id)
                    {
                        let _ = ui.set_property("layout-locked", Value::Bool(locked));
                    }
                    Value::Void
                })
                .unwrap_or_else(|error| {
                    eprintln!("failed to install dynamic layout lock callback: {error:?}");
                });
        }
    }
}

fn move_widget_window_and_schedule_save(
    request: WidgetMoveRequest<'_>,
    layouts: &Rc<RefCell<LayoutStore>>,
    state_path: &Path,
    save_timer: &Timer,
) {
    let WidgetMoveRequest {
        window,
        dx,
        dy,
        id,
        always_on_top,
        opacity_percent,
    } = request;
    if widget_layout_is_locked(layouts, id) {
        return;
    }
    let scale = window.scale_factor();
    let current = window.position();
    let x = current.x + (dx * scale).round() as i32;
    let y = current.y + (dy * scale).round() as i32;
    window.set_position(WindowPosition::Physical(PhysicalPosition::new(x, y)));

    let updated = {
        let mut store = layouts.borrow_mut();
        if let Some(instance) = store.widgets.iter_mut().find(|instance| instance.id == id) {
            instance.layout.x = x;
            instance.layout.y = y;
            instance.layout.always_on_top = always_on_top;
            instance.layout.opacity_percent = opacity_percent;
            let size = window.size();
            instance.layout.width = size.width as i32;
            instance.layout.height = size.height as i32;
            true
        } else {
            false
        }
    };

    if updated {
        schedule_layout_save(layouts, state_path, save_timer);
    }
}

fn schedule_layout_save(layouts: &Rc<RefCell<LayoutStore>>, state_path: &Path, save_timer: &Timer) {
    let layouts = layouts.clone();
    let state_path = state_path.to_path_buf();
    save_timer.start(
        TimerMode::SingleShot,
        DRAG_LAYOUT_SAVE_DEBOUNCE,
        move || {
            let store = layouts.borrow();
            if let Err(error) = save_layout_store(&state_path, &store) {
                eprintln!("failed to save widget layout: {error:#}");
            }
        },
    );
}

fn toggle_widget_layout_lock_and_save(
    layouts: &Rc<RefCell<LayoutStore>>,
    state_path: &Path,
    id: &str,
) -> Option<bool> {
    let mut store = layouts.borrow_mut();
    let instance = store
        .widgets
        .iter_mut()
        .find(|instance| instance.id == id)?;
    instance.layout.locked = !instance.layout.locked;
    let locked = instance.layout.locked;
    if let Err(error) = save_layout_store(state_path, &store) {
        eprintln!("failed to save widget layout lock: {error:#}");
    }
    Some(locked)
}

fn widget_layout_is_locked(layouts: &Rc<RefCell<LayoutStore>>, id: &str) -> bool {
    layouts
        .borrow()
        .widgets
        .iter()
        .find(|instance| instance.id == id)
        .map(|instance| instance.layout.locked)
        .unwrap_or(false)
}

fn callback_number_arg(args: &[Value], index: usize) -> Option<f64> {
    match args.get(index)? {
        Value::Number(value) => Some(*value),
        _ => None,
    }
}

fn dynamic_bool_property(ui: &ComponentInstance, name: &str) -> Option<bool> {
    match ui.get_property(name).ok()? {
        Value::Bool(value) => Some(value),
        _ => None,
    }
}

fn dynamic_number_property(ui: &ComponentInstance, name: &str) -> Option<i32> {
    match ui.get_property(name).ok()? {
        Value::Number(value) => Some(value.round() as i32),
        _ => None,
    }
}

fn refresh_runtime_symbols_from_store(
    widgets: &Rc<RefCell<Vec<WidgetRuntime>>>,
    layouts: &Rc<RefCell<LayoutStore>>,
    quote_cache: &QuoteCache,
    coin_icons: &CoinIconRegistry,
    plugin_catalog: &plugin::PluginCatalog,
) {
    let store = layouts.borrow();
    let settings = store.settings.clone().normalized();
    let proxy_url = settings::effective_network_proxy_url(&settings);
    for runtime in widgets.borrow_mut().iter_mut() {
        let Some(instance) = store
            .widgets
            .iter()
            .find(|instance| instance.id == runtime.id)
        else {
            continue;
        };
        let symbols = normalized_symbols_for_instance(instance, Some(plugin_catalog));
        if runtime.symbols != symbols {
            runtime.symbols = symbols;
            apply_runtime_view_with_icons(
                &runtime.ui,
                &widget_runtime_view(
                    &runtime.id,
                    &runtime.symbols,
                    quote_cache,
                    &settings,
                    Instant::now(),
                    false,
                ),
                &runtime.symbols,
                coin_icons,
                proxy_url.as_deref(),
            );
        }
    }
}

fn apply_instance_to_widget(
    ui: &WidgetUi,
    instance: &WidgetInstance,
    index: usize,
    settings: AppSettings,
    quote_cache: &QuoteCache,
    coin_icons: &CoinIconRegistry,
    set_position: bool,
    plugin_catalog: &plugin::PluginCatalog,
) {
    let locale = i18n::resolve_locale(settings.language);
    let text = i18n::text(locale);
    let symbols = normalized_symbols_for_instance(instance, Some(plugin_catalog));
    let layout = layout_for_instance(instance, index, settings.clone(), Some(plugin_catalog));

    ui.set_widget_id(instance.id.clone().into());
    ui.window()
        .set_size(LogicalSize::new(layout.width as f32, layout.height as f32));
    ui.set_widget_size(layout.width, layout.height);
    let proxy_url = settings::effective_network_proxy_url(&settings);
    apply_runtime_view_with_icons(
        ui,
        &widget_runtime_view(
            &instance.id,
            &symbols,
            quote_cache,
            &settings,
            Instant::now(),
            false,
        ),
        &symbols,
        coin_icons,
        proxy_url.as_deref(),
    );
    ui.set_pairs_heading_text(widget_heading(instance.widget_type(), locale).into());
    ui.set_empty_text(text.empty_pairs.into());
    ui.set_pin_to_top(instance.layout.always_on_top);
    ui.set_layout_locked(instance.layout.locked);
    ui.set_theme_name(widget_theme_name(settings.theme).into());
    ui.set_red_up_enabled(settings.red_up_enabled);
    ui.set_content_opacity(instance.layout.opacity_percent);
    ui.set_compact_mode(instance.widget_type() == WidgetType::MiniTicker);

    ui.request_redraw_now_and_later();
    if set_position {
        ui.window()
            .set_position(WindowPosition::Physical(PhysicalPosition::new(
                layout.x, layout.y,
            )));
    }
}

fn apply_widget_visibility(ui: &WidgetUi, visible: bool) -> Result<()> {
    if visible {
        ui.show()
    } else {
        ui.hide()
    }
}

pub(crate) fn update_market_feed_config_from_store(
    market_feed_config: &Arc<Mutex<market::MarketFeedConfig>>,
    store: &LayoutStore,
    plugin_catalog: &plugin::PluginCatalog,
) {
    let settings = store.settings.clone().normalized();
    update_market_feed_config(
        market_feed_config,
        settings,
        symbols_from_store(store, plugin_catalog),
    );
}

fn widget_heading(widget_type: WidgetType, locale: i18n::Locale) -> &'static str {
    match widget_type {
        WidgetType::QuoteBoard => i18n::text(locale).symbols,
        WidgetType::MiniTicker => widget_type_title(widget_type, locale),
    }
}

fn widget_theme_name(preference: settings::ThemePreference) -> &'static str {
    match theme::resolve_theme(preference) {
        theme::ResolvedTheme::Light => "light",
        theme::ResolvedTheme::Dark => "dark",
    }
}

pub(crate) fn apply_settings_to_widgets(
    widgets: &Rc<RefCell<Vec<WidgetRuntime>>>,
    layouts: &Rc<RefCell<LayoutStore>>,
    quote_cache: &QuoteCache,
    coin_icons: &CoinIconRegistry,
    plugin_catalog: &plugin::PluginCatalog,
) {
    let store = layouts.borrow();
    let settings = store.settings.clone().normalized();
    for runtime in widgets.borrow_mut().iter_mut() {
        if let Some((index, instance)) = store
            .widgets
            .iter()
            .enumerate()
            .find(|(_, instance)| instance.id == runtime.id)
        {
            apply_instance_to_widget(
                &runtime.ui,
                instance,
                index,
                settings.clone(),
                quote_cache,
                coin_icons,
                false,
                plugin_catalog,
            );
            runtime.symbols = normalized_symbols_for_instance(instance, Some(plugin_catalog));
        }
    }
}

fn apply_runtime_view_with_icons(
    ui: &WidgetUi,
    view: &widget_runtime::WidgetRuntimeView,
    symbols: &[String],
    coin_icons: &CoinIconRegistry,
    proxy_url: Option<&str>,
) {
    let quote_icons = coin_icons.icons_for_symbols(symbols, proxy_url);
    apply_runtime_view_to_widget(ui, view, &quote_icons);
}

fn widget_runtime_view(
    widget_id: &str,
    symbols: &[String],
    quote_cache: &QuoteCache,
    settings: &AppSettings,
    now: Instant,
    has_market_error: bool,
) -> widget_runtime::WidgetRuntimeView {
    let locale = i18n::resolve_locale(settings.language);
    let text = i18n::text(locale);
    widget_runtime::build_widget_runtime_view(
        widget_id,
        symbols,
        quote_cache,
        text.source_prefix,
        provider_labels(locale),
        runtime_text_labels(text),
        has_market_error,
        now,
    )
}

fn runtime_text_labels(text: &'static i18n::UiText) -> RuntimeTextLabels<'static> {
    RuntimeTextLabels {
        no_pairs: text.runtime_no_pairs,
        connecting: text.runtime_connecting,
        connection_error: text.runtime_connection_error,
        updated: text.runtime_updated,
        stale: text.runtime_stale,
        fallback: text.runtime_fallback,
        source_error: text.runtime_source_error,
        live_count_prefix: text.runtime_live_count_prefix,
        live_count_suffix: text.runtime_live_count_suffix,
    }
}

fn provider_labels(locale: i18n::Locale) -> ProviderLabels<'static> {
    ProviderLabels {
        binance: "Binance",
        okx: "OKX",
        hyperliquid: "Hyperliquid",
        mixed: match locale {
            i18n::Locale::ZhHans => "多个源",
            i18n::Locale::En => "Mixed",
        },
    }
}

fn update_market_feed_config(
    market_feed_config: &Arc<Mutex<market::MarketFeedConfig>>,
    settings: AppSettings,
    symbols: Vec<String>,
) {
    if let Ok(mut config) = market_feed_config.lock() {
        config.provider = settings.market_provider;
        config.refresh_interval_seconds = settings.refresh_interval_seconds;
        config.fallback_enabled = settings.market_fallback_enabled;
        config.enabled_sources = settings::enabled_market_sources(&settings);
        config.proxy_url = settings::effective_network_proxy_url(&settings);
        config.symbols = settings::normalized_symbols(symbols);
    }
}

fn notify_market_error(
    throttle: &Rc<RefCell<notifications::NotificationThrottle>>,
    locale: i18n::Locale,
    error: &str,
) {
    let now = Instant::now();
    if throttle
        .borrow_mut()
        .should_notify("market-feed", error, now)
    {
        notifications::show(
            i18n::text(locale).tray_tooltip,
            &i18n::market_error_notification_body(locale, error),
        );
    }
}

fn notify_alerts(
    throttle: &Rc<RefCell<notifications::NotificationThrottle>>,
    rules: &[settings::AlertRule],
    quote_cache: &QuoteCache,
) {
    let now = Instant::now();
    for alert in widget_runtime::evaluate_alerts_from_cache(rules, quote_cache) {
        let key = format!("alert:{}", alert.rule_id);
        if throttle.borrow_mut().should_notify(&key, &alert.body, now) {
            notifications::show(&alert.title, &alert.body);
        }
    }
}

fn notify_update_available(
    throttle: &Rc<RefCell<notifications::NotificationThrottle>>,
    locale: i18n::Locale,
    update: &updater::UpdateInfo,
) {
    let body = update_notification_body(update, locale);
    if throttle
        .borrow_mut()
        .should_notify("update-available", &body, Instant::now())
    {
        notifications::show(i18n::update_available_notification_title(locale), &body);
    }
}

fn update_notification_body(update: &updater::UpdateInfo, locale: i18n::Locale) -> String {
    i18n::update_available_notification_body(
        locale,
        &update.tag_name,
        update.asset_name.as_deref(),
        update.checksum_asset_name.as_deref(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_theme_name_uses_resolved_theme_names() {
        assert_eq!(widget_theme_name(settings::ThemePreference::Light), "light");
        assert_eq!(widget_theme_name(settings::ThemePreference::Dark), "dark");
    }

    #[test]
    fn update_notification_body_follows_locale() {
        let update = updater::UpdateInfo {
            tag_name: "v1.2.3".to_string(),
            version: semver::Version::parse("1.2.3").unwrap(),
            html_url: "https://example.com/releases/v1.2.3".to_string(),
            asset_name: Some("crypto-hud.exe".to_string()),
            asset_url: Some("https://example.com/crypto-hud.exe".to_string()),
            checksum_asset_name: Some("checksums.txt".to_string()),
            checksum_asset_url: Some("https://example.com/checksums.txt".to_string()),
        };

        assert_eq!(
            update_notification_body(&update, i18n::Locale::En),
            "v1.2.3 is available. Download crypto-hud.exe and verify it with checksums.txt from GitHub Releases."
        );
        assert_eq!(
            update_notification_body(&update, i18n::Locale::ZhHans),
            "已发布 v1.2.3。请在 GitHub Releases 下载 crypto-hud.exe，并使用 checksums.txt 校验。"
        );
    }
}
