use crypto_hud_shell_state::LanguagePreference;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    En,
    ZhHans,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetText {
    QuoteBoard,
    MiniTicker,
}

#[derive(Debug, Clone, Copy)]
pub struct UiText {
    pub tray_tooltip: &'static str,
    pub tray_settings: &'static str,
    pub tray_quit: &'static str,
    pub settings_title: &'static str,
    pub tab_widgets: &'static str,
    pub tab_plugin_market: &'static str,
    pub tab_market_data: &'static str,
    pub tab_appearance: &'static str,
    pub tab_system: &'static str,
    pub always_on_top: &'static str,
    pub default_always_on_top: &'static str,
    pub opacity: &'static str,
    pub default_opacity: &'static str,
    pub widget_scale: &'static str,
    pub red_up_color: &'static str,
    pub market_provider: &'static str,
    pub refresh_interval: &'static str,
    pub seconds_unit: &'static str,
    pub market_provider_help: &'static str,
    pub refresh_interval_help: &'static str,
    pub default_symbols: &'static str,
    pub market_fallback: &'static str,
    pub market_fallback_help: &'static str,
    pub alert_settings: &'static str,
    pub alert_enabled: &'static str,
    pub alert_symbol: &'static str,
    pub alert_condition: &'static str,
    pub alert_threshold: &'static str,
    pub alert_clear: &'static str,
    pub symbols: &'static str,
    pub symbols_help: &'static str,
    pub empty_pairs: &'static str,
    pub auto_start: &'static str,
    pub show_main_window_on_startup: &'static str,
    pub shortcut: &'static str,
    pub tray_icon: &'static str,
    pub tray_hover_display: &'static str,
    pub network_proxy_settings: &'static str,
    pub network_proxy_enabled: &'static str,
    pub network_proxy_url: &'static str,
    pub network_proxy_http_example: &'static str,
    pub network_proxy_socks_example: &'static str,
    pub network_proxy_help: &'static str,
    pub app_version: &'static str,
    pub about_us: &'static str,
    pub icon_cache: &'static str,
    pub icon_cache_help: &'static str,
    pub clear_icon_cache: &'static str,
    pub custom_components: &'static str,
    pub custom_components_help: &'static str,
    pub open_custom_components_folder: &'static str,
    pub theme: &'static str,
    pub language: &'static str,
    pub appearance_interface: &'static str,
    pub appearance_widget_defaults: &'static str,
    pub theme_help: &'static str,
    pub language_help: &'static str,
    pub red_up_color_help: &'static str,
    pub default_opacity_help: &'static str,
    pub default_always_on_top_help: &'static str,
    pub system_startup: &'static str,
    pub system_tray: &'static str,
    pub system_app_info: &'static str,
    pub system_maintenance: &'static str,
    pub auto_start_help: &'static str,
    pub show_main_window_on_startup_help: &'static str,
    pub shortcut_help: &'static str,
    pub tray_icon_help: &'static str,
    pub tray_hover_display_help: &'static str,
    pub apply: &'static str,
    pub widget_library: &'static str,
    pub my_widgets: &'static str,
    pub selected_widget: &'static str,
    pub selected_widget_description: &'static str,
    pub widget_name: &'static str,
    pub lock_position_help: &'static str,
    pub widget_scale_help: &'static str,
    pub opacity_help: &'static str,
    pub widget_show_coin_logos: &'static str,
    pub widget_show_coin_logos_help: &'static str,
    pub widget_hide_quote_asset: &'static str,
    pub widget_hide_quote_asset_help: &'static str,
    pub widget_topmost: &'static str,
    pub widget_topmost_help: &'static str,
    pub advanced_options: &'static str,
    pub reset_widget_positions: &'static str,
    pub hide_all_widgets: &'static str,
    pub reset: &'static str,
    pub preview: &'static str,
    pub preview_pairs: &'static str,
    pub preview_updated: &'static str,
    pub preview_source_ok: &'static str,
    pub app_settings: &'static str,
    pub add_widget: &'static str,
    pub apply_widget: &'static str,
    pub no_widgets: &'static str,
    pub quote_board_title: &'static str,
    pub plugin_market_description: &'static str,
    pub my_widgets_description: &'static str,
    pub market_settings_description: &'static str,
    pub appearance_settings_description: &'static str,
    pub system_settings_description: &'static str,
    pub quote_board_description: &'static str,
    pub mini_ticker_title: &'static str,
    pub mini_ticker_description: &'static str,
    pub market_settings: &'static str,
    pub appearance_settings: &'static str,
    pub system_settings: &'static str,
    pub settings_path_label: &'static str,
    pub close: &'static str,
    pub source_prefix: &'static str,
    pub runtime_no_pairs: &'static str,
    pub runtime_connecting: &'static str,
    pub runtime_connection_error: &'static str,
    pub runtime_updated: &'static str,
    pub runtime_stale: &'static str,
    pub runtime_fallback: &'static str,
    pub runtime_source_error: &'static str,
    pub runtime_live_count_prefix: &'static str,
    pub runtime_live_count_suffix: &'static str,
    pub widget_visible: &'static str,
    pub widget_hidden: &'static str,
    pub delete_widget: &'static str,
    pub status_alert_invalid: &'static str,
    pub status_auto_start_failed: &'static str,
    pub status_shortcut_failed: &'static str,
    pub status_network_proxy_invalid: &'static str,
    pub status_icon_cache_clear_failed: &'static str,
    pub status_custom_components_folder_open_failed: &'static str,
    pub status_symbol_catalog_fallback: &'static str,
}

