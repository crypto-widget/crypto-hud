use crypto_hud_shell_state::{
    self as settings, LayoutStore, WidgetInstance, WidgetKind as WidgetType,
};
use slint::{Image, ModelRc, SharedString, VecModel};

use crate::{i18n, plugin, theme, PluginMarketItem};

use super::{
    format_symbols_input, widget_default_number, widget_display_name, widget_scale_percent,
    widget_scale_percent_bounds, widget_type_description, widget_type_title,
    PREVIEW_KIND_FOCUS_TICKER, PREVIEW_KIND_GENERIC, PREVIEW_KIND_MARKET_BOARD,
    PREVIEW_KIND_MARKET_COMPASS, PREVIEW_KIND_STATUS_STRIP, PREVIEW_KIND_TRUST_CARD,
    STATUS_STRIP_PLUGIN_ID,
};

pub(super) fn string_model(values: Vec<&'static str>) -> ModelRc<SharedString> {
    ModelRc::new(VecModel::from(
        values
            .into_iter()
            .map(SharedString::from)
            .collect::<Vec<_>>(),
    ))
}

pub(super) fn owned_string_model(values: Vec<String>) -> ModelRc<SharedString> {
    ModelRc::new(VecModel::from(
        values
            .into_iter()
            .map(SharedString::from)
            .collect::<Vec<_>>(),
    ))
}

pub(super) fn bool_model(values: Vec<bool>) -> ModelRc<bool> {
    ModelRc::new(VecModel::from(values))
}

pub(super) fn int_model(values: Vec<i32>) -> ModelRc<i32> {
    ModelRc::new(VecModel::from(values))
}

pub(super) fn image_model(values: Vec<Image>) -> ModelRc<Image> {
    ModelRc::new(VecModel::from(values))
}

pub(super) fn widget_instance_options(
    store: &LayoutStore,
    locale: i18n::Locale,
    plugin_catalog: &plugin::PluginCatalog,
) -> Vec<String> {
    store
        .widgets
        .iter()
        .enumerate()
        .map(|(index, widget)| {
            widget_display_name_for_catalog(widget, index, locale, plugin_catalog)
        })
        .collect()
}

pub(super) fn widget_display_name_for_catalog(
    widget: &WidgetInstance,
    index: usize,
    locale: i18n::Locale,
    catalog: &plugin::PluginCatalog,
) -> String {
    let Some(definition) = catalog.find(&widget.plugin_id) else {
        return widget_display_name(widget, index, locale);
    };
    let number = widget_default_number(widget, index);
    localized_bundled_default_name(widget, &definition, number, locale)
        .unwrap_or_else(|| widget_display_name(widget, index, locale))
}

fn localized_bundled_default_name(
    widget: &WidgetInstance,
    definition: &plugin::PluginDefinition,
    number: u64,
    locale: i18n::Locale,
) -> Option<String> {
    if widget.builtin_widget_type().is_some() || definition.source != plugin::PluginSource::Builtin
    {
        return None;
    }
    let title = i18n::builtin_plugin_title(locale, &definition.id)?;
    let canonical = format!("{} {number}", definition.name);
    if !widget.name.trim().is_empty() && widget.name != canonical {
        return None;
    }
    Some(format!(
        "{title} {}",
        i18n::ltr_isolate_for_locale(locale, &number.to_string())
    ))
}

pub(super) fn normalize_widget_name_for_catalog(
    widget: &WidgetInstance,
    raw: &str,
    index: usize,
    locale: i18n::Locale,
    catalog: &plugin::PluginCatalog,
) -> String {
    let number = widget_default_number(widget, index);
    if let Some(definition) = catalog.find(&widget.plugin_id) {
        if widget.builtin_widget_type().is_none()
            && definition.source == plugin::PluginSource::Builtin
            && i18n::builtin_plugin_title(locale, &definition.id).is_some()
            && raw.trim().is_empty()
        {
            return format!("{} {number}", definition.name);
        }
        if let Some(localized) = localized_bundled_default_name(widget, &definition, number, locale)
        {
            if raw.trim().is_empty() || raw == localized {
                return format!("{} {number}", definition.name);
            }
        }
    }
    super::normalize_widget_name(raw, widget.widget_type(), number, locale)
}

