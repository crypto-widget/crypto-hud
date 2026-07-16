use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use crypto_hud_market as market;
use crypto_hud_runtime as widget_runtime;
use crypto_hud_runtime::{
    ProviderLabels, QuoteCache, QuoteState, RuntimeTextLabels, WidgetDisplayOptions,
};
use crypto_hud_shell_state as settings;
use settings::{
    save_layout_store, AppSettings, LayoutStore, WidgetInstance, WidgetKind as WidgetType,
    WidgetSize,
};
use slint::{
    Brush, Color, ComponentHandle, LogicalSize, PhysicalPosition, SharedString, Timer, TimerMode,
    WindowPosition,
};
use slint_interpreter::{ComponentInstance, Value};

use crate::{
    coin_icons::CoinIconRegistry,
    feature_flags, i18n, notifications, plugin,
    settings_window::{apply_available_update, widget_type_title},
    state_bridge::{
        layout_for_instance, market_subscriptions_from_store, normalized_symbols_for_instance,
        widget_definitions_from_catalog,
    },
    theme, updater,
    widget_host::{
        apply_runtime_view_to_widget, logical_size_from_physical, WidgetRuntime, WidgetUi,
    },
    window_manager::schedule_widget_shell_window_configuration,
    SettingsWindow,
};

const NOTIFICATION_COOLDOWN: Duration = Duration::from_secs(300);
const DRAG_LAYOUT_SAVE_DEBOUNCE: Duration = Duration::from_millis(300);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PluginReloadSummary {
    pub(crate) applied: bool,
    pub(crate) generation: u64,
    pub(crate) plugin_count: usize,
    pub(crate) diagnostic_count: usize,
    pub(crate) changed_plugin_count: usize,
    pub(crate) replaced_instance_count: usize,
}

pub(crate) struct PluginReloadDeps<'a> {
    pub(crate) widgets: &'a Rc<RefCell<Vec<WidgetRuntime>>>,
    pub(crate) layouts: &'a Rc<RefCell<LayoutStore>>,
    pub(crate) state_path: &'a Path,
    pub(crate) state_dir: &'a Path,
    pub(crate) quote_cache: Rc<RefCell<QuoteCache>>,
    pub(crate) coin_icons: Rc<CoinIconRegistry>,
    pub(crate) market_feed_config: &'a Arc<Mutex<market::MarketFeedConfig>>,
    pub(crate) plugin_catalog: Rc<plugin::PluginCatalog>,
    pub(crate) widgets_hidden: &'a Rc<RefCell<bool>>,
    pub(crate) plan: plugin::PluginReloadPlan,
}

fn stage_runtime_replacements<T, E>(
    indices: &[usize],
    mut build: impl FnMut(usize) -> std::result::Result<Option<T>, E>,
) -> std::result::Result<Vec<T>, E> {
    let mut staged = Vec::new();
    for index in indices {
        if let Some(candidate) = build(*index)? {
            staged.push(candidate);
        }
    }
    Ok(staged)
}

fn should_show_reloaded_widget(instance_visible: bool, globally_hidden: bool) -> bool {
    instance_visible && !globally_hidden
}

pub(crate) fn reload_plugins_and_runtimes(
    deps: PluginReloadDeps<'_>,
) -> Result<PluginReloadSummary> {
    let reload = deps.plugin_catalog.reload_incremental(&deps.plan);
    let replacement = reload.catalog;
    let plugin_count = replacement.plugins().len();
    let diagnostics = replacement.errors();
    for message in replacement.diagnostic_messages(deps.state_dir) {
        eprintln!("plugin catalog warning: {message}");
    }
    let changed_plugin_count = reload.changed_plugin_ids.len();
    let instances = deps.layouts.borrow().widgets.clone();
    let reload_indices = runtime_reload_instance_indices(&instances, &reload.changed_plugin_ids);
    let settings = deps.layouts.borrow().settings.clone().normalized();
    let quote_cache = deps.quote_cache.borrow();
    let globally_hidden = *deps.widgets_hidden.borrow();
    let staged = stage_runtime_replacements(
        &reload_indices,
        |index| -> Result<Option<(WidgetRuntime, bool)>> {
            let instance = &instances[index];
            let Some(definition) = replacement.find(&instance.plugin_id) else {
                return Ok(None);
            };
            if !definition.is_available() {
                return Ok(None);
            }
            let runtime = build_widget_runtime(WidgetRuntimeBuildRequest {
                instance,
                index,
                settings: settings.clone(),
                quote_cache: &quote_cache,
                coin_icons: &deps.coin_icons,
                layouts: deps.layouts.clone(),
                state_path: deps.state_path,
                definition_catalog: &replacement,
                handler_catalog: deps.plugin_catalog.clone(),
            })?;
            Ok(Some((
                runtime,
                should_show_reloaded_widget(instance.visible, globally_hidden),
            )))
        },
    )?;
    drop(quote_cache);

    if !deps.plan.is_current(deps.state_dir) {
        eprintln!(
            "discarded stale plugin reload generation {}",
            deps.plan.generation
        );
        return Ok(PluginReloadSummary {
            applied: false,
            generation: deps.plan.generation,
            plugin_count: deps.plugin_catalog.plugins().len(),
            diagnostic_count: deps.plugin_catalog.errors().len(),
            changed_plugin_count: 0,
            replaced_instance_count: 0,
        });
    }

    let previous_catalog = (*deps.plugin_catalog).clone();
    deps.plugin_catalog.replace_with(replacement);
    for (runtime, visible) in &staged {
        if *visible {
            if let Err(error) = runtime.ui.show() {
                for (staged_runtime, _) in &staged {
                    let _ = staged_runtime.ui.hide();
                }
                deps.plugin_catalog.replace_with(previous_catalog);
                return Err(error)
                    .with_context(|| format!("failed to show reloaded widget {}", runtime.id));
            }
        }
    }

    let replaced_instance_count = staged.len();
    let mut old_runtimes = Vec::new();
    {
        let mut runtimes = deps.widgets.borrow_mut();
        let mut retained = Vec::with_capacity(runtimes.len() + staged.len());
        for runtime in runtimes.drain(..) {
            if reload.changed_plugin_ids.contains(&runtime.plugin_id) {
                old_runtimes.push(runtime);
            } else {
                retained.push(runtime);
            }
        }
        retained.extend(staged.into_iter().map(|(runtime, _)| runtime));
        let order = instances
            .iter()
            .enumerate()
            .map(|(index, instance)| (instance.id.clone(), index))
            .collect::<HashMap<_, _>>();
        retained.sort_by_key(|runtime| order.get(&runtime.id).copied().unwrap_or(usize::MAX));
        *runtimes = retained;
    }
    for runtime in old_runtimes {
        if let Err(error) = runtime.ui.hide() {
            eprintln!("failed to hide replaced widget {}: {error:#}", runtime.id);
        }
    }

    if changed_plugin_count > 0 {
        update_market_feed_config_from_store(
            deps.market_feed_config,
            &deps.layouts.borrow(),
            &deps.plugin_catalog,
        );
    }
    schedule_widget_shell_window_configuration();
    eprintln!(
        "applied plugin reload generation {}: {} sources compiled, {} plugins changed, {} instances replaced, {} last-known-good plugins retained",
        deps.plan.generation,
        reload.compiled_source_count,
        changed_plugin_count,
        replaced_instance_count,
        reload.retained_plugin_ids.len()
    );

    Ok(PluginReloadSummary {
        applied: true,
        generation: deps.plan.generation,
        plugin_count,
        diagnostic_count: diagnostics.len(),
        changed_plugin_count,
        replaced_instance_count,
    })
}