const EN_TEXT: UiText = UiText {
    tray_tooltip: "Crypto HUD",
    tray_settings: "Main Window",
    tray_quit: "Quit",
    settings_title: "Crypto HUD",
    tab_widgets: "Widgets",
    tab_plugin_market: "Widget Library",
    tab_market_data: "Market Data",
    tab_appearance: "Appearance",
    tab_system: "System",
    always_on_top: "Lock position",
    default_always_on_top: "Pin new widgets on top",
    opacity: "Opacity",
    default_opacity: "Default opacity",
    widget_scale: "Scale",
    red_up_color: "Red for gains",
    market_provider: "Enabled sources",
    refresh_interval: "Refresh interval",
    seconds_unit: "sec",
    market_provider_help: "Shown in pair search and used by selected pairs.",
    refresh_interval_help: "Seconds between quote refreshes.",
    default_symbols: "Default pairs for new widgets",
    market_fallback: "Fallback to another source",
    market_fallback_help: "Switches source when the current feed fails.",
    alert_settings: "Alert Rule",
    alert_enabled: "Enable alert",
    alert_symbol: "Pair",
    alert_condition: "Condition",
    alert_threshold: "Threshold",
    alert_clear: "Clear Alert",
    symbols: "Pairs",
    symbols_help: "Up to 20, selected pairs appear as tags",
    empty_pairs: "No pairs configured",
    auto_start: "Launch at login",
    show_main_window_on_startup: "Show main window on startup",
    shortcut: "Show/hide shortcut",
    tray_icon: "Show tray icon",
    tray_hover_display: "Show on tray hover",
    network_proxy_settings: "Network Proxy",
    network_proxy_enabled: "Enable network proxy",
    network_proxy_url: "Proxy address",
    network_proxy_http_example: "http://127.0.0.1:7890",
    network_proxy_socks_example: "socks5://127.0.0.1:1080",
    network_proxy_help: "Routes quotes, symbol search, and updates through the proxy.",
    app_version: "Version",
    about_us: "About us",
    icon_cache: "Icon cache",
    icon_cache_help: "Removes locally cached coin logos.",
    clear_icon_cache: "Clear Icons",
    custom_components: "Custom widgets",
    custom_components_help: "Opens the folder for local widget plugins.",
    open_custom_components_folder: "Open Folder",
    theme: "Theme",
    language: "Language",
    appearance_interface: "Interface",
    appearance_widget_defaults: "Widget Defaults",
    theme_help: "Updates colors for the main window and desktop widgets.",
    language_help: "Refreshes interface text right away.",
    red_up_color_help: "Shows gains in red and losses in green.",
    default_opacity_help: "Sets how transparent new widgets appear.",
    default_always_on_top_help: "Keeps newly added widgets above other windows.",
    system_startup: "Startup",
    system_tray: "Tray",
    system_app_info: "App Info",
    system_maintenance: "Maintenance",
    auto_start_help: "Runs Crypto HUD after you sign in.",
    show_main_window_on_startup_help: "Opens the main window on app launch.",
    shortcut_help: "Use Alt+C to hide or restore widgets.",
    tray_icon_help: "Keeps quick access for the main window and quit.",
    tray_hover_display_help: "Temporarily shows widgets while hovering the tray icon.",
    apply: "Apply",
    widget_library: "Widget Library",
    my_widgets: "My Widgets",
    selected_widget: "Selected Widget:",
    selected_widget_description: "The settings below only affect the selected widget.",
    widget_name: "Name",
    lock_position_help: "Keep this widget fixed at its current position.",
    widget_scale_help: "Adjust the overall widget size.",
    opacity_help: "Adjust widget transparency.",
    widget_show_coin_logos: "Show coin logos",
    widget_show_coin_logos_help: "Display token icons beside pairs.",
    widget_hide_quote_asset: "Hide quote asset",
    widget_hide_quote_asset_help: "Show BTC instead of BTC/USDC.",
    widget_topmost: "Always on top",
    widget_topmost_help: "Keep this widget above other windows.",
    advanced_options: "Advanced options",
    reset_widget_positions: "Reset",
    hide_all_widgets: "Hide All",
    reset: "Reset",
    preview: "Preview",
    preview_pairs: "Pairs",
    preview_updated: "Updated 3s",
    preview_source_ok: "Source OK",
    app_settings: "Widget Display",
    add_widget: "Add",
    apply_widget: "Apply Widget",
    no_widgets: "No widgets",
    quote_board_title: "Quote Board",
    plugin_market_description: "Available built-in and local widgets.",
    my_widgets_description: "Manage added widgets, pairs, and window display.",
    market_settings_description:
        "Configure quote sources, refresh rate, default pairs, and alerts.",
    appearance_settings_description: "Adjust theme, language, and default widget display.",
    system_settings_description: "Manage startup behavior, proxy, tray, and app info.",
    quote_board_description: "Shows up to 20 pairs for tracking major markets together.",
    mini_ticker_title: "Mini Ticker",
    mini_ticker_description: "Shows 1 pair in a compact window for desktop corners.",
    market_settings: "Quote Settings",
    appearance_settings: "Appearance Settings",
    system_settings: "System Settings",
    settings_path_label: "Settings file",
    close: "Close",
    source_prefix: "Live feed",
    runtime_no_pairs: "No pairs",
    runtime_connecting: "Connecting",
    runtime_connection_error: "Connection failed",
    runtime_updated: "Updated",
    runtime_stale: "Stale",
    runtime_fallback: "Fallback",
    runtime_source_error: "Source issue",
    runtime_live_count_prefix: "",
    runtime_live_count_suffix: " live",
    widget_visible: "Shown",
    widget_hidden: "Hidden",
    delete_widget: "Remove widget",
    status_alert_invalid: "Alert rule is incomplete",
    status_auto_start_failed: "Auto-start failed",
    status_shortcut_failed: "Shortcut registration failed",
    status_network_proxy_invalid: "Network proxy is invalid",
    status_icon_cache_clear_failed: "Icon cache clear failed",
    status_custom_components_folder_open_failed: "Could not open custom widgets folder",
    status_symbol_catalog_fallback: "Using popular pairs list",
};