pub(super) fn widget_instance_detail_options(
    store: &LayoutStore,
    locale: i18n::Locale,
    plugin_catalog: &plugin::PluginCatalog,
) -> Vec<String> {
    let text = i18n::text(locale);
    store
        .widgets
        .iter()
        .map(|widget| {
            let symbols = format_symbols_input(&widget.symbols);
            let status = widget_plugin_status_suffix(widget, plugin_catalog, locale);
            let base = if symbols.is_empty() {
                text.empty_pairs.to_string()
            } else {
                symbols
            };
            if let Some(status) = status {
                format!("{base} · {status}")
            } else {
                base
            }
        })
        .collect()
}

fn widget_plugin_status_suffix(
    widget: &WidgetInstance,
    plugin_catalog: &plugin::PluginCatalog,
    locale: i18n::Locale,
) -> Option<String> {
    match plugin_catalog.find(&widget.plugin_id) {
        None => Some(i18n::plugin_unavailable_id(locale, &widget.plugin_id)),
        Some(definition) => match &definition.status {
            plugin::PluginStatus::Available => None,
            plugin::PluginStatus::Disabled(reason) => {
                Some(i18n::plugin_disabled_reason(locale, reason))
            }
            plugin::PluginStatus::Unavailable(reason) => {
                Some(i18n::plugin_unavailable_reason(locale, reason))
            }
        },
    }
}

pub(super) fn widget_plugin_needs_relink(
    widget: &WidgetInstance,
    plugin_catalog: &plugin::PluginCatalog,
) -> bool {
    plugin_catalog
        .find(&widget.plugin_id)
        .is_none_or(|definition| !definition.is_available())
}

pub(super) fn widget_plugin_relink_options(
    widget: &WidgetInstance,
    plugin_catalog: &plugin::PluginCatalog,
    locale: i18n::Locale,
) -> Vec<String> {
    plugin_catalog
        .available_replacements(&widget.plugin_id)
        .iter()
        .map(|definition| plugin_market_title(definition, locale))
        .collect()
}

pub(super) fn widget_visibility_options(store: &LayoutStore) -> Vec<bool> {
    store.widgets.iter().map(|widget| widget.visible).collect()
}

pub(super) fn widget_preview_kind_options(store: &LayoutStore) -> Vec<i32> {
    store
        .widgets
        .iter()
        .map(|widget| plugin_preview_kind(&widget.plugin_id))
        .collect()
}

pub(super) struct WidgetPreviewImageOptions {
    pub counts: Vec<i32>,
    pub image_1: Vec<Image>,
    pub image_2: Vec<Image>,
    pub image_3: Vec<Image>,
    pub image_4: Vec<Image>,
    pub image_5: Vec<Image>,
}

pub(super) fn widget_preview_image_options(
    store: &LayoutStore,
    plugin_catalog: &plugin::PluginCatalog,
) -> WidgetPreviewImageOptions {
    let previews = store
        .widgets
        .iter()
        .map(|widget| {
            plugin_catalog
                .find(&widget.plugin_id)
                .map(|definition| {
                    widget_preview_path(
                        widget,
                        &definition,
                        theme::resolve_theme(settings::ThemePreference::System),
                    )
                    .map(|path| load_preview_images(std::slice::from_ref(path)))
                    .unwrap_or_else(empty_preview_images)
                })
                .unwrap_or_else(empty_preview_images)
        })
        .collect::<Vec<_>>();

    WidgetPreviewImageOptions {
        counts: previews.iter().map(|preview| preview.count).collect(),
        image_1: previews
            .iter()
            .map(|preview| preview.images[0].clone())
            .collect(),
        image_2: previews
            .iter()
            .map(|preview| preview.images[1].clone())
            .collect(),
        image_3: previews
            .iter()
            .map(|preview| preview.images[2].clone())
            .collect(),
        image_4: previews
            .iter()
            .map(|preview| preview.images[3].clone())
            .collect(),
        image_5: previews
            .iter()
            .map(|preview| preview.images[4].clone())
            .collect(),
    }
}

pub(super) fn widget_scale_options(
    store: &LayoutStore,
    plugin_catalog: &plugin::PluginCatalog,
) -> Vec<i32> {
    store
        .widgets
        .iter()
        .map(|widget| widget_scale_percent(widget, plugin_catalog))
        .collect()
}

