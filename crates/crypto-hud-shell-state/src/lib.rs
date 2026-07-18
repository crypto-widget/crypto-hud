#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
use std::{
    collections::{HashMap, HashSet},
    env,
    ffi::{OsStr, OsString},
    fmt,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

pub use crypto_hud_core::{
    clamp_refresh_interval, default_enabled_market_sources, default_market_symbols,
    format_market_pair_display, format_market_pair_source, format_market_pair_symbol,
    market_pair_source, normalize_market_pair_key, normalize_market_symbols,
    normalize_symbol_token, AlertCondition, AlertRule, MarketDataSource, MarketPair,
    MarketProviderPreference, MarketType, DEFAULT_REFRESH_INTERVAL_SECONDS, MAX_MARKET_SYMBOLS,
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
pub const WIDGET_CONFIG_SHOW_HEADER: &str = "show_header";
pub const WIDGET_CONFIG_THEME: &str = "theme";
pub const WIDGET_CONFIG_PLUGIN_PARAMETERS: &str = "plugin_parameters";
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
const QUOTE_BOARD_HEADERLESS_ONE_ROW_HEIGHT: i32 = 49;
const LEGACY_QUOTE_BOARD_HEIGHT_WITH_FOOTER: i32 = 232;
pub const DEFAULT_LAYOUT_SCAN_SLOTS: usize = 80;
pub const DEFAULT_MONITOR_DPI: u32 = 96;
pub const MIN_VISIBLE_WIDGET_PX: i32 = 8;
pub const MIN_WIDGET_WIDTH: i32 = 12;
pub const MIN_WIDGET_HEIGHT: i32 = 8;
pub const MAX_WIDGET_WIDTH: i32 = 6000;
pub const MAX_WIDGET_HEIGHT: i32 = 4000;
pub const MIN_WIDGET_SCALE_PERCENT: i32 = 30;
pub const MAX_WIDGET_SCALE_PERCENT: i32 = 300;
pub const DEFAULT_WIDGET_SCALE_PERCENT: i32 = 100;
pub const PARKED_WIDGET_X: i32 = -32_000;
pub const PARKED_WIDGET_Y: i32 = -32_000;
pub const MIN_SYMBOLS_PER_WIDGET: usize = 1;
pub const MAX_SYMBOLS_PER_WIDGET: usize = MAX_MARKET_SYMBOLS;
pub const MINI_TICKER_SYMBOL_LIMIT: usize = 1;
pub const MAX_TRAY_MARKET_SYMBOLS: usize = 8;
pub const MIN_TRAY_MARKET_SWITCH_INTERVAL_SECONDS: i32 = 2;
pub const MAX_TRAY_MARKET_SWITCH_INTERVAL_SECONDS: i32 = 300;
pub const DEFAULT_TRAY_MARKET_SWITCH_INTERVAL_SECONDS: i32 = 5;
pub const BUILTIN_QUOTE_BOARD_PLUGIN_ID: &str = "builtin.quote-board";
pub const BUILTIN_MINI_TICKER_PLUGIN_ID: &str = "builtin.mini-ticker";
pub const LAYOUT_STATE_FILE_NAME: &str = "layouts.json";
pub const LEGACY_LAYOUT_STATE_FILE_NAME: &str = "poc-layouts.json";
// Bump this version whenever persisted fields are added or changed in a way
// that an older release could ignore and later overwrite. The top-level loader
// is strict, but nested structures such as AppSettings remain serde-compatible
// and may otherwise discard fields they do not recognize.
pub const LAYOUT_STORE_SCHEMA_VERSION: u32 = 2;
const PREVIOUS_LAYOUT_STORE_SCHEMA_VERSION: u32 = 1;

const LAYOUT_STORE_SCHEMA_VERSION_FIELD: &str = "schema_version";

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopWorkArea {
    /// Physical desktop coordinate reported by the platform.
    pub x: i32,
    /// Physical desktop coordinate reported by the platform.
    pub y: i32,
    /// Physical work-area width reported by the platform.
    pub width: i32,
    /// Physical work-area height reported by the platform.
    pub height: i32,
    /// Effective monitor DPI used to convert persisted logical widget sizes.
    pub dpi: u32,
    pub is_primary: bool,
}

impl DesktopWorkArea {
    pub const fn from_desktop_size(desktop_size: (i32, i32)) -> Self {
        Self {
            x: 0,
            y: 0,
            width: desktop_size.0,
            height: desktop_size.1,
            dpi: DEFAULT_MONITOR_DPI,
            is_primary: true,
        }
    }

    pub const fn effective_dpi(self) -> u32 {
        if self.dpi == 0 {
            DEFAULT_MONITOR_DPI
        } else {
            self.dpi
        }
    }

    pub fn physical_length(self, logical_length: i32) -> i32 {
        let logical_length = i64::from(logical_length.max(1));
        let dpi = i64::from(self.effective_dpi());
        ((logical_length * dpi + i64::from(DEFAULT_MONITOR_DPI / 2))
            / i64::from(DEFAULT_MONITOR_DPI))
        .clamp(1, i64::from(i32::MAX)) as i32
    }

    pub fn physical_size(self, logical_size: WidgetSize) -> WidgetSize {
        WidgetSize {
            width: self.physical_length(logical_size.width),
            height: self.physical_length(logical_size.height),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    pub fn builtin_widget_type(&self) -> Option<WidgetKind> {
        WidgetKind::from_plugin_id(&self.plugin_id)
    }

    pub fn widget_type(&self) -> WidgetKind {
        self.builtin_widget_type().unwrap_or_default()
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyLayoutStore {
    #[serde(default)]
    pub settings: AppSettings,
    #[serde(default)]
    pub symbols: Vec<String>,
    #[serde(default)]
    pub widgets: HashMap<String, WidgetLayout>,
}

#[derive(Debug, Clone)]
pub enum PersistedLayoutStore {
    Current(LayoutStore),
    Legacy(LegacyLayoutStore),
}

#[derive(Debug)]
struct ParsedPersistedLayoutStore {
    store: PersistedLayoutStore,
    requires_schema_migration: bool,
}

#[derive(Serialize)]
struct VersionedLayoutStore<'a> {
    schema_version: u32,
    #[serde(flatten)]
    store: &'a LayoutStore,
}

impl<'de> Deserialize<'de> for PersistedLayoutStore {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        parse_persisted_layout_store_value(value)
            .map(|parsed| parsed.store)
            .map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutStoreReadErrorKind {
    Read,
    Parse,
}

#[derive(Debug)]
pub enum LayoutStoreReadError {
    Read {
        path: PathBuf,
        source: io::Error,
    },
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
}

impl LayoutStoreReadError {
    pub const fn kind(&self) -> LayoutStoreReadErrorKind {
        match self {
            Self::Read { .. } => LayoutStoreReadErrorKind::Read,
            Self::Parse { .. } => LayoutStoreReadErrorKind::Parse,
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            Self::Read { path, .. } | Self::Parse { path, .. } => path,
        }
    }
}

impl fmt::Display for LayoutStoreReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(formatter, "failed to read {}: {source}", path.display())
            }
            Self::Parse { path, source } => {
                write!(formatter, "failed to parse {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for LayoutStoreReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutStoreLoadSource {
    Current,
    Legacy(PathBuf),
    Missing,
    DefaultsAfterError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutStoreLoadWarning {
    pub kind: LayoutStoreReadErrorKind,
    pub path: PathBuf,
    pub error: String,
    pub preserved_path: Option<PathBuf>,
    pub preservation_error: Option<String>,
}

impl fmt::Display for LayoutStoreLoadWarning {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "saved state could not be loaded from {} ({}); defaults were loaded",
            self.path.display(),
            self.error
        )?;
        if let Some(path) = &self.preserved_path {
            write!(
                formatter,
                "; the unreadable file was preserved at {}",
                path.display()
            )
        } else if let Some(error) = &self.preservation_error {
            write!(
                formatter,
                "; the original file remains in place, but creating a backup failed: {error}"
            )
        } else {
            Ok(())
        }
    }
}

#[derive(Debug)]
pub struct LoadedLayoutStore {
    pub store: LayoutStore,
    pub source: LayoutStoreLoadSource,
    pub warning: Option<LayoutStoreLoadWarning>,
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum LanguagePreference {
    System,
    #[default]
    En,
    ZhHans,
    ZhHant,
    Es419,
    PtBr,
    Vi,
    Id,
    Tr,
    Ko,
    Ja,
    Ru,
    Ar,
}

const LANGUAGE_PREFERENCE_CONFIG_VARIANTS: &[&str] = &[
    "system", "en", "zh-CN", "zh-TW", "es-419", "pt-BR", "vi", "id", "tr", "ko", "ja", "ru", "ar",
    "zh_hans", "zh_hant", "es_419", "pt_br",
];

impl Serialize for LanguagePreference {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.config_tag())
    }
}

impl LanguagePreference {
    pub const ALL: [Self; 13] = [
        Self::System,
        Self::En,
        Self::ZhHans,
        Self::ZhHant,
        Self::Es419,
        Self::PtBr,
        Self::Vi,
        Self::Id,
        Self::Tr,
        Self::Ko,
        Self::Ja,
        Self::Ru,
        Self::Ar,
    ];

    pub const fn from_index(index: i32) -> Self {
        match index {
            1 => Self::En,
            2 => Self::ZhHans,
            3 => Self::ZhHant,
            4 => Self::Es419,
            5 => Self::PtBr,
            6 => Self::Vi,
            7 => Self::Id,
            8 => Self::Tr,
            9 => Self::Ko,
            10 => Self::Ja,
            11 => Self::Ru,
            12 => Self::Ar,
            _ => Self::System,
        }
    }

    pub const fn index(self) -> i32 {
        match self {
            Self::System => 0,
            Self::En => 1,
            Self::ZhHans => 2,
            Self::ZhHant => 3,
            Self::Es419 => 4,
            Self::PtBr => 5,
            Self::Vi => 6,
            Self::Id => 7,
            Self::Tr => 8,
            Self::Ko => 9,
            Self::Ja => 10,
            Self::Ru => 11,
            Self::Ar => 12,
        }
    }

    pub fn from_locale_tag(value: &str) -> Option<Self> {
        language_preference_from_locale_tag(value)
    }

    pub const fn config_tag(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::En => "en",
            Self::ZhHans => "zh-CN",
            Self::ZhHant => "zh-TW",
            Self::Es419 => "es-419",
            Self::PtBr => "pt-BR",
            Self::Vi => "vi",
            Self::Id => "id",
            Self::Tr => "tr",
            Self::Ko => "ko",
            Self::Ja => "ja",
            Self::Ru => "ru",
            Self::Ar => "ar",
        }
    }
}

impl<'de> Deserialize<'de> for LanguagePreference {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        language_preference_from_config_tag(&value)
            .ok_or_else(|| de::Error::unknown_variant(&value, LANGUAGE_PREFERENCE_CONFIG_VARIANTS))
    }
}

fn language_preference_from_config_tag(value: &str) -> Option<LanguagePreference> {
    let (language, _subtags) = normalized_locale_tag_parts(value);
    if language == "system" {
        return Some(LanguagePreference::System);
    }
    language_preference_from_locale_tag(value)
}

fn language_preference_from_locale_tag(value: &str) -> Option<LanguagePreference> {
    let (language, subtags) = normalized_locale_tag_parts(value);

    match language.as_str() {
        "en" => Some(LanguagePreference::En),
        "zh" | "cmn" | "yue" => Some(chinese_language_preference(&language, &subtags)),
        "es" if spanish_subtags_are_latin_american(&subtags) => Some(LanguagePreference::Es419),
        "pt" if subtags.iter().any(|subtag| subtag == "br") => Some(LanguagePreference::PtBr),
        "vi" => Some(LanguagePreference::Vi),
        "id" | "in" => Some(LanguagePreference::Id),
        "tr" => Some(LanguagePreference::Tr),
        "ko" => Some(LanguagePreference::Ko),
        "ja" => Some(LanguagePreference::Ja),
        "ru" => Some(LanguagePreference::Ru),
        "ar" => Some(LanguagePreference::Ar),
        _ => None,
    }
}

fn normalized_locale_tag_parts(value: &str) -> (String, Vec<String>) {
    let normalized = value
        .trim()
        .split(['.', '@'])
        .next()
        .unwrap_or_default()
        .replace('-', "_")
        .to_ascii_lowercase();
    let mut parts = normalized.split('_');
    let language = parts.next().unwrap_or_default().to_string();
    let subtags = parts.map(str::to_string).collect::<Vec<_>>();
    (language, subtags)
}

fn chinese_language_preference(language: &str, subtags: &[String]) -> LanguagePreference {
    if subtags.iter().any(|subtag| subtag == "hans") {
        return LanguagePreference::ZhHans;
    }
    if subtags.iter().any(|subtag| subtag == "hant") {
        return LanguagePreference::ZhHant;
    }
    if language == "yue"
        || subtags
            .iter()
            .any(|subtag| matches!(subtag.as_str(), "tw" | "hk" | "mo"))
    {
        LanguagePreference::ZhHant
    } else {
        LanguagePreference::ZhHans
    }
}

fn spanish_subtags_are_latin_american(subtags: &[String]) -> bool {
    subtags.iter().any(|subtag| {
        matches!(
            subtag.as_str(),
            "419"
                | "ar"
                | "bo"
                | "br"
                | "bz"
                | "cl"
                | "co"
                | "cr"
                | "cu"
                | "do"
                | "ec"
                | "gt"
                | "hn"
                | "mx"
                | "ni"
                | "pa"
                | "pe"
                | "pr"
                | "py"
                | "sv"
                | "us"
                | "uy"
                | "ve"
        )
    })
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
    pub tray_market_enabled: bool,
    #[serde(default = "default_tray_market_symbols")]
    pub tray_market_symbols: Vec<String>,
    #[serde(default = "default_tray_market_switch_interval_seconds")]
    pub tray_market_switch_interval_seconds: i32,
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
            auto_start_enabled: false,
            show_main_window_on_startup: default_show_main_window_on_startup(),
            shortcut: ShortcutPreference::default(),
            theme: ThemePreference::default(),
            language: LanguagePreference::default(),
            tray_icon_enabled: default_tray_icon_enabled(),
            tray_hover_display_enabled: false,
            tray_market_enabled: false,
            tray_market_symbols: default_tray_market_symbols(),
            tray_market_switch_interval_seconds: default_tray_market_switch_interval_seconds(),
            network_proxy_enabled: false,
            network_proxy_url: String::new(),
            alert_rules: Vec::new(),
        }
    }
}