const ZH_HANS_TEXT: UiText = UiText {
    tray_tooltip: "Crypto HUD",
    tray_settings: "主界面",
    tray_quit: "退出",
    settings_title: "Crypto HUD",
    tab_widgets: "小组件",
    tab_plugin_market: "组件库",
    tab_market_data: "行情",
    tab_appearance: "外观",
    tab_system: "系统",
    always_on_top: "锁定位置",
    default_always_on_top: "新建组件默认置顶",
    opacity: "透明度",
    default_opacity: "默认透明度",
    widget_scale: "缩放",
    red_up_color: "红涨绿跌",
    market_provider: "启用数据源",
    refresh_interval: "刷新频率",
    seconds_unit: "秒",
    market_provider_help: "控制搜索目录和已选交易对的数据源。",
    refresh_interval_help: "每次刷新行情之间的秒数。",
    default_symbols: "新建组件默认交易对",
    market_fallback: "异常时切换备用源",
    market_fallback_help: "当前数据源异常时自动切换备用源。",
    alert_settings: "告警规则",
    alert_enabled: "启用告警",
    alert_symbol: "交易对",
    alert_condition: "条件",
    alert_threshold: "阈值",
    alert_clear: "清除告警",
    symbols: "交易对",
    symbols_help: "最多 20 个，选择后以标签显示",
    empty_pairs: "未配置交易对",
    auto_start: "开机启动",
    show_main_window_on_startup: "启动时显示主界面",
    shortcut: "显示/隐藏快捷键",
    tray_icon: "显示托盘图标",
    tray_hover_display: "悬停托盘时显示",
    network_proxy_settings: "网络代理",
    network_proxy_enabled: "启用网络代理",
    network_proxy_url: "代理地址",
    network_proxy_http_example: "http://127.0.0.1:7890",
    network_proxy_socks_example: "socks5://127.0.0.1:1080",
    network_proxy_help: "行情、交易对搜索和更新请求会走代理。",
    app_version: "版本",
    about_us: "关于我们",
    icon_cache: "图标缓存",
    icon_cache_help: "移除本地已缓存的币种 Logo。",
    clear_icon_cache: "清空图标",
    custom_components: "自定义小组件",
    custom_components_help: "打开本地小组件插件目录。",
    open_custom_components_folder: "打开目录",
    theme: "主题",
    language: "语言",
    appearance_interface: "界面",
    appearance_widget_defaults: "小组件默认值",
    theme_help: "同步调整主界面和桌面小组件配色。",
    language_help: "切换后界面文案会立即刷新。",
    red_up_color_help: "开启后上涨显示红色，下跌显示绿色。",
    default_opacity_help: "控制新建小组件默认透明度。",
    default_always_on_top_help: "开启后新建小组件默认置顶。",
    system_startup: "启动",
    system_tray: "托盘",
    system_app_info: "应用信息",
    system_maintenance: "维护",
    auto_start_help: "登录 Windows 后自动运行应用。",
    show_main_window_on_startup_help: "启动应用时同时打开主界面。",
    shortcut_help: "按 Alt+C 快速隐藏或恢复小组件。",
    tray_icon_help: "保留托盘入口，方便打开主界面或退出。",
    tray_hover_display_help: "鼠标悬停托盘图标时临时显示小组件。",
    apply: "应用",
    widget_library: "小组件库",
    my_widgets: "我的小组件",
    selected_widget: "当前所选：",
    selected_widget_description: "以下设置仅影响当前选择的小组件，不影响其他小组件。",
    widget_name: "名称",
    lock_position_help: "锁定后将固定在当前位置。",
    widget_scale_help: "调整小组件整体大小。",
    opacity_help: "调整小组件透明度。",
    widget_show_coin_logos: "显示币种 Logo",
    widget_show_coin_logos_help: "在交易对旁显示币种图标。",
    widget_hide_quote_asset: "隐藏报价币",
    widget_hide_quote_asset_help: "如 BTC/USDC 仅显示 BTC。",
    widget_topmost: "总在最前",
    widget_topmost_help: "始终显示在其他窗口之上。",
    advanced_options: "高级选项",
    reset_widget_positions: "重置位置",
    hide_all_widgets: "隐藏全部",
    reset: "重置",
    preview: "预览",
    preview_pairs: "交易对",
    preview_updated: "3秒前更新",
    preview_source_ok: "数据正常",
    app_settings: "显示设置",
    add_widget: "添加",
    apply_widget: "应用小组件",
    no_widgets: "暂无小组件",
    quote_board_title: "行情面板",
    plugin_market_description: "可用的内置和本地小组件。",
    my_widgets_description: "管理已添加的小组件、交易对和窗口显示。",
    market_settings_description: "配置行情源、刷新频率、默认交易对和告警规则。",
    appearance_settings_description: "调整主题、语言和新建小组件默认显示。",
    system_settings_description: "管理启动行为、网络代理、托盘和应用信息。",
    quote_board_description: "最多显示 20 个交易对，适合同时观察主流市场价格。",
    mini_ticker_title: "迷你行情",
    mini_ticker_description: "只显示 1 个交易对，适合贴在桌面边角持续观察。",
    market_settings: "行情设置",
    appearance_settings: "外观设置",
    system_settings: "系统设置",
    settings_path_label: "设置文件",
    close: "关闭",
    source_prefix: "实时行情",
    runtime_no_pairs: "未配置交易对",
    runtime_connecting: "连接中",
    runtime_connection_error: "连接失败",
    runtime_updated: "已更新",
    runtime_stale: "已过期",
    runtime_fallback: "备用源",
    runtime_source_error: "数据源异常",
    runtime_live_count_prefix: "已连 ",
    runtime_live_count_suffix: "",
    widget_visible: "已显示",
    widget_hidden: "已隐藏",
    delete_widget: "移除小组件",
    status_alert_invalid: "告警规则不完整",
    status_auto_start_failed: "自启动设置失败",
    status_shortcut_failed: "快捷键注册失败",
    status_network_proxy_invalid: "网络代理无效",
    status_icon_cache_clear_failed: "图标缓存清理失败",
    status_custom_components_folder_open_failed: "无法打开自定义小组件目录",
    status_symbol_catalog_fallback: "正在使用热门交易对目录",
};