pub(super) fn widget_scale_min_options(
    store: &LayoutStore,
    plugin_catalog: &plugin::PluginCatalog,
) -> Vec<i32> {
    store
        .widgets
        .iter()
        .map(|widget| widget_scale_percent_bounds(widget, plugin_catalog).0)
        .collect()
}

pub(super) fn widget_scale_max_options(
    store: &LayoutStore,
    plugin_catalog: &plugin::PluginCatalog,
) -> Vec<i32> {
    store
        .widgets
        .iter()
        .map(|widget| widget_scale_percent_bounds(widget, plugin_catalog).1)
        .collect()
}

pub(super) fn widget_theme_options(
    widget: &WidgetInstance,
    plugin_catalog: &plugin::PluginCatalog,
    locale: i18n::Locale,
) -> Vec<String> {
    let Some(plugin) = plugin_catalog.find(&widget.plugin_id) else {
        return Vec::new();
    };
    if plugin.themes.len() <= 1 {
        return Vec::new();
    }

    let mut options = vec![i18n::theme_options(locale)[0].to_string()];
    options.extend(
        plugin
            .themes
            .iter()
            .map(|theme| widget_theme_label(theme, locale)),
    );
    options
}

pub(super) fn widget_theme_index(
    widget: &WidgetInstance,
    plugin_catalog: &plugin::PluginCatalog,
) -> i32 {
    let Some(plugin) = plugin_catalog.find(&widget.plugin_id) else {
        return 0;
    };
    if plugin.themes.len() <= 1 {
        return 0;
    }

    let preference = settings::widget_theme_preference(widget);
    if preference == settings::WIDGET_THEME_SYSTEM {
        return 0;
    }

    plugin
        .themes
        .iter()
        .position(|theme| theme.id == preference)
        .map(|index| index as i32 + 1)
        .unwrap_or(0)
}

pub(super) fn widget_theme_preference_for_index(
    widget: &WidgetInstance,
    plugin_catalog: &plugin::PluginCatalog,
    index: i32,
) -> String {
    if index <= 0 {
        return settings::WIDGET_THEME_SYSTEM.to_string();
    }
    let Some(plugin) = plugin_catalog.find(&widget.plugin_id) else {
        return settings::WIDGET_THEME_SYSTEM.to_string();
    };
    if plugin.themes.len() <= 1 {
        return settings::WIDGET_THEME_SYSTEM.to_string();
    }

    plugin
        .themes
        .get(index as usize - 1)
        .map(|theme| theme.id.clone())
        .unwrap_or_else(|| settings::WIDGET_THEME_SYSTEM.to_string())
}

pub(super) const PARAMETER_KIND_INTEGER: i32 = 0;
pub(super) const PARAMETER_KIND_BOOLEAN: i32 = 1;
pub(super) const PARAMETER_KIND_CHOICE: i32 = 2;
pub(super) const PARAMETER_KIND_DECIMAL: i32 = 3;
pub(super) const PARAMETER_KIND_COLOR: i32 = 4;
pub(super) const PARAMETER_KIND_STRING: i32 = 5;

#[derive(Default)]
pub(super) struct WidgetParameterOptions {
    pub kinds: Vec<i32>,
    pub labels: Vec<String>,
    pub helps: Vec<String>,
    pub units: Vec<String>,
    pub integer_values: Vec<i32>,
    pub minimums: Vec<i32>,
    pub maximums: Vec<i32>,
    pub steps: Vec<i32>,
    pub boolean_values: Vec<bool>,
    pub text_values: Vec<String>,
}

