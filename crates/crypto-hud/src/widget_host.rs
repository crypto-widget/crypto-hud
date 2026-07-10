use std::{cell::RefCell, rc::Rc, time::Duration};

use anyhow::{Context, Result};
use crypto_hud_runtime as widget_runtime;
use crypto_hud_runtime::QuoteRowView;
use slint::{ComponentHandle, Image, Model, ModelRc, SharedString, Timer, VecModel};
use slint_interpreter::{ComponentInstance, Struct as SlintStruct, Value};

use crate::{i18n, plugin, PriceCardWindow, QuoteRow};

const STATUS_STRIP_DEFAULT_CELL_WIDTH: i32 = 122;
const STATUS_STRIP_MIN_CELL_WIDTH: i32 = 112;
const STATUS_STRIP_MAX_VISIBLE_ROWS: usize = 5;
// The Slint window keeps a transparent edge around the visible 84px strip so
// rounded corners can be antialiased by Slint instead of hard-clipped by Win32.
const STATUS_STRIP_WINDOW_PADDING: i32 = 4;

pub(crate) struct WidgetRuntime {
    pub(crate) id: String,
    pub(crate) plugin_id: String,
    pub(crate) ui: WidgetUi,
    pub(crate) symbols: Vec<String>,
    pub(crate) show_coin_logos: bool,
    pub(crate) display_options: widget_runtime::WidgetDisplayOptions,
    pub(crate) widget_scale: f32,
    pub(crate) theme_name: String,
    pub(crate) locale: i18n::Locale,
}

pub(crate) enum WidgetUi {
    BuiltinPriceCard(PriceCardWindow),
    DynamicSlint(DynamicWidgetUi),
}

pub(crate) struct DynamicWidgetUi {
    pub(crate) instance: ComponentInstance,
}

impl WidgetUi {
    pub(crate) fn from_plugin(plugin: &plugin::PluginDefinition) -> Result<Self> {
        match &plugin.renderer {
            plugin::PluginRendererDefinition::Builtin(renderer) => match renderer {
                plugin::BuiltinRenderer::QuoteBoard | plugin::BuiltinRenderer::MiniTicker => {
                    Ok(Self::BuiltinPriceCard(
                        PriceCardWindow::new().context("failed to create Slint price card")?,
                    ))
                }
            },
            plugin::PluginRendererDefinition::Slint {
                definition: Some(definition),
                ..
            } => Ok(Self::DynamicSlint(DynamicWidgetUi {
                instance: definition
                    .create()
                    .context("failed to create dynamic Slint widget")?,
            })),
            plugin::PluginRendererDefinition::Slint {
                definition: None, ..
            } => bail_plugin_unavailable(plugin),
        }
    }

    pub(crate) fn show(&self) -> Result<()> {
        match self {
            Self::BuiltinPriceCard(ui) => ui.show().context("failed to show Slint price card"),
            Self::DynamicSlint(ui) => ui
                .instance
                .show()
                .context("failed to show dynamic Slint widget"),
        }
    }

    pub(crate) fn hide(&self) -> Result<()> {
        match self {
            Self::BuiltinPriceCard(ui) => ui.hide().context("failed to hide Slint price card"),
            Self::DynamicSlint(ui) => ui
                .instance
                .hide()
                .context("failed to hide dynamic Slint widget"),
        }
    }

    pub(crate) fn window(&self) -> &slint::Window {
        match self {
            Self::BuiltinPriceCard(ui) => ui.window(),
            Self::DynamicSlint(ui) => ui.instance.window(),
        }
    }

    pub(crate) fn request_redraw_now_and_later(&self) {
        match self {
            Self::BuiltinPriceCard(ui) => {
                ui.window().request_redraw();
                let weak = ui.as_weak();
                Timer::single_shot(Duration::from_millis(40), move || {
                    if let Some(ui) = weak.upgrade() {
                        ui.window().request_redraw();
                    }
                });
            }
            Self::DynamicSlint(ui) => {
                ui.instance.window().request_redraw();
                let weak = ui.instance.as_weak();
                Timer::single_shot(Duration::from_millis(40), move || {
                    if let Some(ui) = weak.upgrade() {
                        ui.window().request_redraw();
                    }
                });
            }
        }
    }

