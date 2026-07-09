#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
use std::{
    collections::{HashMap, HashSet},
    env,
    ffi::{OsStr, OsString},
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub use crypto_hud_core::{
    clamp_refresh_interval, default_enabled_market_sources, default_market_symbols,
    format_market_pair_display, format_market_pair_source, format_market_pair_symbol,
    market_pair_source, normalize_market_pair_key, normalize_market_symbols,
    normalize_symbol_token, AlertCondition, AlertRule, MarketDataSource, MarketPair,
    MarketProviderPreference, MarketType, DEFAULT_REFRESH_INTERVAL_SECONDS,
    MAX_REFRESH_INTERVAL_SECONDS, MIN_REFRESH_INTERVAL_SECONDS,
};

pub const MIN_OPACITY_PERCENT: i32 = 20;
pub const MAX_OPACITY_PERCENT: i32 = 100;
pub const DEFAULT_OPACITY_PERCENT: i32 = 96;
pub const DEFAULT_NEXT_WIDGET_NUMBER: u64 = 1;
pub const DEFAULT_WIDGET_POSITION_X: i32 = 96;
pub const DEFAULT_WIDGET_POSITION_Y: i32 = 96;
pub const LEGACY_DEFAULT_POSITION_STEP: i32 = 32;
pub const DEFAULT_LAYOUT_MARGIN_X: i32 = 96;
pub const DEFAULT_LAYOUT_MARGIN_Y: i32 = 96;
pub const DEFAULT_LAYOUT_GAP: i32 = 24;
pub const WIDGET_CONFIG_SHOW_COIN_LOGOS: &str = "show_coin_logos";
pub const WIDGET_CONFIG_HIDE_QUOTE_ASSET: &str = "hide_quote_asset";
pub const WIDGET_CONFIG_THEME: &str = "theme";
pub const WIDGET_THEME_SYSTEM: &str = "system";
pub const QUOTE_BOARD_WIDTH: i32 = 286;
pub const QUOTE_BOARD_HEIGHT: i32 = 194;
pub const MINI_TICKER_WIDTH: i32 = 236;
pub const MINI_TICKER_HEIGHT: i32 = 112;
const QUOTE_BOARD_WIDTH_WITHOUT_COIN_LOGOS: i32 = 274;
const QUOTE_BOARD_WIDTH_WITHOUT_QUOTE_ASSET: i32 = 246;
const QUOTE_BOARD_WIDTH_COMPACT_SYMBOLS: i32 = 224;
const QUOTE_BOARD_ONE_ROW_HEIGHT: i32 = 80;
const QUOTE_BOARD_MULTI_ROW_BASE_HEIGHT: i32 = 70;
const QUOTE_BOARD_ROW_HEIGHT_STEP: i32 = 31;
const QUOTE_BOARD_LEGACY_ROW_HEIGHT_LIMIT: i32 = 5;
const QUOTE_BOARD_EXTENDED_ROW_HEIGHT_STEP: i32 = 26;
const LEGACY_QUOTE_BOARD_HEIGHT_WITH_FOOTER: i32 = 232;
pub const DEFAULT_LAYOUT_SCAN_SLOTS: usize = 80;
pub const MIN_VISIBLE_WIDGET_PX: i32 = 8;
pub const MIN_WIDGET_WIDTH: i32 = 12;
pub const MIN_WIDGET_HEIGHT: i32 = 8;
pub const MAX_WIDGET_WIDTH: i32 = 6000;
pub const MAX_WIDGET_HEIGHT: i32 = 4000;
pub const MIN_WIDGET_SCALE_PERCENT: i32 = 10;
pub const MAX_WIDGET_SCALE_PERCENT: i32 = 300;
pub const DEFAULT_WIDGET_SCALE_PERCENT: i32 = 100;
pub const PARKED_WIDGET_X: i32 = -32_000;
pub const PARKED_WIDGET_Y: i32 = -32_000;
pub const MIN_SYMBOLS_PER_WIDGET: usize = 1;
pub const MAX_SYMBOLS_PER_WIDGET: usize = 20;
pub const MINI_TICKER_SYMBOL_LIMIT: usize = 1;
pub const BUILTIN_QUOTE_BOARD_PLUGIN_ID: &str = "builtin.quote-board";
pub const BUILTIN_MINI_TICKER_PLUGIN_ID: &str = "builtin.mini-ticker";
pub const LAYOUT_STATE_FILE_NAME: &str = "layouts.json";
pub const LEGACY_LAYOUT_STATE_FILE_NAME: &str = "poc-layouts.json";

static SAVE_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WidgetKind {
    #[default]
    QuoteBoard,
    MiniTicker,
}

impl WidgetKind {
    pub const fn id_prefix(self) -> &'static str {
        match self {
            Self::QuoteBoard => "quote-board",
            Self::MiniTicker => "mini-ticker",
        }
    }

    pub const fn plugin_id(self) -> &'static str {
        match self {
            Self::QuoteBoard => BUILTIN_QUOTE_BOARD_PLUGIN_ID,
            Self::MiniTicker => BUILTIN_MINI_TICKER_PLUGIN_ID,
        }
    }

    pub fn from_plugin_id(plugin_id: &str) -> Option<Self> {
        match plugin_id {
            BUILTIN_QUOTE_BOARD_PLUGIN_ID => Some(Self::QuoteBoard),
            BUILTIN_MINI_TICKER_PLUGIN_ID => Some(Self::MiniTicker),
            _ => None,
        }
    }

    pub const fn symbol_limit(self) -> usize {
        match self {
            Self::QuoteBoard => MAX_SYMBOLS_PER_WIDGET,
            Self::MiniTicker => MINI_TICKER_SYMBOL_LIMIT,
        }
    }

    pub const fn min_symbol_limit(self) -> usize {
        MIN_SYMBOLS_PER_WIDGET
    }

    pub const fn default_size(self) -> WidgetSize {
        match self {
            Self::QuoteBoard => WidgetSize {
                width: QUOTE_BOARD_WIDTH,
                height: QUOTE_BOARD_HEIGHT,
            },
            Self::MiniTicker => WidgetSize {
                width: MINI_TICKER_WIDTH,
                height: MINI_TICKER_HEIGHT,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidgetSize {
    pub width: i32,
    pub height: i32,
}

impl WidgetSize {
    pub const fn tuple(self) -> (i32, i32) {
        (self.width, self.height)
    }
}

impl From<(i32, i32)> for WidgetSize {
    fn from(value: (i32, i32)) -> Self {
        Self {
            width: value.0,
            height: value.1,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum WidgetSizePolicy {
    #[default]
    Fixed,
    SymbolBlocks {
        block_width: i32,
        block_height: i32,
        padding_width: i32,
        padding_height: i32,
    },
    SymbolGrid {
        cell_width: i32,
        cell_height: i32,
        content_padding_width: i32,
        content_padding_height: i32,
        columns: Option<usize>,
        rows: Option<usize>,
    },
}

impl WidgetSizePolicy {
    fn size_for_symbol_count(
        self,
        default_size: WidgetSize,
        symbol_count: usize,
        min_symbols: usize,
        max_symbols: usize,
    ) -> WidgetSize {
        match self {
            Self::Fixed => default_size,
            Self::SymbolBlocks {
                block_width,
                block_height,
                padding_width,
                padding_height,
            } => {
                let max_symbols = max_symbols.max(1);
                let min_symbols = min_symbols.clamp(1, max_symbols);
                let count = symbol_count.clamp(min_symbols, max_symbols) as i32;
                WidgetSize {
                    width: block_width
                        .saturating_mul(count)
                        .saturating_add(padding_width),
                    height: block_height.saturating_add(padding_height),
                }
            }
            Self::SymbolGrid {
                cell_width,
                cell_height,
                content_padding_width,
                content_padding_height,
                columns,
                rows,
            } => {
                let max_symbols = max_symbols.max(1);
                let min_symbols = min_symbols.clamp(1, max_symbols);
                let count = symbol_count.clamp(min_symbols, max_symbols);
                let (columns, rows) = symbol_grid_tracks(count, columns, rows);
                WidgetSize {
                    width: cell_width
                        .saturating_mul(columns as i32)
                        .saturating_add(content_padding_width),
                    height: cell_height
                        .saturating_mul(rows as i32)
                        .saturating_add(content_padding_height),
                }
            }
        }
    }
}

fn symbol_grid_tracks(
    symbol_count: usize,
    columns: Option<usize>,
    rows: Option<usize>,
) -> (usize, usize) {
    let count = symbol_count.max(1);
    match (columns, rows) {
        (Some(max_columns), Some(max_rows)) => {
            let columns = count.min(max_columns).max(1);
            let rows = count.div_ceil(max_columns).min(max_rows).max(1);
            (columns, rows)
        }
        (Some(max_columns), None) => {
            let columns = count.min(max_columns).max(1);
            let rows = count.div_ceil(max_columns).max(1);
            (columns, rows)
        }
        (None, Some(max_rows)) => {
            let rows = count.min(max_rows).max(1);
            let columns = count.div_ceil(max_rows).max(1);
            (columns, rows)
        }
        (None, None) => (count, 1),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetDefinition {
    pub id: String,
    pub name: String,
    pub default_size: WidgetSize,
    pub size_policy: WidgetSizePolicy,
    pub min_symbol_limit: usize,
    pub symbol_limit: usize,
    pub default_symbols: Vec<String>,
}

impl WidgetDefinition {
    pub fn builtin(widget_type: WidgetKind) -> Self {
        Self {
            id: widget_type.plugin_id().to_string(),
            name: persisted_widget_title(widget_type).to_string(),
            default_size: widget_type.default_size(),
            size_policy: WidgetSizePolicy::Fixed,
            min_symbol_limit: widget_type.min_symbol_limit(),
            symbol_limit: widget_type.symbol_limit(),
            default_symbols: default_symbols_for_type(widget_type),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct WidgetRect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl WidgetRect {
    fn overlaps(self, other: WidgetRect, gap: i32) -> bool {
        self.x < other.x + other.width + gap
            && self.x + self.width + gap > other.x
            && self.y < other.y + other.height + gap
            && self.y + self.height + gap > other.y
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetLayout {
    pub x: i32,
    pub y: i32,
    pub always_on_top: bool,
    pub opacity_percent: i32,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub scale_percent: i32,
    #[serde(default)]
    pub width: i32,
    #[serde(default)]
    pub height: i32,
}

impl Default for WidgetLayout {
    fn default() -> Self {
        Self {
            x: DEFAULT_WIDGET_POSITION_X,
            y: DEFAULT_WIDGET_POSITION_Y,
            always_on_top: default_widgets_always_on_top(),
            opacity_percent: default_opacity_percent(),
            locked: false,
            scale_percent: DEFAULT_WIDGET_SCALE_PERCENT,
            width: QUOTE_BOARD_WIDTH,
            height: QUOTE_BOARD_HEIGHT,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct LayoutStore {
    #[serde(default)]
    pub settings: AppSettings,
    #[serde(default)]
    pub selected_widget_id: Option<String>,
    #[serde(default = "default_next_widget_number")]
    pub next_widget_number: u64,
    #[serde(default)]
    pub widgets: Vec<WidgetInstance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetInstance {
    pub id: String,
    #[serde(default)]
    pub plugin_id: String,
    #[serde(default, rename = "widget_type", skip_serializing)]
    pub legacy_widget_type: Option<WidgetKind>,
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_widget_visible")]
    pub visible: bool,
    #[serde(default)]
    pub layout: WidgetLayout,
    #[serde(default)]
    pub symbols: Vec<String>,
    #[serde(default = "default_widget_config")]
    pub config: serde_json::Value,
}

impl WidgetInstance {
    pub fn widget_type(&self) -> WidgetKind {
        WidgetKind::from_plugin_id(&self.plugin_id).unwrap_or_default()
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct LegacyLayoutStore {
    #[serde(default)]
    pub settings: AppSettings,
    #[serde(default)]
    pub symbols: Vec<String>,
    #[serde(default)]
    pub widgets: HashMap<String, WidgetLayout>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum PersistedLayoutStore {
    Current(LayoutStore),
    Legacy(LegacyLayoutStore),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutPreference {
    #[default]
    AltC,
    CtrlSpace,
    CtrlShiftSpace,
    AltSpace,
    Disabled,
}

impl ShortcutPreference {
    pub const fn from_index(index: i32) -> Self {
        match index {
            1..=4 => Self::Disabled,
            _ => Self::AltC,
        }
    }

    pub const fn index(self) -> i32 {
        match self {
            Self::AltC => 0,
            Self::CtrlSpace | Self::CtrlShiftSpace | Self::AltSpace | Self::Disabled => 1,
        }
    }

    pub const fn normalized(self) -> Self {
        match self {
            Self::AltC | Self::Disabled => self,
            Self::CtrlSpace | Self::CtrlShiftSpace | Self::AltSpace => Self::Disabled,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguagePreference {
    System,
    #[default]
    En,
    ZhHans,
}

impl LanguagePreference {
    pub const fn from_index(index: i32) -> Self {
        match index {
            1 => Self::En,
            2 => Self::ZhHans,
            _ => Self::System,
        }
    }

    pub const fn index(self) -> i32 {
        match self {
            Self::System => 0,
            Self::En => 1,
            Self::ZhHans => 2,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemePreference {
    pub const fn from_index(index: i32) -> Self {
        match index {
            1 => Self::Light,
            2 => Self::Dark,
            _ => Self::System,
        }
    }

    pub const fn index(self) -> i32 {
        match self {
            Self::System => 0,
            Self::Light => 1,
            Self::Dark => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_widgets_always_on_top")]
    pub widgets_always_on_top: bool,
    #[serde(default = "default_opacity_percent")]
    pub opacity_percent: i32,
    #[serde(default = "default_widget_scale_percent")]
    pub widget_scale_percent: i32,
    #[serde(default)]
    pub red_up_enabled: bool,
    #[serde(default)]
    pub market_provider: MarketProviderPreference,
    #[serde(default = "default_market_source_enabled")]
    pub market_binance_enabled: bool,
    #[serde(default = "default_market_source_enabled")]
    pub market_coinbase_enabled: bool,
    #[serde(default = "default_market_source_enabled")]
    pub market_okx_enabled: bool,
    #[serde(default = "default_market_source_enabled")]
    pub market_hyperliquid_enabled: bool,
    #[serde(default = "default_refresh_interval_seconds")]
    pub refresh_interval_seconds: i32,
    #[serde(default = "default_market_symbols")]
    pub market_default_symbols: Vec<String>,
    #[serde(default = "default_market_fallback_enabled")]
    pub market_fallback_enabled: bool,
    #[serde(default)]
    pub auto_start_enabled: bool,
    #[serde(default = "default_show_main_window_on_startup")]
    pub show_main_window_on_startup: bool,
    #[serde(default)]
    pub shortcut: ShortcutPreference,
    #[serde(default)]
    pub theme: ThemePreference,
    #[serde(default)]
    pub language: LanguagePreference,
    #[serde(default = "default_tray_icon_enabled")]
    pub tray_icon_enabled: bool,
    #[serde(default)]
    pub tray_hover_display_enabled: bool,
    #[serde(default)]
    pub network_proxy_enabled: bool,
    #[serde(default)]
    pub network_proxy_url: String,
    #[serde(default)]
    pub alert_rules: Vec<AlertRule>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            widgets_always_on_top: default_widgets_always_on_top(),
            opacity_percent: default_opacity_percent(),
            widget_scale_percent: default_widget_scale_percent(),
            red_up_enabled: false,
            market_provider: MarketProviderPreference::default(),
            market_binance_enabled: default_market_source_enabled(),
            market_coinbase_enabled: default_market_source_enabled(),
            market_okx_enabled: default_market_source_enabled(),
            market_hyperliquid_enabled: default_market_source_enabled(),
            refresh_interval_seconds: default_refresh_interval_seconds(),
            market_default_symbols: default_market_symbols(),
            market_fallback_enabled: default_market_fallback_enabled(),
            auto_start_enabled: false,
            show_main_window_on_startup: default_show_main_window_on_startup(),
            shortcut: ShortcutPreference::default(),
            theme: ThemePreference::default(),
            language: LanguagePreference::default(),
            tray_icon_enabled: default_tray_icon_enabled(),
            tray_hover_display_enabled: false,
            network_proxy_enabled: false,
            network_proxy_url: String::new(),
            alert_rules: Vec::new(),
        }
    }
}

impl AppSettings {
    pub fn normalized(self) -> Self {
        Self {
            widgets_always_on_top: self.widgets_always_on_top,
            opacity_percent: clamp_opacity(self.opacity_percent),
            widget_scale_percent: clamp_default_widget_scale_percent(self.widget_scale_percent),
            red_up_enabled: self.red_up_enabled,
            market_provider: self.market_provider,
            market_binance_enabled: source_enabled_or_default(
                self.market_binance_enabled,
                self.market_coinbase_enabled,
                self.market_okx_enabled,
                self.market_hyperliquid_enabled,
                MarketDataSource::Binance,
            ),
            market_coinbase_enabled: source_enabled_or_default(
                self.market_binance_enabled,
                self.market_coinbase_enabled,
                self.market_okx_enabled,
                self.market_hyperliquid_enabled,
                MarketDataSource::Coinbase,
            ),
            market_okx_enabled: source_enabled_or_default(
                self.market_binance_enabled,
                self.market_coinbase_enabled,
                self.market_okx_enabled,
                self.market_hyperliquid_enabled,
                MarketDataSource::Okx,
            ),
            market_hyperliquid_enabled: source_enabled_or_default(
                self.market_binance_enabled,
                self.market_coinbase_enabled,
                self.market_okx_enabled,
                self.market_hyperliquid_enabled,
                MarketDataSource::Hyperliquid,
            ),
            refresh_interval_seconds: clamp_refresh_interval(self.refresh_interval_seconds),
            market_default_symbols: normalize_market_symbols(self.market_default_symbols),
            market_fallback_enabled: self.market_fallback_enabled,
            auto_start_enabled: self.auto_start_enabled,
            show_main_window_on_startup: self.show_main_window_on_startup,
            shortcut: self.shortcut.normalized(),
            theme: self.theme,
            language: self.language,
            tray_icon_enabled: self.tray_icon_enabled,
            tray_hover_display_enabled: self.tray_hover_display_enabled,
            network_proxy_enabled: self.network_proxy_enabled,
            network_proxy_url: normalize_network_proxy_url(self.network_proxy_url),
            alert_rules: normalize_alert_rules(self.alert_rules),
        }
    }
}

pub fn default_widgets_always_on_top() -> bool {
    true
}

pub fn default_opacity_percent() -> i32 {
    DEFAULT_OPACITY_PERCENT
}

pub fn default_show_main_window_on_startup() -> bool {
    true
}

pub fn clamp_opacity(value: i32) -> i32 {
    value.clamp(MIN_OPACITY_PERCENT, MAX_OPACITY_PERCENT)
}

pub fn default_widget_scale_percent() -> i32 {
    DEFAULT_WIDGET_SCALE_PERCENT
}

pub fn clamp_default_widget_scale_percent(value: i32) -> i32 {
    value.clamp(MIN_WIDGET_SCALE_PERCENT, MAX_WIDGET_SCALE_PERCENT)
}

pub fn default_refresh_interval_seconds() -> i32 {
    DEFAULT_REFRESH_INTERVAL_SECONDS
}

pub fn default_market_source_enabled() -> bool {
    true
}

pub fn default_market_fallback_enabled() -> bool {
    true
}

fn source_enabled_or_default(
    binance_enabled: bool,
    coinbase_enabled: bool,
    okx_enabled: bool,
    hyperliquid_enabled: bool,
    source: MarketDataSource,
) -> bool {
    if binance_enabled || coinbase_enabled || okx_enabled || hyperliquid_enabled {
        match source {
            MarketDataSource::Binance => binance_enabled,
            MarketDataSource::Coinbase => coinbase_enabled,
            MarketDataSource::Okx => okx_enabled,
            MarketDataSource::Hyperliquid => hyperliquid_enabled,
        }
    } else {
        source == MarketDataSource::Binance
    }
}

pub fn enabled_market_sources(settings: &AppSettings) -> Vec<MarketDataSource> {
    let settings = settings.clone().normalized();
    let mut sources = Vec::new();
    if settings.market_binance_enabled {
        sources.push(MarketDataSource::Binance);
    }
    if settings.market_coinbase_enabled {
        sources.push(MarketDataSource::Coinbase);
    }
    if settings.market_okx_enabled {
        sources.push(MarketDataSource::Okx);
    }
    if settings.market_hyperliquid_enabled {
        sources.push(MarketDataSource::Hyperliquid);
    }
    if sources.is_empty() {
        default_enabled_market_sources()
    } else {
        sources
    }
}

pub fn default_tray_icon_enabled() -> bool {
    true
}

pub fn normalize_network_proxy_url(proxy_url: String) -> String {
    proxy_url.trim().to_string()
}

pub fn effective_network_proxy_url(settings: &AppSettings) -> Option<String> {
    let proxy_url = settings.network_proxy_url.trim();
    if settings.network_proxy_enabled && !proxy_url.is_empty() {
        Some(proxy_url.to_string())
    } else {
        None
    }
}

pub fn default_next_widget_number() -> u64 {
    DEFAULT_NEXT_WIDGET_NUMBER
}

pub fn default_widget_visible() -> bool {
    true
}

pub fn default_widget_config() -> serde_json::Value {
    Value::Object(Map::new())
}

pub fn widget_show_coin_logos(instance: &WidgetInstance) -> bool {
    widget_config_show_coin_logos(&instance.config)
}

pub fn widget_hide_quote_asset(instance: &WidgetInstance) -> bool {
    widget_config_hide_quote_asset(&instance.config)
}

pub fn widget_theme_preference(instance: &WidgetInstance) -> String {
    widget_config_string(&instance.config, WIDGET_CONFIG_THEME, WIDGET_THEME_SYSTEM)
}

pub fn set_widget_theme_preference(instance: &mut WidgetInstance, theme: &str) {
    let theme = normalized_widget_theme_preference(theme);
    widget_config_object_mut(instance)
        .insert(WIDGET_CONFIG_THEME.to_string(), Value::String(theme));
}

pub fn set_widget_display_config(
    instance: &mut WidgetInstance,
    show_coin_logos: bool,
    hide_quote_asset: bool,
) {
    let config = widget_config_object_mut(instance);
    config.insert(
        WIDGET_CONFIG_SHOW_COIN_LOGOS.to_string(),
        Value::Bool(show_coin_logos),
    );
    config.insert(
        WIDGET_CONFIG_HIDE_QUOTE_ASSET.to_string(),
        Value::Bool(hide_quote_asset),
    );
}

fn widget_config_show_coin_logos(config: &serde_json::Value) -> bool {
    widget_config_bool(config, WIDGET_CONFIG_SHOW_COIN_LOGOS, true)
}

fn widget_config_hide_quote_asset(config: &serde_json::Value) -> bool {
    widget_config_bool(config, WIDGET_CONFIG_HIDE_QUOTE_ASSET, false)
}

fn widget_config_bool(config: &serde_json::Value, key: &str, default: bool) -> bool {
    config
        .as_object()
        .and_then(|config| config.get(key))
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

fn widget_config_string(config: &serde_json::Value, key: &str, default: &str) -> String {
    config
        .as_object()
        .and_then(|config| config.get(key))
        .and_then(Value::as_str)
        .map(normalized_widget_theme_preference)
        .unwrap_or_else(|| default.to_string())
}

fn normalized_widget_theme_preference(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        WIDGET_THEME_SYSTEM.to_string()
    } else {
        value.chars().take(64).collect()
    }
}

fn widget_config_object_mut(instance: &mut WidgetInstance) -> &mut Map<String, Value> {
    if !instance.config.is_object() {
        instance.config = default_widget_config();
    }
    instance
        .config
        .as_object_mut()
        .expect("widget config should be an object")
}

pub fn normalize_alert_rules(rules: Vec<AlertRule>) -> Vec<AlertRule> {
    let mut seen_ids = HashSet::new();
    rules
        .into_iter()
        .enumerate()
        .filter_map(|(index, rule)| normalize_alert_rule(rule, index, &mut seen_ids))
        .collect()
}

fn normalize_alert_rule(
    rule: AlertRule,
    index: usize,
    seen_ids: &mut HashSet<String>,
) -> Option<AlertRule> {
    if !rule.threshold.is_finite() {
        return None;
    }
    let symbol = normalize_market_pair_key(&rule.symbol)?;
    let mut id = rule.id.trim().to_string();
    if id.is_empty() {
        id = format!("alert-{}", index + 1);
    }
    if seen_ids.contains(&id) {
        let base = id;
        let mut suffix = 2;
        loop {
            id = format!("{base}-{suffix}");
            if !seen_ids.contains(&id) {
                break;
            }
            suffix += 1;
        }
    }
    seen_ids.insert(id.clone());

    Some(AlertRule {
        id,
        symbol,
        condition: rule.condition,
        threshold: rule.threshold,
        enabled: rule.enabled,
    })
}

pub fn save_layout_store(path: &Path, store: &LayoutStore) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let contents = format!("{}\n", serde_json::to_string_pretty(store)?);
    let temp_path = temporary_layout_store_path(path);
    let result = write_layout_store_temp_file(&temp_path, contents.as_bytes())
        .and_then(|()| replace_file(&temp_path, path));
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn write_layout_store_temp_file(path: &Path, contents: &[u8]) -> Result<()> {
    let mut file =
        File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    file.write_all(contents)
        .with_context(|| format!("failed to write {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("failed to sync {}", path.display()))
}

fn temporary_layout_store_path(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .unwrap_or_else(|| OsStr::new("layout-store"));
    let counter = SAVE_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut temp_name = OsString::from(".");
    temp_name.push(file_name);
    temp_name.push(format!(".{}.{}.tmp", std::process::id(), counter));
    parent.join(temp_name)
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> Result<()> {
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source = wide_null(source.as_os_str());
    let destination = wide_null(destination.as_os_str());
    let replaced = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if replaced == 0 {
        return Err(std::io::Error::last_os_error()).context("failed to replace layout store file");
    }
    Ok(())
}

#[cfg(windows)]
fn wide_null(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> Result<()> {
    fs::rename(source, destination)
        .with_context(|| format!("failed to replace {}", destination.display()))
}

pub fn load_persisted_layout_store(path: &Path) -> Option<PersistedLayoutStore> {
    fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str::<PersistedLayoutStore>(&contents).ok())
}

pub fn state_path() -> Result<PathBuf> {
    if let Some(dir) =
        env_os_with_legacy("CRYPTO_HUD_STATE_DIR", &["CRYPTO_WIDGET_SLINT_STATE_DIR"])
    {
        return Ok(PathBuf::from(dir).join(LAYOUT_STATE_FILE_NAME));
    }

    let Some(project_dirs) = directories::ProjectDirs::from("com", "cryptohud", "CryptoHud") else {
        return Ok(PathBuf::from(".crypto-hud-state").join(LAYOUT_STATE_FILE_NAME));
    };

    Ok(project_dirs.config_dir().join(LAYOUT_STATE_FILE_NAME))
}

fn env_os_with_legacy(primary: &str, legacy: &[&str]) -> Option<OsString> {
    env::var_os(primary).or_else(|| legacy.iter().find_map(env::var_os))
}

pub fn state_dir_for_path(path: &Path) -> PathBuf {
    path.parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn load_layout_store(
    path: &Path,
    requested_widget_count: usize,
    catalog: &[WidgetDefinition],
    desktop_size: (i32, i32),
) -> LayoutStore {
    let mut migrated_from_legacy_state = false;
    let persisted = load_persisted_layout_store(path).or_else(|| {
        for legacy_path in legacy_layout_store_paths(path) {
            if legacy_path.as_path() == path {
                continue;
            }
            if let Some(persisted) = load_persisted_layout_store(&legacy_path) {
                migrated_from_legacy_state = true;
                return Some(persisted);
            }
        }
        None
    });

    let mut store = persisted
        .map(|persisted| match persisted {
            PersistedLayoutStore::Current(store) => store,
            PersistedLayoutStore::Legacy(store) => migrate_legacy_store(store, desktop_size),
        })
        .unwrap_or_default();
    normalize_store_with_catalog(&mut store, requested_widget_count, catalog, desktop_size);
    if migrated_from_legacy_state {
        if let Err(error) = save_layout_store(path, &store) {
            eprintln!(
                "failed to save migrated layout state to {}: {error:#}",
                path.display()
            );
        }
    }
    store
}

fn legacy_layout_store_paths(path: &Path) -> Vec<PathBuf> {
    let mut paths = vec![path.with_file_name(LEGACY_LAYOUT_STATE_FILE_NAME)];
    // Previous alpha builds used the old organization qualifier before the
    // public project name settled on Crypto HUD.
    if let Some(project_dirs) = directories::ProjectDirs::from("com", "cryptowidget", "CryptoHud") {
        paths.push(project_dirs.config_dir().join(LAYOUT_STATE_FILE_NAME));
        paths.push(
            project_dirs
                .config_dir()
                .join(LEGACY_LAYOUT_STATE_FILE_NAME),
        );
    }
    // Earliest prototype builds used the SlintPoc application name and the old
    // poc-layouts.json state file.
    if let Some(project_dirs) = directories::ProjectDirs::from("com", "cryptowidget", "SlintPoc") {
        paths.push(
            project_dirs
                .config_dir()
                .join(LEGACY_LAYOUT_STATE_FILE_NAME),
        );
    }
    paths
}

pub fn migrate_legacy_store(legacy: LegacyLayoutStore, desktop_size: (i32, i32)) -> LayoutStore {
    let settings = legacy.settings.clone().normalized();
    let symbols = normalized_symbols_for_type(WidgetKind::QuoteBoard, legacy.symbols);
    let mut widgets = legacy.widgets.into_iter().collect::<Vec<_>>();
    widgets.sort_by(|left, right| left.0.cmp(&right.0));

    let mut store = LayoutStore {
        settings,
        selected_widget_id: None,
        next_widget_number: DEFAULT_NEXT_WIDGET_NUMBER,
        widgets: widgets
            .into_iter()
            .enumerate()
            .map(|(index, (id, layout))| WidgetInstance {
                id,
                plugin_id: WidgetKind::QuoteBoard.plugin_id().to_string(),
                legacy_widget_type: None,
                name: default_persisted_widget_name(WidgetKind::QuoteBoard, index as u64 + 1),
                visible: true,
                layout,
                symbols: symbols.clone(),
                config: default_widget_config(),
            })
            .collect(),
    };
    normalize_store_with_catalog(&mut store, 0, &[], desktop_size);
    store
}

pub fn normalize_store(store: &mut LayoutStore, requested_widget_count: usize) {
    normalize_store_with_catalog(store, requested_widget_count, &[], (1920, 1080));
}

pub fn normalize_store_with_catalog(
    store: &mut LayoutStore,
    requested_widget_count: usize,
    catalog: &[WidgetDefinition],
    desktop_size: (i32, i32),
) {
    store.settings = store.settings.clone().normalized();
    store.next_widget_number = store.next_widget_number.max(DEFAULT_NEXT_WIDGET_NUMBER);

    if store.widgets.is_empty() && requested_widget_count > 0 {
        let settings = store.settings.clone();
        for index in 0..requested_widget_count {
            add_widget_instance(store, WidgetKind::QuoteBoard, &settings, desktop_size);
            if requested_widget_count > 1 {
                if let Some(instance) = store.widgets.last_mut() {
                    instance.symbols =
                        initial_widget_symbols_for_slot(WidgetKind::QuoteBoard, &settings, index);
                    let size = widget_size_from_scale_percent(
                        default_widget_size_for_instance(instance, catalog),
                        settings.widget_scale_percent,
                    );
                    instance.layout.scale_percent = settings.widget_scale_percent;
                    instance.layout.width = size.width;
                    instance.layout.height = size.height;
                }
            }
        }
        store.selected_widget_id = store.widgets.first().map(|widget| widget.id.clone());
    }

    let should_migrate_legacy_default_cascade = store.widgets.len() > 1
        && store
            .widgets
            .iter()
            .enumerate()
            .any(|(index, instance)| is_legacy_default_layout(&instance.layout, index));

    let mut seen_ids = std::collections::HashSet::new();
    let mut max_suffix = 0;
    for (index, instance) in store.widgets.iter_mut().enumerate() {
        if let Some(legacy_widget_type) = instance.legacy_widget_type.take() {
            instance.plugin_id = legacy_widget_type.plugin_id().to_string();
        }
        if instance.plugin_id.trim().is_empty() {
            instance.plugin_id = WidgetKind::QuoteBoard.plugin_id().to_string();
        }
        if instance.config.is_null() {
            instance.config = default_widget_config();
        }

        if instance.id.trim().is_empty() || seen_ids.contains(&instance.id) {
            let number = store.next_widget_number;
            store.next_widget_number += 1;
            instance.id = format!("{}-{number}", instance.widget_type().id_prefix());
        }
        seen_ids.insert(instance.id.clone());
        max_suffix = max_suffix.max(widget_id_suffix(&instance.id).unwrap_or(0));

        if instance.name.trim().is_empty() {
            instance.name = default_persisted_widget_name(instance.widget_type(), index as u64 + 1);
        }
        instance.symbols = normalized_symbols_for_instance(instance, catalog);
        let default_size = default_widget_size_for_instance(instance, catalog);
        let has_missing_scale_with_persisted_size =
            instance.layout.scale_percent <= 0 && layout_has_persisted_size(&instance.layout);
        let defer_quote_board_size_migration = has_missing_scale_with_persisted_size
            && instance.widget_type() == WidgetKind::QuoteBoard;
        let defer_dynamic_size_migration = has_missing_scale_with_persisted_size
            && definition_for_instance(instance, catalog)
                .map(|definition| definition.size_policy != WidgetSizePolicy::Fixed)
                .unwrap_or(false);
        normalize_layout_size_and_scale(
            &mut instance.layout,
            default_size,
            store.settings.widget_scale_percent,
            defer_quote_board_size_migration || defer_dynamic_size_migration,
        );
        migrate_legacy_quote_board_footer_size(instance);
        migrate_quote_board_display_size(instance);
        if defer_quote_board_size_migration && instance.layout.scale_percent <= 0 {
            let default_size = default_widget_size_for_instance(instance, catalog);
            normalize_layout_size_and_scale(
                &mut instance.layout,
                default_size,
                store.settings.widget_scale_percent,
                false,
            );
        }
        let instance_size = widget_size_for_instance(instance, catalog);
        if should_migrate_legacy_default_cascade
            && is_legacy_default_layout(&instance.layout, index)
        {
            let layout =
                default_layout_for_size(index, instance_size, store.settings.clone(), desktop_size);
            instance.layout.x = layout.x;
            instance.layout.y = layout.y;
        }
        if layout_needs_recovery_for_size(&instance.layout, instance_size, desktop_size) {
            let layout =
                default_layout_for_size(index, instance_size, store.settings.clone(), desktop_size);
            instance.layout.x = layout.x;
            instance.layout.y = layout.y;
        }
        instance.layout.opacity_percent = clamp_opacity(instance.layout.opacity_percent);
    }

    if store.next_widget_number <= max_suffix {
        store.next_widget_number = max_suffix + 1;
    }

    let selected_is_valid = store
        .selected_widget_id
        .as_deref()
        .map(|id| store.widgets.iter().any(|widget| widget.id == id))
        .unwrap_or(false);
    if !selected_is_valid {
        store.selected_widget_id = store.widgets.first().map(|widget| widget.id.clone());
    }
}

pub fn add_widget_instance(
    store: &mut LayoutStore,
    widget_type: WidgetKind,
    settings: &AppSettings,
    desktop_size: (i32, i32),
) -> String {
    let number = next_widget_number(store, widget_type);
    let id = format!("{}-{number}", widget_type.id_prefix());
    let symbols = default_symbols_for_type_from_settings(widget_type, settings);
    let config = default_widget_config();
    let size = widget_size_from_scale_percent(
        default_widget_size_for_parts(
            Some(widget_type),
            widget_type.default_size(),
            &symbols,
            &config,
        ),
        settings.widget_scale_percent,
    );
    let instance = WidgetInstance {
        id: id.clone(),
        plugin_id: widget_type.plugin_id().to_string(),
        legacy_widget_type: None,
        name: default_persisted_widget_name(widget_type, number),
        visible: true,
        layout: next_available_layout_for_size(store, size, settings.clone(), desktop_size),
        symbols,
        config,
    };
    store.selected_widget_id = Some(id.clone());
    store.widgets.push(instance);
    id
}

fn initial_widget_symbols_for_slot(
    widget_type: WidgetKind,
    settings: &AppSettings,
    slot: usize,
) -> Vec<String> {
    let defaults = default_symbols_for_type_from_settings(widget_type, settings);
    defaults
        .get(slot % defaults.len().max(1))
        .cloned()
        .map(|symbol| vec![symbol])
        .unwrap_or_else(|| {
            default_symbols_for_type(widget_type)
                .into_iter()
                .take(1)
                .collect()
        })
}

pub fn add_plugin_instance(
    store: &mut LayoutStore,
    plugin: &WidgetDefinition,
    settings: &AppSettings,
    desktop_size: (i32, i32),
) -> String {
    if let Some(widget_type) = WidgetKind::from_plugin_id(&plugin.id) {
        return add_widget_instance(store, widget_type, settings, desktop_size);
    }

    let prefix = plugin_instance_id_prefix(&plugin.id);
    let number = next_instance_number(store, &prefix);
    let id = format!("{prefix}-{number}");
    let symbols = default_symbols_for_definition_from_settings(plugin, settings);
    let size = widget_size_from_scale_percent(
        widget_definition_size_for_symbols(plugin, &symbols),
        settings.widget_scale_percent,
    );
    let instance = WidgetInstance {
        id: id.clone(),
        plugin_id: plugin.id.clone(),
        legacy_widget_type: None,
        name: format!("{} {number}", plugin.name),
        visible: true,
        layout: next_available_layout_for_size(store, size, settings.clone(), desktop_size),
        symbols,
        config: default_widget_config(),
    };
    store.selected_widget_id = Some(id.clone());
    store.widgets.push(instance);
    id
}

pub fn layout_for_instance(
    instance: &WidgetInstance,
    index: usize,
    settings: AppSettings,
    catalog: &[WidgetDefinition],
    desktop_size: (i32, i32),
) -> WidgetLayout {
    let size = widget_size_for_instance(instance, catalog);
    if layout_needs_recovery_for_size(&instance.layout, size, desktop_size) {
        let mut layout = default_layout_for_size(index, size, settings.clone(), desktop_size);
        layout.scale_percent =
            widget_scale_percent_for_instance(instance, catalog, settings.widget_scale_percent);
        layout
    } else {
        let mut layout = instance.layout.clone();
        layout.opacity_percent = clamp_opacity(layout.opacity_percent);
        layout.width = size.width;
        layout.height = size.height;
        if layout.scale_percent <= 0 {
            layout.scale_percent =
                widget_scale_percent_for_instance(instance, catalog, settings.widget_scale_percent);
        }
        layout
    }
}

pub fn default_layout_for_index(
    index: usize,
    settings: AppSettings,
    desktop_size: (i32, i32),
) -> WidgetLayout {
    default_layout_for_widget(index, WidgetKind::QuoteBoard, settings, desktop_size)
}

pub fn default_layout_for_widget(
    slot: usize,
    widget_type: WidgetKind,
    settings: AppSettings,
    desktop_size: (i32, i32),
) -> WidgetLayout {
    let size =
        widget_size_from_scale_percent(widget_type.default_size(), settings.widget_scale_percent);
    default_layout_for_size(slot, size, settings, desktop_size)
}

pub fn default_layout_for_size(
    slot: usize,
    size: WidgetSize,
    settings: AppSettings,
    desktop_size: (i32, i32),
) -> WidgetLayout {
    let size = clamp_widget_size(size);
    let (desktop_width, desktop_height) = desktop_size;
    let row_stride = QUOTE_BOARD_HEIGHT + DEFAULT_LAYOUT_GAP;
    let usable_height = (desktop_height - DEFAULT_LAYOUT_MARGIN_Y * 2).max(row_stride);
    let rows_per_column = (usable_height / row_stride).max(1) as usize;
    let row = slot % rows_per_column;
    let column = slot / rows_per_column;
    let column_stride = QUOTE_BOARD_WIDTH + DEFAULT_LAYOUT_GAP;
    let x = (desktop_width - DEFAULT_LAYOUT_MARGIN_X - size.width - column_stride * column as i32)
        .max(DEFAULT_LAYOUT_MARGIN_X);
    let y = DEFAULT_LAYOUT_MARGIN_Y + row_stride * row as i32;

    WidgetLayout {
        x,
        y,
        always_on_top: settings.widgets_always_on_top,
        opacity_percent: settings.opacity_percent,
        locked: false,
        scale_percent: clamp_default_widget_scale_percent(settings.widget_scale_percent),
        width: size.width,
        height: size.height,
    }
}

pub fn next_available_layout(
    store: &LayoutStore,
    widget_type: WidgetKind,
    settings: AppSettings,
    desktop_size: (i32, i32),
) -> WidgetLayout {
    let size =
        widget_size_from_scale_percent(widget_type.default_size(), settings.widget_scale_percent);
    next_available_layout_for_size(store, size, settings, desktop_size)
}

pub fn next_available_layout_for_size(
    store: &LayoutStore,
    size: WidgetSize,
    settings: AppSettings,
    desktop_size: (i32, i32),
) -> WidgetLayout {
    next_available_layout_for_size_with_catalog(store, size, settings, &[], desktop_size)
}

pub fn next_available_layout_for_size_with_catalog(
    store: &LayoutStore,
    size: WidgetSize,
    settings: AppSettings,
    catalog: &[WidgetDefinition],
    desktop_size: (i32, i32),
) -> WidgetLayout {
    for slot in 0..DEFAULT_LAYOUT_SCAN_SLOTS {
        let candidate = default_layout_for_size(slot, size, settings.clone(), desktop_size);
        if !layout_overlaps_existing_for_size(
            store,
            &candidate,
            size,
            settings.clone(),
            catalog,
            desktop_size,
        ) {
            return candidate;
        }
    }

    default_layout_for_size(store.widgets.len(), size, settings, desktop_size)
}

pub fn layout_has_visible_area_for_size(
    layout: &WidgetLayout,
    size: WidgetSize,
    desktop_size: (i32, i32),
) -> bool {
    let (desktop_width, desktop_height) = desktop_size;
    let visible_left = layout.x.max(0);
    let visible_top = layout.y.max(0);
    let visible_right = (layout.x + size.width).min(desktop_width);
    let visible_bottom = (layout.y + size.height).min(desktop_height);

    visible_right - visible_left >= MIN_VISIBLE_WIDGET_PX
        && visible_bottom - visible_top >= MIN_VISIBLE_WIDGET_PX
}

pub fn clamp_widget_size(size: WidgetSize) -> WidgetSize {
    WidgetSize {
        width: size.width.clamp(MIN_WIDGET_WIDTH, MAX_WIDGET_WIDTH),
        height: size.height.clamp(MIN_WIDGET_HEIGHT, MAX_WIDGET_HEIGHT),
    }
}

pub fn widget_scale_percent_bounds(default_size: WidgetSize) -> (i32, i32) {
    let width = default_size.width.max(1);
    let height = default_size.height.max(1);
    let min_width_scale = div_ceil_i32(MIN_WIDGET_WIDTH * 100, width);
    let min_height_scale = div_ceil_i32(MIN_WIDGET_HEIGHT * 100, height);
    let max_width_scale = (MAX_WIDGET_WIDTH * 100) / width;
    let max_height_scale = (MAX_WIDGET_HEIGHT * 100) / height;
    let min_scale = MIN_WIDGET_SCALE_PERCENT
        .max(min_width_scale)
        .max(min_height_scale);
    let max_scale = MAX_WIDGET_SCALE_PERCENT
        .min(max_width_scale)
        .min(max_height_scale)
        .max(min_scale);
    (min_scale, max_scale)
}

pub fn widget_size_from_scale_percent(default_size: WidgetSize, scale_percent: i32) -> WidgetSize {
    let (min_scale, max_scale) = widget_scale_percent_bounds(default_size);
    let scale_percent = scale_percent.clamp(min_scale, max_scale);
    clamp_widget_size(WidgetSize {
        width: scale_dimension(default_size.width, scale_percent),
        height: scale_dimension(default_size.height, scale_percent),
    })
}

pub fn widget_layout_scale_percent_for_size(
    layout: &WidgetLayout,
    default_size: WidgetSize,
    fallback_scale_percent: i32,
) -> i32 {
    if layout.scale_percent > 0 {
        let (min_scale, max_scale) = widget_scale_percent_bounds(default_size);
        return layout.scale_percent.clamp(min_scale, max_scale);
    }

    let has_persisted_size = layout.width > 0 || layout.height > 0;
    if has_persisted_size {
        let size = clamp_widget_size(WidgetSize {
            width: if layout.width > 0 {
                layout.width
            } else {
                default_size.width
            },
            height: if layout.height > 0 {
                layout.height
            } else {
                default_size.height
            },
        });
        return widget_content_scale_percent_for_size(size, default_size);
    }

    let (min_scale, max_scale) = widget_scale_percent_bounds(default_size);
    fallback_scale_percent.clamp(min_scale, max_scale)
}

pub fn widget_scale_percent_for_instance(
    instance: &WidgetInstance,
    catalog: &[WidgetDefinition],
    fallback_scale_percent: i32,
) -> i32 {
    widget_layout_scale_percent_for_size(
        &instance.layout,
        default_widget_size_for_instance(instance, catalog),
        fallback_scale_percent,
    )
}

pub fn resize_widget_to_content(
    instance: &mut WidgetInstance,
    catalog: &[WidgetDefinition],
    scale_percent: i32,
) {
    let default_size = default_widget_size_for_instance(instance, catalog);
    let (min_scale, max_scale) = widget_scale_percent_bounds(default_size);
    let scale_percent = scale_percent.clamp(min_scale, max_scale);
    let size = widget_size_from_scale_percent(default_size, scale_percent);
    instance.layout.scale_percent = scale_percent;
    instance.layout.width = size.width;
    instance.layout.height = size.height;
}

pub fn resize_widget_to_current_content(
    instance: &mut WidgetInstance,
    catalog: &[WidgetDefinition],
    fallback_scale_percent: i32,
) {
    let scale_percent =
        widget_scale_percent_for_instance(instance, catalog, fallback_scale_percent);
    resize_widget_to_content(instance, catalog, scale_percent);
}

pub fn widget_scale_percent_for_size(size: WidgetSize, default_size: WidgetSize) -> i32 {
    let width = default_size.width.max(1);
    let height = default_size.height.max(1);
    let width_scale = div_round_i32(size.width.max(1) * 100, width);
    let height_scale = div_round_i32(size.height.max(1) * 100, height);
    let scale = div_round_i32(width_scale + height_scale, 2);
    let (min_scale, max_scale) = widget_scale_percent_bounds(default_size);
    scale.clamp(min_scale, max_scale)
}

pub fn widget_content_scale_percent_for_size(size: WidgetSize, default_size: WidgetSize) -> i32 {
    let width = default_size.width.max(1);
    let height = default_size.height.max(1);
    let width_scale = div_round_i32(size.width.max(1) * 100, width);
    let height_scale = div_round_i32(size.height.max(1) * 100, height);
    let (min_scale, max_scale) = widget_scale_percent_bounds(default_size);
    width_scale.min(height_scale).clamp(min_scale, max_scale)
}

pub fn layout_needs_recovery_for_size(
    layout: &WidgetLayout,
    size: WidgetSize,
    desktop_size: (i32, i32),
) -> bool {
    is_parked_position(layout.x, layout.y)
        || !layout_has_visible_area_for_size(layout, size, desktop_size)
}

pub fn is_parked_position(x: i32, y: i32) -> bool {
    x <= -10_000 || y <= -10_000
}

pub fn is_legacy_default_layout(layout: &WidgetLayout, index: usize) -> bool {
    layout.x == DEFAULT_WIDGET_POSITION_X + LEGACY_DEFAULT_POSITION_STEP * index as i32
        && layout.y == DEFAULT_WIDGET_POSITION_Y + LEGACY_DEFAULT_POSITION_STEP * index as i32
}

pub fn select_widget_by_index(store: &mut LayoutStore, selected_index: i32) {
    let index = selected_index.max(0) as usize;
    if let Some(widget) = store.widgets.get(index) {
        store.selected_widget_id = Some(widget.id.clone());
    }
}

pub fn move_widget_in_store(
    store: &mut LayoutStore,
    selected_index: i32,
    direction: i32,
) -> Option<usize> {
    let len = store.widgets.len();
    if len <= 1 || direction == 0 {
        return None;
    }

    let index = (selected_index.max(0) as usize).min(len - 1);
    let next_index = if direction < 0 {
        index.saturating_sub(1)
    } else {
        (index + 1).min(len - 1)
    };

    if index == next_index {
        return None;
    }

    let instance = store.widgets.remove(index);
    let selected_id = instance.id.clone();
    store.widgets.insert(next_index, instance);
    store.selected_widget_id = Some(selected_id);
    Some(next_index)
}

pub fn remove_widget_from_store_by_id(
    store: &mut LayoutStore,
    widget_id: &str,
    desktop_size: (i32, i32),
) -> bool {
    let Some(index) = store
        .widgets
        .iter()
        .position(|widget| widget.id == widget_id)
    else {
        return false;
    };

    store.widgets.remove(index);
    let next_index = index
        .saturating_sub(1)
        .min(store.widgets.len().saturating_sub(1));
    store.selected_widget_id = store
        .widgets
        .get(next_index)
        .map(|widget| widget.id.clone());
    normalize_store_with_catalog(store, 0, &[], desktop_size);
    true
}

pub fn parse_symbols_for_type(input: &str, widget_type: WidgetKind) -> Vec<String> {
    parse_symbols_with_limit(
        input,
        widget_type.symbol_limit(),
        default_symbols_for_type(widget_type),
    )
}

pub fn parse_symbols_for_instance(
    input: &str,
    instance: &WidgetInstance,
    catalog: &[WidgetDefinition],
    settings: &AppSettings,
) -> Vec<String> {
    parse_symbols_with_limit(
        input,
        symbol_limit_for_instance(instance, catalog),
        default_symbols_for_instance_from_settings(instance, catalog, settings),
    )
}

pub fn parse_symbols_with_limit(input: &str, limit: usize, fallback: Vec<String>) -> Vec<String> {
    let symbols = input
        .split([',', ';', ' ', '\n', '\t'])
        .filter_map(crypto_hud_core::normalize_market_pair_key)
        .fold(Vec::new(), |mut symbols, symbol| {
            if symbols.len() < limit && !symbols.contains(&symbol) {
                symbols.push(symbol);
            }
            symbols
        });

    if symbols.is_empty() {
        fallback
    } else {
        symbols
    }
}

pub fn normalized_symbols(symbols: Vec<String>) -> Vec<String> {
    normalized_symbols_for_type(WidgetKind::QuoteBoard, symbols)
}

pub fn normalized_symbols_for_type(widget_type: WidgetKind, symbols: Vec<String>) -> Vec<String> {
    normalized_symbols_with_bounds(
        symbols,
        widget_type.min_symbol_limit(),
        widget_type.symbol_limit(),
        default_symbols_for_type(widget_type),
    )
}

pub fn normalized_symbols_for_instance(
    instance: &WidgetInstance,
    catalog: &[WidgetDefinition],
) -> Vec<String> {
    normalized_symbols_with_bounds(
        instance.symbols.clone(),
        symbol_min_for_instance(instance, catalog),
        symbol_limit_for_instance(instance, catalog),
        default_symbols_for_instance(instance, catalog),
    )
}

pub fn normalized_symbols_with_limit(
    symbols: Vec<String>,
    limit: usize,
    fallback: Vec<String>,
) -> Vec<String> {
    normalized_symbols_with_bounds(symbols, MIN_SYMBOLS_PER_WIDGET, limit, fallback)
}

pub fn normalized_symbols_with_bounds(
    symbols: Vec<String>,
    min: usize,
    limit: usize,
    fallback: Vec<String>,
) -> Vec<String> {
    let limit = limit.clamp(MIN_SYMBOLS_PER_WIDGET, MAX_SYMBOLS_PER_WIDGET);
    let min = min.clamp(MIN_SYMBOLS_PER_WIDGET, limit);
    let mut normalized = symbols
        .iter()
        .filter_map(|symbol| crypto_hud_core::normalize_market_pair_key(symbol))
        .fold(Vec::new(), |mut symbols, symbol| {
            if symbols.len() < limit && !symbols.contains(&symbol) {
                symbols.push(symbol);
            }
            symbols
        });

    for symbol in fallback
        .into_iter()
        .chain(default_market_symbols())
        .filter_map(|symbol| crypto_hud_core::normalize_market_pair_key(&symbol))
    {
        if normalized.len() >= min {
            break;
        }
        if normalized.len() < limit && !normalized.contains(&symbol) {
            normalized.push(symbol);
        }
    }

    normalized
}

pub fn default_symbols_for_type(widget_type: WidgetKind) -> Vec<String> {
    default_symbols_with_limit(widget_type.symbol_limit())
}

pub fn default_symbols_for_type_from_settings(
    widget_type: WidgetKind,
    settings: &AppSettings,
) -> Vec<String> {
    normalized_symbols_with_bounds(
        settings.market_default_symbols.clone(),
        widget_type.min_symbol_limit(),
        widget_type.symbol_limit(),
        default_symbols_for_type(widget_type),
    )
}

pub fn default_symbols_for_definition_from_settings(
    definition: &WidgetDefinition,
    settings: &AppSettings,
) -> Vec<String> {
    let fallback = default_symbols_for_definition(definition);
    if is_custom_market_default_symbols(&settings.market_default_symbols) {
        return normalized_symbols_with_bounds(
            settings.market_default_symbols.clone(),
            definition.min_symbol_limit,
            definition.symbol_limit,
            fallback,
        );
    }

    fallback
}

fn is_custom_market_default_symbols(symbols: &[String]) -> bool {
    let normalized = normalize_market_symbols(symbols.to_vec());
    normalized != default_market_symbols()
}

pub fn default_symbols_for_definition(definition: &WidgetDefinition) -> Vec<String> {
    let fallback = default_symbols_with_limit(definition.symbol_limit);
    if definition.default_symbols.is_empty() {
        return fallback;
    }

    normalized_symbols_with_bounds(
        definition.default_symbols.clone(),
        definition.min_symbol_limit,
        definition.symbol_limit,
        fallback,
    )
}

pub fn default_symbols_for_instance_from_settings(
    instance: &WidgetInstance,
    catalog: &[WidgetDefinition],
    settings: &AppSettings,
) -> Vec<String> {
    if let Some(definition) = definition_for_instance(instance, catalog) {
        default_symbols_for_definition_from_settings(definition, settings)
    } else {
        default_symbols_for_type_from_settings(instance.widget_type(), settings)
    }
}

pub fn default_symbols_for_instance(
    instance: &WidgetInstance,
    catalog: &[WidgetDefinition],
) -> Vec<String> {
    if let Some(definition) = definition_for_instance(instance, catalog) {
        default_symbols_for_definition(definition)
    } else {
        default_symbols_for_type(instance.widget_type())
    }
}

pub fn default_symbols_with_limit(limit: usize) -> Vec<String> {
    default_market_symbols().into_iter().take(limit).collect()
}

pub fn symbol_min_for_instance(instance: &WidgetInstance, catalog: &[WidgetDefinition]) -> usize {
    definition_for_instance(instance, catalog)
        .map(|definition| definition.min_symbol_limit)
        .unwrap_or_else(|| instance.widget_type().min_symbol_limit())
}

pub fn symbol_limit_for_instance(instance: &WidgetInstance, catalog: &[WidgetDefinition]) -> usize {
    definition_for_instance(instance, catalog)
        .map(|definition| definition.symbol_limit)
        .unwrap_or_else(|| instance.widget_type().symbol_limit())
}

pub fn widget_size_for_instance(
    instance: &WidgetInstance,
    catalog: &[WidgetDefinition],
) -> WidgetSize {
    let default_size = default_widget_size_for_instance(instance, catalog);
    if instance.layout.scale_percent > 0 {
        return widget_size_from_scale_percent(default_size, instance.layout.scale_percent);
    }
    clamp_widget_size(WidgetSize {
        width: if instance.layout.width > 0 {
            instance.layout.width
        } else {
            default_size.width
        },
        height: if instance.layout.height > 0 {
            instance.layout.height
        } else {
            default_size.height
        },
    })
}

pub fn default_widget_size_for_instance(
    instance: &WidgetInstance,
    catalog: &[WidgetDefinition],
) -> WidgetSize {
    let builtin_type = WidgetKind::from_plugin_id(&instance.plugin_id);
    let default_size = definition_for_instance(instance, catalog)
        .map(|definition| widget_definition_size_for_symbols(definition, &instance.symbols))
        .unwrap_or_else(|| builtin_type.unwrap_or_default().default_size());
    default_widget_size_for_parts(
        builtin_type,
        default_size,
        &instance.symbols,
        &instance.config,
    )
}

fn default_widget_size_for_parts(
    widget_type: Option<WidgetKind>,
    default_size: WidgetSize,
    symbols: &[String],
    config: &serde_json::Value,
) -> WidgetSize {
    if widget_type == Some(WidgetKind::QuoteBoard) {
        quote_board_display_size(
            default_size,
            widget_config_show_coin_logos(config),
            widget_config_hide_quote_asset(config),
            quote_board_row_count_for_symbols(symbols),
        )
    } else {
        default_size
    }
}

pub fn widget_definition_size_for_symbols(
    definition: &WidgetDefinition,
    symbols: &[String],
) -> WidgetSize {
    definition.size_policy.size_for_symbol_count(
        definition.default_size,
        symbols.len(),
        definition.min_symbol_limit,
        definition.symbol_limit,
    )
}

pub fn widget_id_suffix(id: &str) -> Option<u64> {
    id.rsplit('-').next()?.parse().ok()
}

fn layout_overlaps_existing_for_size(
    store: &LayoutStore,
    candidate: &WidgetLayout,
    size: WidgetSize,
    settings: AppSettings,
    catalog: &[WidgetDefinition],
    desktop_size: (i32, i32),
) -> bool {
    let candidate_rect = widget_rect_for_size(candidate, size);
    store.widgets.iter().enumerate().any(|(index, instance)| {
        if is_parked_position(instance.layout.x, instance.layout.y) {
            return false;
        }
        let layout = layout_for_instance(instance, index, settings.clone(), catalog, desktop_size);
        candidate_rect.overlaps(
            widget_rect_for_size(&layout, widget_size_for_instance(instance, catalog)),
            DEFAULT_LAYOUT_GAP,
        )
    })
}

fn layout_has_persisted_size(layout: &WidgetLayout) -> bool {
    layout.width > 0 || layout.height > 0
}

fn normalize_layout_size_and_scale(
    layout: &mut WidgetLayout,
    default_size: WidgetSize,
    fallback_scale_percent: i32,
    defer_dynamic_size_migration: bool,
) {
    let had_explicit_scale = layout.scale_percent > 0;
    let had_persisted_size = layout_has_persisted_size(layout);
    let scale_percent =
        widget_layout_scale_percent_for_size(layout, default_size, fallback_scale_percent);

    if defer_dynamic_size_migration && had_persisted_size && !had_explicit_scale {
        let size = clamp_widget_size(WidgetSize {
            width: if layout.width > 0 {
                layout.width
            } else {
                default_size.width
            },
            height: if layout.height > 0 {
                layout.height
            } else {
                default_size.height
            },
        });
        layout.scale_percent = 0;
        layout.width = size.width;
        layout.height = size.height;
        return;
    }

    layout.scale_percent = scale_percent;
    let size = widget_size_from_scale_percent(default_size, scale_percent);
    layout.width = size.width;
    layout.height = size.height;
}

fn migrate_legacy_quote_board_footer_size(instance: &mut WidgetInstance) {
    if instance.widget_type() != WidgetKind::QuoteBoard {
        return;
    }

    let current_size = WidgetSize {
        width: instance.layout.width,
        height: instance.layout.height,
    };
    let legacy_default_size = WidgetSize {
        width: QUOTE_BOARD_WIDTH,
        height: LEGACY_QUOTE_BOARD_HEIGHT_WITH_FOOTER,
    };
    let scale_percent = widget_scale_percent_for_size(current_size, legacy_default_size);
    let legacy_size = widget_size_from_scale_percent(legacy_default_size, scale_percent);
    if current_size != legacy_size {
        return;
    }

    let new_size =
        widget_size_from_scale_percent(WidgetKind::QuoteBoard.default_size(), scale_percent);
    instance.layout.scale_percent = scale_percent;
    instance.layout.width = new_size.width;
    instance.layout.height = new_size.height;
}

fn migrate_quote_board_display_size(instance: &mut WidgetInstance) {
    if instance.widget_type() != WidgetKind::QuoteBoard {
        return;
    }

    let current_size = WidgetSize {
        width: instance.layout.width,
        height: instance.layout.height,
    };
    let full_default_size = WidgetKind::QuoteBoard.default_size();
    let previous_display_size = quote_board_previous_display_size(
        full_default_size,
        widget_show_coin_logos(instance),
        widget_hide_quote_asset(instance),
    );
    let Some(scale_percent) = quote_board_matching_display_scale(
        current_size,
        &[previous_display_size, full_default_size],
    ) else {
        return;
    };

    let display_size = widget_size_from_scale_percent(
        quote_board_display_size(
            full_default_size,
            widget_show_coin_logos(instance),
            widget_hide_quote_asset(instance),
            quote_board_row_count_for_symbols(&instance.symbols),
        ),
        scale_percent,
    );
    instance.layout.scale_percent = scale_percent;
    instance.layout.width = display_size.width;
    instance.layout.height = display_size.height;
}

fn quote_board_matching_display_scale(
    current_size: WidgetSize,
    base_sizes: &[WidgetSize],
) -> Option<i32> {
    base_sizes.iter().find_map(|base_size| {
        let scale_percent = widget_scale_percent_for_size(current_size, *base_size);
        let scaled_size = widget_size_from_scale_percent(*base_size, scale_percent);
        (current_size == scaled_size).then_some(scale_percent)
    })
}

fn quote_board_previous_display_size(
    default_size: WidgetSize,
    show_coin_logos: bool,
    hide_quote_asset: bool,
) -> WidgetSize {
    WidgetSize {
        width: quote_board_display_width(default_size, show_coin_logos, hide_quote_asset),
        height: default_size.height,
    }
}

fn quote_board_display_size(
    default_size: WidgetSize,
    show_coin_logos: bool,
    hide_quote_asset: bool,
    row_count: usize,
) -> WidgetSize {
    WidgetSize {
        width: quote_board_display_width(default_size, show_coin_logos, hide_quote_asset),
        height: quote_board_height_for_row_count(row_count),
    }
}

fn quote_board_display_width(
    default_size: WidgetSize,
    show_coin_logos: bool,
    hide_quote_asset: bool,
) -> i32 {
    match (show_coin_logos, hide_quote_asset) {
        (true, false) => default_size.width,
        (false, false) => QUOTE_BOARD_WIDTH_WITHOUT_COIN_LOGOS,
        (true, true) => QUOTE_BOARD_WIDTH_WITHOUT_QUOTE_ASSET,
        (false, true) => QUOTE_BOARD_WIDTH_COMPACT_SYMBOLS,
    }
}

fn quote_board_row_count_for_symbols(symbols: &[String]) -> usize {
    symbols
        .len()
        .clamp(MIN_SYMBOLS_PER_WIDGET, MAX_SYMBOLS_PER_WIDGET)
}

fn quote_board_height_for_row_count(row_count: usize) -> i32 {
    let row_count = row_count.clamp(MIN_SYMBOLS_PER_WIDGET, MAX_SYMBOLS_PER_WIDGET) as i32;
    if row_count <= 1 {
        QUOTE_BOARD_ONE_ROW_HEIGHT
    } else if row_count <= QUOTE_BOARD_LEGACY_ROW_HEIGHT_LIMIT {
        QUOTE_BOARD_MULTI_ROW_BASE_HEIGHT + (row_count - 1) * QUOTE_BOARD_ROW_HEIGHT_STEP
    } else {
        QUOTE_BOARD_HEIGHT
            + (row_count - QUOTE_BOARD_LEGACY_ROW_HEIGHT_LIMIT)
                * QUOTE_BOARD_EXTENDED_ROW_HEIGHT_STEP
    }
}

fn div_ceil_i32(value: i32, divisor: i32) -> i32 {
    (value + divisor - 1) / divisor
}

fn div_round_i32(value: i32, divisor: i32) -> i32 {
    (value + divisor / 2) / divisor
}

fn scale_dimension(value: i32, scale_percent: i32) -> i32 {
    div_round_i32(value.max(1) * scale_percent, 100)
}

fn widget_rect_for_size(layout: &WidgetLayout, size: WidgetSize) -> WidgetRect {
    WidgetRect {
        x: layout.x,
        y: layout.y,
        width: size.width,
        height: size.height,
    }
}

fn next_widget_number(store: &mut LayoutStore, widget_type: WidgetKind) -> u64 {
    next_instance_number(store, widget_type.id_prefix())
}

fn next_instance_number(store: &mut LayoutStore, id_prefix: &str) -> u64 {
    loop {
        let number = store.next_widget_number.max(DEFAULT_NEXT_WIDGET_NUMBER);
        store.next_widget_number = number + 1;
        let id = format!("{id_prefix}-{number}");
        if !store.widgets.iter().any(|widget| widget.id == id) {
            return number;
        }
    }
}

fn plugin_instance_id_prefix(plugin_id: &str) -> String {
    let slug = plugin_id
        .rsplit(['.', '-'])
        .find(|part| !part.is_empty())
        .unwrap_or("plugin")
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
        .take(24)
        .collect::<String>();
    if slug.is_empty() {
        "plugin-widget".to_string()
    } else {
        format!("plugin-{slug}")
    }
}

fn definition_for_instance<'a>(
    instance: &WidgetInstance,
    catalog: &'a [WidgetDefinition],
) -> Option<&'a WidgetDefinition> {
    catalog
        .iter()
        .find(|definition| definition.id == instance.plugin_id)
}

fn persisted_widget_title(widget_type: WidgetKind) -> &'static str {
    match widget_type {
        WidgetKind::QuoteBoard => "Quote Board",
        WidgetKind::MiniTicker => "Mini Ticker",
    }
}

fn default_persisted_widget_name(widget_type: WidgetKind, number: u64) -> String {
    format!("{} {number}", persisted_widget_title(widget_type))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_settings_json_defaults_new_fields() {
        let settings = serde_json::from_str::<AppSettings>(
            r#"{"widgets_always_on_top":false,"opacity_percent":77}"#,
        )
        .unwrap()
        .normalized();

        assert!(!settings.widgets_always_on_top);
        assert_eq!(settings.opacity_percent, 77);
        assert_eq!(settings.widget_scale_percent, DEFAULT_WIDGET_SCALE_PERCENT);
        assert!(!settings.red_up_enabled);
        assert_eq!(settings.market_provider, MarketProviderPreference::Auto);
        assert!(settings.market_binance_enabled);
        assert!(settings.market_coinbase_enabled);
        assert!(settings.market_okx_enabled);
        assert!(settings.market_hyperliquid_enabled);
        assert_eq!(
            settings.refresh_interval_seconds,
            DEFAULT_REFRESH_INTERVAL_SECONDS
        );
        assert_eq!(settings.market_default_symbols, default_market_symbols());
        assert!(settings.market_fallback_enabled);
        assert!(!settings.auto_start_enabled);
        assert!(settings.show_main_window_on_startup);
        assert_eq!(settings.shortcut, ShortcutPreference::AltC);
        assert_eq!(settings.theme, ThemePreference::System);
        assert_eq!(settings.language, LanguagePreference::En);
        assert!(settings.tray_icon_enabled);
        assert!(!settings.tray_hover_display_enabled);
        assert!(!settings.network_proxy_enabled);
        assert!(settings.network_proxy_url.is_empty());
        assert!(settings.alert_rules.is_empty());
    }

    #[test]
    fn widget_display_config_defaults_and_updates() {
        let mut widget = WidgetInstance {
            id: "quote-board-1".to_string(),
            plugin_id: WidgetKind::QuoteBoard.plugin_id().to_string(),
            legacy_widget_type: None,
            name: "Quote Board 1".to_string(),
            visible: true,
            layout: WidgetLayout::default(),
            symbols: vec!["binance:spot:BTC/USDC".to_string()],
            config: default_widget_config(),
        };

        assert!(widget_show_coin_logos(&widget));
        assert!(!widget_hide_quote_asset(&widget));
        assert_eq!(widget_theme_preference(&widget), WIDGET_THEME_SYSTEM);

        set_widget_display_config(&mut widget, false, true);
        set_widget_theme_preference(&mut widget, " light ");

        assert!(!widget_show_coin_logos(&widget));
        assert!(widget_hide_quote_asset(&widget));
        assert_eq!(widget_theme_preference(&widget), "light");
        assert_eq!(
            widget.config[WIDGET_CONFIG_SHOW_COIN_LOGOS],
            Value::Bool(false)
        );
        assert_eq!(
            widget.config[WIDGET_CONFIG_HIDE_QUOTE_ASSET],
            Value::Bool(true)
        );
        assert_eq!(
            widget.config[WIDGET_CONFIG_THEME],
            Value::String("light".to_string())
        );
    }

    #[test]
    fn quote_board_default_size_tracks_display_config_and_rows() {
        let mut widget = WidgetInstance {
            id: "quote-board-1".to_string(),
            plugin_id: WidgetKind::QuoteBoard.plugin_id().to_string(),
            legacy_widget_type: None,
            name: "Quote Board 1".to_string(),
            visible: true,
            layout: WidgetLayout::default(),
            symbols: vec!["binance:spot:BTC/USDC".to_string()],
            config: default_widget_config(),
        };

        assert_eq!(
            default_widget_size_for_instance(&widget, &[]),
            WidgetSize {
                width: 286,
                height: QUOTE_BOARD_ONE_ROW_HEIGHT,
            }
        );

        set_widget_display_config(&mut widget, false, false);
        assert_eq!(default_widget_size_for_instance(&widget, &[]).width, 274);
        assert_eq!(
            default_widget_size_for_instance(&widget, &[]).height,
            QUOTE_BOARD_ONE_ROW_HEIGHT
        );

        set_widget_display_config(&mut widget, true, true);
        assert_eq!(default_widget_size_for_instance(&widget, &[]).width, 246);

        set_widget_display_config(&mut widget, false, true);
        assert_eq!(default_widget_size_for_instance(&widget, &[]).width, 224);

        widget.symbols = vec![
            "binance:spot:BTC/USDT".to_string(),
            "binance:spot:ETH/USDT".to_string(),
        ];
        assert_eq!(default_widget_size_for_instance(&widget, &[]).height, 101);

        widget.symbols = vec![
            "binance:spot:BTC/USDT".to_string(),
            "binance:spot:ETH/USDT".to_string(),
            "binance:spot:SOL/USDT".to_string(),
            "binance:spot:BNB/USDT".to_string(),
            "binance:spot:DOGE/USDT".to_string(),
        ];
        assert_eq!(default_widget_size_for_instance(&widget, &[]).height, 194);

        widget.symbols = (0..10)
            .map(|index| format!("binance:spot:COIN{index}/USDT"))
            .collect();
        assert_eq!(default_widget_size_for_instance(&widget, &[]).height, 324);
    }

    #[test]
    fn quote_board_symbol_limit_allows_twenty_pairs() {
        let symbols = (0..25)
            .map(|index| format!("COIN{index}/USDT"))
            .collect::<Vec<_>>();
        let normalized = normalized_symbols_for_type(WidgetKind::QuoteBoard, symbols);

        assert_eq!(WidgetKind::QuoteBoard.symbol_limit(), 20);
        assert_eq!(normalized.len(), 20);
        assert_eq!(
            normalized.last().map(String::as_str),
            Some("binance:spot:COIN19/USDT")
        );
    }

    #[test]
    fn plugin_symbol_block_size_policy_tracks_symbols_without_quote_board_config() {
        let catalog = vec![WidgetDefinition {
            id: "com.example.status-strip".to_string(),
            name: "Status Strip".to_string(),
            default_size: WidgetSize {
                width: 688,
                height: 92,
            },
            size_policy: WidgetSizePolicy::SymbolBlocks {
                block_width: 136,
                block_height: 84,
                padding_width: 8,
                padding_height: 8,
            },
            min_symbol_limit: 1,
            symbol_limit: 5,
            default_symbols: Vec::new(),
        }];
        let mut widget = WidgetInstance {
            id: "plugin-strip-1".to_string(),
            plugin_id: "com.example.status-strip".to_string(),
            legacy_widget_type: None,
            name: "Status Strip 1".to_string(),
            visible: true,
            layout: WidgetLayout::default(),
            symbols: vec!["binance:spot:BTC/USDT".to_string()],
            config: default_widget_config(),
        };

        set_widget_display_config(&mut widget, false, true);

        assert_eq!(
            default_widget_size_for_instance(&widget, &catalog),
            WidgetSize {
                width: 144,
                height: 92,
            }
        );

        widget.symbols = vec![
            "binance:spot:BTC/USDT".to_string(),
            "binance:spot:ETH/USDT".to_string(),
            "binance:spot:SOL/USDT".to_string(),
        ];
        assert_eq!(
            default_widget_size_for_instance(&widget, &catalog),
            WidgetSize {
                width: 416,
                height: 92,
            }
        );
    }

    #[test]
    fn plugin_symbol_grid_size_policy_tracks_rows_and_columns() {
        let catalog = vec![WidgetDefinition {
            id: "com.example.grid".to_string(),
            name: "Grid".to_string(),
            default_size: WidgetSize {
                width: 416,
                height: 176,
            },
            size_policy: WidgetSizePolicy::SymbolGrid {
                cell_width: 136,
                cell_height: 84,
                content_padding_width: 8,
                content_padding_height: 8,
                columns: Some(3),
                rows: None,
            },
            min_symbol_limit: 1,
            symbol_limit: 5,
            default_symbols: Vec::new(),
        }];
        let mut widget = WidgetInstance {
            id: "plugin-grid-1".to_string(),
            plugin_id: "com.example.grid".to_string(),
            legacy_widget_type: None,
            name: "Grid 1".to_string(),
            visible: true,
            layout: WidgetLayout::default(),
            symbols: vec!["binance:spot:BTC/USDT".to_string()],
            config: default_widget_config(),
        };

        assert_eq!(
            default_widget_size_for_instance(&widget, &catalog),
            WidgetSize {
                width: 144,
                height: 92,
            }
        );

        widget.symbols = vec![
            "binance:spot:BTC/USDT".to_string(),
            "binance:spot:ETH/USDT".to_string(),
            "binance:spot:SOL/USDT".to_string(),
            "binance:spot:BNB/USDT".to_string(),
            "binance:spot:DOGE/USDT".to_string(),
        ];
        assert_eq!(
            default_widget_size_for_instance(&widget, &catalog),
            WidgetSize {
                width: 416,
                height: 176,
            }
        );
    }

    #[test]
    fn plugin_definition_default_symbols_seed_new_instances() {
        let settings = AppSettings::default();
        let mut store = LayoutStore::default();
        let plugin = WidgetDefinition {
            id: "com.example.watchlist".to_string(),
            name: "Watchlist".to_string(),
            default_size: WidgetSize {
                width: 300,
                height: 180,
            },
            size_policy: WidgetSizePolicy::Fixed,
            min_symbol_limit: 1,
            symbol_limit: 5,
            default_symbols: vec![
                "okx:spot:ETH/USDT".to_string(),
                "binance:spot:SOL/USDT".to_string(),
            ],
        };

        add_plugin_instance(&mut store, &plugin, &settings, (1920, 1080));

        assert_eq!(
            store.widgets[0].symbols,
            vec!["okx:spot:ETH/USDT", "binance:spot:SOL/USDT"]
        );
    }

    #[test]
    fn enum_serialization_is_stable_snake_case() {
        assert_eq!(
            serde_json::to_string(&MarketProviderPreference::Okx).unwrap(),
            r#""okx""#
        );
        assert_eq!(
            serde_json::to_string(&ShortcutPreference::AltC).unwrap(),
            r#""alt_c""#
        );
        assert_eq!(
            serde_json::to_string(&ShortcutPreference::CtrlShiftSpace).unwrap(),
            r#""ctrl_shift_space""#
        );
        assert_eq!(
            serde_json::to_string(&LanguagePreference::ZhHans).unwrap(),
            r#""zh_hans""#
        );
        assert_eq!(
            serde_json::to_string(&ThemePreference::Dark).unwrap(),
            r#""dark""#
        );
    }

    #[test]
    fn setting_indices_map_to_expected_values() {
        assert_eq!(
            MarketProviderPreference::from_index(2),
            MarketProviderPreference::Okx
        );
        assert_eq!(
            ShortcutPreference::from_index(1),
            ShortcutPreference::Disabled
        );
        assert_eq!(
            ShortcutPreference::from_index(4),
            ShortcutPreference::Disabled
        );
        assert_eq!(LanguagePreference::from_index(1), LanguagePreference::En);
        assert_eq!(ThemePreference::from_index(2), ThemePreference::Dark);
    }

    #[test]
    fn deprecated_shortcut_preferences_normalize_to_disabled() {
        assert_eq!(
            ShortcutPreference::CtrlSpace.normalized(),
            ShortcutPreference::Disabled
        );
        assert_eq!(
            ShortcutPreference::CtrlShiftSpace.normalized(),
            ShortcutPreference::Disabled
        );
        assert_eq!(
            ShortcutPreference::AltSpace.normalized(),
            ShortcutPreference::Disabled
        );
        assert_eq!(ShortcutPreference::Disabled.index(), 1);
    }

    #[test]
    fn normalizes_market_settings_bounds_and_symbols() {
        let settings = AppSettings {
            widget_scale_percent: 999,
            refresh_interval_seconds: 1,
            market_default_symbols: vec![
                "btc".to_string(),
                "ETHUSDT".to_string(),
                "sol/usdt".to_string(),
                "SOL-USDT".to_string(),
                "bnb".to_string(),
                "xrp".to_string(),
                "doge".to_string(),
            ],
            ..AppSettings::default()
        }
        .normalized();

        assert_eq!(
            settings.refresh_interval_seconds,
            MIN_REFRESH_INTERVAL_SECONDS
        );
        assert_eq!(settings.widget_scale_percent, MAX_WIDGET_SCALE_PERCENT);
        assert_eq!(
            settings.market_default_symbols,
            vec![
                "binance:spot:BTC/USDT",
                "binance:spot:ETH/USDT",
                "binance:spot:SOL/USDT",
            ]
        );

        let low_scale_settings = AppSettings {
            widget_scale_percent: 1,
            ..AppSettings::default()
        }
        .normalized();
        assert_eq!(
            low_scale_settings.widget_scale_percent,
            MIN_WIDGET_SCALE_PERCENT
        );
    }

    #[test]
    fn widget_scale_converts_between_percent_and_size() {
        let default_size = WidgetSize {
            width: QUOTE_BOARD_WIDTH,
            height: QUOTE_BOARD_HEIGHT,
        };
        let scaled_10 = widget_size_from_scale_percent(default_size, 10);
        assert_eq!(scaled_10.width, 29);
        assert_eq!(scaled_10.height, 19);
        assert_eq!(widget_scale_percent_for_size(scaled_10, default_size), 10);

        let scaled = widget_size_from_scale_percent(default_size, 125);

        assert_eq!(scaled.width, 358);
        assert_eq!(scaled.height, 243);
        assert_eq!(widget_scale_percent_for_size(scaled, default_size), 125);

        let scaled_300 = widget_size_from_scale_percent(default_size, 300);
        assert_eq!(scaled_300.width, 858);
        assert_eq!(scaled_300.height, 582);
        assert_eq!(widget_scale_percent_for_size(scaled_300, default_size), 300);

        let mini_ticker_size = WidgetKind::MiniTicker.default_size();
        let (min_scale, max_scale) = widget_scale_percent_bounds(mini_ticker_size);
        assert_eq!(min_scale, MIN_WIDGET_SCALE_PERCENT);
        assert_eq!(max_scale, MAX_WIDGET_SCALE_PERCENT);
    }

    #[test]
    fn normalizes_missing_layout_scale_from_legacy_size() {
        let mut store = serde_json::from_str::<LayoutStore>(
            r#"{
              "widgets": [
                {
                  "id": "quote-board-1",
                  "plugin_id": "builtin.quote-board",
                  "name": "Quote Board 1",
                  "layout": {
                    "x": 24,
                    "y": 48,
                    "always_on_top": false,
                    "opacity_percent": 92,
                    "width": 429,
                    "height": 120
                  },
                  "symbols": ["BTC"]
                }
              ]
            }"#,
        )
        .unwrap();

        normalize_store(&mut store, 0);

        assert_eq!(store.widgets[0].layout.scale_percent, 150);
        assert_eq!(store.widgets[0].layout.width, 429);
        assert_eq!(store.widgets[0].layout.height, 120);
    }

    #[test]
    fn resize_widget_to_content_preserves_explicit_scale() {
        let mut widget = WidgetInstance {
            id: "quote-board-1".to_string(),
            plugin_id: WidgetKind::QuoteBoard.plugin_id().to_string(),
            legacy_widget_type: None,
            name: "Quote Board 1".to_string(),
            visible: true,
            layout: WidgetLayout::default(),
            symbols: vec!["binance:spot:BTC/USDT".to_string()],
            config: default_widget_config(),
        };

        resize_widget_to_content(&mut widget, &[], 150);
        widget.symbols.push("binance:spot:ETH/USDT".to_string());
        resize_widget_to_current_content(&mut widget, &[], DEFAULT_WIDGET_SCALE_PERCENT);

        assert_eq!(widget.layout.scale_percent, 150);
        assert_eq!(widget.layout.width, 429);
        assert_eq!(widget.layout.height, 152);
    }

    #[test]
    fn widget_content_scale_uses_smaller_dimension_ratio() {
        let default_size = WidgetSize {
            width: 286,
            height: 80,
        };

        assert_eq!(
            widget_content_scale_percent_for_size(
                WidgetSize {
                    width: 572,
                    height: 240,
                },
                default_size,
            ),
            200
        );
        assert_eq!(
            widget_content_scale_percent_for_size(
                WidgetSize {
                    width: 858,
                    height: 120,
                },
                default_size,
            ),
            150
        );
    }

    #[test]
    fn normalizes_legacy_quote_board_footer_height() {
        let mut store = serde_json::from_str::<LayoutStore>(
            r#"{
              "widgets": [
                {
                  "id": "quote-board-1",
                  "plugin_id": "builtin.quote-board",
                  "name": "Quote Board 1",
                  "layout": {
                    "x": 24,
                    "y": 48,
                    "always_on_top": false,
                    "opacity_percent": 92,
                    "width": 286,
                    "height": 232
                  },
                  "symbols": ["BTC"]
                }
              ]
            }"#,
        )
        .unwrap();

        normalize_store(&mut store, 0);

        assert_eq!(store.widgets[0].layout.width, QUOTE_BOARD_WIDTH);
        assert_eq!(store.widgets[0].layout.height, QUOTE_BOARD_ONE_ROW_HEIGHT);
    }

    #[test]
    fn normalizes_quote_board_display_width() {
        let mut store = serde_json::from_str::<LayoutStore>(
            r#"{
              "widgets": [
                {
                  "id": "quote-board-1",
                  "plugin_id": "builtin.quote-board",
                  "name": "Quote Board 1",
                  "layout": {
                    "x": 24,
                    "y": 48,
                    "always_on_top": false,
                    "opacity_percent": 92,
                    "width": 286,
                    "height": 194
                  },
                  "symbols": ["BTC"],
                  "config": {
                    "show_coin_logos": false,
                    "hide_quote_asset": true
                  }
                }
              ]
            }"#,
        )
        .unwrap();

        normalize_store(&mut store, 0);

        assert_eq!(
            store.widgets[0].layout.width,
            QUOTE_BOARD_WIDTH_COMPACT_SYMBOLS
        );
        assert_eq!(store.widgets[0].layout.height, QUOTE_BOARD_ONE_ROW_HEIGHT);
    }

    #[test]
    fn normalizes_quote_board_display_height_from_symbol_count() {
        let mut store = serde_json::from_str::<LayoutStore>(
            r#"{
              "widgets": [
                {
                  "id": "quote-board-1",
                  "plugin_id": "builtin.quote-board",
                  "name": "Quote Board 1",
                  "layout": {
                    "x": 24,
                    "y": 48,
                    "always_on_top": false,
                    "opacity_percent": 92,
                    "width": 286,
                    "height": 194
                  },
                  "symbols": ["BTC", "ETH"]
                }
              ]
            }"#,
        )
        .unwrap();

        normalize_store(&mut store, 0);

        assert_eq!(store.widgets[0].layout.width, QUOTE_BOARD_WIDTH);
        assert_eq!(store.widgets[0].layout.height, 101);
    }

    #[test]
    fn normalizes_network_proxy_url_and_effective_value() {
        let disabled = AppSettings {
            network_proxy_enabled: false,
            network_proxy_url: "  socks5://127.0.0.1:1080  ".to_string(),
            ..AppSettings::default()
        }
        .normalized();

        assert_eq!(disabled.network_proxy_url, "socks5://127.0.0.1:1080");
        assert_eq!(effective_network_proxy_url(&disabled), None);

        let enabled = AppSettings {
            network_proxy_enabled: true,
            network_proxy_url: "  http://127.0.0.1:7890  ".to_string(),
            ..AppSettings::default()
        }
        .normalized();

        assert_eq!(
            effective_network_proxy_url(&enabled),
            Some("http://127.0.0.1:7890".to_string())
        );
    }

    #[test]
    fn normalizes_layout_size_and_lock_defaults() {
        let mut store = serde_json::from_str::<LayoutStore>(
            r#"{
              "widgets": [
                {
                  "id": "quote-board-1",
                  "plugin_id": "builtin.quote-board",
                  "name": "Quote Board 1",
                  "layout": {
                    "x": 24,
                    "y": 48,
                    "always_on_top": false,
                    "opacity_percent": 92
                  },
                  "symbols": ["BTC"]
                },
                {
                  "id": "mini-ticker-2",
                  "plugin_id": "builtin.mini-ticker",
                  "name": "Mini Ticker 2",
                  "layout": {
                    "x": 72,
                    "y": 96,
                    "always_on_top": false,
                    "opacity_percent": 92,
                    "locked": true,
                    "width": 20,
                    "height": 20000
                  },
                  "symbols": ["ETH"]
                }
              ]
            }"#,
        )
        .unwrap();

        normalize_store(&mut store, 0);

        assert!(!store.widgets[0].layout.locked);
        assert_eq!(store.widgets[0].layout.width, QUOTE_BOARD_WIDTH);
        assert_eq!(store.widgets[0].layout.height, QUOTE_BOARD_ONE_ROW_HEIGHT);
        assert!(store.widgets[1].layout.locked);
        assert_eq!(
            store.widgets[1].layout.scale_percent,
            MIN_WIDGET_SCALE_PERCENT
        );
        assert_eq!(store.widgets[1].layout.width, 24);
        assert_eq!(store.widgets[1].layout.height, 11);
    }

    #[test]
    fn normalizes_widget_symbols_to_definition_bounds() {
        let mut store = LayoutStore {
            settings: AppSettings {
                market_default_symbols: vec![
                    "ETH".to_string(),
                    "SOL".to_string(),
                    "BNB".to_string(),
                ],
                ..AppSettings::default()
            },
            widgets: vec![WidgetInstance {
                id: "tri-card-1".to_string(),
                plugin_id: "com.example.tri-card".to_string(),
                legacy_widget_type: None,
                name: "Tri Card 1".to_string(),
                visible: true,
                layout: WidgetLayout::default(),
                symbols: vec!["BTC".to_string()],
                config: default_widget_config(),
            }],
            ..LayoutStore::default()
        };
        let catalog = vec![WidgetDefinition {
            id: "com.example.tri-card".to_string(),
            name: "Tri Card".to_string(),
            default_size: WidgetSize {
                width: 300,
                height: 180,
            },
            size_policy: WidgetSizePolicy::Fixed,
            min_symbol_limit: 2,
            symbol_limit: 3,
            default_symbols: Vec::new(),
        }];

        normalize_store_with_catalog(&mut store, 0, &catalog, (1920, 1080));

        assert_eq!(
            store.widgets[0].symbols,
            vec!["binance:spot:BTC/USDT", "binance:spot:ETH/USDT"]
        );
    }

    #[test]
    fn normalizes_alert_rules() {
        let settings = AppSettings {
            alert_rules: vec![
                AlertRule {
                    id: " breakout ".to_string(),
                    symbol: "btcusdt".to_string(),
                    condition: AlertCondition::PriceAbove,
                    threshold: 100000.0,
                    enabled: true,
                },
                AlertRule {
                    id: "breakout".to_string(),
                    symbol: "eth/usdt".to_string(),
                    condition: AlertCondition::ChangePercentBelow,
                    threshold: -5.0,
                    enabled: false,
                },
                AlertRule {
                    id: String::new(),
                    symbol: "???".to_string(),
                    condition: AlertCondition::PriceBelow,
                    threshold: f64::NAN,
                    enabled: true,
                },
            ],
            ..AppSettings::default()
        }
        .normalized();

        assert_eq!(settings.alert_rules.len(), 2);
        assert_eq!(settings.alert_rules[0].id, "breakout");
        assert_eq!(settings.alert_rules[0].symbol, "binance:spot:BTC/USDT");
        assert_eq!(settings.alert_rules[1].id, "breakout-2");
        assert_eq!(settings.alert_rules[1].symbol, "binance:spot:ETH/USDT");
        assert!(!settings.alert_rules[1].enabled);
    }

    #[test]
    fn save_layout_store_replaces_existing_file_and_removes_temp_file() {
        let dir = std::env::temp_dir().join(format!(
            "crypto-hud-shell-state-save-{}-{}",
            std::process::id(),
            SAVE_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let path = dir.join(LAYOUT_STATE_FILE_NAME);
        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, "previous contents").unwrap();

        let store = LayoutStore {
            settings: AppSettings {
                opacity_percent: 77,
                ..AppSettings::default()
            },
            ..LayoutStore::default()
        };

        save_layout_store(&path, &store).unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("\"opacity_percent\": 77"));
        assert!(fs::read_dir(&dir).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")
        }));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_layout_store_migrates_legacy_state_file_name() {
        let dir = std::env::temp_dir().join(format!(
            "crypto-hud-shell-state-migrate-{}-{}",
            std::process::id(),
            SAVE_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let legacy_path = dir.join(LEGACY_LAYOUT_STATE_FILE_NAME);
        let path = dir.join(LAYOUT_STATE_FILE_NAME);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            &legacy_path,
            r#"{
              "settings": { "opacity_percent": 77 },
              "widgets": []
            }"#,
        )
        .unwrap();

        let store = load_layout_store(&path, 0, &[], (1920, 1080));

        assert_eq!(store.settings.opacity_percent, 77);
        assert!(path.exists());
        assert!(fs::read_to_string(&path)
            .unwrap()
            .contains("\"opacity_percent\": 77"));

        let _ = fs::remove_dir_all(&dir);
    }
}