pub fn resolve_locale(preference: LanguagePreference) -> Locale {
    match preference {
        LanguagePreference::En => Locale::En,
        LanguagePreference::ZhHans => Locale::ZhHans,
        LanguagePreference::System => locale_from_system(),
    }
}

pub fn locale_from_system() -> Locale {
    sys_locale::get_locale()
        .as_deref()
        .map(locale_from_tag)
        .unwrap_or(Locale::En)
}

pub fn locale_from_tag(locale: &str) -> Locale {
    if locale.trim().to_ascii_lowercase().starts_with("zh") {
        Locale::ZhHans
    } else {
        Locale::En
    }
}

pub fn text(locale: Locale) -> &'static UiText {
    match locale {
        Locale::En => &EN_TEXT,
        Locale::ZhHans => &ZH_HANS_TEXT,
    }
}

pub fn provider_options(locale: Locale) -> Vec<&'static str> {
    match locale {
        Locale::En => vec!["Auto", "Binance", "OKX"],
        Locale::ZhHans => vec!["自动", "Binance", "OKX"],
    }
}

pub fn shortcut_options(locale: Locale) -> Vec<&'static str> {
    match locale {
        Locale::En => vec!["Alt+C", "Disabled"],
        Locale::ZhHans => vec!["Alt+C", "禁用"],
    }
}