    pub(crate) fn set_widget_id(&self, value: SharedString) {
        match self {
            Self::BuiltinPriceCard(ui) => ui.set_widget_id(value),
            Self::DynamicSlint(ui) => ui.set_required_property("widget-id", Value::from(value)),
        }
    }

    fn set_quote_rows(&self, value: ModelRc<QuoteRow>) {
        match self {
            Self::BuiltinPriceCard(ui) => ui.set_quote_rows(value),
            Self::DynamicSlint(ui) => {
                ui.set_required_property("quote-rows", quote_rows_model_value(&value));
            }
        }
    }

    fn set_quote_icons(&self, value: ModelRc<Image>) {
        match self {
            Self::BuiltinPriceCard(ui) => ui.set_quote_icons(value),
            Self::DynamicSlint(ui) => {
                ui.set_optional_property("quote-icons", image_model_value(&value));
            }
        }
    }

    fn set_quote_icon_ready(&self, values: &[bool]) {
        if let Self::DynamicSlint(ui) = self {
            ui.set_optional_property("quote-icon-ready", bool_model_value(values));
        }
    }

    fn set_hide_quote_asset(&self, value: bool) {
        match self {
            Self::BuiltinPriceCard(ui) => ui.set_hide_quote_asset(value),
            Self::DynamicSlint(ui) => {
                ui.set_optional_property("hide-quote-asset", Value::Bool(value));
            }
        }
    }

    fn set_show_coin_logos(&self, value: bool) {
        match self {
            Self::BuiltinPriceCard(ui) => ui.set_show_coin_logos(value),
            Self::DynamicSlint(ui) => {
                ui.set_optional_property("show-coin-logos", Value::Bool(value));
            }
        }
    }

    fn set_quote_assets(&self, values: &[String]) {
        if let Self::DynamicSlint(ui) = self {
            ui.set_optional_property("quote-assets", string_model_value(values));
        }
    }

    fn set_quote_chart_line_paths(&self, values: &[String]) {
        if let Self::DynamicSlint(ui) = self {
            ui.set_optional_property("quote-chart-line-paths", string_model_value(values));
        }
    }

    fn set_quote_chart_fill_paths(&self, values: &[String]) {
        if let Self::DynamicSlint(ui) = self {
            ui.set_optional_property("quote-chart-fill-paths", string_model_value(values));
        }
    }

    fn set_quote_chart_up_candle_paths(&self, values: &[String]) {
        if let Self::DynamicSlint(ui) = self {
            ui.set_optional_property("quote-chart-up-candle-paths", string_model_value(values));
        }
    }

    fn set_quote_chart_down_candle_paths(&self, values: &[String]) {
        if let Self::DynamicSlint(ui) = self {
            ui.set_optional_property("quote-chart-down-candle-paths", string_model_value(values));
        }
    }

    fn set_quote_chart_ready(&self, values: &[bool]) {
        if let Self::DynamicSlint(ui) = self {
            ui.set_optional_property("quote-chart-ready", bool_model_value(values));
        }
    }

    fn set_quote_chart_positive(&self, values: &[bool]) {
        if let Self::DynamicSlint(ui) = self {
            ui.set_optional_property("quote-chart-positive", bool_model_value(values));
        }
    }

    pub(crate) fn set_pairs_heading_text(&self, value: SharedString) {
        match self {
            Self::BuiltinPriceCard(ui) => ui.set_pairs_heading_text(value),
            Self::DynamicSlint(ui) => {
                ui.set_required_property("pairs-heading-text", Value::from(value));
            }
        }
    }

    fn set_source_text(&self, value: SharedString) {
        match self {
            Self::BuiltinPriceCard(ui) => ui.set_source_text(value),
            Self::DynamicSlint(ui) => ui.set_required_property("source-text", Value::from(value)),
        }
    }