impl AppSettings {
    pub fn normalized(self) -> Self {
        let network_proxy_url = normalize_network_proxy_url(self.network_proxy_url);
        let proxy_has_userinfo = network_proxy_url_has_userinfo(&network_proxy_url);
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
            auto_start_enabled: self.auto_start_enabled,
            show_main_window_on_startup: self.show_main_window_on_startup,
            shortcut: self.shortcut.normalized(),
            theme: self.theme,
            language: self.language,
            tray_icon_enabled: self.tray_icon_enabled,
            tray_hover_display_enabled: self.tray_hover_display_enabled,
            tray_market_enabled: self.tray_market_enabled,
            tray_market_symbols: normalize_tray_market_symbols(self.tray_market_symbols),
            tray_market_switch_interval_seconds: clamp_tray_market_switch_interval_seconds(
                self.tray_market_switch_interval_seconds,
            ),
            network_proxy_enabled: self.network_proxy_enabled && !proxy_has_userinfo,
            network_proxy_url: if proxy_has_userinfo {
                String::new()
            } else {
                network_proxy_url
            },
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

pub fn default_tray_market_symbols() -> Vec<String> {
    default_market_symbols().into_iter().take(1).collect()
}

pub const fn default_tray_market_switch_interval_seconds() -> i32 {
    DEFAULT_TRAY_MARKET_SWITCH_INTERVAL_SECONDS
}

pub fn clamp_tray_market_switch_interval_seconds(value: i32) -> i32 {
    value.clamp(
        MIN_TRAY_MARKET_SWITCH_INTERVAL_SECONDS,
        MAX_TRAY_MARKET_SWITCH_INTERVAL_SECONDS,
    )
}

pub fn normalize_tray_market_symbols(symbols: Vec<String>) -> Vec<String> {
    normalized_symbols_with_limit(
        symbols,
        MAX_TRAY_MARKET_SYMBOLS,
        default_tray_market_symbols(),
    )
}

pub fn normalize_network_proxy_url(proxy_url: String) -> String {
    proxy_url.trim().to_string()
}

pub fn network_proxy_url_has_userinfo(proxy_url: &str) -> bool {
    let authority_and_path = proxy_url
        .split_once("://")
        .map(|(_, remainder)| remainder)
        .unwrap_or(proxy_url);
    authority_and_path
        .split(['/', '?', '#'])
        .next()
        .is_some_and(|authority| authority.contains('@'))
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

pub fn widget_show_header(instance: &WidgetInstance) -> bool {
    widget_config_show_header(&instance.config)
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
    show_header: bool,
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
    config.insert(
        WIDGET_CONFIG_SHOW_HEADER.to_string(),
        Value::Bool(show_header),
    );
}

pub fn widget_integer_parameter(
    instance: &WidgetInstance,
    key: &str,
    default: i32,
    minimum: i32,
    maximum: i32,
) -> i32 {
    widget_plugin_parameter(instance, key)
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .unwrap_or(default)
        .clamp(minimum, maximum)
}

pub fn widget_plugin_parameter<'a>(instance: &'a WidgetInstance, key: &str) -> Option<&'a Value> {
    instance
        .config
        .as_object()
        .and_then(|config| config.get(WIDGET_CONFIG_PLUGIN_PARAMETERS))
        .and_then(Value::as_object)
        .and_then(|parameters| parameters.get(key))
}

pub fn set_widget_plugin_parameter(instance: &mut WidgetInstance, key: &str, value: Value) {
    let config = widget_config_object_mut(instance);
    let parameters = config
        .entry(WIDGET_CONFIG_PLUGIN_PARAMETERS.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !parameters.is_object() {
        *parameters = Value::Object(Map::new());
    }
    if let Some(parameters) = parameters.as_object_mut() {
        parameters.insert(key.to_string(), value);
    }
}

pub fn set_widget_integer_parameter(
    instance: &mut WidgetInstance,
    key: &str,
    value: i32,
    minimum: i32,
    maximum: i32,
) {
    set_widget_plugin_parameter(
        instance,
        key,
        Value::Number(value.clamp(minimum, maximum).into()),
    );
}

fn widget_config_show_coin_logos(config: &serde_json::Value) -> bool {
    widget_config_bool(config, WIDGET_CONFIG_SHOW_COIN_LOGOS, true)
}

fn widget_config_hide_quote_asset(config: &serde_json::Value) -> bool {
    widget_config_bool(config, WIDGET_CONFIG_HIDE_QUOTE_ASSET, false)
}

fn widget_config_show_header(config: &serde_json::Value) -> bool {
    widget_config_bool(config, WIDGET_CONFIG_SHOW_HEADER, true)
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
    let persisted = VersionedLayoutStore {
        schema_version: LAYOUT_STORE_SCHEMA_VERSION,
        store,
    };
    let contents = format!("{}\n", serde_json::to_string_pretty(&persisted)?);
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

pub fn try_load_persisted_layout_store(
    path: &Path,
) -> std::result::Result<Option<PersistedLayoutStore>, LayoutStoreReadError> {
    try_load_persisted_layout_store_with_metadata(path)
        .map(|persisted| persisted.map(|persisted| persisted.store))
}

fn try_load_persisted_layout_store_with_metadata(
    path: &Path,
) -> std::result::Result<Option<ParsedPersistedLayoutStore>, LayoutStoreReadError> {
    let contents = match fs::read(path) {
        Ok(contents) => contents,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(LayoutStoreReadError::Read {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    parse_persisted_layout_store(&contents)
        .map(Some)
        .map_err(|source| LayoutStoreReadError::Parse {
            path: path.to_path_buf(),
            source,
        })
}

fn parse_persisted_layout_store(
    contents: &[u8],
) -> std::result::Result<ParsedPersistedLayoutStore, serde_json::Error> {
    let value = serde_json::from_slice::<Value>(contents)?;
    parse_persisted_layout_store_value(value).map_err(|message| {
        serde_json::Error::io(io::Error::new(io::ErrorKind::InvalidData, message))
    })
}

fn parse_persisted_layout_store_value(
    value: Value,
) -> std::result::Result<ParsedPersistedLayoutStore, String> {
    let Value::Object(mut object) = value else {
        return Err("layout state must be a JSON object".to_string());
    };

    if let Some(schema_value) = object.remove(LAYOUT_STORE_SCHEMA_VERSION_FIELD) {
        let schema_version = schema_value.as_u64().ok_or_else(|| {
            format!("{LAYOUT_STORE_SCHEMA_VERSION_FIELD} must be an unsigned integer")
        })?;
        if ![
            u64::from(PREVIOUS_LAYOUT_STORE_SCHEMA_VERSION),
            u64::from(LAYOUT_STORE_SCHEMA_VERSION),
        ]
        .contains(&schema_version)
        {
            return Err(format!(
                "unsupported layout state schema version {schema_version}; expected {PREVIOUS_LAYOUT_STORE_SCHEMA_VERSION} or {LAYOUT_STORE_SCHEMA_VERSION}"
            ));
        }
        for required_field in [
            "settings",
            "selected_widget_id",
            "next_widget_number",
            "widgets",
        ] {
            if !object.contains_key(required_field) {
                return Err(format!(
                    "layout state schema {schema_version} is missing required field `{required_field}`"
                ));
            }
        }
        let store = serde_json::from_value::<LayoutStore>(Value::Object(object))
            .map_err(|error| error.to_string())?;
        return Ok(ParsedPersistedLayoutStore {
            store: PersistedLayoutStore::Current(store),
            requires_schema_migration: schema_version != u64::from(LAYOUT_STORE_SCHEMA_VERSION),
        });
    }

    // Published unversioned current files serialize widgets as an array and
    // include current-only selection/counter fields. Legacy files use a widget
    // map plus the former store-wide symbols field. Keep these discriminators
    // explicit so serde defaults cannot turn arbitrary objects into valid state.
    let is_current = object.contains_key("selected_widget_id")
        || object.contains_key("next_widget_number")
        || matches!(object.get("widgets"), Some(Value::Array(_)));
    let is_legacy = object.contains_key("symbols")
        || matches!(object.get("widgets"), Some(Value::Object(_)))
        || (!is_current && object.contains_key("settings"));

    if is_current && is_legacy {
        return Err("layout state mixes current and legacy top-level fields".to_string());
    }

    if is_current {
        let store = serde_json::from_value::<LayoutStore>(Value::Object(object))
            .map_err(|error| error.to_string())?;
        return Ok(ParsedPersistedLayoutStore {
            store: PersistedLayoutStore::Current(store),
            requires_schema_migration: true,
        });
    }

    if is_legacy {
        let store = serde_json::from_value::<LegacyLayoutStore>(Value::Object(object))
            .map_err(|error| error.to_string())?;
        return Ok(ParsedPersistedLayoutStore {
            store: PersistedLayoutStore::Legacy(store),
            requires_schema_migration: true,
        });
    }

    Err("layout state has no recognized current or legacy fields".to_string())
}

pub fn load_persisted_layout_store(path: &Path) -> Option<PersistedLayoutStore> {
    match try_load_persisted_layout_store(path) {
        Ok(persisted) => persisted,
        Err(error) => {
            eprintln!("{error}");
            None
        }
    }
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
    let loaded =
        load_layout_store_with_diagnostics(path, requested_widget_count, catalog, desktop_size);
    if let Some(warning) = &loaded.warning {
        eprintln!("layout state recovery warning: {warning}");
    }
    loaded.store
}

pub fn load_layout_store_with_diagnostics(
    path: &Path,
    requested_widget_count: usize,
    catalog: &[WidgetDefinition],
    desktop_size: (i32, i32),
) -> LoadedLayoutStore {
    load_layout_store_with_diagnostics_and_work_areas(
        path,
        requested_widget_count,
        catalog,
        desktop_size,
        &[DesktopWorkArea::from_desktop_size(desktop_size)],
    )
}

pub fn load_layout_store_with_diagnostics_and_work_areas(
    path: &Path,
    requested_widget_count: usize,
    catalog: &[WidgetDefinition],
    desktop_size: (i32, i32),
    work_areas: &[DesktopWorkArea],
) -> LoadedLayoutStore {
    match try_load_persisted_layout_store_with_metadata(path) {
        Ok(Some(parsed)) => {
            let original_store = match &parsed.store {
                PersistedLayoutStore::Current(store) => Some(store.clone()),
                PersistedLayoutStore::Legacy(_) => None,
            };
            let loaded = finish_loaded_layout_store(
                parsed.store,
                LayoutStoreLoadSource::Current,
                requested_widget_count,
                catalog,
                desktop_size,
                work_areas,
            );
            if parsed.requires_schema_migration || original_store.as_ref() != Some(&loaded.store) {
                if let Err(error) = save_layout_store(path, &loaded.store) {
                    eprintln!(
                        "failed to save normalized layout state at {}: {error:#}",
                        path.display()
                    );
                }
            }
            return loaded;
        }
        Ok(None) => {}
        Err(error) => {
            return default_layout_store_after_error(
                error,
                requested_widget_count,
                catalog,
                desktop_size,
                work_areas,
            );
        }
    }

    for legacy_path in legacy_layout_store_paths(path) {
        if legacy_path.as_path() == path {
            continue;
        }
        match try_load_persisted_layout_store_with_metadata(&legacy_path) {
            Ok(Some(parsed)) => {
                let loaded = finish_loaded_layout_store(
                    parsed.store,
                    LayoutStoreLoadSource::Legacy(legacy_path),
                    requested_widget_count,
                    catalog,
                    desktop_size,
                    work_areas,
                );
                if let Err(error) = save_layout_store(path, &loaded.store) {
                    eprintln!(
                        "failed to save migrated layout state to {}: {error:#}",
                        path.display()
                    );
                }
                return loaded;
            }
            Ok(None) => {}
            Err(error) => {
                return default_layout_store_after_error(
                    error,
                    requested_widget_count,
                    catalog,
                    desktop_size,
                    work_areas,
                );
            }
        }
    }

    let mut store = LayoutStore::default();
    normalize_store_with_catalog_and_work_areas(
        &mut store,
        requested_widget_count,
        catalog,
        desktop_size,
        work_areas,
    );
    LoadedLayoutStore {
        store,
        source: LayoutStoreLoadSource::Missing,
        warning: None,
    }
}

fn finish_loaded_layout_store(
    persisted: PersistedLayoutStore,
    source: LayoutStoreLoadSource,
    requested_widget_count: usize,
    catalog: &[WidgetDefinition],
    desktop_size: (i32, i32),
    work_areas: &[DesktopWorkArea],
) -> LoadedLayoutStore {
    let mut store = match persisted {
        PersistedLayoutStore::Current(store) => store,
        PersistedLayoutStore::Legacy(store) => {
            migrate_legacy_store_in_work_areas(store, desktop_size, work_areas)
        }
    };
    normalize_store_with_catalog_and_work_areas(
        &mut store,
        requested_widget_count,
        catalog,
        desktop_size,
        work_areas,
    );
    LoadedLayoutStore {
        store,
        source,
        warning: None,
    }
}

fn default_layout_store_after_error(
    error: LayoutStoreReadError,
    requested_widget_count: usize,
    catalog: &[WidgetDefinition],
    desktop_size: (i32, i32),
    work_areas: &[DesktopWorkArea],
) -> LoadedLayoutStore {
    let warning = layout_store_load_warning(&error);
    let mut store = LayoutStore::default();
    normalize_store_with_catalog_and_work_areas(
        &mut store,
        requested_widget_count,
        catalog,
        desktop_size,
        work_areas,
    );
    LoadedLayoutStore {
        store,
        source: LayoutStoreLoadSource::DefaultsAfterError,
        warning: Some(warning),
    }
}

fn layout_store_load_warning(error: &LayoutStoreReadError) -> LayoutStoreLoadWarning {
    let path = error.path().to_path_buf();
    let preservation = preserve_unreadable_layout_store(&path);
    LayoutStoreLoadWarning {
        kind: error.kind(),
        path,
        error: error.to_string(),
        preserved_path: preservation.as_ref().ok().cloned(),
        preservation_error: preservation.err().map(|error| format!("{error:#}")),
    }
}

fn preserve_unreadable_layout_store(path: &Path) -> Result<PathBuf> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .unwrap_or_else(|| OsStr::new("layout-store"));
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let counter = SAVE_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut backup_name = OsString::from(file_name);
    backup_name.push(format!(
        ".corrupt-{timestamp}-{}-{counter}",
        std::process::id()
    ));
    let backup_path = parent.join(backup_name);
    fs::copy(path, &backup_path).with_context(|| {
        format!(
            "failed to preserve unreadable state {} as {}",
            path.display(),
            backup_path.display()
        )
    })?;
    Ok(backup_path)
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
    migrate_legacy_store_in_work_areas(
        legacy,
        desktop_size,
        &[DesktopWorkArea::from_desktop_size(desktop_size)],
    )
}

fn migrate_legacy_store_in_work_areas(
    legacy: LegacyLayoutStore,
    desktop_size: (i32, i32),
    work_areas: &[DesktopWorkArea],
) -> LayoutStore {
    let settings = legacy.settings.clone().normalized();
    let has_explicit_symbols = !legacy.symbols.is_empty();
    let symbols = normalized_symbols_for_type(WidgetKind::QuoteBoard, legacy.symbols);
    let mut widgets = legacy.widgets.into_iter().collect::<Vec<_>>();
    widgets.sort_by(|left, right| left.0.cmp(&right.0));
    if widgets.is_empty() && has_explicit_symbols {
        widgets.push((
            format!("{}-1", WidgetKind::QuoteBoard.id_prefix()),
            WidgetLayout::default(),
        ));
    }

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
                name: default_persisted_widget_name(
                    WidgetKind::QuoteBoard,
                    u64::try_from(index).unwrap_or(u64::MAX).saturating_add(1),
                ),
                visible: true,
                layout,
                symbols: symbols.clone(),
                config: default_widget_config(),
            })
            .collect(),
    };
    normalize_store_with_catalog_and_work_areas(&mut store, 0, &[], desktop_size, work_areas);
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
    normalize_store_with_catalog_and_work_areas(
        store,
        requested_widget_count,
        catalog,
        desktop_size,
        &[DesktopWorkArea::from_desktop_size(desktop_size)],
    );
}

pub fn normalize_store_with_catalog_and_work_areas(
    store: &mut LayoutStore,
    requested_widget_count: usize,
    catalog: &[WidgetDefinition],
    desktop_size: (i32, i32),
    work_areas: &[DesktopWorkArea],
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

    let mut reserved_ids = store
        .widgets
        .iter()
        .map(|instance| instance.id.trim())
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .collect::<HashSet<_>>();
    let mut seen_ids = HashSet::new();
    let mut next_widget_number = store.next_widget_number;
    let mut max_suffix = 0;
    for (index, instance) in store.widgets.iter_mut().enumerate() {
        if instance.plugin_id.trim().is_empty() {
            instance.plugin_id = instance
                .legacy_widget_type
                .take()
                .unwrap_or_default()
                .plugin_id()
                .to_string();
        } else {
            // A non-empty plug-in id is the current persistence contract.  The
            // legacy field must never replace an unavailable third-party id.
            instance.legacy_widget_type = None;
        }
        let builtin_widget_type = instance.builtin_widget_type();
        let plugin_definition = definition_for_instance(instance, catalog);
        let opaque_plugin_state = builtin_widget_type.is_none() && plugin_definition.is_none();
        if !opaque_plugin_state && instance.config.is_null() {
            instance.config = default_widget_config();
        }

        if instance.id.trim().is_empty() || seen_ids.contains(&instance.id) {
            loop {
                let number = next_widget_number;
                next_widget_number = advance_widget_number(number);
                let id_prefix = builtin_widget_type
                    .map(WidgetKind::id_prefix)
                    .map(str::to_string)
                    .unwrap_or_else(|| plugin_instance_id_prefix(&instance.plugin_id));
                let candidate = format!("{id_prefix}-{number}");
                if !reserved_ids.contains(&candidate) {
                    reserved_ids.insert(candidate.clone());
                    instance.id = candidate;
                    break;
                }
            }
        }
        seen_ids.insert(instance.id.clone());
        max_suffix = max_suffix.max(widget_id_suffix(&instance.id).unwrap_or(0));

        if instance.name.trim().is_empty() && !opaque_plugin_state {
            let number = u64::try_from(index).unwrap_or(u64::MAX).saturating_add(1);
            instance.name = if let Some(widget_type) = builtin_widget_type {
                default_persisted_widget_name(widget_type, number)
            } else if let Some(definition) = plugin_definition {
                format!("{} {number}", definition.name)
            } else {
                instance.name.clone()
            };
        }
        if !opaque_plugin_state {
            instance.symbols = normalized_symbols_for_instance(instance, catalog);
        }
        if !opaque_plugin_state {
            let default_size = default_widget_size_for_instance(instance, catalog);
            let has_missing_scale_with_persisted_size =
                instance.layout.scale_percent <= 0 && layout_has_persisted_size(&instance.layout);
            let defer_quote_board_size_migration = has_missing_scale_with_persisted_size
                && builtin_widget_type == Some(WidgetKind::QuoteBoard);
            let defer_dynamic_size_migration = has_missing_scale_with_persisted_size
                && plugin_definition
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
                let layout = default_layout_for_size_in_work_areas(
                    index,
                    instance_size,
                    store.settings.clone(),
                    desktop_size,
                    work_areas,
                );
                instance.layout.x = layout.x;
                instance.layout.y = layout.y;
            }
            if layout_needs_recovery_for_size_in_work_areas(
                &instance.layout,
                instance_size,
                work_areas,
            ) {
                let layout = default_layout_for_size_in_work_areas(
                    index,
                    instance_size,
                    store.settings.clone(),
                    desktop_size,
                    work_areas,
                );
                instance.layout.x = layout.x;
                instance.layout.y = layout.y;
            }
            instance.layout.opacity_percent = clamp_opacity(instance.layout.opacity_percent);
        }
    }

    store.next_widget_number =
        next_widget_number_after_normalization(next_widget_number, max_suffix);

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
    layout_for_instance_in_work_areas(
        instance,
        index,
        settings,
        catalog,
        desktop_size,
        &[DesktopWorkArea::from_desktop_size(desktop_size)],
    )
}

pub fn layout_for_instance_in_work_areas(
    instance: &WidgetInstance,
    index: usize,
    settings: AppSettings,
    catalog: &[WidgetDefinition],
    desktop_size: (i32, i32),
    work_areas: &[DesktopWorkArea],
) -> WidgetLayout {
    let size = widget_size_for_instance(instance, catalog);
    if layout_needs_recovery_for_size_in_work_areas(&instance.layout, size, work_areas) {
        let mut layout = default_layout_for_size_in_work_areas(
            index,
            size,
            settings.clone(),
            desktop_size,
            work_areas,
        );
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
    default_layout_for_size_in_work_areas(
        slot,
        size,
        settings,
        desktop_size,
        &[DesktopWorkArea::from_desktop_size(desktop_size)],
    )
}

pub fn default_layout_for_size_in_work_areas(
    slot: usize,
    size: WidgetSize,
    settings: AppSettings,
    desktop_size: (i32, i32),
    work_areas: &[DesktopWorkArea],
) -> WidgetLayout {
    let size = clamp_widget_size(size);
    let ordered_work_areas = ordered_work_areas(work_areas, desktop_size);
    let mut remaining_slot = slot;
    let mut selected = ordered_work_areas[0];
    let mut selected_row = 0;
    let mut selected_column = 0;

    for work_area in &ordered_work_areas {
        let physical_size = work_area.physical_size(size);
        let margin_x = work_area.physical_length(DEFAULT_LAYOUT_MARGIN_X);
        let margin_y = work_area.physical_length(DEFAULT_LAYOUT_MARGIN_Y);
        let gap = work_area.physical_length(DEFAULT_LAYOUT_GAP);
        let usable_width = (work_area.width - margin_x.saturating_mul(2)).max(physical_size.width);
        let usable_height =
            (work_area.height - margin_y.saturating_mul(2)).max(physical_size.height);
        let columns = ((usable_width + gap) / (physical_size.width + gap)).max(1) as usize;
        let rows = ((usable_height + gap) / (physical_size.height + gap)).max(1) as usize;
        let capacity = rows.saturating_mul(columns).max(1);
        if remaining_slot < capacity {
            selected = *work_area;
            selected_row = remaining_slot % rows;
            selected_column = remaining_slot / rows;
            break;
        }
        remaining_slot = remaining_slot.saturating_sub(capacity);
    }

    let physical_size = selected.physical_size(size);
    let margin_x = selected.physical_length(DEFAULT_LAYOUT_MARGIN_X);
    let margin_y = selected.physical_length(DEFAULT_LAYOUT_MARGIN_Y);
    let gap = selected.physical_length(DEFAULT_LAYOUT_GAP);
    let row_stride = physical_size.height.saturating_add(gap);
    let column_stride = physical_size.width.saturating_add(gap);
    let x = (selected.x + selected.width
        - margin_x
        - physical_size.width
        - column_stride.saturating_mul(selected_column as i32))
    .max(selected.x + margin_x);
    let y = selected.y + margin_y + row_stride.saturating_mul(selected_row as i32);

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
    next_available_layout_for_size_with_catalog_and_work_areas(
        store,
        size,
        settings,
        catalog,
        desktop_size,
        &[DesktopWorkArea::from_desktop_size(desktop_size)],
    )
}

pub fn next_available_layout_for_size_with_catalog_and_work_areas(
    store: &LayoutStore,
    size: WidgetSize,
    settings: AppSettings,
    catalog: &[WidgetDefinition],
    desktop_size: (i32, i32),
    work_areas: &[DesktopWorkArea],
) -> WidgetLayout {
    let scan_slots = DEFAULT_LAYOUT_SCAN_SLOTS.saturating_mul(work_areas.len().max(1));
    for slot in 0..scan_slots {
        let candidate = default_layout_for_size_in_work_areas(
            slot,
            size,
            settings.clone(),
            desktop_size,
            work_areas,
        );
        if !layout_overlaps_existing_for_size(
            store,
            &candidate,
            size,
            settings.clone(),
            catalog,
            desktop_size,
            work_areas,
        ) {
            return candidate;
        }
    }

    default_layout_for_size_in_work_areas(
        store.widgets.len(),
        size,
        settings,
        desktop_size,
        work_areas,
    )
}

pub fn reset_widget_positions_in_work_areas(
    store: &mut LayoutStore,
    catalog: &[WidgetDefinition],
    desktop_size: (i32, i32),
    work_areas: &[DesktopWorkArea],
) -> bool {
    let sizes = store
        .widgets
        .iter()
        .map(|instance| widget_size_for_instance(instance, catalog))
        .collect::<Vec<_>>();
    let positions = packed_widget_positions(&sizes, desktop_size, work_areas);
    let mut changed = false;
    for (instance, (x, y)) in store.widgets.iter_mut().zip(positions) {
        if instance.layout.x != x || instance.layout.y != y {
            instance.layout.x = x;
            instance.layout.y = y;
            changed = true;
        }
    }
    changed
}

fn packed_widget_positions(
    sizes: &[WidgetSize],
    desktop_size: (i32, i32),
    work_areas: &[DesktopWorkArea],
) -> Vec<(i32, i32)> {
    let ordered_work_areas = ordered_work_areas(work_areas, desktop_size);
    let clamped_sizes = sizes
        .iter()
        .copied()
        .map(clamp_widget_size)
        .collect::<Vec<_>>();
    let fallback_work_area = ordered_work_areas
        .iter()
        .copied()
        .max_by_key(|area| {
            (
                i64::from(area.x) + i64::from(area.width),
                i64::from(area.y) + i64::from(area.height),
                area.is_primary,
            )
        })
        .unwrap_or(ordered_work_areas[0]);
    let mut placed = Vec::<WidgetRect>::new();
    let mut positions = Vec::with_capacity(sizes.len());

    for (size_index, &logical_size) in clamped_sizes.iter().enumerate() {
        let mut selected = None;
        'areas: for work_area in &ordered_work_areas {
            let size = work_area.physical_size(logical_size);
            let margin_x = work_area.physical_length(DEFAULT_LAYOUT_MARGIN_X);
            let margin_y = work_area.physical_length(DEFAULT_LAYOUT_MARGIN_Y);
            let gap = work_area.physical_length(DEFAULT_LAYOUT_GAP);
            let left = work_area.x.saturating_add(margin_x);
            let top = work_area.y.saturating_add(margin_y);
            let right = work_area
                .x
                .saturating_add(work_area.width)
                .saturating_sub(margin_x);
            let bottom = work_area
                .y
                .saturating_add(work_area.height)
                .saturating_sub(margin_y);
            let mut candidate_x = vec![right.saturating_sub(size.width)];
            let mut candidate_y = vec![top];
            for rect in &placed {
                candidate_x.push(rect.x.saturating_sub(gap).saturating_sub(size.width));
                candidate_y.push(rect.y.saturating_add(rect.height).saturating_add(gap));
            }
            candidate_x.sort_unstable_by(|left, right| right.cmp(left));
            candidate_x.dedup();
            candidate_y.sort_unstable();
            candidate_y.dedup();

            for y in candidate_y {
                for &x in &candidate_x {
                    let candidate = WidgetRect {
                        x,
                        y,
                        width: size.width,
                        height: size.height,
                    };
                    if rect_fits_bounds(candidate, left, top, right, bottom)
                        && !placed
                            .iter()
                            .any(|existing| candidate.overlaps(*existing, gap))
                    {
                        selected = Some(candidate);
                        break 'areas;
                    }
                }
            }
        }

        let selected = selected.unwrap_or_else(|| {
            // Windows extend toward positive x/y from their origin. Using the
            // rightmost work area keeps oversized fallbacks from covering a
            // neighboring monitor to its right.
            let work_area = fallback_work_area;
            let size = work_area.physical_size(logical_size);
            let margin_x = work_area.physical_length(DEFAULT_LAYOUT_MARGIN_X);
            let margin_y = work_area.physical_length(DEFAULT_LAYOUT_MARGIN_Y);
            let left = work_area.x.saturating_add(margin_x);
            let top = work_area.y.saturating_add(margin_y);
            let maximum_x = work_area
                .x
                .saturating_add(work_area.width)
                .saturating_sub(MIN_VISIBLE_WIDGET_PX);
            let maximum_y = work_area
                .y
                .saturating_add(work_area.height)
                .saturating_sub(MIN_VISIBLE_WIDGET_PX);
            let fallback_slots = i32::try_from(sizes.len().saturating_sub(1)).unwrap_or(i32::MAX);
            let horizontal_rank = clamped_sizes
                .iter()
                .enumerate()
                .filter(|(index, candidate)| {
                    candidate.width < logical_size.width
                        || (candidate.width == logical_size.width && *index < size_index)
                })
                .count();
            let vertical_rank = clamped_sizes
                .iter()
                .enumerate()
                .filter(|(index, candidate)| {
                    candidate.height > logical_size.height
                        || (candidate.height == logical_size.height && *index > size_index)
                })
                .count();
            let horizontal_rank = i32::try_from(horizontal_rank).unwrap_or(i32::MAX);
            let vertical_rank = i32::try_from(vertical_rank).unwrap_or(i32::MAX);
            let maximum_horizontal_span = maximum_x.saturating_sub(left).max(0);
            let horizontal_step = if fallback_slots > 0 && maximum_horizontal_span > 0 {
                maximum_horizontal_span / fallback_slots
            } else {
                0
            };
            let maximum_vertical_offset = maximum_y.saturating_sub(top).max(0);
            let vertical_step = if fallback_slots > 0 && maximum_vertical_offset > 0 {
                maximum_vertical_offset / fallback_slots
            } else {
                0
            };
            // When every work area is full, visibility is more important than
            // avoiding overlap. Width and height ranks run along opposing axes,
            // leaving exposed edges even for mixed sizes. Keeping every origin
            // inside this work area also keeps its DPI calculation stable.
            let x = left.saturating_add(
                horizontal_rank
                    .saturating_mul(horizontal_step)
                    .min(maximum_horizontal_span),
            );
            let y = top.saturating_add(
                vertical_rank
                    .saturating_mul(vertical_step)
                    .min(maximum_vertical_offset),
            );
            WidgetRect {
                x,
                y,
                width: size.width,
                height: size.height,
            }
        });
        positions.push((selected.x, selected.y));
        placed.push(selected);
    }

    positions
}

fn rect_fits_bounds(rect: WidgetRect, left: i32, top: i32, right: i32, bottom: i32) -> bool {
    i64::from(rect.x) >= i64::from(left)
        && i64::from(rect.y) >= i64::from(top)
        && i64::from(rect.x) + i64::from(rect.width) <= i64::from(right)
        && i64::from(rect.y) + i64::from(rect.height) <= i64::from(bottom)
}

pub fn layout_has_visible_area_for_size(
    layout: &WidgetLayout,
    size: WidgetSize,
    desktop_size: (i32, i32),
) -> bool {
    layout_has_visible_area_for_size_in_work_areas(
        layout,
        size,
        &[DesktopWorkArea::from_desktop_size(desktop_size)],
    )
}

pub fn layout_has_visible_area_for_size_in_work_areas(
    layout: &WidgetLayout,
    size: WidgetSize,
    work_areas: &[DesktopWorkArea],
) -> bool {
    let size_work_area = work_area_for_position(layout.x, layout.y, work_areas)
        .copied()
        .unwrap_or_else(|| DesktopWorkArea::from_desktop_size((1, 1)));
    let physical_size = size_work_area.physical_size(size);
    work_areas.iter().any(|work_area| {
        if work_area.width <= 0 || work_area.height <= 0 {
            return false;
        }
        let widget_left = i64::from(layout.x);
        let widget_top = i64::from(layout.y);
        let widget_right = widget_left + i64::from(physical_size.width);
        let widget_bottom = widget_top + i64::from(physical_size.height);
        let work_left = i64::from(work_area.x);
        let work_top = i64::from(work_area.y);
        let work_right = work_left + i64::from(work_area.width);
        let work_bottom = work_top + i64::from(work_area.height);
        let visible_width = widget_right.min(work_right) - widget_left.max(work_left);
        let visible_height = widget_bottom.min(work_bottom) - widget_top.max(work_top);

        visible_width >= i64::from(MIN_VISIBLE_WIDGET_PX)
            && visible_height >= i64::from(MIN_VISIBLE_WIDGET_PX)
    })
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
    layout_needs_recovery_for_size_in_work_areas(
        layout,
        size,
        &[DesktopWorkArea::from_desktop_size(desktop_size)],
    )
}

pub fn layout_needs_recovery_for_size_in_work_areas(
    layout: &WidgetLayout,
    size: WidgetSize,
    work_areas: &[DesktopWorkArea],
) -> bool {
    is_parked_position(layout.x, layout.y)
        || !layout_has_visible_area_for_size_in_work_areas(layout, size, work_areas)
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
    remove_widget_from_store_by_id_in_work_areas(
        store,
        widget_id,
        desktop_size,
        &[DesktopWorkArea::from_desktop_size(desktop_size)],
    )
}

pub fn remove_widget_from_store_by_id_in_work_areas(
    store: &mut LayoutStore,
    widget_id: &str,
    desktop_size: (i32, i32),
    work_areas: &[DesktopWorkArea],
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
    normalize_store_with_catalog_and_work_areas(store, 0, &[], desktop_size, work_areas);
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
    if instance.builtin_widget_type().is_none()
        && definition_for_instance(instance, catalog).is_none()
    {
        return instance.symbols.clone();
    }
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
    if instance.builtin_widget_type().is_none()
        && definition_for_instance(instance, catalog).is_none()
    {
        return clamp_widget_size(WidgetSize {
            width: if instance.layout.width > 0 {
                instance.layout.width
            } else {
                QUOTE_BOARD_WIDTH
            },
            height: if instance.layout.height > 0 {
                instance.layout.height
            } else {
                QUOTE_BOARD_HEIGHT
            },
        });
    }
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
    let builtin_type = instance.builtin_widget_type();
    let default_size = definition_for_instance(instance, catalog)
        .map(|definition| widget_definition_size_for_symbols(definition, &instance.symbols))
        .or_else(|| builtin_type.map(WidgetKind::default_size))
        .unwrap_or_else(|| {
            clamp_widget_size(WidgetSize {
                width: if instance.layout.width > 0 {
                    instance.layout.width
                } else {
                    QUOTE_BOARD_WIDTH
                },
                height: if instance.layout.height > 0 {
                    instance.layout.height
                } else {
                    QUOTE_BOARD_HEIGHT
                },
            })
        });
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
            widget_config_show_header(config),
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
    work_areas: &[DesktopWorkArea],
) -> bool {
    let fallback_work_area = DesktopWorkArea::from_desktop_size(desktop_size);
    let candidate_work_area = work_area_for_position(candidate.x, candidate.y, work_areas)
        .copied()
        .unwrap_or(fallback_work_area);
    let candidate_rect = widget_rect_for_size(candidate, candidate_work_area.physical_size(size));
    store.widgets.iter().enumerate().any(|(index, instance)| {
        if is_parked_position(instance.layout.x, instance.layout.y) {
            return false;
        }
        let layout = layout_for_instance_in_work_areas(
            instance,
            index,
            settings.clone(),
            catalog,
            desktop_size,
            work_areas,
        );
        let instance_work_area = work_area_for_position(layout.x, layout.y, work_areas)
            .copied()
            .unwrap_or(fallback_work_area);
        let gap = candidate_work_area
            .physical_length(DEFAULT_LAYOUT_GAP)
            .max(instance_work_area.physical_length(DEFAULT_LAYOUT_GAP));
        candidate_rect.overlaps(
            widget_rect_for_size(
                &layout,
                instance_work_area.physical_size(widget_size_for_instance(instance, catalog)),
            ),
            gap,
        )
    })
}

fn ordered_work_areas(
    work_areas: &[DesktopWorkArea],
    desktop_size: (i32, i32),
) -> Vec<DesktopWorkArea> {
    let mut ordered = work_areas
        .iter()
        .copied()
        .filter(|area| area.width > 0 && area.height > 0)
        .collect::<Vec<_>>();
    if ordered.is_empty() {
        ordered.push(DesktopWorkArea::from_desktop_size(desktop_size));
    }
    ordered.sort_by_key(|area| {
        (
            !area.is_primary,
            area.x,
            area.y,
            area.width,
            area.height,
            area.dpi,
        )
    });
    ordered.dedup();
    ordered
}

fn work_area_for_position(
    x: i32,
    y: i32,
    work_areas: &[DesktopWorkArea],
) -> Option<&DesktopWorkArea> {
    work_areas
        .iter()
        .filter(|area| area.width > 0 && area.height > 0)
        .find(|area| {
            let right = i64::from(area.x) + i64::from(area.width);
            let bottom = i64::from(area.y) + i64::from(area.height);
            i64::from(x) >= i64::from(area.x)
                && i64::from(x) < right
                && i64::from(y) >= i64::from(area.y)
                && i64::from(y) < bottom
        })
        .or_else(|| {
            work_areas
                .iter()
                .filter(|area| area.width > 0 && area.height > 0)
                .min_by_key(|area| {
                    let nearest_x = i64::from(x)
                        .clamp(i64::from(area.x), i64::from(area.x) + i64::from(area.width));
                    let nearest_y = i64::from(y).clamp(
                        i64::from(area.y),
                        i64::from(area.y) + i64::from(area.height),
                    );
                    i64::from(x).abs_diff(nearest_x) + i64::from(y).abs_diff(nearest_y)
                })
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
    if instance.builtin_widget_type() != Some(WidgetKind::QuoteBoard) {
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
    if instance.builtin_widget_type() != Some(WidgetKind::QuoteBoard) {
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
            widget_show_header(instance),
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
    show_header: bool,
    row_count: usize,
) -> WidgetSize {
    WidgetSize {
        width: quote_board_display_width(default_size, show_coin_logos, hide_quote_asset),
        height: quote_board_height_for_row_count(row_count, show_header),
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

fn quote_board_height_for_row_count(row_count: usize, show_header: bool) -> i32 {
    let row_count = row_count.clamp(MIN_SYMBOLS_PER_WIDGET, MAX_SYMBOLS_PER_WIDGET) as i32;
    if !show_header {
        return QUOTE_BOARD_HEADERLESS_ONE_ROW_HEIGHT
            + (row_count - 1) * QUOTE_BOARD_EXTENDED_ROW_HEIGHT_STEP;
    }
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

const fn advance_widget_number(number: u64) -> u64 {
    match number.checked_add(1) {
        Some(next) => next,
        None => DEFAULT_NEXT_WIDGET_NUMBER,
    }
}

fn next_widget_number_after_normalization(next: u64, max_suffix: u64) -> u64 {
    let next = next.max(DEFAULT_NEXT_WIDGET_NUMBER);
    match max_suffix.checked_add(1) {
        Some(after_suffix) => next.max(after_suffix),
        None => next,
    }
}

fn next_instance_number(store: &mut LayoutStore, id_prefix: &str) -> u64 {
    loop {
        let number = store.next_widget_number.max(DEFAULT_NEXT_WIDGET_NUMBER);
        store.next_widget_number = advance_widget_number(number);
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
            r#"{"widgets_always_on_top":false,"opacity_percent":77,"market_fallback_enabled":true}"#,
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
        assert!(!settings.auto_start_enabled);
        assert!(settings.show_main_window_on_startup);
        assert_eq!(settings.shortcut, ShortcutPreference::AltC);
        assert_eq!(settings.theme, ThemePreference::System);
        assert_eq!(settings.language, LanguagePreference::En);
        assert!(settings.tray_icon_enabled);
        assert!(!settings.tray_hover_display_enabled);
        assert!(!settings.tray_market_enabled);
        assert_eq!(settings.tray_market_symbols, default_tray_market_symbols());
        assert_eq!(
            settings.tray_market_switch_interval_seconds,
            DEFAULT_TRAY_MARKET_SWITCH_INTERVAL_SECONDS
        );
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
        assert!(widget_show_header(&widget));
        assert_eq!(widget_theme_preference(&widget), WIDGET_THEME_SYSTEM);

        set_widget_display_config(&mut widget, false, true, false);
        set_widget_theme_preference(&mut widget, " light ");

        assert!(!widget_show_coin_logos(&widget));
        assert!(widget_hide_quote_asset(&widget));
        assert!(!widget_show_header(&widget));
        assert_eq!(widget_theme_preference(&widget), "light");
        assert_eq!(
            widget.config[WIDGET_CONFIG_SHOW_COIN_LOGOS],
            Value::Bool(false)
        );
        assert_eq!(
            widget.config[WIDGET_CONFIG_HIDE_QUOTE_ASSET],
            Value::Bool(true)
        );
        assert_eq!(widget.config[WIDGET_CONFIG_SHOW_HEADER], Value::Bool(false));
        assert_eq!(
            widget.config[WIDGET_CONFIG_THEME],
            Value::String("light".to_string())
        );

        let restored: WidgetInstance =
            serde_json::from_str(&serde_json::to_string(&widget).unwrap()).unwrap();
        assert!(!widget_show_header(&restored));
    }

    #[test]
    fn widget_integer_parameters_default_persist_and_clamp() {
        let mut widget = WidgetInstance {
            id: "market-compass-1".to_string(),
            plugin_id: "com.cryptohud.market-compass".to_string(),
            legacy_widget_type: None,
            name: "Market Compass 1".to_string(),
            visible: true,
            layout: WidgetLayout::default(),
            symbols: vec!["binance:spot:BTC/USDT".to_string()],
            config: default_widget_config(),
        };

        assert_eq!(
            widget_integer_parameter(&widget, "switch-interval-seconds", 5, 1, 60),
            5
        );
        set_widget_integer_parameter(&mut widget, "switch-interval-seconds", 90, 1, 60);
        assert_eq!(
            widget_integer_parameter(&widget, "switch-interval-seconds", 5, 1, 60),
            60
        );
        assert_eq!(
            widget.config[WIDGET_CONFIG_PLUGIN_PARAMETERS]["switch-interval-seconds"],
            Value::Number(60.into())
        );
    }

    #[test]
    fn typed_plugin_parameters_round_trip_without_coercion() {
        let mut widget = WidgetInstance {
            id: "typed-plugin-1".to_string(),
            plugin_id: "com.example.typed-plugin".to_string(),
            legacy_widget_type: None,
            name: "Typed plugin".to_string(),
            visible: true,
            layout: WidgetLayout::default(),
            symbols: vec!["binance:spot:BTC/USDT".to_string()],
            config: default_widget_config(),
        };

        set_widget_plugin_parameter(&mut widget, "enabled", Value::Bool(true));
        set_widget_plugin_parameter(&mut widget, "caption", Value::String("Market".to_string()));
        set_widget_plugin_parameter(
            &mut widget,
            "opacity",
            Value::Number(serde_json::Number::from_f64(0.75).unwrap()),
        );

        let restored: WidgetInstance =
            serde_json::from_str(&serde_json::to_string(&widget).unwrap()).unwrap();
        assert_eq!(
            widget_plugin_parameter(&restored, "enabled"),
            Some(&Value::Bool(true))
        );
        assert_eq!(
            widget_plugin_parameter(&restored, "caption"),
            Some(&Value::String("Market".to_string()))
        );
        assert_eq!(
            widget_plugin_parameter(&restored, "opacity").and_then(Value::as_f64),
            Some(0.75)
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

        set_widget_display_config(&mut widget, false, false, true);
        assert_eq!(default_widget_size_for_instance(&widget, &[]).width, 274);
        assert_eq!(
            default_widget_size_for_instance(&widget, &[]).height,
            QUOTE_BOARD_ONE_ROW_HEIGHT
        );

        set_widget_display_config(&mut widget, true, true, true);
        assert_eq!(default_widget_size_for_instance(&widget, &[]).width, 246);

        set_widget_display_config(&mut widget, false, true, false);
        assert_eq!(default_widget_size_for_instance(&widget, &[]).width, 224);
        assert_eq!(
            default_widget_size_for_instance(&widget, &[]).height,
            QUOTE_BOARD_HEADERLESS_ONE_ROW_HEIGHT
        );

        widget.symbols = vec![
            "binance:spot:BTC/USDT".to_string(),
            "binance:spot:ETH/USDT".to_string(),
        ];
        assert_eq!(default_widget_size_for_instance(&widget, &[]).height, 75);

        widget.symbols = vec![
            "binance:spot:BTC/USDT".to_string(),
            "binance:spot:ETH/USDT".to_string(),
            "binance:spot:SOL/USDT".to_string(),
            "binance:spot:BNB/USDT".to_string(),
            "binance:spot:DOGE/USDT".to_string(),
        ];
        assert_eq!(default_widget_size_for_instance(&widget, &[]).height, 153);

        widget.symbols = (0..10)
            .map(|index| format!("binance:spot:COIN{index}/USDT"))
            .collect();
        assert_eq!(default_widget_size_for_instance(&widget, &[]).height, 283);
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

        set_widget_display_config(&mut widget, false, true, true);

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
    fn enum_serialization_uses_stable_config_tags() {
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
        for (preference, serialized) in [
            (LanguagePreference::System, "system"),
            (LanguagePreference::En, "en"),
            (LanguagePreference::ZhHans, "zh-CN"),
            (LanguagePreference::ZhHant, "zh-TW"),
            (LanguagePreference::Es419, "es-419"),
            (LanguagePreference::PtBr, "pt-BR"),
            (LanguagePreference::Vi, "vi"),
            (LanguagePreference::Id, "id"),
            (LanguagePreference::Tr, "tr"),
            (LanguagePreference::Ko, "ko"),
            (LanguagePreference::Ja, "ja"),
            (LanguagePreference::Ru, "ru"),
            (LanguagePreference::Ar, "ar"),
        ] {
            let json = format!(r#""{serialized}""#);
            assert_eq!(serde_json::to_string(&preference).unwrap(), json);
            assert_eq!(
                serde_json::from_str::<LanguagePreference>(&json).unwrap(),
                preference
            );
        }
        assert_eq!(
            serde_json::to_string(&ThemePreference::Dark).unwrap(),
            r#""dark""#
        );
    }

    #[test]
    fn language_preference_deserializes_common_locale_tags() {
        for (json_value, expected) in [
            ("system", LanguagePreference::System),
            ("zh_hans", LanguagePreference::ZhHans),
            ("zh_hant", LanguagePreference::ZhHant),
            ("es_419", LanguagePreference::Es419),
            ("pt_br", LanguagePreference::PtBr),
            ("en-US", LanguagePreference::En),
            ("zh-CN", LanguagePreference::ZhHans),
            ("zh", LanguagePreference::ZhHans),
            ("cmn-Hans-SG", LanguagePreference::ZhHans),
            ("zh-Hans-HK", LanguagePreference::ZhHans),
            ("zh-Hans-SG", LanguagePreference::ZhHans),
            ("zh-TW", LanguagePreference::ZhHant),
            ("zh-HK", LanguagePreference::ZhHant),
            ("zh-MO", LanguagePreference::ZhHant),
            ("zh-Hant-CN", LanguagePreference::ZhHant),
            ("zh_Hant_HK.UTF-8", LanguagePreference::ZhHant),
            ("yue-HK", LanguagePreference::ZhHant),
            ("es-419", LanguagePreference::Es419),
            ("es-AR", LanguagePreference::Es419),
            ("es-CO", LanguagePreference::Es419),
            ("es-MX", LanguagePreference::Es419),
            ("es-US", LanguagePreference::Es419),
            ("es-PE", LanguagePreference::Es419),
            ("pt-BR", LanguagePreference::PtBr),
            ("pt_BR.UTF-8", LanguagePreference::PtBr),
            ("pt_BR@calendar=gregorian", LanguagePreference::PtBr),
            ("vi-VN", LanguagePreference::Vi),
            ("id-ID", LanguagePreference::Id),
            ("in_ID.UTF-8", LanguagePreference::Id),
            ("tr-TR", LanguagePreference::Tr),
            ("ko-KR", LanguagePreference::Ko),
            ("ja-JP", LanguagePreference::Ja),
            ("ru-KZ", LanguagePreference::Ru),
            ("ar-SA", LanguagePreference::Ar),
            ("ar_SA.UTF-8@calendar=gregorian", LanguagePreference::Ar),
        ] {
            let json = serde_json::to_string(json_value).unwrap();
            assert_eq!(
                serde_json::from_str::<LanguagePreference>(&json).unwrap(),
                expected,
                "{json_value} should map to {expected:?}"
            );
        }

        assert!(
            serde_json::from_str::<LanguagePreference>(r#""pt-PT""#).is_err(),
            "pt-PT should not silently select Brazilian Portuguese"
        );
        assert!(
            serde_json::from_str::<LanguagePreference>(r#""pt""#).is_err(),
            "generic pt should not silently select Brazilian Portuguese"
        );
        assert!(
            serde_json::from_str::<LanguagePreference>(r#""es-ES""#).is_err(),
            "es-ES should not silently select Latin American Spanish"
        );
        assert!(
            serde_json::from_str::<LanguagePreference>(r#""es""#).is_err(),
            "generic es should not silently select Latin American Spanish"
        );
    }

    #[test]
    fn language_preference_error_lists_canonical_and_legacy_config_tags() {
        let error = serde_json::from_str::<LanguagePreference>(r#""fr-FR""#)
            .unwrap_err()
            .to_string();

        for expected in ["zh-CN", "zh-TW", "es-419", "pt-BR", "zh_hans", "es_419"] {
            assert!(
                error.contains(expected),
                "unknown language preference error should list accepted config tag {expected}: {error}"
            );
        }
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
        assert_eq!(
            LanguagePreference::from_index(3),
            LanguagePreference::ZhHant
        );
        assert_eq!(LanguagePreference::from_index(12), LanguagePreference::Ar);
        assert_eq!(
            LanguagePreference::from_index(99),
            LanguagePreference::System
        );
        assert_eq!(LanguagePreference::PtBr.index(), 5);
        assert_eq!(ThemePreference::from_index(2), ThemePreference::Dark);
    }

    #[test]
    fn language_preference_indices_round_trip_all_supported_languages() {
        for (index, preference) in LanguagePreference::ALL.into_iter().enumerate() {
            let index = index as i32;
            assert_eq!(preference.index(), index);
            assert_eq!(LanguagePreference::from_index(index), preference);
        }
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
                "binance:spot:BNB/USDT",
                "binance:spot:XRP/USDT",
                "binance:spot:DOGE/USDT",
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
    fn normalizes_tray_market_settings_bounds_and_symbols() {
        let settings = AppSettings {
            tray_market_enabled: true,
            tray_market_symbols: vec![
                "btc".to_string(),
                "BTCUSDT".to_string(),
                "eth/usdt".to_string(),
                "???".to_string(),
                "sol".to_string(),
                "bnb".to_string(),
                "xrp".to_string(),
                "ada".to_string(),
                "doge".to_string(),
                "avax".to_string(),
                "link".to_string(),
            ],
            tray_market_switch_interval_seconds: 1,
            ..AppSettings::default()
        }
        .normalized();

        assert!(settings.tray_market_enabled);
        assert_eq!(
            settings.tray_market_symbols,
            vec![
                "binance:spot:BTC/USDT",
                "binance:spot:ETH/USDT",
                "binance:spot:SOL/USDT",
                "binance:spot:BNB/USDT",
                "binance:spot:XRP/USDT",
                "binance:spot:ADA/USDT",
                "binance:spot:DOGE/USDT",
                "binance:spot:AVAX/USDT",
            ]
        );
        assert_eq!(
            settings.tray_market_switch_interval_seconds,
            MIN_TRAY_MARKET_SWITCH_INTERVAL_SECONDS
        );

        let settings = AppSettings {
            tray_market_symbols: vec!["???".to_string()],
            tray_market_switch_interval_seconds: i32::MAX,
            ..AppSettings::default()
        }
        .normalized();
        assert_eq!(settings.tray_market_symbols, default_tray_market_symbols());
        assert_eq!(
            settings.tray_market_switch_interval_seconds,
            MAX_TRAY_MARKET_SWITCH_INTERVAL_SECONDS
        );
    }

    #[test]
    fn widget_scale_converts_between_percent_and_size() {
        let default_size = WidgetSize {
            width: QUOTE_BOARD_WIDTH,
            height: QUOTE_BOARD_HEIGHT,
        };
        let scaled_min = widget_size_from_scale_percent(default_size, 10);
        assert_eq!(scaled_min.width, 86);
        assert_eq!(scaled_min.height, 58);
        assert_eq!(
            widget_scale_percent_for_size(scaled_min, default_size),
            MIN_WIDGET_SCALE_PERCENT
        );

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
    fn normalizes_quote_board_height_without_header() {
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
                    "height": 101
                  },
                  "symbols": ["BTC", "ETH"],
                  "config": { "show_header": false }
                }
              ]
            }"#,
        )
        .unwrap();

        normalize_store(&mut store, 0);

        assert!(!widget_show_header(&store.widgets[0]));
        assert_eq!(store.widgets[0].layout.width, QUOTE_BOARD_WIDTH);
        assert_eq!(store.widgets[0].layout.height, 75);
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
        assert_eq!(store.widgets[1].layout.width, 71);
        assert_eq!(store.widgets[1].layout.height, 34);
    }

    #[test]
    fn normalizing_duplicate_widget_ids_skips_all_reserved_ids() {
        let widget = |id: &str| WidgetInstance {
            id: id.to_string(),
            plugin_id: WidgetKind::QuoteBoard.plugin_id().to_string(),
            legacy_widget_type: None,
            name: id.to_string(),
            visible: true,
            layout: WidgetLayout::default(),
            symbols: default_market_symbols(),
            config: default_widget_config(),
        };
        let mut store = LayoutStore {
            next_widget_number: 1,
            widgets: vec![
                widget("quote-board-1"),
                widget("quote-board-1"),
                widget("quote-board-2"),
            ],
            ..LayoutStore::default()
        };

        normalize_store(&mut store, 0);

        let ids = store
            .widgets
            .iter()
            .map(|widget| widget.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, ["quote-board-1", "quote-board-3", "quote-board-2"]);
        assert_eq!(ids.iter().copied().collect::<HashSet<_>>().len(), ids.len());
        assert_eq!(store.next_widget_number, 4);
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
        let persisted_json = serde_json::from_str::<Value>(&contents).unwrap();
        assert_eq!(
            persisted_json[LAYOUT_STORE_SCHEMA_VERSION_FIELD],
            LAYOUT_STORE_SCHEMA_VERSION
        );
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
    fn symbols_only_legacy_state_migrates_without_losing_symbols() {
        let dir = std::env::temp_dir().join(format!(
            "crypto-hud-shell-state-symbols-only-{}-{}",
            std::process::id(),
            SAVE_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let path = dir.join(LAYOUT_STATE_FILE_NAME);
        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, r#"{"symbols":["ETH"]}"#).unwrap();

        let loaded = load_layout_store_with_diagnostics(&path, 1, &[], (1920, 1080));

        assert_eq!(loaded.source, LayoutStoreLoadSource::Current);
        assert!(loaded.warning.is_none());
        assert_eq!(loaded.store.widgets.len(), 1);
        assert_eq!(loaded.store.widgets[0].symbols, ["binance:spot:ETH/USDT"]);
        let persisted_json =
            serde_json::from_str::<Value>(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            persisted_json[LAYOUT_STORE_SCHEMA_VERSION_FIELD],
            LAYOUT_STORE_SCHEMA_VERSION
        );
        assert!(persisted_json.get("symbols").is_none());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unversioned_current_state_is_loaded_and_upgraded() {
        let dir = std::env::temp_dir().join(format!(
            "crypto-hud-shell-state-unversioned-current-{}-{}",
            std::process::id(),
            SAVE_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let path = dir.join(LAYOUT_STATE_FILE_NAME);
        fs::create_dir_all(&dir).unwrap();
        let old_store = LayoutStore {
            settings: AppSettings {
                opacity_percent: 77,
                ..AppSettings::default()
            },
            ..LayoutStore::default()
        };
        let old_contents = serde_json::to_string_pretty(&old_store).unwrap();
        assert!(!old_contents.contains(LAYOUT_STORE_SCHEMA_VERSION_FIELD));
        fs::write(&path, old_contents).unwrap();

        let loaded = load_layout_store_with_diagnostics(&path, 0, &[], (1920, 1080));

        assert_eq!(loaded.source, LayoutStoreLoadSource::Current);
        assert!(loaded.warning.is_none());
        assert_eq!(loaded.store.settings.opacity_percent, 77);
        let persisted_json =
            serde_json::from_str::<Value>(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            persisted_json[LAYOUT_STORE_SCHEMA_VERSION_FIELD],
            LAYOUT_STORE_SCHEMA_VERSION
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn schema_one_state_defaults_tray_market_fields_and_upgrades() {
        let dir = std::env::temp_dir().join(format!(
            "crypto-hud-shell-state-schema-one-{}-{}",
            std::process::id(),
            SAVE_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let path = dir.join(LAYOUT_STATE_FILE_NAME);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            &path,
            r#"{
              "schema_version": 1,
              "settings": { "opacity_percent": 77 },
              "selected_widget_id": null,
              "next_widget_number": 1,
              "widgets": []
            }"#,
        )
        .unwrap();

        let loaded = load_layout_store_with_diagnostics(&path, 0, &[], (1920, 1080));

        assert_eq!(loaded.source, LayoutStoreLoadSource::Current);
        assert!(loaded.warning.is_none());
        assert_eq!(loaded.store.settings.opacity_percent, 77);
        assert!(!loaded.store.settings.tray_market_enabled);
        assert_eq!(
            loaded.store.settings.tray_market_symbols,
            default_tray_market_symbols()
        );
        assert_eq!(
            loaded.store.settings.tray_market_switch_interval_seconds,
            DEFAULT_TRAY_MARKET_SWITCH_INTERVAL_SECONDS
        );
        let persisted_json =
            serde_json::from_str::<Value>(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            persisted_json[LAYOUT_STORE_SCHEMA_VERSION_FIELD],
            LAYOUT_STORE_SCHEMA_VERSION
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn legacy_widget_map_is_migrated_to_versioned_current_state() {
        let dir = std::env::temp_dir().join(format!(
            "crypto-hud-shell-state-legacy-map-{}-{}",
            std::process::id(),
            SAVE_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let path = dir.join(LAYOUT_STATE_FILE_NAME);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            &path,
            r#"{
              "settings": { "opacity_percent": 77 },
              "symbols": ["ETH"],
              "widgets": {
                "quote-board-4": {
                  "x": 120,
                  "y": 140,
                  "always_on_top": false,
                  "opacity_percent": 88
                }
              }
            }"#,
        )
        .unwrap();

        let loaded = load_layout_store_with_diagnostics(&path, 0, &[], (1920, 1080));

        assert_eq!(loaded.source, LayoutStoreLoadSource::Current);
        assert!(loaded.warning.is_none());
        assert_eq!(loaded.store.settings.opacity_percent, 77);
        assert_eq!(loaded.store.widgets[0].id, "quote-board-4");
        assert_eq!(loaded.store.widgets[0].symbols, ["binance:spot:ETH/USDT"]);
        let persisted_json =
            serde_json::from_str::<Value>(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            persisted_json[LAYOUT_STORE_SCHEMA_VERSION_FIELD],
            LAYOUT_STORE_SCHEMA_VERSION
        );
        assert!(persisted_json["widgets"].is_array());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn loading_current_state_removes_proxy_credentials_from_disk() {
        let dir = std::env::temp_dir().join(format!(
            "crypto-hud-shell-state-proxy-migration-{}-{}",
            std::process::id(),
            SAVE_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let path = dir.join(LAYOUT_STATE_FILE_NAME);
        let secret = "do-not-retain-this-password";
        let store = LayoutStore {
            settings: AppSettings {
                network_proxy_enabled: true,
                network_proxy_url: format!("http://alice:{secret}@127.0.0.1:7890"),
                ..AppSettings::default()
            },
            ..LayoutStore::default()
        };
        save_layout_store(&path, &store).unwrap();
        assert!(fs::read_to_string(&path).unwrap().contains(secret));

        let loaded = load_layout_store_with_diagnostics(&path, 0, &[], (1920, 1080));

        assert_eq!(loaded.source, LayoutStoreLoadSource::Current);
        assert!(!loaded.store.settings.network_proxy_enabled);
        assert!(loaded.store.settings.network_proxy_url.is_empty());
        let persisted_contents = fs::read_to_string(&path).unwrap();
        assert!(!persisted_contents.contains(secret));
        assert!(!persisted_contents.contains("alice:"));
        let persisted = try_load_persisted_layout_store(&path).unwrap().unwrap();
        let PersistedLayoutStore::Current(persisted) = persisted else {
            panic!("sanitized state should retain the current layout schema");
        };
        assert!(!persisted.settings.network_proxy_enabled);
        assert!(persisted.settings.network_proxy_url.is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn loading_current_state_persists_normalized_dynamic_widget_size() {
        let dir = std::env::temp_dir().join(format!(
            "crypto-hud-shell-state-dynamic-size-migration-{}-{}",
            std::process::id(),
            SAVE_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let path = dir.join(LAYOUT_STATE_FILE_NAME);
        let plugin_id = "com.cryptohud.status-strip";
        let catalog = vec![WidgetDefinition {
            id: plugin_id.to_string(),
            name: "Status Strip".to_string(),
            default_size: WidgetSize {
                width: 618,
                height: 92,
            },
            size_policy: WidgetSizePolicy::SymbolGrid {
                cell_width: 122,
                cell_height: 84,
                content_padding_width: 8,
                content_padding_height: 8,
                columns: Some(5),
                rows: None,
            },
            min_symbol_limit: 1,
            symbol_limit: 5,
            default_symbols: default_market_symbols(),
        }];
        let store = LayoutStore {
            selected_widget_id: Some("plugin-strip-3".to_string()),
            widgets: vec![WidgetInstance {
                id: "plugin-strip-3".to_string(),
                plugin_id: plugin_id.to_string(),
                legacy_widget_type: None,
                name: "Status Strip 3".to_string(),
                visible: true,
                layout: WidgetLayout {
                    x: 164,
                    y: 164,
                    always_on_top: false,
                    opacity_percent: 96,
                    locked: true,
                    scale_percent: 150,
                    width: 624,
                    height: 138,
                },
                symbols: vec!["BTC".to_string(), "ETH".to_string(), "SOL".to_string()],
                config: default_widget_config(),
            }],
            ..LayoutStore::default()
        };
        save_layout_store(&path, &store).unwrap();

        let loaded = load_layout_store_with_diagnostics(&path, 0, &catalog, (1920, 1080));

        assert_eq!(loaded.store.widgets[0].layout.width, 561);
        assert_eq!(loaded.store.widgets[0].layout.height, 138);
        let persisted = try_load_persisted_layout_store(&path).unwrap().unwrap();
        let PersistedLayoutStore::Current(persisted) = persisted else {
            panic!("normalized state should retain the current layout schema");
        };
        assert_eq!(persisted.widgets[0].layout.width, 561);
        assert_eq!(persisted.widgets[0].layout.height, 138);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn loading_current_state_migrates_obsolete_ten_percent_widget_scale() {
        let dir = std::env::temp_dir().join(format!(
            "crypto-hud-shell-state-minimum-scale-migration-{}-{}",
            std::process::id(),
            SAVE_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let path = dir.join(LAYOUT_STATE_FILE_NAME);
        let store = LayoutStore {
            selected_widget_id: Some("mini-ticker-1".to_string()),
            widgets: vec![WidgetInstance {
                id: "mini-ticker-1".to_string(),
                plugin_id: WidgetKind::MiniTicker.plugin_id().to_string(),
                legacy_widget_type: None,
                name: "Mini Ticker 1".to_string(),
                visible: true,
                layout: WidgetLayout {
                    scale_percent: 10,
                    width: 24,
                    height: 11,
                    ..WidgetLayout::default()
                },
                symbols: vec!["ETH".to_string()],
                config: default_widget_config(),
            }],
            ..LayoutStore::default()
        };
        save_layout_store(&path, &store).unwrap();

        let loaded = load_layout_store_with_diagnostics(
            &path,
            0,
            &[WidgetDefinition::builtin(WidgetKind::MiniTicker)],
            (1920, 1080),
        );

        assert_eq!(
            loaded.store.widgets[0].layout.scale_percent,
            MIN_WIDGET_SCALE_PERCENT
        );
        assert_eq!(loaded.store.widgets[0].layout.width, 71);
        assert_eq!(loaded.store.widgets[0].layout.height, 34);
        let persisted = try_load_persisted_layout_store(&path).unwrap().unwrap();
        let PersistedLayoutStore::Current(persisted) = persisted else {
            panic!("minimum-scale migration should retain the current layout schema");
        };
        assert_eq!(
            persisted.widgets[0].layout.scale_percent,
            MIN_WIDGET_SCALE_PERCENT
        );
        assert_eq!(persisted.widgets[0].layout.width, 71);
        assert_eq!(persisted.widgets[0].layout.height, 34);

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

    #[test]
    fn persisted_layout_loader_distinguishes_missing_read_and_parse_failures() {
        let dir = std::env::temp_dir().join(format!(
            "crypto-hud-shell-state-load-errors-{}-{}",
            std::process::id(),
            SAVE_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let missing_path = dir.join("missing.json");
        fs::create_dir_all(&dir).unwrap();

        assert!(try_load_persisted_layout_store(&missing_path)
            .unwrap()
            .is_none());

        let unreadable_path = dir.join("directory.json");
        fs::create_dir(&unreadable_path).unwrap();
        let read_error = try_load_persisted_layout_store(&unreadable_path).unwrap_err();
        assert_eq!(read_error.kind(), LayoutStoreReadErrorKind::Read);

        let malformed_path = dir.join("malformed.json");
        fs::write(&malformed_path, b"{not valid json").unwrap();
        let parse_error = try_load_persisted_layout_store(&malformed_path).unwrap_err();
        assert_eq!(parse_error.kind(), LayoutStoreReadErrorKind::Parse);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn malformed_current_state_is_preserved_without_falling_back_to_legacy() {
        let dir = std::env::temp_dir().join(format!(
            "crypto-hud-shell-state-corrupt-{}-{}",
            std::process::id(),
            SAVE_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let path = dir.join(LAYOUT_STATE_FILE_NAME);
        let legacy_path = dir.join(LEGACY_LAYOUT_STATE_FILE_NAME);
        let malformed_contents = b"{not valid json";
        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, malformed_contents).unwrap();
        fs::write(
            &legacy_path,
            r#"{
              "settings": { "opacity_percent": 77 },
              "widgets": []
            }"#,
        )
        .unwrap();

        let loaded = load_layout_store_with_diagnostics(&path, 0, &[], (1920, 1080));

        assert_eq!(loaded.source, LayoutStoreLoadSource::DefaultsAfterError);
        assert_eq!(
            loaded.store.settings.opacity_percent,
            DEFAULT_OPACITY_PERCENT
        );
        let warning = loaded.warning.expect("malformed state should be reported");
        assert_eq!(warning.kind, LayoutStoreReadErrorKind::Parse);
        assert_eq!(warning.path, path);
        let preserved_path = warning
            .preserved_path
            .as_ref()
            .expect("malformed state should be backed up");
        assert_eq!(fs::read(preserved_path).unwrap(), malformed_contents);
        assert_eq!(fs::read(&path).unwrap(), malformed_contents);
        assert!(warning.to_string().contains("defaults were loaded"));
        assert!(warning.to_string().contains("was preserved at"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn misspelled_top_level_field_is_preserved_and_not_overwritten() {
        let dir = std::env::temp_dir().join(format!(
            "crypto-hud-shell-state-unknown-field-{}-{}",
            std::process::id(),
            SAVE_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let path = dir.join(LAYOUT_STATE_FILE_NAME);
        let suspicious_contents = br#"{"widgtes":[]}"#;
        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, suspicious_contents).unwrap();

        let loaded = load_layout_store_with_diagnostics(&path, 0, &[], (1920, 1080));

        assert_eq!(loaded.source, LayoutStoreLoadSource::DefaultsAfterError);
        let warning = loaded
            .warning
            .expect("the misspelled field should be reported");
        assert_eq!(warning.kind, LayoutStoreReadErrorKind::Parse);
        let preserved_path = warning
            .preserved_path
            .expect("the suspicious state should be backed up");
        assert_eq!(fs::read(&path).unwrap(), suspicious_contents);
        assert_eq!(fs::read(preserved_path).unwrap(), suspicious_contents);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn future_schema_state_is_preserved_and_not_downgraded() {
        let dir = std::env::temp_dir().join(format!(
            "crypto-hud-shell-state-future-schema-{}-{}",
            std::process::id(),
            SAVE_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let path = dir.join(LAYOUT_STATE_FILE_NAME);
        let future_contents = format!(
            r#"{{
              "schema_version": {},
              "settings": {{}},
              "selected_widget_id": null,
              "next_widget_number": 1,
              "widgets": [],
              "future_field": true
            }}"#,
            LAYOUT_STORE_SCHEMA_VERSION + 1
        );
        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, &future_contents).unwrap();

        let loaded = load_layout_store_with_diagnostics(&path, 0, &[], (1920, 1080));

        assert_eq!(loaded.source, LayoutStoreLoadSource::DefaultsAfterError);
        let warning = loaded
            .warning
            .expect("the future schema should be reported");
        assert_eq!(warning.kind, LayoutStoreReadErrorKind::Parse);
        assert!(warning.error.contains("unsupported layout state schema"));
        let preserved_path = warning
            .preserved_path
            .expect("the future state should be backed up");
        assert_eq!(fs::read_to_string(&path).unwrap(), future_contents);
        assert_eq!(fs::read_to_string(preserved_path).unwrap(), future_contents);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn current_schema_rejects_unknown_top_level_fields() {
        let contents = format!(
            r#"{{
              "schema_version": {LAYOUT_STORE_SCHEMA_VERSION},
              "settings": {{}},
              "selected_widget_id": null,
              "next_widget_number": 1,
              "widgets": [],
              "unexpected": true
            }}"#
        );

        let error = serde_json::from_str::<PersistedLayoutStore>(&contents).unwrap_err();

        assert!(error.to_string().contains("unknown field `unexpected`"));
    }

    #[test]
    fn multi_monitor_work_areas_preserve_valid_secondary_screen_positions() {
        let work_areas = [
            DesktopWorkArea {
                x: -1920,
                y: 0,
                width: 1920,
                height: 1040,
                dpi: DEFAULT_MONITOR_DPI,
                is_primary: false,
            },
            DesktopWorkArea {
                x: 0,
                y: 0,
                width: 1920,
                height: 1040,
                dpi: DEFAULT_MONITOR_DPI,
                is_primary: true,
            },
            DesktopWorkArea {
                x: 1920,
                y: -200,
                width: 2560,
                height: 1440,
                dpi: DEFAULT_MONITOR_DPI,
                is_primary: false,
            },
        ];
        let size = WidgetSize {
            width: QUOTE_BOARD_WIDTH,
            height: QUOTE_BOARD_HEIGHT,
        };
        let layout_at = |x, y| WidgetLayout {
            x,
            y,
            width: size.width,
            height: size.height,
            ..WidgetLayout::default()
        };

        for layout in [layout_at(-1800, 100), layout_at(2200, -100)] {
            assert!(layout_has_visible_area_for_size_in_work_areas(
                &layout,
                size,
                &work_areas
            ));
            assert!(!layout_needs_recovery_for_size_in_work_areas(
                &layout,
                size,
                &work_areas
            ));
        }

        let truly_offscreen = layout_at(5000, 1600);
        assert!(!layout_has_visible_area_for_size_in_work_areas(
            &truly_offscreen,
            size,
            &work_areas
        ));
        assert!(layout_needs_recovery_for_size_in_work_areas(
            &truly_offscreen,
            size,
            &work_areas
        ));
    }

    #[test]
    fn unavailable_plugin_state_stays_opaque_during_normalization() {
        let original_layout = WidgetLayout {
            x: 42_000,
            y: -17_000,
            always_on_top: false,
            opacity_percent: 137,
            locked: true,
            scale_percent: 0,
            width: 777,
            height: 333,
        };
        let original_symbols = vec!["vendor-specific-symbol".to_string(), "???".to_string()];
        let original_config = serde_json::json!({
            "opaque": { "version": 7, "values": [1, 2, 3] }
        });
        let mut store = LayoutStore {
            widgets: vec![WidgetInstance {
                id: "vendor-widget-9".to_string(),
                plugin_id: "com.example.unavailable".to_string(),
                legacy_widget_type: Some(WidgetKind::QuoteBoard),
                name: String::new(),
                visible: true,
                layout: original_layout.clone(),
                symbols: original_symbols.clone(),
                config: original_config.clone(),
            }],
            ..LayoutStore::default()
        };

        normalize_store_with_catalog(&mut store, 0, &[], (1920, 1080));

        let widget = &store.widgets[0];
        assert_eq!(widget.plugin_id, "com.example.unavailable");
        assert!(widget.name.is_empty());
        assert_eq!(widget.symbols, original_symbols);
        assert_eq!(widget.config, original_config);
        assert_eq!(widget.layout, original_layout);
        assert_eq!(widget.legacy_widget_type, None);
    }

    #[test]
    fn work_area_dpi_converts_logical_widget_size_to_physical_pixels() {
        let logical_size = WidgetSize {
            width: QUOTE_BOARD_WIDTH,
            height: QUOTE_BOARD_HEIGHT,
        };
        for (dpi, expected_width) in [(120, 358), (144, 429), (192, 572)] {
            let work_area = DesktopWorkArea {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
                dpi,
                is_primary: true,
            };
            let physical_size = work_area.physical_size(logical_size);
            assert_eq!(physical_size.width, expected_width);
            let layout = WidgetLayout {
                x: -(physical_size.width - MIN_VISIBLE_WIDGET_PX),
                y: 100,
                ..WidgetLayout::default()
            };
            assert!(layout_has_visible_area_for_size_in_work_areas(
                &layout,
                logical_size,
                &[work_area],
            ));
        }
    }

    #[test]
    fn cross_monitor_visibility_uses_one_origin_monitor_dpi() {
        let work_areas = [
            DesktopWorkArea {
                x: -1920,
                y: 0,
                width: 1920,
                height: 1080,
                dpi: 96,
                is_primary: true,
            },
            DesktopWorkArea {
                x: 100,
                y: 0,
                width: 1920,
                height: 1080,
                dpi: 192,
                is_primary: false,
            },
        ];
        let logical_size = WidgetSize {
            width: 40,
            height: 40,
        };
        let nearer_low_dpi_monitor = WidgetLayout {
            x: 45,
            y: 100,
            width: logical_size.width,
            height: logical_size.height,
            ..WidgetLayout::default()
        };
        let nearer_high_dpi_monitor = WidgetLayout {
            x: 55,
            ..nearer_low_dpi_monitor.clone()
        };

        assert_eq!(
            work_area_for_position(
                nearer_low_dpi_monitor.x,
                nearer_low_dpi_monitor.y,
                &work_areas,
            )
            .unwrap()
            .dpi,
            96
        );
        assert!(!layout_has_visible_area_for_size_in_work_areas(
            &nearer_low_dpi_monitor,
            logical_size,
            &work_areas,
        ));
        assert_eq!(
            work_area_for_position(
                nearer_high_dpi_monitor.x,
                nearer_high_dpi_monitor.y,
                &work_areas,
            )
            .unwrap()
            .dpi,
            192
        );
        assert!(layout_has_visible_area_for_size_in_work_areas(
            &nearer_high_dpi_monitor,
            logical_size,
            &work_areas,
        ));
    }

    #[test]
    fn reset_positions_pack_actual_sizes_without_overlap_across_monitors() {
        let work_areas = [
            DesktopWorkArea {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
                dpi: 120,
                is_primary: true,
            },
            DesktopWorkArea {
                x: 1920,
                y: -160,
                width: 2560,
                height: 1440,
                dpi: 192,
                is_primary: false,
            },
        ];
        let sizes = [(500, 300), (700, 240), (300, 500), (640, 360)];
        let definitions = sizes
            .iter()
            .enumerate()
            .map(|(index, &(width, height))| WidgetDefinition {
                id: format!("com.example.layout-{index}"),
                name: format!("Layout {index}"),
                default_size: WidgetSize { width, height },
                size_policy: WidgetSizePolicy::Fixed,
                min_symbol_limit: 1,
                symbol_limit: 1,
                default_symbols: vec!["BTC".to_string()],
            })
            .collect::<Vec<_>>();
        let mut store = LayoutStore {
            widgets: definitions
                .iter()
                .enumerate()
                .map(|(index, definition)| WidgetInstance {
                    id: format!("layout-{index}"),
                    plugin_id: definition.id.clone(),
                    legacy_widget_type: None,
                    name: definition.name.clone(),
                    visible: true,
                    layout: WidgetLayout {
                        x: 0,
                        y: 0,
                        scale_percent: 0,
                        width: definition.default_size.width,
                        height: definition.default_size.height,
                        ..WidgetLayout::default()
                    },
                    symbols: vec!["BTC".to_string()],
                    config: default_widget_config(),
                })
                .collect(),
            ..LayoutStore::default()
        };

        assert!(reset_widget_positions_in_work_areas(
            &mut store,
            &definitions,
            (1920, 1080),
            &work_areas,
        ));

        let rects = store
            .widgets
            .iter()
            .map(|instance| {
                let area =
                    work_area_for_position(instance.layout.x, instance.layout.y, &work_areas)
                        .copied()
                        .unwrap();
                widget_rect_for_size(
                    &instance.layout,
                    area.physical_size(widget_size_for_instance(instance, &definitions)),
                )
            })
            .collect::<Vec<_>>();
        for (index, rect) in rects.iter().enumerate() {
            for other in rects.iter().skip(index + 1) {
                assert!(!rect.overlaps(*other, 0), "reset layouts must not overlap");
            }
        }
        for instance in &store.widgets {
            assert!(layout_has_visible_area_for_size_in_work_areas(
                &instance.layout,
                widget_size_for_instance(instance, &definitions),
                &work_areas,
            ));
        }
    }

    #[test]
    fn packed_fallback_keeps_oversized_widgets_visible_when_the_desktop_is_full() {
        let work_area = DesktopWorkArea {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            dpi: 96,
            is_primary: true,
        };
        let oversized = WidgetSize {
            width: MAX_WIDGET_WIDTH,
            height: MAX_WIDGET_HEIGHT,
        };
        let regular = WidgetSize {
            width: 300,
            height: 200,
        };

        for sizes in [
            vec![oversized, oversized, oversized],
            vec![oversized, regular],
            vec![regular, oversized],
        ] {
            let positions = packed_widget_positions(&sizes, (1920, 1080), &[work_area]);
            assert_eq!(positions.len(), sizes.len());
            let visible_rects = positions
                .iter()
                .copied()
                .zip(sizes.iter().copied())
                .map(|((x, y), size)| {
                    let layout = WidgetLayout {
                        x,
                        y,
                        width: size.width,
                        height: size.height,
                        ..WidgetLayout::default()
                    };
                    assert!(layout_has_visible_area_for_size_in_work_areas(
                        &layout,
                        size,
                        &[work_area],
                    ));
                    WidgetRect {
                        x: x.max(work_area.x),
                        y: y.max(work_area.y),
                        width: x
                            .saturating_add(size.width)
                            .min(work_area.x.saturating_add(work_area.width))
                            .saturating_sub(x.max(work_area.x)),
                        height: y
                            .saturating_add(size.height)
                            .min(work_area.y.saturating_add(work_area.height))
                            .saturating_sub(y.max(work_area.y)),
                    }
                })
                .collect::<Vec<_>>();
            for (index, rect) in visible_rects.iter().enumerate() {
                for other in visible_rects.iter().skip(index + 1) {
                    let rect_contains_other = rect.x <= other.x
                        && rect.y <= other.y
                        && rect.x.saturating_add(rect.width) >= other.x.saturating_add(other.width)
                        && rect.y.saturating_add(rect.height)
                            >= other.y.saturating_add(other.height);
                    let other_contains_rect = other.x <= rect.x
                        && other.y <= rect.y
                        && other.x.saturating_add(other.width) >= rect.x.saturating_add(rect.width)
                        && other.y.saturating_add(other.height)
                            >= rect.y.saturating_add(rect.height);
                    assert!(
                        !rect_contains_other && !other_contains_rect,
                        "fallback windows should retain independently exposed edges: {visible_rects:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn packed_fallback_origins_keep_the_selected_dpi_with_a_left_hand_monitor() {
        let work_areas = [
            DesktopWorkArea {
                x: -2560,
                y: 0,
                width: 2560,
                height: 1440,
                dpi: 192,
                is_primary: false,
            },
            DesktopWorkArea {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
                dpi: 96,
                is_primary: true,
            },
        ];
        let oversized = WidgetSize {
            width: MAX_WIDGET_WIDTH,
            height: MAX_WIDGET_HEIGHT,
        };
        let regular = WidgetSize {
            width: 300,
            height: 200,
        };

        for sizes in [vec![oversized, regular], vec![regular, oversized]] {
            let positions = packed_widget_positions(&sizes, (4480, 1440), &work_areas);
            let visible_rects = positions
                .iter()
                .copied()
                .zip(sizes.iter().copied())
                .map(|((x, y), logical_size)| {
                    let origin_area = work_area_for_position(x, y, &work_areas).unwrap();
                    assert_eq!(origin_area.dpi, 96);
                    assert!((0..1920).contains(&x) && (0..1080).contains(&y));
                    let physical_size = origin_area.physical_size(logical_size);
                    WidgetRect {
                        x,
                        y,
                        width: x
                            .saturating_add(physical_size.width)
                            .min(1920)
                            .saturating_sub(x),
                        height: y
                            .saturating_add(physical_size.height)
                            .min(1080)
                            .saturating_sub(y),
                    }
                })
                .collect::<Vec<_>>();
            let first = visible_rects[0];
            let second = visible_rects[1];
            let first_contains_second = first.x <= second.x
                && first.y <= second.y
                && first.x.saturating_add(first.width) >= second.x.saturating_add(second.width)
                && first.y.saturating_add(first.height) >= second.y.saturating_add(second.height);
            let second_contains_first = second.x <= first.x
                && second.y <= first.y
                && second.x.saturating_add(second.width) >= first.x.saturating_add(first.width)
                && second.y.saturating_add(second.height) >= first.y.saturating_add(first.height);
            assert!(!first_contains_second && !second_contains_first);
        }
    }

    #[test]
    fn oversized_fallback_uses_the_rightmost_monitor_to_avoid_cross_screen_coverage() {
        let work_areas = [
            DesktopWorkArea {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
                dpi: 96,
                is_primary: true,
            },
            DesktopWorkArea {
                x: 1920,
                y: 0,
                width: 2560,
                height: 1440,
                dpi: 192,
                is_primary: false,
            },
        ];
        let oversized = WidgetSize {
            width: MAX_WIDGET_WIDTH,
            height: MAX_WIDGET_HEIGHT,
        };
        let regular = WidgetSize {
            width: 300,
            height: 200,
        };

        for sizes in [vec![oversized, regular], vec![regular, oversized]] {
            let positions = packed_widget_positions(&sizes, (4480, 1440), &work_areas);
            let oversized_index = sizes.iter().position(|size| *size == oversized).unwrap();
            let regular_index = 1 - oversized_index;
            let oversized_area = work_area_for_position(
                positions[oversized_index].0,
                positions[oversized_index].1,
                &work_areas,
            )
            .unwrap();
            let regular_area = work_area_for_position(
                positions[regular_index].0,
                positions[regular_index].1,
                &work_areas,
            )
            .unwrap();

            assert_eq!(oversized_area.dpi, 192);
            assert!(positions[oversized_index].0 >= 1920);
            assert_eq!(regular_area.dpi, 96);
            assert!(positions[regular_index].0 < 1920);
        }
    }

    #[test]
    fn widget_number_wraps_without_overflow_and_skips_existing_ids() {
        let mut store = LayoutStore {
            next_widget_number: u64::MAX,
            widgets: vec![WidgetInstance {
                id: format!("quote-board-{}", u64::MAX),
                plugin_id: WidgetKind::QuoteBoard.plugin_id().to_string(),
                legacy_widget_type: None,
                name: "Existing".to_string(),
                visible: true,
                layout: WidgetLayout::default(),
                symbols: default_market_symbols(),
                config: default_widget_config(),
            }],
            ..LayoutStore::default()
        };
        let settings = store.settings.clone();

        let id = add_widget_instance(&mut store, WidgetKind::QuoteBoard, &settings, (1920, 1080));

        assert_eq!(id, "quote-board-1");
        assert_eq!(store.next_widget_number, 2);
    }

    #[test]
    fn persisted_proxy_userinfo_is_removed_during_settings_migration() {
        let settings = AppSettings {
            network_proxy_enabled: true,
            network_proxy_url: "http://alice:secret@127.0.0.1:7890".to_string(),
            ..AppSettings::default()
        }
        .normalized();

        assert!(!settings.network_proxy_enabled);
        assert!(settings.network_proxy_url.is_empty());
        assert_eq!(effective_network_proxy_url(&settings), None);
    }
}
