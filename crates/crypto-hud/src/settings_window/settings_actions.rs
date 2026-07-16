use crypto_hud_runtime::{normalize_plugin_color, valid_bounded_plugin_string};
use crypto_hud_shell_state as settings;
use serde_json::Value;
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
        .is_some_and(|definition| definition.is_available());
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
        .is_some_and(|definition| definition.is_available());
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
    apply_widget_parameter_value_to_store(
        store,
        selected_index,
        parameter_index,
        Value::Number(value.into()),
        plugin_catalog,
    )
}

pub(super) fn apply_widget_parameter_value_to_store(
    store: &mut LayoutStore,
    selected_index: i32,
    parameter_index: i32,
    candidate: Value,
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
        .and_then(|definition| {
            definition
                .parameters
                .get(parameter_index.max(0) as usize)
                .cloned()
        })
    else {
        return false;
    };
    let value = match &parameter {
        plugin::PluginParameter::Integer {
            minimum, maximum, ..
        } => candidate
            .as_i64()
            .and_then(|value| i32::try_from(value).ok())
            .map(|value| Value::Number(value.clamp(*minimum, *maximum).into())),
        plugin::PluginParameter::Boolean { .. } => candidate.as_bool().map(Value::Bool),
        plugin::PluginParameter::Choice { options, .. } => candidate
            .as_str()
            .filter(|candidate| options.iter().any(|option| option.value == *candidate))
            .map(|value| Value::String(value.to_string())),
        plugin::PluginParameter::Decimal {
            minimum,
            maximum,
            precision,
            ..
        } => candidate
            .as_f64()
            .filter(|value| value.is_finite())
            .and_then(|value| {
                let scale = 10_f64.powi(i32::from(*precision));
                let value = ((value.clamp(*minimum, *maximum) * scale).round() / scale)
                    .clamp(*minimum, *maximum);
                serde_json::Number::from_f64(value).map(Value::Number)
            }),
        plugin::PluginParameter::Color { .. } => candidate
            .as_str()
            .and_then(normalize_plugin_color)
            .map(Value::String),
        plugin::PluginParameter::String {
            min_length,
            max_length,
            ..
        } => candidate
            .as_str()
            .filter(|value| valid_bounded_plugin_string(value, *min_length, *max_length))
            .map(|value| Value::String(value.to_string())),
    };
    let Some(value) = value else {
        return false;
    };
    settings::set_widget_plugin_parameter(instance, parameter.key(), value);
    normalize_store_with_catalog(store, 0, Some(plugin_catalog));
    true
}

pub(super) fn step_widget_parameter_in_store(
    store: &mut LayoutStore,
    selected_index: i32,
    parameter_index: i32,
    direction: i32,
    plugin_catalog: &plugin::PluginCatalog,
) -> bool {
    if direction == 0 {
        return false;
    }
    let index = selected_index.max(0) as usize;
    let Some(instance) = store.widgets.get(index) else {
        return false;
    };
    let Some(parameter) = plugin_catalog
        .find(&instance.plugin_id)
        .filter(|definition| definition.is_available())
        .and_then(|definition| {
            definition
                .parameters
                .get(parameter_index.max(0) as usize)
                .cloned()
        })
    else {
        return false;
    };
    let current =
        parameter.normalized_value(settings::widget_plugin_parameter(instance, parameter.key()));
    let candidate = match &parameter {
        plugin::PluginParameter::Choice { options, .. } => {
            let Ok(option_count) = i32::try_from(options.len()) else {
                return false;
            };
            if option_count == 0 {
                return false;
            }
            let current_index = current
                .as_str()
                .and_then(|value| options.iter().position(|option| option.value == value))
                .unwrap_or_default() as i32;
            let next_index = (current_index + direction.signum()).rem_euclid(option_count);
            Value::String(options[next_index as usize].value.clone())
        }
        plugin::PluginParameter::Decimal { step, .. } => {
            let next = current.as_f64().unwrap_or_default() + step * f64::from(direction.signum());
            let Some(number) = serde_json::Number::from_f64(next) else {
                return false;
            };
            Value::Number(number)
        }
        _ => return false,
    };
    apply_widget_parameter_value_to_store(
        store,
        selected_index,
        parameter_index,
        candidate,
        plugin_catalog,
    )
}

pub(super) fn relink_missing_plugin_in_store(
    store: &mut LayoutStore,
    selected_index: i32,
    replacement_index: i32,
    plugin_catalog: &plugin::PluginCatalog,
) -> Option<String> {
    select_widget_by_index(store, selected_index);
    let widget_index = selected_index.max(0) as usize;
    let current_plugin_id = store.widgets.get(widget_index)?.plugin_id.clone();
    if plugin_catalog
        .find(&current_plugin_id)
        .is_some_and(|definition| definition.is_available())
    {
        return None;
    }
    let replacement = plugin_catalog
        .available_replacements(&current_plugin_id)
        .get(replacement_index.max(0) as usize)
        .cloned()?;
    let definitions = widget_definitions_from_catalog(plugin_catalog);
    let instance = store.widgets.get_mut(widget_index)?;
    let scale_percent = if instance.layout.scale_percent > 0 {
        instance.layout.scale_percent
    } else {
        settings::DEFAULT_WIDGET_SCALE_PERCENT
    };
    instance.plugin_id = replacement.id.clone();
    instance.legacy_widget_type = None;
    apply_widget_scale_to_instance(instance, &definitions, scale_percent);
    normalize_store_with_catalog(store, 0, Some(plugin_catalog));
    Some(replacement.name)
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