    fn set_source_name_text(&self, value: SharedString) {
        match self {
            Self::BuiltinPriceCard(ui) => ui.set_source_name_text(value),
            Self::DynamicSlint(ui) => {
                ui.set_optional_property("source-name-text", Value::from(value));
            }
        }
    }

    pub(crate) fn set_updated_text(&self, value: SharedString) {
        match self {
            Self::BuiltinPriceCard(ui) => ui.set_updated_text(value),
            Self::DynamicSlint(ui) => ui.set_required_property("updated-text", Value::from(value)),
        }
    }

    pub(crate) fn set_empty_text(&self, value: SharedString) {
        match self {
            Self::BuiltinPriceCard(ui) => ui.set_empty_text(value),
            Self::DynamicSlint(ui) => ui.set_required_property("empty-text", Value::from(value)),
        }
    }

    pub(crate) fn set_rtl_layout(&self, value: bool) {
        match self {
            Self::BuiltinPriceCard(ui) => ui.set_rtl_layout(value),
            Self::DynamicSlint(ui) => ui.set_optional_property("rtl-layout", Value::Bool(value)),
        }
    }

    pub(crate) fn set_pin_to_top(&self, value: bool) {
        match self {
            Self::BuiltinPriceCard(ui) => ui.set_pin_to_top(value),
            Self::DynamicSlint(ui) => ui.set_required_property("pin-to-top", Value::Bool(value)),
        }
    }

    pub(crate) fn set_layout_locked(&self, value: bool) {
        match self {
            Self::BuiltinPriceCard(ui) => ui.set_layout_locked(value),
            Self::DynamicSlint(ui) => {
                ui.set_required_property("layout-locked", Value::Bool(value));
            }
        }
    }

    pub(crate) fn set_widget_size(&self, width: i32, height: i32) {
        match self {
            Self::BuiltinPriceCard(ui) => {
                ui.set_widget_width(width);
                ui.set_widget_height(height);
            }
            Self::DynamicSlint(ui) => {
                ui.set_required_property("widget-width", Value::Number(width.into()));
                ui.set_required_property("widget-height", Value::Number(height.into()));
            }
        }
    }

    pub(crate) fn set_widget_scale(&self, scale: f32) {
        match self {
            Self::BuiltinPriceCard(ui) => ui.set_widget_scale(scale),
            Self::DynamicSlint(ui) => {
                ui.set_optional_property("widget-scale", Value::Number(scale as f64));
            }
        }
    }

    pub(crate) fn set_theme_name(&self, value: SharedString) {
        match self {
            Self::BuiltinPriceCard(ui) => ui.set_theme_name(value),
            Self::DynamicSlint(ui) => {
                ui.set_required_property("theme-name", Value::from(value));
            }
        }
    }

    pub(crate) fn set_red_up_enabled(&self, value: bool) {
        match self {
            Self::BuiltinPriceCard(ui) => ui.set_red_up_enabled(value),
            Self::DynamicSlint(ui) => {
                ui.set_required_property("red-up-enabled", Value::Bool(value));
            }
        }
    }

    pub(crate) fn set_content_opacity(&self, value: i32) {
        match self {
            Self::BuiltinPriceCard(ui) => ui.set_content_opacity(value),
            Self::DynamicSlint(ui) => {
                ui.set_required_property("content-opacity", Value::Number(value.into()));
            }
        }
    }

    pub(crate) fn set_compact_mode(&self, value: bool) {
        match self {
            Self::BuiltinPriceCard(ui) => ui.set_compact_mode(value),
            Self::DynamicSlint(ui) => ui.set_optional_property("compact-mode", Value::Bool(value)),
        }
    }

    fn set_chart_line_path(&self, value: SharedString) {
        if let Self::DynamicSlint(ui) = self {
            ui.set_optional_property("chart-line-path", Value::from(value));
        }
    }

    fn set_chart_fill_path(&self, value: SharedString) {
        if let Self::DynamicSlint(ui) = self {
            ui.set_optional_property("chart-fill-path", Value::from(value));
        }
    }