pub(super) fn widget_parameter_options(
    widget: &WidgetInstance,
    plugin_catalog: &plugin::PluginCatalog,
    locale: i18n::Locale,
) -> WidgetParameterOptions {
    let Some(definition) = plugin_catalog.find(&widget.plugin_id) else {
        return WidgetParameterOptions::default();
    };
    let mut options = WidgetParameterOptions::default();
    for parameter in &definition.parameters {
        options.labels.push(localized_parameter_text(
            parameter.name(),
            parameter.name_zh_hans(),
            locale,
        ));
        options.helps.push(localized_parameter_text(
            parameter.description(),
            parameter.description_zh_hans(),
            locale,
        ));
        let value =
            parameter.normalized_value(settings::widget_plugin_parameter(widget, parameter.key()));
        let mut kind = PARAMETER_KIND_INTEGER;
        let mut unit = String::new();
        let mut integer_value = 0;
        let mut minimum = 0;
        let mut maximum = 0;
        let mut step = 1;
        let mut boolean_value = false;
        let mut text_value = String::new();
        match parameter {
            plugin::PluginParameter::Integer {
                minimum: parameter_minimum,
                maximum: parameter_maximum,
                step: parameter_step,
                unit: parameter_unit,
                unit_zh_hans,
                ..
            } => {
                integer_value = value.as_i64().unwrap_or_default() as i32;
                minimum = *parameter_minimum;
                maximum = *parameter_maximum;
                step = *parameter_step;
                unit = localized_parameter_text(parameter_unit, unit_zh_hans, locale);
            }
            plugin::PluginParameter::Boolean { .. } => {
                kind = PARAMETER_KIND_BOOLEAN;
                boolean_value = value.as_bool().unwrap_or_default();
            }
            plugin::PluginParameter::Choice {
                options: choices, ..
            } => {
                kind = PARAMETER_KIND_CHOICE;
                text_value = value
                    .as_str()
                    .and_then(|value| choices.iter().find(|choice| choice.value == value))
                    .map(|choice| {
                        localized_parameter_text(&choice.name, &choice.name_zh_hans, locale)
                    })
                    .unwrap_or_default();
            }
            plugin::PluginParameter::Decimal {
                precision,
                unit: parameter_unit,
                unit_zh_hans,
                ..
            } => {
                kind = PARAMETER_KIND_DECIMAL;
                text_value = format!(
                    "{:.precision$}",
                    value.as_f64().unwrap_or_default(),
                    precision = usize::from(*precision)
                );
                unit = localized_parameter_text(parameter_unit, unit_zh_hans, locale);
            }
            plugin::PluginParameter::Color { .. } => {
                kind = PARAMETER_KIND_COLOR;
                text_value = value.as_str().unwrap_or_default().to_string();
            }
            plugin::PluginParameter::String { .. } => {
                kind = PARAMETER_KIND_STRING;
                text_value = value.as_str().unwrap_or_default().to_string();
            }
        }
        options.kinds.push(kind);
        options.units.push(unit);
        options.integer_values.push(integer_value);
        options.minimums.push(minimum);
        options.maximums.push(maximum);
        options.steps.push(step);
        options.boolean_values.push(boolean_value);
        options.text_values.push(text_value);
    }
    options
}

fn localized_parameter_text(english: &str, zh_hans: &str, locale: i18n::Locale) -> String {
    match locale {
        i18n::Locale::ZhHans if !zh_hans.trim().is_empty() => zh_hans.to_string(),
        _ => english.to_string(),
    }
}

fn widget_theme_label(theme: &plugin::PluginTheme, locale: i18n::Locale) -> String {
    match theme.role {
        plugin::PluginThemeRole::Light => i18n::theme_options(locale)[1].to_string(),
        plugin::PluginThemeRole::Dark => i18n::theme_options(locale)[2].to_string(),
        plugin::PluginThemeRole::Default if theme.id == "default" => {
            i18n::default_theme_label(locale).to_string()
        }
        plugin::PluginThemeRole::Default => theme.name.clone(),
    }
}

pub(crate) fn widget_type_usage_text(
    store: &LayoutStore,
    widget_type: WidgetType,
    locale: i18n::Locale,
) -> String {
    plugin_usage_text(store, widget_type.plugin_id(), locale)
}

fn plugin_usage_text(store: &LayoutStore, plugin_id: &str, locale: i18n::Locale) -> String {
    let count = store
        .widgets
        .iter()
        .filter(|widget| widget.plugin_id == plugin_id)
        .count();
    i18n::widget_usage_text(locale, count)
}

pub(super) fn plugin_market_items_model(
    catalog: &plugin::PluginCatalog,
    store: &LayoutStore,
    locale: i18n::Locale,
) -> ModelRc<PluginMarketItem> {
    ModelRc::new(VecModel::from(
        catalog
            .market_plugins()
            .map(|definition| plugin_market_item(&definition, store, locale))
            .collect::<Vec<_>>(),
    ))
}

