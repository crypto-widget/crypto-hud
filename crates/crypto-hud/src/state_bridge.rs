use crypto_hud_shell_state as settings;
#[cfg(test)]
use settings::LegacyLayoutStore;
#[cfg(test)]
use settings::WidgetKind as WidgetType;
use settings::{
    AppSettings, LayoutStore, WidgetDefinition, WidgetInstance, WidgetLayout, WidgetSize,
    WidgetSizePolicy,
};

use crate::{feature_flags, plugin, window_manager::desktop_size};

const WIDGET_REORDER_ROW_HEIGHT: f32 = 72.0;

pub(crate) fn widget_definitions_from_catalog(
    catalog: &plugin::PluginCatalog,
) -> Vec<WidgetDefinition> {
    catalog
        .plugins()
        .iter()
        .map(widget_definition_from_plugin)
        .collect()
}

pub(crate) fn widget_definitions_from_optional_catalog(
    catalog: Option<&plugin::PluginCatalog>,
) -> Vec<WidgetDefinition> {
    catalog
        .map(widget_definitions_from_catalog)
        .unwrap_or_default()
}

fn widget_definition_from_plugin(plugin: &plugin::PluginDefinition) -> WidgetDefinition {
    WidgetDefinition {
        id: plugin.id.clone(),
        name: plugin.name.clone(),
        default_size: WidgetSize {
            width: plugin.default_size.width,
            height: plugin.default_size.height,
        },
        size_policy: widget_size_policy_from_plugin(plugin.size_policy),
        min_symbol_limit: plugin.min_symbol_limit,
        symbol_limit: plugin.symbol_limit,
        default_symbols: plugin.default_symbols.clone(),
    }
}

fn widget_size_policy_from_plugin(policy: plugin::PluginSizePolicy) -> WidgetSizePolicy {
    match policy {
        plugin::PluginSizePolicy::Fixed => WidgetSizePolicy::Fixed,
        plugin::PluginSizePolicy::SymbolBlocks {
            block_size,
            padding,
        } => WidgetSizePolicy::SymbolBlocks {
            block_width: block_size.width,
            block_height: block_size.height,
            padding_width: padding.width,
            padding_height: padding.height,
        },
        plugin::PluginSizePolicy::SymbolGrid {
            cell_size,
            content_padding,
            columns,
            rows,
        } => WidgetSizePolicy::SymbolGrid {
            cell_width: cell_size.width,
            cell_height: cell_size.height,
            content_padding_width: content_padding.width,
            content_padding_height: content_padding.height,
            columns,
            rows,
        },
    }
}

pub(crate) fn layout_for_instance(
    instance: &WidgetInstance,
    index: usize,
    settings: AppSettings,
    plugin_catalog: Option<&plugin::PluginCatalog>,
) -> WidgetLayout {
    let definitions = widget_definitions_from_optional_catalog(plugin_catalog);
    settings::layout_for_instance(instance, index, settings, &definitions, desktop_size())
}

#[cfg(test)]
pub(crate) fn layout_has_visible_area(layout: &WidgetLayout, widget_type: WidgetType) -> bool {
    layout_has_visible_area_for_size(layout, widget_type.default_size().tuple())
}

#[cfg(test)]
pub(crate) fn layout_has_visible_area_for_size(layout: &WidgetLayout, size: (i32, i32)) -> bool {
    settings::layout_has_visible_area_for_size(layout, WidgetSize::from(size), desktop_size())
}

#[cfg(test)]
pub(crate) fn default_layout_for_index(index: usize, settings: AppSettings) -> WidgetLayout {
    settings::default_layout_for_index(index, settings, desktop_size())
}

#[cfg(test)]
pub(crate) fn default_layout_for_widget(
    slot: usize,
    widget_type: WidgetType,
    settings: AppSettings,
) -> WidgetLayout {
    settings::default_layout_for_widget(slot, widget_type, settings, desktop_size())
}

#[cfg(test)]
pub(crate) fn default_layout_for_size(
    slot: usize,
    size: (i32, i32),
    settings: AppSettings,
) -> WidgetLayout {
    settings::default_layout_for_size(slot, WidgetSize::from(size), settings, desktop_size())
}

pub(crate) fn load_layout_store(
    path: &std::path::Path,
    requested_widget_count: usize,
    plugin_definitions: &[WidgetDefinition],
) -> LayoutStore {
    settings::load_layout_store(
        path,
        requested_widget_count,
        plugin_definitions,
        desktop_size(),
    )
}