    fn set_chart_up_candle_path(&self, value: SharedString) {
        if let Self::DynamicSlint(ui) = self {
            ui.set_optional_property("chart-up-candle-path", Value::from(value));
        }
    }

    fn set_chart_down_candle_path(&self, value: SharedString) {
        if let Self::DynamicSlint(ui) = self {
            ui.set_optional_property("chart-down-candle-path", Value::from(value));
        }
    }

    fn set_chart_ready(&self, value: bool) {
        if let Self::DynamicSlint(ui) = self {
            ui.set_optional_property("chart-ready", Value::Bool(value));
        }
    }

    fn set_chart_end_y_ratio(&self, value: i32) {
        if let Self::DynamicSlint(ui) = self {
            ui.set_optional_property("chart-end-y-ratio", Value::Number(value.into()));
        }
    }

    fn set_chart_positive(&self, value: bool) {
        if let Self::DynamicSlint(ui) = self {
            ui.set_optional_property("chart-positive", Value::Bool(value));
        }
    }

    fn set_quote_cell_widths(&self, widths: Vec<i32>) {
        if let Self::DynamicSlint(ui) = self {
            ui.set_optional_property("quote-cell-widths", int_model_value(widths));
        }
    }
}

impl DynamicWidgetUi {
    fn set_required_property(&self, name: &str, value: Value) {
        if let Err(error) = self.instance.set_property(name, value) {
            eprintln!("failed to set dynamic widget property {name}: {error:?}");
        }
    }

    fn set_optional_property(&self, name: &str, value: Value) {
        let _ = self.instance.set_property(name, value);
    }
}

fn bail_plugin_unavailable(plugin: &plugin::PluginDefinition) -> Result<WidgetUi> {
    let reason = match &plugin.status {
        plugin::PluginStatus::Disabled(reason) => reason.as_str(),
        plugin::PluginStatus::Unavailable(reason) => reason.as_str(),
        plugin::PluginStatus::Available => plugin::SLINT_RENDERER_UNCOMPILED_REASON,
    };
    anyhow::bail!("plugin {} is unavailable: {reason}", plugin.id)
}

pub(crate) fn request_widget_redraws(widgets: &Rc<RefCell<Vec<WidgetRuntime>>>) {
    for runtime in widgets.borrow().iter() {
        runtime.ui.request_redraw_now_and_later();
    }
}

fn quote_rows_model(rows: &[QuoteRowView]) -> ModelRc<QuoteRow> {
    ModelRc::new(VecModel::from(
        rows.iter()
            .map(|row| QuoteRow {
                symbol: row.symbol.clone().into(),
                price: row.price.clone().into(),
                change: row.change.clone().into(),
                positive: row.positive,
            })
            .collect::<Vec<_>>(),
    ))
}

fn quote_rows_model_value(rows: &ModelRc<QuoteRow>) -> Value {
    let values = (0..rows.row_count())
        .filter_map(|index| rows.row_data(index))
        .map(|row| {
            Value::Struct(SlintStruct::from_iter([
                ("symbol".to_string(), Value::from(row.symbol)),
                ("price".to_string(), Value::from(row.price)),
                ("change".to_string(), Value::from(row.change)),
                ("positive".to_string(), Value::Bool(row.positive)),
            ]))
        })
        .collect::<Vec<_>>();
    Value::Model(ModelRc::new(VecModel::from(values)))
}

fn quote_icons_model(icons: &[Image]) -> ModelRc<Image> {
    ModelRc::new(VecModel::from(icons.to_vec()))
}

fn image_model_value(images: &ModelRc<Image>) -> Value {
    let values = (0..images.row_count())
        .filter_map(|index| images.row_data(index))
        .map(Value::Image)
        .collect::<Vec<_>>();
    Value::Model(ModelRc::new(VecModel::from(values)))
}

fn int_model_value(values: Vec<i32>) -> Value {
    Value::Model(ModelRc::new(VecModel::from(
        values
            .into_iter()
            .map(|value| Value::Number(value.into()))
            .collect::<Vec<_>>(),
    )))
}