struct WidgetMoveRequest<'a> {
    window: &'a slint::Window,
    dx: f32,
    dy: f32,
    id: &'a str,
    always_on_top: bool,
    opacity_percent: i32,
    plugin_catalog: &'a plugin::PluginCatalog,
}

struct ApplyInstanceRequest<'a> {
    ui: &'a WidgetUi,
    instance: &'a WidgetInstance,
    index: usize,
    settings: AppSettings,
    quote_cache: &'a QuoteCache,
    coin_icons: &'a CoinIconRegistry,
    set_position: bool,
    plugin_catalog: &'a plugin::PluginCatalog,
}

struct WidgetRuntimeBuildRequest<'a> {
    instance: &'a WidgetInstance,
    index: usize,
    settings: AppSettings,
    quote_cache: &'a QuoteCache,
    coin_icons: &'a CoinIconRegistry,
    layouts: Rc<RefCell<LayoutStore>>,
    state_path: &'a Path,
    definition_catalog: &'a plugin::PluginCatalog,
    handler_catalog: Rc<plugin::PluginCatalog>,
}

struct ApplyRuntimeViewRequest<'a> {
    ui: &'a WidgetUi,
    view: &'a widget_runtime::WidgetRuntimeView,
    symbols: &'a [String],
    coin_icons: &'a CoinIconRegistry,
    proxy_url: Option<&'a str>,
    show_coin_logos: bool,
    display_options: WidgetDisplayOptions,
    widget_scale: f32,
}

pub(crate) struct RuntimeEventTimerDeps {
    pub(crate) widgets: Rc<RefCell<Vec<WidgetRuntime>>>,
    pub(crate) layouts: Rc<RefCell<LayoutStore>>,
    pub(crate) quote_cache: Rc<RefCell<QuoteCache>>,
    pub(crate) coin_icons: Rc<CoinIconRegistry>,
    pub(crate) plugin_catalog: Rc<plugin::PluginCatalog>,
    pub(crate) settings_window: slint::Weak<SettingsWindow>,
    pub(crate) market_updates: market::MarketFeed,
    pub(crate) update_events: Option<std::sync::mpsc::Receiver<updater::UpdateEvent>>,
}

pub(crate) fn install_runtime_event_timer(deps: RuntimeEventTimerDeps) -> Timer {
    let RuntimeEventTimerDeps {
        widgets,
        layouts,
        quote_cache,
        coin_icons,
        plugin_catalog,
        settings_window,
        market_updates,
        update_events,
    } = deps;
    let notification_throttle = Rc::new(RefCell::new(notifications::NotificationThrottle::new(
        NOTIFICATION_COOLDOWN,
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
                                QuoteState::new_with_chart_status(
                                    snapshot.price,
                                    snapshot.change_percent_24h,
                                    snapshot.chart_closes_24h.clone(),
                                    snapshot
                                        .chart_candles_24h
                                        .iter()
                                        .map(|candle| widget_runtime::ChartCandle {
                                            open_time_millis: candle.open_time_millis,
                                            open: candle.open,
                                            high: candle.high,
                                            low: candle.low,
                                            close: candle.close,
                                        })
                                        .collect(),
                                    snapshot.source,
                                    updated_at,
                                    snapshot.chart_updated_at,
                                    snapshot.chart_error.clone(),
                                ),
                            );
                        }

                        let cache = quote_cache.borrow();
                        if feature_flags::ALERT_RULES_ENABLED {
                            notify_alerts(
                                &notification_throttle,
                                i18n::resolve_locale(settings.language),
                                &settings.alert_rules,
                                &cache,
                            );
                        }
                    }
                    market::MarketEvent::Error(error) => {
                        *market_error_active.borrow_mut() = true;
                        eprintln!("market data update failed: {error}");
                    }
                }
            }

            if let Some(update_events) = &update_events {
                while let Ok(event) = update_events.try_recv() {
                    match event {
                        updater::UpdateEvent::Available(update) => {
                            let settings = layouts.borrow().settings.clone().normalized();
                            let locale = i18n::resolve_locale(settings.language);
                            if let Some(ui) = settings_window.upgrade() {
                                apply_available_update(&ui, &update, locale);
                            }
                            notify_update_available(&notification_throttle, locale, &update);
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
                apply_runtime_view_with_icons(ApplyRuntimeViewRequest {
                    ui: &widget.ui,
                    view: &widget_runtime_view(
                        &widget.id,
                        &widget.symbols,
                        &cache,
                        &settings,
                        now,
                        has_market_error,
                        widget.display_options,
                    ),
                    symbols: &widget.symbols,
                    coin_icons: &coin_icons,
                    proxy_url: proxy_url.as_deref(),
                    show_coin_logos: widget.show_coin_logos,
                    display_options: widget.display_options,
                    widget_scale: widget.widget_scale,
                });
            }
        });
    }
    timer
}