#[cfg(test)]
pub(crate) fn migrate_legacy_store(legacy: LegacyLayoutStore) -> LayoutStore {
    settings::migrate_legacy_store(legacy, desktop_size())
}

#[cfg(test)]
pub(crate) fn normalize_store(store: &mut LayoutStore, requested_widget_count: usize) {
    settings::normalize_store_with_catalog(store, requested_widget_count, &[], desktop_size());
}

pub(crate) fn normalize_store_with_catalog(
    store: &mut LayoutStore,
    requested_widget_count: usize,
    plugin_catalog: Option<&plugin::PluginCatalog>,
) {
    let definitions = widget_definitions_from_optional_catalog(plugin_catalog);
    settings::normalize_store_with_catalog(
        store,
        requested_widget_count,
        &definitions,
        desktop_size(),
    );
}

#[cfg(test)]
pub(crate) fn add_widget_instance(
    store: &mut LayoutStore,
    widget_type: WidgetType,
    settings: &AppSettings,
) -> String {
    settings::add_widget_instance(store, widget_type, settings, desktop_size())
}

pub(crate) fn add_plugin_instance(
    store: &mut LayoutStore,
    plugin: &plugin::PluginDefinition,
    settings: &AppSettings,
) -> String {
    settings::add_plugin_instance(
        store,
        &widget_definition_from_plugin(plugin),
        settings,
        desktop_size(),
    )
}

pub(crate) fn select_widget_by_index(store: &mut LayoutStore, selected_index: i32) {
    settings::select_widget_by_index(store, selected_index);
}

pub(crate) fn move_widget_in_store(
    store: &mut LayoutStore,
    selected_index: i32,
    direction: i32,
) -> Option<usize> {
    settings::move_widget_in_store(store, selected_index, direction)
}

pub(crate) fn remove_widget_from_store_by_id(store: &mut LayoutStore, widget_id: &str) -> bool {
    settings::remove_widget_from_store_by_id(store, widget_id, desktop_size())
}

pub(crate) fn widget_reorder_steps(delta_y: f32) -> i32 {
    let threshold = WIDGET_REORDER_ROW_HEIGHT / 2.0;
    if delta_y >= threshold {
        ((delta_y - threshold) / WIDGET_REORDER_ROW_HEIGHT).floor() as i32 + 1
    } else if delta_y <= -threshold {
        -(((-delta_y - threshold) / WIDGET_REORDER_ROW_HEIGHT).floor() as i32 + 1)
    } else {
        0
    }
}

#[cfg(test)]
pub(crate) fn default_symbols() -> Vec<String> {
    settings::default_symbols_for_type(WidgetType::QuoteBoard)
}

#[cfg(test)]
pub(crate) fn parse_symbols(input: &str) -> Vec<String> {
    settings::parse_symbols_for_type(input, WidgetType::QuoteBoard)
}

#[cfg(test)]
pub(crate) fn parse_symbols_for_type(input: &str, widget_type: WidgetType) -> Vec<String> {
    settings::parse_symbols_for_type(input, widget_type)
}

pub(crate) fn symbols_from_store(
    store: &LayoutStore,
    plugin_catalog: &plugin::PluginCatalog,
) -> Vec<String> {
    let mut symbols = Vec::new();
    let definitions = widget_definitions_from_catalog(plugin_catalog);
    for widget in &store.widgets {
        for symbol in settings::normalized_symbols_for_instance(widget, &definitions) {
            if !symbols.contains(&symbol) {
                symbols.push(symbol);
            }
        }
    }
    if feature_flags::ALERT_RULES_ENABLED {
        for rule in &store.settings.alert_rules {
            if !rule.enabled {
                continue;
            }
            if let Some(symbol) = settings::normalize_market_pair_key(&rule.symbol) {
                if !symbols.contains(&symbol) {
                    symbols.push(symbol);
                }
            }
        }
    }
    symbols
}

pub(crate) fn normalized_symbols_for_instance(
    instance: &WidgetInstance,
    plugin_catalog: Option<&plugin::PluginCatalog>,
) -> Vec<String> {
    let definitions = widget_definitions_from_optional_catalog(plugin_catalog);
    settings::normalized_symbols_for_instance(instance, &definitions)
}
