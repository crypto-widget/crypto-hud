use crypto_hud_shell_state as settings;
use settings::{clamp_opacity, AppSettings, LayoutStore};

use crate::{
    i18n, plugin,
    state_bridge::{
        add_plugin_instance, normalize_store_with_catalog, remove_widget_from_store_by_id,
        select_widget_by_index, widget_definitions_from_catalog,
    },
};

use super::{
    apply_widget_scale_to_instance, normalize_widget_name,
    settings_models::widget_theme_preference_for_index, widget_default_number,
    widget_scale_percent_for_definitions,
};

pub(super) fn add_plugin_widget_to_store(
    store: &mut LayoutStore,
    plugin: &plugin::PluginDefinition,
    settings: &AppSettings,
) -> Option<String> {
    if !plugin.is_available() {
        return None;
    }

    let id = add_plugin_instance(store, plugin, settings);
    Some(id)
}

pub(super) struct WidgetSettingsUpdate<'a> {
    pub(super) selected_index: i32,
    pub(super) widget_name: &'a str,
    pub(super) always_on_top: bool,
    pub(super) layout_locked: bool,
    pub(super) opacity_percent: i32,
    pub(super) widget_scale_percent: i32,
    pub(super) widget_theme_index: i32,
    pub(super) show_coin_logos: bool,
    pub(super) hide_quote_asset: bool,
    pub(super) show_header: bool,
    pub(super) locale: i18n::Locale,
    pub(super) plugin_catalog: &'a plugin::PluginCatalog,
}

pub(super) fn apply_widget_settings_to_store(
    store: &mut LayoutStore,
    update: WidgetSettingsUpdate<'_>,
) -> bool {
    let WidgetSettingsUpdate {
        selected_index,
        widget_name,
        always_on_top,
        layout_locked,
        opacity_percent,
        widget_scale_percent,
        widget_theme_index,
        show_coin_logos,
        hide_quote_asset,
        show_header,
        locale,
        plugin_catalog,
    } = update;
    let definitions = widget_definitions_from_catalog(plugin_catalog);
    select_widget_by_index(store, selected_index);
    let selected_id = store.selected_widget_id.clone();
    let editable = selected_id
        .as_deref()
        .and_then(|id| store.widgets.iter().find(|instance| instance.id == id))
        .and_then(|instance| plugin_catalog.find(&instance.plugin_id))
        .is_some_and(plugin::PluginDefinition::is_available);
    if !editable {
        return false;
    }
    let changed = if let Some(instance) = selected_id
        .as_deref()
        .and_then(|id| store.widgets.iter_mut().find(|instance| instance.id == id))
    {
        let fallback_number = widget_default_number(instance, selected_index.max(0) as usize);
        instance.name =
            normalize_widget_name(widget_name, instance.widget_type(), fallback_number, locale);
        instance.layout.always_on_top = always_on_top;
        instance.layout.locked = layout_locked;
        instance.layout.opacity_percent = clamp_opacity(opacity_percent);
        let display_config_changed = settings::widget_show_coin_logos(instance) != show_coin_logos
            || settings::widget_hide_quote_asset(instance) != hide_quote_asset
            || settings::widget_show_header(instance) != show_header;
        let scale_percent = if display_config_changed {
            widget_scale_percent_for_definitions(instance, &definitions)
        } else {
            widget_scale_percent
        };
        let theme_preference =
            widget_theme_preference_for_index(instance, plugin_catalog, widget_theme_index);
        settings::set_widget_theme_preference(instance, &theme_preference);
        settings::set_widget_display_config(
            instance,
            show_coin_logos,
            hide_quote_asset,
            show_header,
        );
        apply_widget_scale_to_instance(instance, &definitions, scale_percent);
        true
    } else {
        false
    };
    normalize_store_with_catalog(store, 0, Some(plugin_catalog));
    changed
}

pub(super) fn apply_widget_scale_to_store(
    store: &mut LayoutStore,
    selected_index: i32,
    widget_scale_percent: i32,
    plugin_catalog: &plugin::PluginCatalog,
) -> bool {
    let definitions = widget_definitions_from_catalog(plugin_catalog);
    select_widget_by_index(store, selected_index);
    let index = selected_index.max(0) as usize;
    let editable = store
        .widgets
        .get(index)
        .and_then(|instance| plugin_catalog.find(&instance.plugin_id))
        .is_some_and(plugin::PluginDefinition::is_available);
    if !editable {
        return false;
    }
    let changed = if let Some(instance) = store.widgets.get_mut(index) {
        apply_widget_scale_to_instance(instance, &definitions, widget_scale_percent);
        true
    } else {
        false
    };
    normalize_store_with_catalog(store, 0, Some(plugin_catalog));
    changed
}

pub(super) fn apply_widget_integer_parameter_to_store(
    store: &mut LayoutStore,
    selected_index: i32,
    parameter_index: i32,
    value: i32,
    plugin_catalog: &plugin::PluginCatalog,
) -> bool {
    select_widget_by_index(store, selected_index);
    let index = selected_index.max(0) as usize;
    let Some(instance) = store.widgets.get_mut(index) else {
        return false;
    };
    let Some(parameter) = plugin_catalog
        .find(&instance.plugin_id)
        .filter(|definition| definition.is_available())
        .and_then(|definition| definition.parameters.get(parameter_index.max(0) as usize))
    else {
        return false;
    };
    let plugin::PluginParameter::Integer {
        key,
        minimum,
        maximum,
        ..
    } = parameter;
    settings::set_widget_integer_parameter(instance, key, value, *minimum, *maximum);
    normalize_store_with_catalog(store, 0, Some(plugin_catalog));
    true
}

pub(super) fn delete_widget_from_store_at_index(
    store: &mut LayoutStore,
    selected_index: i32,
) -> bool {
    let index = selected_index.max(0) as usize;
    let widget_id = store.widgets.get(index).map(|widget| widget.id.clone());
    widget_id
        .as_deref()
        .map(|widget_id| remove_widget_from_store_by_id(store, widget_id))
        .unwrap_or(false)
}