fn runtime_reload_instance_indices(
    instances: &[WidgetInstance],
    changed_plugin_ids: &BTreeSet<String>,
) -> Vec<usize> {
    instances
        .iter()
        .enumerate()
        .filter_map(|(index, instance)| {
            changed_plugin_ids
                .contains(&instance.plugin_id)
                .then_some(index)
        })
        .collect()
}

fn build_widget_runtime(request: WidgetRuntimeBuildRequest<'_>) -> Result<WidgetRuntime> {
    let WidgetRuntimeBuildRequest {
        instance,
        index,
        settings,
        quote_cache,
        coin_icons,
        layouts,
        state_path,
        definition_catalog,
        handler_catalog,
    } = request;
    let plugin = definition_catalog
        .find(&instance.plugin_id)
        .with_context(|| format!("unknown widget plugin {}", instance.plugin_id))?;
    if !plugin.is_available() {
        anyhow::bail!("widget plugin {} is unavailable", instance.plugin_id);
    }

    let ui = WidgetUi::from_plugin(&plugin)
        .with_context(|| format!("failed to create widget plugin {}", instance.plugin_id))?;
    apply_instance_to_widget(ApplyInstanceRequest {
        ui: &ui,
        instance,
        index,
        settings: settings.clone(),
        quote_cache,
        coin_icons,
        set_position: true,
        plugin_catalog: definition_catalog,
    });
    install_drag_handler(
        &ui,
        layouts.clone(),
        state_path.to_path_buf(),
        instance.id.clone(),
        handler_catalog,
    )?;
    install_layout_lock_handler(&ui, layouts, state_path.to_path_buf(), instance.id.clone())?;
    let symbols = normalized_symbols_for_instance(instance, Some(definition_catalog));
    let layout = layout_for_instance(instance, index, settings.clone(), Some(definition_catalog));
    let theme_name = widget_theme_name(instance, definition_catalog);
    let locale = i18n::resolve_locale(settings.language);
    Ok(WidgetRuntime {
        id: instance.id.clone(),
        plugin_id: instance.plugin_id.clone(),
        ui,
        symbols,
        show_coin_logos: settings::widget_show_coin_logos(instance),
        display_options: widget_display_options(instance),
        widget_scale: widget_scale_for_instance(instance, &layout, definition_catalog),
        theme_name,
        locale,
    })
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
            let layout =
                layout_for_instance(instance, index, settings.clone(), Some(&plugin_catalog));
            let theme_name = widget_theme_name(instance, &plugin_catalog);
            apply_instance_to_widget(ApplyInstanceRequest {
                ui: &runtime.ui,
                instance,
                index,
                settings: settings.clone(),
                quote_cache: &quote_cache_ref,
                coin_icons: &coin_icons,
                set_position: set_positions,
                plugin_catalog: &plugin_catalog,
            });
            apply_widget_visibility(&runtime.ui, instance.visible)?;
            runtime.symbols = normalized_symbols_for_instance(instance, Some(&plugin_catalog));
            runtime.show_coin_logos = settings::widget_show_coin_logos(instance);
            runtime.display_options = widget_display_options(instance);
            runtime.widget_scale = widget_scale_for_instance(instance, &layout, &plugin_catalog);
            runtime.theme_name = theme_name;
            runtime.locale = i18n::resolve_locale(settings.language);
            continue;
        }

        let Some(plugin) = plugin_catalog.find(&instance.plugin_id) else {
            eprintln!(
                "widget {} references unknown plugin {}",
                instance.id, instance.plugin_id
            );
            continue;
        };
        if !plugin.is_available() {
            eprintln!(
                "widget {} plugin {} is not available",
                instance.id, instance.plugin_id
            );
            continue;
        }
        let runtime = build_widget_runtime(WidgetRuntimeBuildRequest {
            instance,
            index,
            settings: settings.clone(),
            quote_cache: &quote_cache_ref,
            coin_icons: &coin_icons,
            layouts: layouts.clone(),
            state_path,
            definition_catalog: &plugin_catalog,
            handler_catalog: plugin_catalog.clone(),
        })?;
        apply_widget_visibility(&runtime.ui, instance.visible)?;
        runtimes.push(runtime);

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
    plugin_catalog: Rc<plugin::PluginCatalog>,
) -> Result<()> {
    match ui {
        WidgetUi::BuiltinPriceCard(ui) => {
            let weak = ui.as_weak();
            let save_timer = Timer::default();
            let plugin_catalog = plugin_catalog.clone();
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
                        plugin_catalog: &plugin_catalog,
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
            let plugin_catalog = plugin_catalog.clone();
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
                            plugin_catalog: &plugin_catalog,
                        },
                        &layouts,
                        &state_path,
                        &save_timer,
                    );
                    Value::Void
                })
                .map_err(|error| {
                    anyhow::anyhow!("failed to install dynamic drag callback: {error:?}")
                })?;
        }
    }
    Ok(())
}

