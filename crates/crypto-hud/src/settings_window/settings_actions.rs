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
    apply_status_strip_auto_size, apply_widget_scale_to_instance, normalize_widget_name,
    widget_default_number,
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
    if let Some(instance) = store.widgets.iter_mut().find(|instance| instance.id == id) {
        apply_status_strip_auto_size(instance, settings.widget_scale_percent);
    }
    Some(id)
}

pub(super) fn apply_widget_settings_to_store(
    store: &mut LayoutStore,
    selected_index: i32,
    widget_name: &str,
    always_on_top: bool,
    layout_locked: bool,
    opacity_percent: i32,
    widget_scale_percent: i32,
    locale: i18n::Locale,
    plugin_catalog: &plugin::PluginCatalog,
) -> bool {
    let definitions = widget_definitions_from_catalog(plugin_catalog);
    select_widget_by_index(store, selected_index);
    let selected_id = store.selected_widget_id.clone();
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
        apply_widget_scale_to_instance(instance, &definitions, widget_scale_percent);
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
    let changed = if let Some(instance) = store.widgets.get_mut(index) {
        apply_widget_scale_to_instance(instance, &definitions, widget_scale_percent);
        true
    } else {
        false
    };
    normalize_store_with_catalog(store, 0, Some(plugin_catalog));
    changed
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
