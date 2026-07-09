use crypto_hud_shell_state::{
    self as settings, LayoutStore, WidgetInstance, WidgetKind as WidgetType,
};
use slint::{Image, ModelRc, SharedString, VecModel};

use crate::{i18n, plugin, PluginMarketItem};

use super::{
    format_symbols_input, widget_display_name, widget_scale_percent, widget_scale_percent_bounds,
    widget_type_description, widget_type_title, PREVIEW_KIND_FOCUS_TICKER, PREVIEW_KIND_GENERIC,
    PREVIEW_KIND_MARKET_BOARD, PREVIEW_KIND_MARKET_COMPASS, PREVIEW_KIND_STATUS_STRIP,
    PREVIEW_KIND_TRUST_CARD, STATUS_STRIP_PLUGIN_ID,
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
    _plugin_catalog: &plugin::PluginCatalog,
) -> Vec<String> {
    store
        .widgets
        .iter()
        .enumerate()
        .map(|(index, widget)| widget_display_name(widget, index, locale))
        .collect()
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
        None => Some(match locale {
            i18n::Locale::ZhHans => format!("插件不可用：{}", widget.plugin_id),
            i18n::Locale::En => format!("Plugin unavailable: {}", widget.plugin_id),
        }),
        Some(definition) => match &definition.status {
            plugin::PluginStatus::Available => None,
            plugin::PluginStatus::Disabled(reason) => Some(match locale {
                i18n::Locale::ZhHans => format!("插件已禁用：{reason}"),
                i18n::Locale::En => format!("Plugin disabled: {reason}"),
            }),
            plugin::PluginStatus::Unavailable(reason) => Some(match locale {
                i18n::Locale::ZhHans => format!("插件不可用：{reason}"),
                i18n::Locale::En => format!("Plugin unavailable: {reason}"),
            }),
        },
    }
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
                .map(plugin_preview_images)
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

fn widget_theme_label(theme: &plugin::PluginTheme, locale: i18n::Locale) -> String {
    match theme.role {
        plugin::PluginThemeRole::Light => i18n::theme_options(locale)[1].to_string(),
        plugin::PluginThemeRole::Dark => i18n::theme_options(locale)[2].to_string(),
        plugin::PluginThemeRole::Default if theme.id == "default" => match locale {
            i18n::Locale::En => "Default".to_string(),
            i18n::Locale::ZhHans => "默认".to_string(),
        },
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
            .map(|definition| plugin_market_item(definition, store, locale))
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
    let mut loaded = definition
        .preview_images
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
        .unwrap_or_else(|| definition.name.clone())
}

fn plugin_market_description(
    definition: &plugin::PluginDefinition,
    locale: i18n::Locale,
) -> String {
    if let Some(widget_type) = WidgetType::from_plugin_id(&definition.id) {
        return widget_type_description(widget_type, locale).to_string();
    }

    let capabilities = definition
        .data_requirements
        .iter()
        .map(|requirement| requirement.capability.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    match locale {
        i18n::Locale::ZhHans => format!(
            "本地 Slint 插件 v{} · {}x{} · {} · {}",
            definition.version,
            definition.default_size.width,
            definition.default_size.height,
            symbol_bounds_description(definition.min_symbol_limit, definition.symbol_limit, locale),
            capabilities
        ),
        i18n::Locale::En => format!(
            "Local Slint plugin v{} · {}x{} · {} · {}",
            definition.version,
            definition.default_size.width,
            definition.default_size.height,
            symbol_bounds_description(definition.min_symbol_limit, definition.symbol_limit, locale),
            capabilities
        ),
    }
}

fn symbol_bounds_description(min: usize, max: usize, locale: i18n::Locale) -> String {
    match locale {
        i18n::Locale::ZhHans if min == max => format!("{max} 个交易对"),
        i18n::Locale::ZhHans if min <= 1 => format!("最多 {max} 个交易对"),
        i18n::Locale::ZhHans => format!("{min}-{max} 个交易对"),
        i18n::Locale::En if min == max => format!("{max} pairs"),
        i18n::Locale::En if min <= 1 => format!("up to {max} pairs"),
        i18n::Locale::En => format!("{min}-{max} pairs"),
    }
}

fn plugin_market_status(definition: &plugin::PluginDefinition, locale: i18n::Locale) -> String {
    match &definition.status {
        plugin::PluginStatus::Available => match (definition.source, locale) {
            (plugin::PluginSource::Builtin, i18n::Locale::ZhHans) => "内置".to_string(),
            (plugin::PluginSource::Builtin, i18n::Locale::En) => "Built-in".to_string(),
            (plugin::PluginSource::LocalUnsigned, _) => String::new(),
            (plugin::PluginSource::TrustedSigned, i18n::Locale::ZhHans) => "已信任".to_string(),
            (plugin::PluginSource::TrustedSigned, i18n::Locale::En) => "Trusted".to_string(),
        },
        plugin::PluginStatus::Disabled(reason) => match locale {
            i18n::Locale::ZhHans => format!("已禁用：{reason}"),
            i18n::Locale::En => format!("Disabled: {reason}"),
        },
        plugin::PluginStatus::Unavailable(reason) => match locale {
            i18n::Locale::ZhHans => format!("不可用：{reason}"),
            i18n::Locale::En => format!("Unavailable: {reason}"),
        },
    }
}