pub(crate) fn plugin_market_item(
    definition: &plugin::PluginDefinition,
    store: &LayoutStore,
    locale: i18n::Locale,
) -> PluginMarketItem {
    let builtin = definition.source == plugin::PluginSource::Builtin;
    let preview_images = plugin_preview_images(definition);
    PluginMarketItem {
        title: plugin_market_title(definition, locale).into(),
        description: plugin_market_description(definition, locale).into(),
        usage: plugin_usage_text(store, &definition.id, locale).into(),
        status: plugin_market_status(definition, locale).into(),
        available: definition.is_available(),
        builtin,
        symbol_limit: definition.symbol_limit as i32,
        preview_kind: plugin_preview_kind(&definition.id),
        preview_frame_index: 0,
        preview_image_count: preview_images.count,
        preview_image_1: preview_images.images[0].clone(),
        preview_image_2: preview_images.images[1].clone(),
        preview_image_3: preview_images.images[2].clone(),
        preview_image_4: preview_images.images[3].clone(),
        preview_image_5: preview_images.images[4].clone(),
    }
}

struct PreviewImages {
    count: i32,
    images: [Image; plugin::MAX_PREVIEW_IMAGES],
}

fn empty_preview_images() -> PreviewImages {
    PreviewImages {
        count: 0,
        images: std::array::from_fn(|_| Image::default()),
    }
}

fn plugin_preview_images(definition: &plugin::PluginDefinition) -> PreviewImages {
    load_preview_images(&definition.preview_images)
}

fn widget_preview_path<'a>(
    widget: &WidgetInstance,
    definition: &'a plugin::PluginDefinition,
    system_theme: theme::ResolvedTheme,
) -> Option<&'a std::path::PathBuf> {
    let preference = settings::widget_theme_preference(widget);
    let theme_id = if preference == settings::WIDGET_THEME_SYSTEM {
        let role = match system_theme {
            theme::ResolvedTheme::Light => plugin::PluginThemeRole::Light,
            theme::ResolvedTheme::Dark => plugin::PluginThemeRole::Dark,
        };
        definition
            .themes
            .iter()
            .find(|theme| theme.role == role)
            .map(|theme| theme.id.as_str())
            .unwrap_or_else(|| plugin::default_theme_id(definition))
    } else {
        definition
            .themes
            .iter()
            .find(|theme| theme.id == preference)
            .map(|theme| theme.id.as_str())
            .unwrap_or_else(|| plugin::default_theme_id(definition))
    };
    let suffix = format!("-{theme_id}");
    definition
        .preview_images
        .iter()
        .find(|path| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .is_some_and(|stem| stem.ends_with(&suffix))
        })
        .or_else(|| definition.preview_images.first())
}

fn load_preview_images(paths: &[std::path::PathBuf]) -> PreviewImages {
    let mut loaded = paths
        .iter()
        .take(plugin::MAX_PREVIEW_IMAGES)
        .filter_map(|path| match Image::load_from_path(path) {
            Ok(image) => Some(image),
            Err(error) => {
                eprintln!(
                    "failed to load plugin preview image {}: {error}",
                    path.display()
                );
                None
            }
        })
        .collect::<Vec<_>>();
    let count = loaded.len() as i32;
    let images = std::array::from_fn(|_| {
        if loaded.is_empty() {
            Image::default()
        } else {
            loaded.remove(0)
        }
    });

    PreviewImages { count, images }
}

fn plugin_preview_kind(plugin_id: &str) -> i32 {
    match plugin_id {
        "com.cryptohud.focus-ticker" => PREVIEW_KIND_FOCUS_TICKER,
        "com.cryptohud.market-board" => PREVIEW_KIND_MARKET_BOARD,
        "com.cryptohud.trust-card" => PREVIEW_KIND_TRUST_CARD,
        "com.cryptohud.market-compass" => PREVIEW_KIND_MARKET_COMPASS,
        STATUS_STRIP_PLUGIN_ID => PREVIEW_KIND_STATUS_STRIP,
        _ => PREVIEW_KIND_GENERIC,
    }
}

fn plugin_market_title(definition: &plugin::PluginDefinition, locale: i18n::Locale) -> String {
    WidgetType::from_plugin_id(&definition.id)
        .map(|widget_type| widget_type_title(widget_type, locale).to_string())
        .or_else(|| {
            (definition.source == plugin::PluginSource::Builtin)
                .then(|| i18n::builtin_plugin_title(locale, &definition.id).map(str::to_string))
                .flatten()
        })
        .unwrap_or_else(|| definition.name.clone())
}