fn install_layout_lock_handler(
    ui: &WidgetUi,
    layouts: Rc<RefCell<LayoutStore>>,
    state_path: PathBuf,
    id: String,
) -> Result<()> {
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
                .map_err(|error| {
                    anyhow::anyhow!("failed to install dynamic layout lock callback: {error:?}")
                })?;
        }
    }
    Ok(())
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
        plugin_catalog,
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
            let size = logical_size_from_physical(window.size(), scale);
            instance.layout.width = size.width.round().max(1.0) as i32;
            instance.layout.height = size.height.round().max(1.0) as i32;
            instance.layout.scale_percent = 0;
            let definitions = widget_definitions_from_catalog(plugin_catalog);
            instance.layout.scale_percent = settings::widget_scale_percent_for_instance(
                instance,
                &definitions,
                settings::DEFAULT_WIDGET_SCALE_PERCENT,
            );
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
    let locale = i18n::resolve_locale(settings.language);
    let proxy_url = settings::effective_network_proxy_url(&settings);
    for runtime in widgets.borrow_mut().iter_mut() {
        let Some((index, instance)) = store
            .widgets
            .iter()
            .enumerate()
            .find(|(_, instance)| instance.id == runtime.id)
        else {
            continue;
        };
        let symbols = normalized_symbols_for_instance(instance, Some(plugin_catalog));
        let show_coin_logos = settings::widget_show_coin_logos(instance);
        let display_options = widget_display_options(instance);
        let layout = layout_for_instance(instance, index, settings.clone(), Some(plugin_catalog));
        let widget_scale = widget_scale_for_instance(instance, &layout, plugin_catalog);
        let theme_name = widget_theme_name(instance, plugin_catalog);
        if runtime.symbols != symbols
            || runtime.show_coin_logos != show_coin_logos
            || runtime.display_options != display_options
            || (runtime.widget_scale - widget_scale).abs() > f32::EPSILON
            || runtime.theme_name != theme_name
            || runtime.locale != locale
        {
            runtime.symbols = symbols;
            runtime.show_coin_logos = show_coin_logos;
            runtime.display_options = display_options;
            runtime.widget_scale = widget_scale;
            runtime.theme_name = theme_name.clone();
            runtime.locale = locale;
            runtime.ui.set_theme_name(theme_name.into());
            apply_runtime_view_with_icons(ApplyRuntimeViewRequest {
                ui: &runtime.ui,
                view: &widget_runtime_view(
                    &runtime.id,
                    &runtime.symbols,
                    quote_cache,
                    &settings,
                    Instant::now(),
                    false,
                    runtime.display_options,
                ),
                symbols: &runtime.symbols,
                coin_icons,
                proxy_url: proxy_url.as_deref(),
                show_coin_logos: runtime.show_coin_logos,
                display_options: runtime.display_options,
                widget_scale: runtime.widget_scale,
            });
            runtime
                .ui
                .set_pairs_heading_text(widget_heading(instance.widget_type(), locale).into());
            runtime
                .ui
                .set_empty_text(i18n::text(locale).empty_pairs.into());
            runtime.ui.set_rtl_layout(i18n::is_rtl(locale));
        }
    }
}

fn apply_instance_to_widget(request: ApplyInstanceRequest<'_>) {
    let ApplyInstanceRequest {
        ui,
        instance,
        index,
        settings,
        quote_cache,
        coin_icons,
        set_position,
        plugin_catalog,
    } = request;
    let locale = i18n::resolve_locale(settings.language);
    let text = i18n::text(locale);
    let symbols = normalized_symbols_for_instance(instance, Some(plugin_catalog));
    let layout = layout_for_instance(instance, index, settings.clone(), Some(plugin_catalog));

    ui.set_widget_id(instance.id.clone().into());
    ui.window()
        .set_size(LogicalSize::new(layout.width as f32, layout.height as f32));
    ui.set_widget_size(layout.width, layout.height);
    let widget_scale = widget_scale_for_instance(instance, &layout, plugin_catalog);
    ui.set_widget_scale(widget_scale);
    let proxy_url = settings::effective_network_proxy_url(&settings);
    apply_runtime_view_with_icons(ApplyRuntimeViewRequest {
        ui,
        view: &widget_runtime_view(
            &instance.id,
            &symbols,
            quote_cache,
            &settings,
            Instant::now(),
            false,
            widget_display_options(instance),
        ),
        symbols: &symbols,
        coin_icons,
        proxy_url: proxy_url.as_deref(),
        show_coin_logos: settings::widget_show_coin_logos(instance),
        display_options: widget_display_options(instance),
        widget_scale,
    });
    ui.set_pairs_heading_text(widget_heading(instance.widget_type(), locale).into());
    ui.set_empty_text(text.empty_pairs.into());
    ui.set_rtl_layout(i18n::is_rtl(locale));
    ui.set_pin_to_top(instance.layout.always_on_top);
    ui.set_layout_locked(instance.layout.locked);
    ui.set_theme_name(widget_theme_name(instance, plugin_catalog).into());
    apply_plugin_parameters(ui, instance, plugin_catalog);
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

fn widget_scale_for_instance(
    instance: &WidgetInstance,
    layout: &settings::WidgetLayout,
    plugin_catalog: &plugin::PluginCatalog,
) -> f32 {
    let definitions = widget_definitions_from_catalog(plugin_catalog);
    if layout.scale_percent > 0 {
        return settings::widget_scale_percent_for_instance(
            instance,
            &definitions,
            settings::DEFAULT_WIDGET_SCALE_PERCENT,
        ) as f32
            / 100.0;
    }
    let base_size = settings::default_widget_size_for_instance(instance, &definitions);
    let size = WidgetSize {
        width: layout.width,
        height: layout.height,
    };
    settings::widget_content_scale_percent_for_size(size, base_size) as f32 / 100.0
}

fn apply_plugin_parameters(
    ui: &WidgetUi,
    instance: &WidgetInstance,
    plugin_catalog: &plugin::PluginCatalog,
) {
    let Some(definition) = plugin_catalog.find(&instance.plugin_id) else {
        return;
    };
    for parameter in &definition.parameters {
        let value = plugin_parameter_slint_value(
            parameter,
            settings::widget_plugin_parameter(instance, parameter.key()),
        );
        if let Some(value) = value {
            ui.set_plugin_parameter(parameter.key(), value);
        }
    }
}

fn plugin_parameter_slint_value(
    parameter: &plugin::PluginParameter,
    candidate: Option<&serde_json::Value>,
) -> Option<Value> {
    let normalized = parameter.normalized_value(candidate);
    match parameter {
        plugin::PluginParameter::Integer { .. } => {
            normalized.as_i64().map(|value| Value::Number(value as f64))
        }
        plugin::PluginParameter::Boolean { .. } => normalized.as_bool().map(Value::Bool),
        plugin::PluginParameter::Choice { .. } | plugin::PluginParameter::String { .. } => {
            normalized
                .as_str()
                .map(|value| Value::String(SharedString::from(value)))
        }
        plugin::PluginParameter::Decimal { .. } => normalized.as_f64().map(Value::Number),
        plugin::PluginParameter::Color { .. } => normalized
            .as_str()
            .and_then(widget_runtime::plugin_color_rgba)
            .map(|[red, green, blue, alpha]| {
                Value::Brush(Brush::SolidColor(Color::from_argb_u8(
                    alpha, red, green, blue,
                )))
            }),
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
        market_subscriptions_from_store(store, plugin_catalog),
    );
}

fn widget_heading(widget_type: WidgetType, locale: i18n::Locale) -> &'static str {
    match widget_type {
        WidgetType::QuoteBoard => i18n::text(locale).symbols,
        WidgetType::MiniTicker => widget_type_title(widget_type, locale),
    }
}