fn string_model_value(values: &[String]) -> Value {
    Value::Model(ModelRc::new(VecModel::from(
        values
            .iter()
            .cloned()
            .map(|value| Value::from(SharedString::from(value)))
            .collect::<Vec<_>>(),
    )))
}

fn bool_model_value(values: &[bool]) -> Value {
    Value::Model(ModelRc::new(VecModel::from(
        values.iter().copied().map(Value::Bool).collect::<Vec<_>>(),
    )))
}

fn quote_cell_widths(rows: &[QuoteRowView], total_width: i32) -> Vec<i32> {
    if rows.is_empty() {
        return vec![total_width.max(STATUS_STRIP_DEFAULT_CELL_WIDTH)];
    }
    let count = rows.len().clamp(1, STATUS_STRIP_MAX_VISIBLE_ROWS);
    let total_width = if total_width > 0 {
        total_width
    } else {
        STATUS_STRIP_DEFAULT_CELL_WIDTH * count as i32
    };
    let min_total = STATUS_STRIP_MIN_CELL_WIDTH * count as i32;
    if total_width <= min_total {
        return split_even_width(total_width, count);
    }

    let weights = rows
        .iter()
        .take(count)
        .map(|row| quote_price_width_weight(&row.price))
        .collect::<Vec<_>>();
    let weight_sum = weights.iter().sum::<i32>().max(1);
    let extra_total = total_width - min_total;
    let mut remaining_extra = extra_total;
    weights
        .iter()
        .enumerate()
        .map(|(index, weight)| {
            let extra = if index + 1 == weights.len() {
                remaining_extra
            } else {
                (extra_total * *weight) / weight_sum
            };
            remaining_extra -= extra;
            STATUS_STRIP_MIN_CELL_WIDTH + extra
        })
        .collect()
}

fn split_even_width(total_width: i32, count: usize) -> Vec<i32> {
    let count_i32 = count as i32;
    let base_width = (total_width / count_i32).max(1);
    let mut remainder = (total_width - base_width * count_i32).max(0);
    (0..count)
        .map(|_| {
            let extra = if remainder > 0 {
                remainder -= 1;
                1
            } else {
                0
            };
            base_width + extra
        })
        .collect()
}

fn quote_price_width_weight(price: &str) -> i32 {
    (price.chars().count() as i32 - 3).clamp(1, 6)
}