pub fn language_options(locale: Locale) -> Vec<&'static str> {
    match locale {
        Locale::En => vec!["System", "English", "Simplified Chinese"],
        Locale::ZhHans => vec!["跟随系统", "English", "简体中文"],
    }
}

pub fn theme_options(locale: Locale) -> Vec<&'static str> {
    match locale {
        Locale::En => vec!["System", "Light", "Dark"],
        Locale::ZhHans => vec!["跟随系统", "浅色", "深色"],
    }
}

pub fn alert_condition_options(locale: Locale) -> Vec<&'static str> {
    match locale {
        Locale::En => vec![
            "Price above",
            "Price below",
            "24h change above",
            "24h change below",
        ],
        Locale::ZhHans => vec!["价格高于", "价格低于", "24h 涨跌高于", "24h 涨跌低于"],
    }
}

pub fn widget_title(locale: Locale, widget: WidgetText) -> &'static str {
    let text = text(locale);
    match widget {
        WidgetText::QuoteBoard => text.quote_board_title,
        WidgetText::MiniTicker => text.mini_ticker_title,
    }
}

pub fn widget_description(locale: Locale, widget: WidgetText) -> &'static str {
    let text = text(locale);
    match widget {
        WidgetText::QuoteBoard => text.quote_board_description,
        WidgetText::MiniTicker => text.mini_ticker_description,
    }
}

pub fn widget_usage_text(locale: Locale, count: usize) -> String {
    match locale {
        Locale::En => format!("Used {count}"),
        Locale::ZhHans => format!("已使用 {count} 个"),
    }
}