fn widget_theme_name(instance: &WidgetInstance, plugin_catalog: &plugin::PluginCatalog) -> String {
    let Some(plugin) = plugin_catalog.find(&instance.plugin_id) else {
        return "default".to_string();
    };
    resolve_plugin_theme_name(
        &plugin,
        &settings::widget_theme_preference(instance),
        theme::resolve_theme(settings::ThemePreference::System),
    )
}

fn resolve_plugin_theme_name(
    plugin: &plugin::PluginDefinition,
    preference: &str,
    system_theme: theme::ResolvedTheme,
) -> String {
    if preference == settings::WIDGET_THEME_SYSTEM {
        let role = match system_theme {
            theme::ResolvedTheme::Light => plugin::PluginThemeRole::Light,
            theme::ResolvedTheme::Dark => plugin::PluginThemeRole::Dark,
        };
        if let Some(theme) = plugin.themes.iter().find(|theme| theme.role == role) {
            return theme.id.clone();
        }
        return plugin::default_theme_id(plugin).to_string();
    }

    plugin
        .themes
        .iter()
        .find(|theme| theme.id == preference)
        .map(|theme| theme.id.clone())
        .unwrap_or_else(|| plugin::default_theme_id(plugin).to_string())
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
    let locale = i18n::resolve_locale(settings.language);
    for runtime in widgets.borrow_mut().iter_mut() {
        if let Some((index, instance)) = store
            .widgets
            .iter()
            .enumerate()
            .find(|(_, instance)| instance.id == runtime.id)
        {
            apply_instance_to_widget(ApplyInstanceRequest {
                ui: &runtime.ui,
                instance,
                index,
                settings: settings.clone(),
                quote_cache,
                coin_icons,
                set_position: false,
                plugin_catalog,
            });
            runtime.symbols = normalized_symbols_for_instance(instance, Some(plugin_catalog));
            runtime.show_coin_logos = settings::widget_show_coin_logos(instance);
            runtime.display_options = widget_display_options(instance);
            runtime.theme_name = widget_theme_name(instance, plugin_catalog);
            runtime.locale = locale;
        }
    }
}

fn apply_runtime_view_with_icons(request: ApplyRuntimeViewRequest<'_>) {
    let ApplyRuntimeViewRequest {
        ui,
        view,
        symbols,
        coin_icons,
        proxy_url,
        show_coin_logos,
        display_options,
        widget_scale,
    } = request;
    let quote_icons = quote_icons_for_display(coin_icons, symbols, proxy_url, show_coin_logos);
    let quote_icon_ready =
        quote_icon_ready_for_display(coin_icons, symbols, proxy_url, show_coin_logos);
    apply_runtime_view_to_widget(
        ui,
        view,
        &quote_icons,
        &quote_icon_ready,
        show_coin_logos,
        display_options,
        widget_scale,
    );
}

fn quote_icons_for_display(
    coin_icons: &CoinIconRegistry,
    symbols: &[String],
    proxy_url: Option<&str>,
    show_coin_logos: bool,
) -> Vec<slint::Image> {
    if show_coin_logos {
        coin_icons.icons_for_symbols(symbols, proxy_url)
    } else {
        Vec::new()
    }
}

fn quote_icon_ready_for_display(
    coin_icons: &CoinIconRegistry,
    symbols: &[String],
    proxy_url: Option<&str>,
    show_coin_logos: bool,
) -> Vec<bool> {
    if show_coin_logos {
        coin_icons.icon_ready_for_symbols(symbols, proxy_url)
    } else {
        Vec::new()
    }
}