pub(crate) fn apply_runtime_view_to_widget(
    ui: &WidgetUi,
    view: &widget_runtime::WidgetRuntimeView,
    quote_icons: &[Image],
    quote_icon_ready: &[bool],
    show_coin_logos: bool,
    display_options: widget_runtime::WidgetDisplayOptions,
    widget_scale: f32,
) {
    let cell_width_basis = if widget_scale.is_finite() && widget_scale > 0.0 {
        (ui.window().size().width as f32 / widget_scale).round() as i32
    } else {
        ui.window().size().width as i32
    };
    ui.set_quote_rows(quote_rows_model(&view.quote_rows));
    ui.set_quote_icons(quote_icons_model(quote_icons));
    ui.set_quote_icon_ready(quote_icon_ready);
    ui.set_show_coin_logos(show_coin_logos);
    ui.set_hide_quote_asset(display_options.hide_quote_asset);
    ui.set_quote_assets(&view.quote_assets);
    ui.set_quote_chart_line_paths(&view.quote_chart_line_paths);
    ui.set_quote_chart_fill_paths(&view.quote_chart_fill_paths);
    ui.set_quote_chart_up_candle_paths(&view.quote_chart_up_candle_paths);
    ui.set_quote_chart_down_candle_paths(&view.quote_chart_down_candle_paths);
    ui.set_quote_chart_ready(&view.quote_chart_ready);
    ui.set_quote_chart_positive(&view.quote_chart_positive);
    ui.set_quote_cell_widths(quote_cell_widths(
        &view.quote_rows,
        cell_width_basis - STATUS_STRIP_WINDOW_PADDING * 2,
    ));
    ui.set_source_text(view.source_text.clone().into());
    ui.set_source_name_text(view.source_name_text.clone().into());
    ui.set_updated_text(view.updated_text.clone().into());
    ui.set_chart_line_path(view.chart_line_path.clone().into());
    ui.set_chart_fill_path(view.chart_fill_path.clone().into());
    ui.set_chart_up_candle_path(view.chart_up_candle_path.clone().into());
    ui.set_chart_down_candle_path(view.chart_down_candle_path.clone().into());
    ui.set_chart_ready(view.chart_ready);
    ui.set_chart_end_y_ratio(view.chart_end_y_ratio);
    ui.set_chart_positive(view.chart_positive);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(price: &str) -> QuoteRowView {
        QuoteRowView {
            symbol: "BTC/USDT".to_string(),
            price: price.to_string(),
            change: "+1.23%".to_string(),
            positive: true,
        }
    }

    #[test]
    fn unavailable_dynamic_widget_uses_localizable_renderer_reason() {
        let plugin = plugin::PluginDefinition {
            id: "local.missing-definition".to_string(),
            name: "Missing Definition".to_string(),
            version: semver::Version::new(0, 1, 0),
            source: plugin::PluginSource::LocalUnsigned,
            renderer: plugin::PluginRendererDefinition::Slint {
                root_dir: std::path::PathBuf::from("."),
                entry: std::path::PathBuf::from("ui/main.slint"),
                component: "MissingDefinition".to_string(),
                definition: None,
            },
            default_size: plugin::PluginSize {
                width: 120,
                height: 80,
            },
            size_policy: plugin::PluginSizePolicy::Fixed,
            min_symbol_limit: plugin::MIN_SYMBOL_LIMIT,
            symbol_limit: 1,
            default_symbols: vec!["binance:spot:BTC/USDT".to_string()],
            preview_images: Vec::new(),
            themes: Vec::new(),
            data_requirements: Vec::new(),
            status: plugin::PluginStatus::Available,
        };

        let error = match WidgetUi::from_plugin(&plugin) {
            Ok(_) => panic!("missing Slint definition should make the plugin unavailable"),
            Err(error) => error.to_string(),
        };

        assert!(error.contains(plugin::SLINT_RENDERER_UNCOMPILED_REASON));
        assert!(
            !error.contains("Slint renderer is not compiled"),
            "fallback reason should match the i18n status reason key"
        );
    }

    #[test]
    fn quote_cell_widths_sum_to_available_width() {
        let widths = quote_cell_widths(&[row("59800"), row("1594"), row("100250")], 408);

        assert_eq!(widths.iter().sum::<i32>(), 408);
        assert!(widths[2] > widths[1]);
        assert!(widths[0] > widths[1]);
    }

    #[test]
    fn quote_cell_widths_handle_empty_rows() {
        assert_eq!(quote_cell_widths(&[], 408), vec![408]);
    }

    #[test]
    fn mini_ticker_compact_layout_does_not_reserve_quote_board_header() {
        let source = include_str!("../ui/price-card-window.slint");

        assert!(
            !source.contains(r#""Pairs""#) && !source.contains(r#""Source""#),
            "runtime labels should come from the localized host text, not English Slint fallbacks"
        );
        assert!(
            source.contains("in property <bool> rtl-layout: false;"),
            "builtin widgets should receive text direction from the host"
        );
        assert!(
            source.contains("x: root.rtl-layout ? parent.width - root.s(64px) : root.s(14px);")
                && source.contains("x: root.rtl-layout ? root.s(36px) : root.s(68px);")
                && source.contains("horizontal-alignment: root.rtl-layout ? left : right;"),
            "quote board header should mirror localized labels in RTL locales"
        );
        assert!(
            source.contains("x: root.rtl-layout ? parent.width / 2 : root.s(14px);")
                && source.contains("x: root.rtl-layout ? root.s(14px) : parent.width / 2;"),
            "compact mini ticker footer labels should swap sides in RTL locales"
        );
        assert!(
            source.contains("visible: !root.compact-mode;"),
            "compact mini ticker should not reserve the quote board header"
        );
        assert!(
            source.contains(
                "y: root.compact-mode ? root.s(16px) : root.quote-board-row-start-y + root.s(index * 26px);"
            ),
            "compact mini ticker should use its own row y offset"
        );
        assert!(
            source.contains("root.s(root.compact-mode ? 36px : 24px)"),
            "compact mini ticker should use its own footer offset"
        );
        assert!(
            source.contains("visible: root.compact-mode;"),
            "quote board should hide the footer while compact mini ticker keeps it"
        );
        assert!(
            source.contains("in property <bool> hide-quote-asset;"),
            "quote board layout should receive the quote-asset display option"
        );
        assert!(
            source.contains("in property <bool> show-coin-logos: true;"),
            "quote board layout should receive the coin-logo display option"
        );
        assert!(
            source.contains("(root.show-coin-logos ? 246px : 224px)"),
            "compact quote board width should be the unscaled quote board scale basis"
        );
        assert!(
            source.contains("root.quote-icon-space-visible ? 25px : 4px"),
            "hidden quote icons should not reserve their original symbol offset"
        );
        assert!(
            source.contains("property <length> quote-board-base-height:"),
            "quote board should derive its height scale basis from row count"
        );
        assert!(
            source.contains("root.quote-rows.length > 20 ? 20 : root.quote-rows.length")
                && source.contains("Math.mod(index, 2) == 0"),
            "quote board should render all rows up to the configured pair limit"
        );
        assert!(
            source.contains("visible: root.show-coin-logos && root.quote-icons.length > index;"),
            "hidden quote icons should not render into the compacted symbol column"
        );
        assert!(
            source.contains(
                "quote-board-change-x: root.quote-board-row-width - root.quote-board-change-width"
            ),
            "change column should follow the compacted price column"
        );
        assert!(
            source.contains(
                "quote-board-symbol-width: root.s(root.hide-quote-asset ? 46px : (root.quote-icon-space-visible ? 80px : 92px))"
            ) && source.contains("quote-board-change-width: root.s(64px)")
                && source.contains(
                    "quote-board-price-width: root.quote-board-change-x - root.quote-board-price-x - root.s(8px)"
                ),
            "quote board should reserve enough width for compact decimal prices"
        );
        assert!(
            source.contains("in property <float> widget-scale: 1.0;"),
            "builtin widgets should receive scale from the host"
        );
        assert!(
            source.contains("property <float> content-scale: root.widget-scale;"),
            "content scale should come from the externally supplied widget scale"
        );
        assert!(
            source.contains("function s(value: length) -> length")
                && source.contains("return value * root.content-scale;"),
            "builtin quote board should use a shared helper for direct scaled layout"
        );
        assert!(
            source.contains("property <length> card-edge-inset: root.light-theme ? 2px : 1px;")
                && source.contains("property <length> window-padding: 6px;")
                && source.contains("property <length> card-rim-size: 2px;")
                && source
                    .contains("x: (root.widget-width * 1px - root.s(root.content-width)) / 2 - root.s(root.card-rim-size);")
                && source.contains("x: edge_shadow.x + root.s(root.card-rim-size);")
                && source.contains("width: root.s(root.content-width + root.card-rim-size * 2);")
                && source.contains("width: edge_shadow.width - root.s(root.card-rim-size * 2);")
                && source.contains("drop-shadow-blur: root.s(root.card-shadow-blur);")
                && source.contains("clip: true;"),
            "the card should keep padding for an unclipped rounded rim"
        );
        assert!(
            source.find("drag_area := TouchArea").unwrap()
                < source.find("card := Rectangle").unwrap(),
            "dragging should stay in the unscaled root coordinate space"
        );
        assert!(
            !source.contains("transform-scale-x: root.content-scale;")
                && !source.contains("transform-scale-y: root.content-scale;"),
            "quote board should avoid transform scaling because the tiny live window clips it"
        );
        assert!(
            source.contains("font-size: root.s(root.compact-mode ? 13px : 12px);")
                && source.contains(
                    "width: root.compact-mode ? root.s(44px) : root.quote-board-symbol-width;"
                ),
            "row content should scale from the same standard content metrics as the card"
        );
    }
}