pub fn default_widget_name(locale: Locale, widget: WidgetText, number: u64) -> String {
    format!("{} {number}", widget_title(locale, widget))
}

pub fn market_error_notification_body(locale: Locale, error: &str) -> String {
    match locale {
        Locale::En => format!("Market data update failed: {error}"),
        Locale::ZhHans => format!("行情更新失败：{error}"),
    }
}

pub fn update_available_notification_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Crypto HUD update available",
        Locale::ZhHans => "Crypto HUD 有可用更新",
    }
}

pub fn update_available_notification_body(
    locale: Locale,
    tag_name: &str,
    asset_name: Option<&str>,
    checksum_asset_name: Option<&str>,
) -> String {
    match (locale, asset_name, checksum_asset_name) {
        (Locale::En, Some(asset), Some(checksum)) => format!(
            "{tag_name} is available. Download {asset} and verify it with {checksum} from GitHub Releases."
        ),
        (Locale::En, Some(asset), None) => {
            format!("{tag_name} is available. Download {asset} from GitHub Releases.")
        }
        (Locale::En, None, _) => format!("{tag_name} is available on GitHub Releases."),
        (Locale::ZhHans, Some(asset), Some(checksum)) => format!(
            "已发布 {tag_name}。请在 GitHub Releases 下载 {asset}，并使用 {checksum} 校验。"
        ),
        (Locale::ZhHans, Some(asset), None) => {
            format!("已发布 {tag_name}。请在 GitHub Releases 下载 {asset}。")
        }
        (Locale::ZhHans, None, _) => {
            format!("已发布 {tag_name}，可在 GitHub Releases 查看。")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_tags_resolve_to_supported_locales() {
        assert_eq!(locale_from_tag("zh-CN"), Locale::ZhHans);
        assert_eq!(locale_from_tag("zh-Hans"), Locale::ZhHans);
        assert_eq!(locale_from_tag("en-US"), Locale::En);
    }

    #[test]
    fn explicit_language_preference_overrides_system() {
        assert_eq!(resolve_locale(LanguagePreference::En), Locale::En);
        assert_eq!(resolve_locale(LanguagePreference::ZhHans), Locale::ZhHans);
    }

    #[test]
    fn settings_and_market_copy_follow_locale() {
        let en = text(Locale::En);
        assert_eq!(en.tab_widgets, "Widgets");
        assert_eq!(
            en.show_main_window_on_startup,
            "Show main window on startup"
        );
        assert_eq!(en.network_proxy_settings, "Network Proxy");
        assert_eq!(en.tray_hover_display, "Show on tray hover");
        assert_eq!(en.seconds_unit, "sec");
        assert_eq!(en.appearance_widget_defaults, "Widget Defaults");
        assert_eq!(en.system_tray, "Tray");
        assert_eq!(en.system_maintenance, "Maintenance");
        assert_eq!(en.app_version, "Version");
        assert_eq!(en.custom_components, "Custom widgets");
        assert_eq!(en.open_custom_components_folder, "Open Folder");
        assert_eq!(en.status_network_proxy_invalid, "Network proxy is invalid");
        assert_eq!(
            en.status_custom_components_folder_open_failed,
            "Could not open custom widgets folder"
        );
        assert_eq!(en.runtime_connection_error, "Connection failed");
        assert_eq!(en.runtime_live_count_suffix, " live");
        assert_eq!(en.tab_plugin_market, "Widget Library");
        assert_eq!(en.tab_market_data, "Market Data");
        assert_eq!(en.widget_visible, "Shown");
        assert_eq!(en.delete_widget, "Remove widget");
        assert_eq!(en.preview_pairs, "Pairs");
        assert_eq!(en.preview_updated, "Updated 3s");
        assert_eq!(en.preview_source_ok, "Source OK");
        assert_eq!(en.widget_show_coin_logos, "Show coin logos");
        assert_eq!(en.widget_hide_quote_asset, "Hide quote asset");
        assert_eq!(en.source_prefix, "Live feed");
        assert_eq!(en.runtime_source_error, "Source issue");
        assert_eq!(
            widget_title(Locale::En, WidgetText::QuoteBoard),
            "Quote Board"
        );
        assert_eq!(
            widget_description(Locale::En, WidgetText::MiniTicker),
            "Shows 1 pair in a compact window for desktop corners."
        );
        assert_eq!(widget_usage_text(Locale::En, 2), "Used 2");
        assert_eq!(
            market_error_notification_body(Locale::En, "timeout"),
            "Market data update failed: timeout"
        );
        assert_eq!(
            update_available_notification_body(
                Locale::En,
                "v1.2.3",
                Some("crypto-hud.exe"),
                Some("checksums.txt")
            ),
            "v1.2.3 is available. Download crypto-hud.exe and verify it with checksums.txt from GitHub Releases."
        );
        assert_eq!(provider_options(Locale::En), vec!["Auto", "Binance", "OKX"]);
        assert_eq!(shortcut_options(Locale::En), vec!["Alt+C", "Disabled"]);
        assert_eq!(
            alert_condition_options(Locale::En),
            vec![
                "Price above",
                "Price below",
                "24h change above",
                "24h change below"
            ]
        );

        let zh = text(Locale::ZhHans);
        assert_eq!(zh.tab_widgets, "小组件");
        assert_eq!(zh.show_main_window_on_startup, "启动时显示主界面");
        assert_eq!(zh.network_proxy_settings, "网络代理");
        assert_eq!(zh.tray_hover_display, "悬停托盘时显示");
        assert_eq!(zh.seconds_unit, "秒");
        assert_eq!(zh.appearance_widget_defaults, "小组件默认值");
        assert_eq!(zh.system_tray, "托盘");
        assert_eq!(zh.system_maintenance, "维护");
        assert_eq!(zh.app_version, "版本");
        assert_eq!(zh.custom_components, "自定义小组件");
        assert_eq!(zh.open_custom_components_folder, "打开目录");
        assert_eq!(zh.status_network_proxy_invalid, "网络代理无效");
        assert_eq!(
            zh.status_custom_components_folder_open_failed,
            "无法打开自定义小组件目录"
        );
        assert_eq!(zh.runtime_connection_error, "连接失败");
        assert_eq!(zh.runtime_live_count_prefix, "已连 ");
        assert_eq!(zh.tab_plugin_market, "组件库");
        assert_eq!(zh.tab_market_data, "行情");
        assert_eq!(zh.widget_visible, "已显示");
        assert_eq!(zh.delete_widget, "移除小组件");
        assert_eq!(zh.selected_widget, "当前所选：");
        assert_eq!(zh.preview_pairs, "交易对");
        assert_eq!(zh.preview_updated, "3秒前更新");
        assert_eq!(zh.preview_source_ok, "数据正常");
        assert_eq!(zh.widget_show_coin_logos, "显示币种 Logo");
        assert_eq!(zh.widget_hide_quote_asset, "隐藏报价币");
        assert_eq!(zh.market_settings, "行情设置");
        assert_eq!(
            widget_title(Locale::ZhHans, WidgetText::QuoteBoard),
            "行情面板"
        );
        assert_eq!(
            widget_description(Locale::ZhHans, WidgetText::MiniTicker),
            "只显示 1 个交易对，适合贴在桌面边角持续观察。"
        );
        assert_eq!(widget_usage_text(Locale::ZhHans, 2), "已使用 2 个");
        assert_eq!(
            market_error_notification_body(Locale::ZhHans, "timeout"),
            "行情更新失败：timeout"
        );
        assert_eq!(
            update_available_notification_body(
                Locale::ZhHans,
                "v1.2.3",
                Some("crypto-hud.exe"),
                Some("checksums.txt")
            ),
            "已发布 v1.2.3。请在 GitHub Releases 下载 crypto-hud.exe，并使用 checksums.txt 校验。"
        );
        assert_eq!(
            provider_options(Locale::ZhHans),
            vec!["自动", "Binance", "OKX"]
        );
        assert_eq!(shortcut_options(Locale::ZhHans), vec!["Alt+C", "禁用"]);
        assert_eq!(
            alert_condition_options(Locale::ZhHans),
            vec!["价格高于", "价格低于", "24h 涨跌高于", "24h 涨跌低于"]
        );
    }
}