fn widget_runtime_view(
    widget_id: &str,
    symbols: &[String],
    quote_cache: &QuoteCache,
    settings: &AppSettings,
    now: Instant,
    has_market_error: bool,
    display_options: WidgetDisplayOptions,
) -> widget_runtime::WidgetRuntimeView {
    let locale = i18n::resolve_locale(settings.language);
    let text = i18n::text(locale);
    widget_runtime::build_widget_runtime_view(widget_runtime::WidgetRuntimeViewParams {
        widget_id,
        symbols,
        quote_cache,
        source_prefix: text.source_prefix,
        provider_labels: provider_labels(locale),
        labels: runtime_text_labels(text, locale),
        has_market_error,
        now,
        display_options,
    })
}

fn widget_display_options(instance: &WidgetInstance) -> WidgetDisplayOptions {
    WidgetDisplayOptions {
        hide_quote_asset: settings::widget_hide_quote_asset(instance),
        show_header: settings::widget_show_header(instance),
    }
}

fn runtime_text_labels(
    text: &'static i18n::UiText,
    locale: i18n::Locale,
) -> RuntimeTextLabels<'static> {
    RuntimeTextLabels {
        no_pairs: text.runtime_no_pairs,
        connecting: text.runtime_connecting,
        connection_error: text.runtime_connection_error,
        updated: text.runtime_updated,
        stale: text.runtime_stale,
        source_error: text.runtime_source_error,
        live_count_prefix: text.runtime_live_count_prefix,
        live_count_suffix: text.runtime_live_count_suffix,
        elapsed_second_unit: text.runtime_elapsed_second_unit,
        elapsed_minute_unit: text.runtime_elapsed_minute_unit,
        isolate_numeric_values: i18n::is_rtl(locale),
    }
}

fn provider_labels(locale: i18n::Locale) -> ProviderLabels<'static> {
    ProviderLabels {
        binance: "Binance",
        coinbase: "Coinbase",
        okx: "OKX",
        hyperliquid: "Hyperliquid",
        mixed: i18n::provider_mixed_label(locale),
    }
}

fn update_market_feed_config(
    market_feed_config: &Arc<Mutex<market::MarketFeedConfig>>,
    settings: AppSettings,
    subscriptions: Vec<market::MarketSubscription>,
) {
    if let Ok(mut config) = market_feed_config.lock() {
        config.provider = settings.market_provider;
        config.refresh_interval_seconds = settings.refresh_interval_seconds;
        config.enabled_sources = settings::enabled_market_sources(&settings);
        config.proxy_url = settings::effective_network_proxy_url(&settings);
        config.subscriptions = normalized_feed_subscriptions(subscriptions);
    }
}

fn normalized_feed_subscriptions(
    subscriptions: Vec<market::MarketSubscription>,
) -> Vec<market::MarketSubscription> {
    subscriptions.into_iter().fold(
        Vec::<market::MarketSubscription>::new(),
        |mut normalized, subscription| {
            let Some(symbol) = settings::normalize_market_pair_key(&subscription.symbol) else {
                return normalized;
            };
            if let Some(existing) = normalized
                .iter_mut()
                .find(|existing| existing.symbol == symbol)
            {
                existing.needs_candles |= subscription.needs_candles;
            } else {
                normalized.push(market::MarketSubscription {
                    symbol,
                    needs_candles: subscription.needs_candles,
                });
            }
            normalized
        },
    )
}