fn plugin_market_description(
    definition: &plugin::PluginDefinition,
    locale: i18n::Locale,
) -> String {
    if let Some(widget_type) = WidgetType::from_plugin_id(&definition.id) {
        return widget_type_description(widget_type, locale).to_string();
    }
    if definition.source == plugin::PluginSource::Builtin {
        if let Some(description) = i18n::builtin_plugin_description(locale, &definition.id) {
            return description.to_string();
        }
    }

    let capabilities = definition
        .data_requirements
        .iter()
        .map(|requirement| requirement.capability.as_str())
        .collect::<Vec<_>>();
    let capabilities = i18n::plugin_capabilities_description(locale, &capabilities);
    let symbol_bounds = i18n::symbol_bounds_description(
        locale,
        definition.min_symbol_limit,
        definition.symbol_limit,
    );
    let description = i18n::local_slint_plugin_description(
        locale,
        &definition.version,
        definition.default_size.width,
        definition.default_size.height,
        &symbol_bounds,
        &capabilities,
    );
    let compatibility = i18n::plugin_compatibility_summary(
        locale,
        definition.schema_version,
        &definition.host_api_version.to_string(),
    );
    format!("{description} · {compatibility}")
}

fn plugin_market_status(definition: &plugin::PluginDefinition, locale: i18n::Locale) -> String {
    match &definition.status {
        plugin::PluginStatus::Available => match definition.source {
            plugin::PluginSource::Builtin => i18n::plugin_builtin_label(locale).to_string(),
            plugin::PluginSource::LocalUnsigned => String::new(),
            plugin::PluginSource::TrustedSigned => i18n::plugin_trusted_label(locale).to_string(),
        },
        plugin::PluginStatus::Disabled(reason) => i18n::plugin_disabled_reason(locale, reason),
        plugin::PluginStatus::Unavailable(reason) => {
            i18n::plugin_unavailable_reason(locale, reason)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundled_widget_fixture() -> (WidgetInstance, plugin::PluginDefinition) {
        let mut definition = plugin::builtin_plugins().remove(0);
        definition.id = "com.cryptohud.focus-ticker".into();
        definition.name = "Focus Ticker".into();
        definition.preview_images = vec!["preview-light.png".into(), "preview-dark.png".into()];
        let widget = WidgetInstance {
            id: "focus-ticker-7".into(),
            plugin_id: definition.id.clone(),
            legacy_widget_type: None,
            name: "Focus Ticker 7".into(),
            visible: true,
            layout: settings::WidgetLayout::default(),
            symbols: vec!["BTC".into()],
            config: settings::default_widget_config(),
        };
        (widget, definition)
    }

    #[test]
    fn bundled_default_names_localize_without_changing_saved_names() {
        let (mut widget, definition) = bundled_widget_fixture();
        let catalog = plugin::PluginCatalog::from_plugins_for_tests(vec![definition.clone()]);
        for locale in i18n::Locale::ALL {
            let expected = format!(
                "{} {}",
                i18n::builtin_plugin_title(locale, &definition.id).unwrap(),
                i18n::ltr_isolate_for_locale(locale, "7")
            );
            assert_eq!(
                widget_display_name_for_catalog(&widget, 0, locale, &catalog),
                expected
            );
            assert_eq!(
                normalize_widget_name_for_catalog(&widget, &expected, 0, locale, &catalog),
                "Focus Ticker 7"
            );
            assert_eq!(widget.name, "Focus Ticker 7");
        }
        widget.name = "My trading desk".into();
        for locale in i18n::Locale::ALL {
            assert_eq!(
                widget_display_name_for_catalog(&widget, 0, locale, &catalog),
                "My trading desk"
            );
            assert_eq!(
                normalize_widget_name_for_catalog(&widget, "My desk Alpha", 0, locale, &catalog),
                "My desk Alpha"
            );
            assert_eq!(
                normalize_widget_name_for_catalog(&widget, "", 0, locale, &catalog),
                "Focus Ticker 7"
            );
        }
        widget.name = "Focus Ticker 99".into();
        assert_eq!(
            widget_display_name_for_catalog(&widget, 0, i18n::Locale::ZhHans, &catalog),
            "Focus Ticker 99"
        );
    }

    #[test]
    fn third_party_default_looking_names_remain_manifest_exact() {
        let (widget, mut definition) = bundled_widget_fixture();
        definition.source = plugin::PluginSource::LocalUnsigned;
        let catalog = plugin::PluginCatalog::from_plugins_for_tests(vec![definition]);
        for locale in i18n::Locale::ALL {
            assert_eq!(
                widget_display_name_for_catalog(&widget, 0, locale, &catalog),
                "Focus Ticker 7"
            );
        }
    }

    #[test]
    fn instance_preview_matches_explicit_and_system_themes() {
        let (mut widget, mut definition) = bundled_widget_fixture();
        settings::set_widget_theme_preference(&mut widget, "light");
        assert_eq!(
            widget_preview_path(&widget, &definition, theme::ResolvedTheme::Dark),
            definition.preview_images.first()
        );
        settings::set_widget_theme_preference(&mut widget, "dark");
        assert_eq!(
            widget_preview_path(&widget, &definition, theme::ResolvedTheme::Light),
            definition.preview_images.get(1)
        );
        settings::set_widget_theme_preference(&mut widget, "system");
        assert_eq!(
            widget_preview_path(&widget, &definition, theme::ResolvedTheme::Light),
            definition.preview_images.first()
        );
        assert_eq!(
            widget_preview_path(&widget, &definition, theme::ResolvedTheme::Dark),
            definition.preview_images.get(1)
        );
        definition.preview_images = vec!["author-cover.png".into()];
        assert_eq!(
            widget_preview_path(&widget, &definition, theme::ResolvedTheme::Dark),
            definition.preview_images.first()
        );
        definition.preview_images.clear();
        assert!(widget_preview_path(&widget, &definition, theme::ResolvedTheme::Dark).is_none());
    }

    #[test]
    fn custom_plugin_theme_labels_preserve_manifest_names_in_every_locale() {
        let custom_theme = plugin::PluginTheme {
            id: "solarized-dark".to_string(),
            name: "Solarized Dark".to_string(),
            role: plugin::PluginThemeRole::Default,
            is_default: false,
        };
        let light_theme = plugin::PluginTheme {
            id: "light".to_string(),
            name: "Light".to_string(),
            role: plugin::PluginThemeRole::Light,
            is_default: false,
        };

        for locale in i18n::Locale::ALL {
            assert_eq!(
                widget_theme_label(&custom_theme, locale),
                "Solarized Dark",
                "custom plugin theme names should stay manifest-exact for {locale:?}"
            );
        }

        assert_eq!(widget_theme_label(&light_theme, i18n::Locale::Ar), "فاتح");
        assert_eq!(custom_theme.id, "solarized-dark");
    }

    #[test]
    fn local_plugin_market_titles_preserve_manifest_names_in_every_locale() {
        let mut local_plugin = plugin::PluginDefinition {
            id: "com.example.portfolio-alpha".to_string(),
            name: "Portfolio Alpha".to_string(),
            version: semver::Version::new(1, 0, 0),
            schema_version: plugin::PLUGIN_MANIFEST_SCHEMA_VERSION,
            host_api_version: semver::VersionReq::STAR,
            source: plugin::PluginSource::LocalUnsigned,
            renderer: plugin::PluginRendererDefinition::Builtin(
                plugin::BuiltinRenderer::QuoteBoard,
            ),
            default_size: plugin::PluginSize {
                width: 320,
                height: 180,
            },
            size_policy: plugin::PluginSizePolicy::Fixed,
            min_symbol_limit: 1,
            symbol_limit: 5,
            default_symbols: Vec::new(),
            preview_images: Vec::new(),
            themes: plugin::single_default_theme(),
            data_requirements: Vec::new(),
            parameters: Vec::new(),
            status: plugin::PluginStatus::Available,
        };

        for locale in i18n::Locale::ALL {
            assert_eq!(
                plugin_market_title(&local_plugin, locale),
                "Portfolio Alpha",
                "local plugin market titles should stay manifest-exact for {locale:?}"
            );
        }

        local_plugin.source = plugin::PluginSource::Builtin;
        for locale in i18n::Locale::ALL {
            assert_eq!(
                plugin_market_title(&local_plugin, locale),
                "Portfolio Alpha",
                "unknown built-in plugin ids should still preserve manifest names for {locale:?}"
            );
        }
        assert_eq!(
            plugin_market_title(&plugin::builtin_plugins()[0], i18n::Locale::Ar),
            "لوحة الأسعار"
        );
    }
}