fn notify_alerts(
    throttle: &Rc<RefCell<notifications::NotificationThrottle>>,
    locale: i18n::Locale,
    rules: &[settings::AlertRule],
    quote_cache: &QuoteCache,
) {
    let now = Instant::now();
    for alert in widget_runtime::evaluate_alerts_from_cache(rules, quote_cache) {
        let title = i18n::alert_notification_title(locale, &alert.symbol);
        let body = i18n::alert_notification_body(
            locale,
            &alert.symbol,
            alert.condition,
            alert.threshold,
            alert.current_value,
        );
        let key = format!("alert:{}", alert.rule_id);
        if throttle.borrow_mut().should_notify(&key, &body, now) {
            notifications::show(&title, &body);
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

    fn runtime_test_widget(symbols: Vec<&str>) -> WidgetInstance {
        WidgetInstance {
            id: "quote-board-1".to_string(),
            plugin_id: WidgetType::QuoteBoard.plugin_id().to_string(),
            legacy_widget_type: None,
            name: "Quote Board 1".to_string(),
            visible: true,
            layout: settings::WidgetLayout::default(),
            symbols: symbols.into_iter().map(str::to_string).collect(),
            config: settings::default_widget_config(),
        }
    }

    #[test]
    fn runtime_reload_plan_targets_every_changed_plugin_instance_in_layout_order() {
        let mut first = runtime_test_widget(vec!["binance:spot:BTC/USDT"]);
        first.id = "a-1".to_string();
        first.plugin_id = "com.example.a".to_string();
        let mut untouched = runtime_test_widget(vec!["binance:spot:ETH/USDT"]);
        untouched.id = "b-1".to_string();
        untouched.plugin_id = "com.example.b".to_string();
        let mut second = runtime_test_widget(vec!["binance:spot:SOL/USDT"]);
        second.id = "a-2".to_string();
        second.plugin_id = "com.example.a".to_string();
        let instances = vec![first, untouched, second];

        let indices = runtime_reload_instance_indices(
            &instances,
            &BTreeSet::from(["com.example.a".to_string()]),
        );

        assert_eq!(indices, vec![0, 2]);
        assert_eq!(instances[1].id, "b-1");
    }

    #[test]
    fn runtime_reload_staging_drops_all_candidates_when_any_build_fails() {
        use std::cell::Cell;

        #[derive(Debug)]
        struct StageProbe(Rc<Cell<usize>>);
        impl Drop for StageProbe {
            fn drop(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }

        let dropped = Rc::new(Cell::new(0));
        let existing_runtime_ids = vec!["a-1", "a-2"];
        let staged: std::result::Result<Vec<StageProbe>, &str> =
            stage_runtime_replacements(&[0, 1], |index| {
                if index == 1 {
                    Err("second instance failed")
                } else {
                    Ok(Some(StageProbe(dropped.clone())))
                }
            });

        assert_eq!(staged.unwrap_err(), "second instance failed");
        assert_eq!(dropped.get(), 1, "the partial staged set must be dropped");
        assert_eq!(existing_runtime_ids, vec!["a-1", "a-2"]);
    }

    #[test]
    fn runtime_reload_never_reshows_a_globally_hidden_widget() {
        assert!(should_show_reloaded_widget(true, false));
        assert!(!should_show_reloaded_widget(true, true));
        assert!(!should_show_reloaded_widget(false, false));
        assert!(!should_show_reloaded_widget(false, true));
    }

    #[test]
    fn feed_subscription_normalization_does_not_apply_a_single_widget_limit() {
        let subscriptions = (0..25)
            .map(|index| market::MarketSubscription {
                symbol: format!("binance:spot:ASSET{index}/USDT"),
                needs_candles: index == 24,
            })
            .collect::<Vec<_>>();

        let normalized = normalized_feed_subscriptions(subscriptions);

        assert_eq!(normalized.len(), 25);
        assert_eq!(normalized[24].symbol, "binance:spot:ASSET24/USDT");
        assert!(normalized[24].needs_candles);
    }

    #[test]
    fn runtime_text_bridge_uses_selected_locale_labels() {
        let zh_text = i18n::text(i18n::Locale::ZhHans);
        let zh_labels = runtime_text_labels(zh_text, i18n::Locale::ZhHans);
        assert_eq!(zh_labels.no_pairs, zh_text.runtime_no_pairs);
        assert_eq!(zh_labels.connecting, zh_text.runtime_connecting);
        assert_eq!(zh_labels.connection_error, zh_text.runtime_connection_error);
        assert_eq!(zh_labels.updated, zh_text.runtime_updated);
        assert_eq!(zh_labels.stale, zh_text.runtime_stale);
        assert_eq!(zh_labels.source_error, zh_text.runtime_source_error);
        assert_eq!(
            zh_labels.live_count_prefix,
            zh_text.runtime_live_count_prefix
        );
        assert_eq!(
            zh_labels.live_count_suffix,
            zh_text.runtime_live_count_suffix
        );
        assert_eq!(
            zh_labels.elapsed_second_unit,
            zh_text.runtime_elapsed_second_unit
        );
        assert_eq!(
            zh_labels.elapsed_minute_unit,
            zh_text.runtime_elapsed_minute_unit
        );
        assert!(!zh_labels.isolate_numeric_values);

        let ar_text = i18n::text(i18n::Locale::Ar);
        let ar_labels = runtime_text_labels(ar_text, i18n::Locale::Ar);
        assert_eq!(ar_labels.source_error, ar_text.runtime_source_error);
        assert_eq!(
            ar_labels.live_count_suffix,
            ar_text.runtime_live_count_suffix
        );
        assert!(ar_labels.isolate_numeric_values);
        assert_eq!(provider_labels(i18n::Locale::Ar).mixed, "مختلط");
    }

    #[test]
    fn runtime_refresh_tracks_locale_sensitive_widget_labels() {
        let source = include_str!("runtime_bridge.rs");
        let refresh_fn = source
            .split("fn refresh_runtime_symbols_from_store(")
            .nth(1)
            .unwrap()
            .split("fn apply_instance_to_widget")
            .next()
            .unwrap();

        for required in [
            "let locale = i18n::resolve_locale(settings.language);",
            "|| runtime.locale != locale",
            "runtime.locale = locale;",
            ".set_pairs_heading_text(widget_heading(instance.widget_type(), locale).into());",
            ".set_empty_text(i18n::text(locale).empty_pairs.into());",
            "runtime.ui.set_rtl_layout(i18n::is_rtl(locale));",
        ] {
            assert!(
                refresh_fn.contains(required),
                "runtime symbol refresh should update locale-sensitive widget labels: {required}"
            );
        }
    }

    #[test]
    fn initial_widget_apply_sets_locale_sensitive_widget_labels() {
        let source = include_str!("runtime_bridge.rs");
        let apply_fn = source
            .split("fn apply_instance_to_widget(")
            .nth(1)
            .unwrap()
            .split("fn widget_scale_for_instance")
            .next()
            .unwrap();

        for required in [
            "let locale = i18n::resolve_locale(settings.language);",
            "let text = i18n::text(locale);",
            "ui.set_pairs_heading_text(widget_heading(instance.widget_type(), locale).into());",
            "ui.set_empty_text(text.empty_pairs.into());",
            "ui.set_rtl_layout(i18n::is_rtl(locale));",
        ] {
            assert!(
                apply_fn.contains(required),
                "initial widget apply should set locale-sensitive widget label: {required}"
            );
        }
    }

    #[test]
    fn widget_theme_name_resolves_per_widget_theme_preference() {
        let catalog = plugin::PluginCatalog::builtins();
        let plugin = catalog.find(WidgetType::QuoteBoard.plugin_id()).unwrap();
        let mut instance = runtime_test_widget(vec!["binance:spot:BTC/USDT"]);

        assert_eq!(
            resolve_plugin_theme_name(
                &plugin,
                settings::WIDGET_THEME_SYSTEM,
                theme::ResolvedTheme::Light
            ),
            "light"
        );
        assert_eq!(
            resolve_plugin_theme_name(
                &plugin,
                settings::WIDGET_THEME_SYSTEM,
                theme::ResolvedTheme::Dark
            ),
            "dark"
        );

        settings::set_widget_theme_preference(&mut instance, "light");
        assert_eq!(widget_theme_name(&instance, &catalog), "light");

        let mut single_theme_plugin = plugin.clone();
        single_theme_plugin.themes = plugin::single_default_theme();
        assert_eq!(
            resolve_plugin_theme_name(
                &single_theme_plugin,
                settings::WIDGET_THEME_SYSTEM,
                theme::ResolvedTheme::Light
            ),
            "default"
        );
        assert_eq!(
            resolve_plugin_theme_name(&single_theme_plugin, "light", theme::ResolvedTheme::Light),
            "default"
        );
    }

    #[test]
    fn widget_scale_for_instance_follows_natural_quote_board_size() {
        let catalog = plugin::PluginCatalog::builtins();
        let mut instance =
            runtime_test_widget(vec!["binance:spot:BTC/USDT", "binance:spot:ETH/USDT"]);
        instance.layout.width = 429;
        instance.layout.height = 152;
        instance.layout.scale_percent = 0;

        assert_eq!(
            widget_scale_for_instance(&instance, &instance.layout, &catalog),
            1.5
        );

        settings::set_widget_display_config(&mut instance, false, true, true);
        instance.layout.width = 336;
        instance.layout.height = 152;
        instance.layout.scale_percent = 0;

        assert_eq!(
            widget_scale_for_instance(&instance, &instance.layout, &catalog),
            1.5
        );
    }

    #[test]
    fn widget_scale_for_instance_uses_status_strip_symbol_count_base() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("plugins");
        let catalog = plugin::PluginCatalog::discover(vec![root]);
        let mut instance = WidgetInstance {
            id: "plugin-strip-1".to_string(),
            plugin_id: "com.cryptohud.status-strip".to_string(),
            legacy_widget_type: None,
            name: "Status Strip 1".to_string(),
            visible: true,
            layout: settings::WidgetLayout {
                width: 748,
                height: 184,
                scale_percent: 0,
                ..settings::WidgetLayout::default()
            },
            symbols: vec![
                "binance:spot:BTC/USDT".to_string(),
                "binance:spot:ETH/USDT".to_string(),
                "binance:spot:SOL/USDT".to_string(),
            ],
            config: settings::default_widget_config(),
        };

        assert_eq!(
            widget_scale_for_instance(&instance, &instance.layout, &catalog),
            2.0
        );

        instance.symbols.truncate(1);
        instance.layout.width = 260;
        instance.layout.height = 184;
        instance.layout.scale_percent = 0;

        assert_eq!(
            widget_scale_for_instance(&instance, &instance.layout, &catalog),
            2.0
        );
    }

    #[test]
    fn extended_plugin_parameters_convert_to_matching_slint_values() {
        let boolean: plugin::PluginParameter = serde_json::from_value(serde_json::json!({
            "kind": "boolean", "key": "enabled", "name": "Enabled", "default": true
        }))
        .unwrap();
        let choice: plugin::PluginParameter = serde_json::from_value(serde_json::json!({
            "kind": "choice", "key": "density", "name": "Density", "default": "compact",
            "options": [
                { "value": "compact", "name": "Compact" },
                { "value": "wide", "name": "Wide" }
            ]
        }))
        .unwrap();
        let decimal: plugin::PluginParameter = serde_json::from_value(serde_json::json!({
            "kind": "decimal", "key": "opacity", "name": "Opacity", "default": 0.5,
            "minimum": 0.0, "maximum": 1.0, "step": 0.1, "precision": 2
        }))
        .unwrap();
        let color: plugin::PluginParameter = serde_json::from_value(serde_json::json!({
            "kind": "color", "key": "accent", "name": "Accent", "default": "#10203040"
        }))
        .unwrap();
        let string: plugin::PluginParameter = serde_json::from_value(serde_json::json!({
            "kind": "string", "key": "caption", "name": "Caption", "default": "Market",
            "maxLength": 24
        }))
        .unwrap();

        assert!(matches!(
            plugin_parameter_slint_value(&boolean, Some(&serde_json::json!(false))),
            Some(Value::Bool(false))
        ));
        assert!(matches!(
            plugin_parameter_slint_value(&choice, Some(&serde_json::json!("wide"))),
            Some(Value::String(value)) if value.as_str() == "wide"
        ));
        assert!(matches!(
            plugin_parameter_slint_value(&decimal, Some(&serde_json::json!(0.756))),
            Some(Value::Number(value)) if (value - 0.76).abs() < f64::EPSILON
        ));
        let Some(Value::Brush(Brush::SolidColor(color))) =
            plugin_parameter_slint_value(&color, None)
        else {
            panic!("color parameter should inject a solid Slint brush");
        };
        let rgba = color.to_argb_u8();
        assert_eq!(
            [rgba.red, rgba.green, rgba.blue, rgba.alpha],
            [0x10, 0x20, 0x30, 0x40]
        );
        assert!(matches!(
            plugin_parameter_slint_value(&string, Some(&serde_json::json!("Pulse"))),
            Some(Value::String(value)) if value.as_str() == "Pulse"
        ));
    }

    #[test]
    fn quote_icons_are_empty_when_coin_logos_are_hidden() {
        let cache_dir = std::env::temp_dir().join(format!(
            "crypto-hud-hidden-icons-test-{}",
            std::process::id()
        ));
        let registry = CoinIconRegistry::new(cache_dir.clone());

        let icons = quote_icons_for_display(
            &registry,
            &["binance:spot:BTC/USDT".to_string()],
            None,
            false,
        );

        assert!(icons.is_empty());
        drop(registry);
        let _ = std::fs::remove_dir_all(cache_dir);
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
