use crypto_hud_core::{
    format_market_pair_symbol, format_pair_change, format_price, AlertCondition,
};
use crypto_hud_shell_state::LanguagePreference;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
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

impl Locale {
    pub const ALL: [Self; 12] = [
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
}

#[cfg(test)]
impl Locale {
    pub const NON_ENGLISH: [Self; 11] = [
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
    pub tray_show_widgets: &'static str,
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
    pub network_proxy_example_hint: &'static str,
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
    pub widget_theme: &'static str,
    pub widget_theme_help: &'static str,
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
    pub runtime_source_error: &'static str,
    pub runtime_live_count_prefix: &'static str,
    pub runtime_live_count_suffix: &'static str,
    pub runtime_elapsed_second_unit: &'static str,
    pub runtime_elapsed_minute_unit: &'static str,
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
    tray_show_widgets: "Show Widgets",
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
    network_proxy_example_hint: "Examples: http://127.0.0.1:7890  ·  socks5://127.0.0.1:1080",
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
    widget_theme: "Widget theme",
    widget_theme_help: "Override this widget, or follow the system.",
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
    runtime_source_error: "Source issue",
    runtime_live_count_prefix: "",
    runtime_live_count_suffix: " live",
    runtime_elapsed_second_unit: "s",
    runtime_elapsed_minute_unit: "m",
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
    tray_show_widgets: "显示小组件",
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
    network_proxy_example_hint: "示例：http://127.0.0.1:7890  ·  socks5://127.0.0.1:1080",
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
    widget_theme: "小组件主题",
    widget_theme_help: "单独设置此小组件，或跟随系统。",
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
    runtime_source_error: "数据源异常",
    runtime_live_count_prefix: "已连 ",
    runtime_live_count_suffix: "",
    runtime_elapsed_second_unit: "秒",
    runtime_elapsed_minute_unit: "分钟",
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

const ZH_HANT_TEXT: UiText = UiText {
    tray_tooltip: "Crypto HUD",
    tray_settings: "主視窗",
    tray_show_widgets: "顯示小工具",
    tray_quit: "結束",
    settings_title: "Crypto HUD",
    tab_widgets: "小工具",
    tab_plugin_market: "工具庫",
    tab_market_data: "行情",
    tab_appearance: "外觀",
    tab_system: "系統",
    always_on_top: "鎖定位置",
    default_always_on_top: "新工具預設置頂",
    opacity: "透明度",
    default_opacity: "預設透明度",
    widget_scale: "縮放",
    red_up_color: "紅漲綠跌",
    market_provider: "啟用資料源",
    refresh_interval: "更新頻率",
    seconds_unit: "秒",
    market_provider_help: "控制搜尋目錄和已選交易對的資料源。",
    refresh_interval_help: "每次更新行情之間的秒數。",
    default_symbols: "新工具預設交易對",
    alert_settings: "警示規則",
    alert_enabled: "啟用警示",
    alert_symbol: "交易對",
    alert_condition: "條件",
    alert_threshold: "門檻",
    alert_clear: "清除警示",
    symbols: "交易對",
    symbols_help: "最多 20 個，選擇後以標籤顯示",
    empty_pairs: "尚未設定交易對",
    auto_start: "登入時啟動",
    show_main_window_on_startup: "啟動時顯示主視窗",
    shortcut: "顯示/隱藏快捷鍵",
    tray_icon: "顯示系統匣圖示",
    tray_hover_display: "游標停在系統匣時顯示",
    network_proxy_settings: "網路代理",
    network_proxy_enabled: "啟用網路代理",
    network_proxy_url: "代理位址",
    network_proxy_example_hint: "範例：http://127.0.0.1:7890  ·  socks5://127.0.0.1:1080",
    network_proxy_help: "行情、交易對搜尋和更新請求會透過代理。",
    app_version: "版本",
    about_us: "關於我們",
    icon_cache: "圖示快取",
    icon_cache_help: "移除本機已快取的幣種 Logo。",
    clear_icon_cache: "清空圖示",
    custom_components: "自訂小工具",
    custom_components_help: "開啟本機小工具外掛目錄。",
    open_custom_components_folder: "開啟目錄",
    theme: "主題",
    language: "語言",
    appearance_interface: "介面",
    appearance_widget_defaults: "小工具預設值",
    theme_help: "同步調整主視窗與桌面小工具配色。",
    language_help: "切換後介面文案會立即更新。",
    red_up_color_help: "開啟後上漲顯示紅色，下跌顯示綠色。",
    default_opacity_help: "控制新工具預設透明度。",
    default_always_on_top_help: "開啟後新工具預設置頂。",
    system_startup: "啟動",
    system_tray: "系統匣",
    system_app_info: "應用程式資訊",
    system_maintenance: "維護",
    auto_start_help: "登入 Windows 後自動執行應用程式。",
    show_main_window_on_startup_help: "啟動應用程式時同時開啟主視窗。",
    shortcut_help: "按 Alt+C 快速隱藏或還原小工具。",
    tray_icon_help: "保留系統匣入口，方便開啟主視窗或結束。",
    tray_hover_display_help: "游標停在系統匣圖示時暫時顯示小工具。",
    apply: "套用",
    widget_library: "小工具庫",
    my_widgets: "我的小工具",
    selected_widget: "目前選取：",
    selected_widget_description: "以下設定只影響目前選取的小工具。",
    widget_name: "名稱",
    lock_position_help: "鎖定後會固定在目前位置。",
    widget_scale_help: "調整小工具整體大小。",
    opacity_help: "調整小工具透明度。",
    widget_show_coin_logos: "顯示幣種 Logo",
    widget_show_coin_logos_help: "在交易對旁顯示幣種圖示。",
    widget_hide_quote_asset: "隱藏報價幣",
    widget_hide_quote_asset_help: "例如 BTC/USDC 只顯示 BTC。",
    widget_theme: "小工具主題",
    widget_theme_help: "單獨設定此小工具，或跟隨系統。",
    widget_topmost: "總在最上層",
    widget_topmost_help: "讓此小工具顯示在其他視窗之上。",
    advanced_options: "進階選項",
    reset_widget_positions: "重設位置",
    hide_all_widgets: "全部隱藏",
    reset: "重設",
    preview: "預覽",
    preview_pairs: "交易對",
    preview_updated: "3 秒前更新",
    preview_source_ok: "資料正常",
    app_settings: "顯示設定",
    add_widget: "新增",
    apply_widget: "套用小工具",
    no_widgets: "暫無小工具",
    quote_board_title: "行情面板",
    plugin_market_description: "可用的內建和本機小工具。",
    my_widgets_description: "管理已新增的小工具、交易對和視窗顯示。",
    market_settings_description: "設定行情源、更新頻率、預設交易對和警示規則。",
    appearance_settings_description: "調整主題、語言和新工具預設顯示。",
    system_settings_description: "管理啟動行為、網路代理、系統匣和應用程式資訊。",
    quote_board_description: "最多顯示 20 個交易對，適合同時觀察主流市場價格。",
    mini_ticker_title: "迷你行情",
    mini_ticker_description: "只顯示 1 個交易對，適合貼在桌面角落持續觀察。",
    market_settings: "行情設定",
    appearance_settings: "外觀設定",
    system_settings: "系統設定",
    settings_path_label: "設定檔",
    close: "關閉",
    source_prefix: "即時行情",
    runtime_no_pairs: "尚未設定交易對",
    runtime_connecting: "連線中",
    runtime_connection_error: "連線失敗",
    runtime_updated: "已更新",
    runtime_stale: "已過期",
    runtime_source_error: "資料源異常",
    runtime_live_count_prefix: "已連 ",
    runtime_live_count_suffix: "",
    runtime_elapsed_second_unit: "秒",
    runtime_elapsed_minute_unit: "分鐘",
    widget_visible: "已顯示",
    widget_hidden: "已隱藏",
    delete_widget: "移除小工具",
    status_alert_invalid: "警示規則不完整",
    status_auto_start_failed: "自動啟動設定失敗",
    status_shortcut_failed: "快捷鍵註冊失敗",
    status_network_proxy_invalid: "網路代理無效",
    status_icon_cache_clear_failed: "圖示快取清理失敗",
    status_custom_components_folder_open_failed: "無法開啟自訂小工具目錄",
    status_symbol_catalog_fallback: "正在使用熱門交易對目錄",
};

const ES_419_TEXT: UiText = UiText {
    tray_tooltip: "Crypto HUD",
    tray_settings: "Ventana principal",
    tray_show_widgets: "Mostrar widgets",
    tray_quit: "Salir",
    settings_title: "Crypto HUD",
    tab_widgets: "Widgets",
    tab_plugin_market: "Biblioteca de widgets",
    tab_market_data: "Mercado",
    tab_appearance: "Apariencia",
    tab_system: "Sistema",
    always_on_top: "Bloquear posición",
    default_always_on_top: "Fijar widgets nuevos arriba",
    opacity: "Opacidad",
    default_opacity: "Opacidad predeterminada",
    widget_scale: "Escala",
    red_up_color: "Rojo para subidas",
    market_provider: "Fuentes activas",
    refresh_interval: "Intervalo de actualización",
    seconds_unit: "s",
    market_provider_help: "Se muestra en la búsqueda de pares y se usa para los pares elegidos.",
    refresh_interval_help: "Segundos entre actualizaciones de precios.",
    default_symbols: "Pares predeterminados para widgets nuevos",
    alert_settings: "Regla de alerta",
    alert_enabled: "Activar alerta",
    alert_symbol: "Par",
    alert_condition: "Condición",
    alert_threshold: "Umbral",
    alert_clear: "Borrar alerta",
    symbols: "Pares",
    symbols_help: "Hasta 20; los pares elegidos aparecen como etiquetas",
    empty_pairs: "No hay pares configurados",
    auto_start: "Iniciar al iniciar sesión",
    show_main_window_on_startup: "Mostrar ventana principal al iniciar",
    shortcut: "Atajo para mostrar/ocultar",
    tray_icon: "Mostrar icono de bandeja",
    tray_hover_display: "Mostrar al pasar por la bandeja",
    network_proxy_settings: "Proxy de red",
    network_proxy_enabled: "Activar proxy de red",
    network_proxy_url: "Dirección del proxy",
    network_proxy_example_hint: "Ejemplos: http://127.0.0.1:7890  ·  socks5://127.0.0.1:1080",
    network_proxy_help: "Envía precios, búsqueda de pares y actualizaciones por el proxy.",
    app_version: "Versión",
    about_us: "Acerca de",
    icon_cache: "Caché de iconos",
    icon_cache_help: "Elimina logos de monedas guardados localmente.",
    clear_icon_cache: "Borrar iconos",
    custom_components: "Widgets personalizados",
    custom_components_help: "Abre la carpeta de plugins de widgets locales.",
    open_custom_components_folder: "Abrir carpeta",
    theme: "Tema",
    language: "Idioma",
    appearance_interface: "Interfaz",
    appearance_widget_defaults: "Valores predeterminados",
    theme_help: "Actualiza colores de la ventana principal y los widgets.",
    language_help: "Actualiza el texto de la interfaz al instante.",
    red_up_color_help: "Muestra subidas en rojo y caídas en verde.",
    default_opacity_help: "Define la transparencia de los widgets nuevos.",
    default_always_on_top_help: "Mantiene los widgets nuevos por encima de otras ventanas.",
    system_startup: "Inicio",
    system_tray: "Bandeja",
    system_app_info: "Información de la app",
    system_maintenance: "Mantenimiento",
    auto_start_help: "Ejecuta Crypto HUD después de iniciar sesión.",
    show_main_window_on_startup_help: "Abre la ventana principal al iniciar la app.",
    shortcut_help: "Usa Alt+C para ocultar o restaurar widgets.",
    tray_icon_help: "Mantiene acceso rápido para abrir la ventana principal o salir.",
    tray_hover_display_help: "Muestra temporalmente los widgets al pasar por el icono de bandeja.",
    apply: "Aplicar",
    widget_library: "Biblioteca de widgets",
    my_widgets: "Mis widgets",
    selected_widget: "Widget seleccionado:",
    selected_widget_description: "Los ajustes siguientes solo afectan al widget seleccionado.",
    widget_name: "Nombre",
    lock_position_help: "Mantiene este widget fijo en su posición actual.",
    widget_scale_help: "Ajusta el tamaño general del widget.",
    opacity_help: "Ajusta la transparencia del widget.",
    widget_show_coin_logos: "Mostrar logos",
    widget_show_coin_logos_help: "Muestra iconos de tokens junto a los pares.",
    widget_hide_quote_asset: "Ocultar activo cotizado",
    widget_hide_quote_asset_help: "Muestra BTC en vez de BTC/USDC.",
    widget_theme: "Tema del widget",
    widget_theme_help: "Sobrescribe este widget o sigue el sistema.",
    widget_topmost: "Siempre arriba",
    widget_topmost_help: "Mantiene este widget por encima de otras ventanas.",
    advanced_options: "Opciones avanzadas",
    reset_widget_positions: "Restablecer",
    hide_all_widgets: "Ocultar todo",
    reset: "Restablecer",
    preview: "Vista previa",
    preview_pairs: "Pares",
    preview_updated: "Actualizado hace 3 s",
    preview_source_ok: "Fuente OK",
    app_settings: "Visualización",
    add_widget: "Agregar",
    apply_widget: "Aplicar widget",
    no_widgets: "Sin widgets",
    quote_board_title: "Panel de cotizaciones",
    mini_ticker_title: "Mini cotizador",
    plugin_market_description: "Widgets integrados y locales disponibles.",
    my_widgets_description: "Gestiona widgets añadidos, pares y visualización.",
    market_settings_description: "Configura fuentes, frecuencia, pares predeterminados y alertas.",
    appearance_settings_description: "Ajusta tema, idioma y visualización predeterminada.",
    system_settings_description: "Gestiona inicio, proxy, bandeja e información de la app.",
    quote_board_description: "Muestra hasta 20 pares para seguir mercados principales juntos.",
    mini_ticker_description: "Muestra 1 par en una ventana compacta para esquinas del escritorio.",
    market_settings: "Ajustes de mercado",
    appearance_settings: "Ajustes de apariencia",
    system_settings: "Ajustes del sistema",
    settings_path_label: "Archivo de ajustes",
    close: "Cerrar",
    source_prefix: "Feed en vivo",
    runtime_no_pairs: "Sin pares",
    runtime_connecting: "Conectando",
    runtime_connection_error: "Falló la conexión",
    runtime_updated: "Actualizado",
    runtime_stale: "Desactualizado",
    runtime_source_error: "Problema de fuente",
    runtime_live_count_prefix: "",
    runtime_live_count_suffix: " en vivo",
    runtime_elapsed_second_unit: " s",
    runtime_elapsed_minute_unit: " min",
    widget_visible: "Visible",
    widget_hidden: "Oculto",
    delete_widget: "Eliminar widget",
    status_alert_invalid: "La regla de alerta está incompleta",
    status_auto_start_failed: "Falló el inicio automático",
    status_shortcut_failed: "Falló el registro del atajo",
    status_network_proxy_invalid: "El proxy de red no es válido",
    status_icon_cache_clear_failed: "No se pudo borrar la caché de iconos",
    status_custom_components_folder_open_failed: "No se pudo abrir la carpeta de widgets",
    status_symbol_catalog_fallback: "Usando lista de pares populares",
};

const PT_BR_TEXT: UiText = UiText {
    tray_tooltip: "Crypto HUD",
    tray_settings: "Janela principal",
    tray_show_widgets: "Mostrar widgets",
    tray_quit: "Sair",
    settings_title: "Crypto HUD",
    tab_widgets: "Widgets",
    tab_plugin_market: "Biblioteca de widgets",
    tab_market_data: "Mercado",
    tab_appearance: "Aparência",
    tab_system: "Sistema",
    always_on_top: "Bloquear posição",
    default_always_on_top: "Fixar novos widgets no topo",
    opacity: "Opacidade",
    default_opacity: "Opacidade padrão",
    widget_scale: "Escala",
    red_up_color: "Vermelho para altas",
    market_provider: "Fontes ativas",
    refresh_interval: "Intervalo de atualização",
    seconds_unit: "s",
    market_provider_help: "Aparece na busca de pares e é usado pelos pares selecionados.",
    refresh_interval_help: "Segundos entre atualizações de cotações.",
    default_symbols: "Pares padrão para novos widgets",
    alert_settings: "Regra de alerta",
    alert_enabled: "Ativar alerta",
    alert_symbol: "Par",
    alert_condition: "Condição",
    alert_threshold: "Limite",
    alert_clear: "Limpar alerta",
    symbols: "Pares",
    symbols_help: "Até 20; pares selecionados aparecem como etiquetas",
    empty_pairs: "Nenhum par configurado",
    auto_start: "Iniciar ao entrar",
    show_main_window_on_startup: "Mostrar janela principal ao iniciar",
    shortcut: "Atalho mostrar/ocultar",
    tray_icon: "Mostrar ícone na bandeja",
    tray_hover_display: "Mostrar ao passar na bandeja",
    network_proxy_settings: "Proxy de rede",
    network_proxy_enabled: "Ativar proxy de rede",
    network_proxy_url: "Endereço do proxy",
    network_proxy_example_hint: "Exemplos: http://127.0.0.1:7890  ·  socks5://127.0.0.1:1080",
    network_proxy_help: "Roteia cotações, busca de pares e atualizações pelo proxy.",
    app_version: "Versão",
    about_us: "Sobre",
    icon_cache: "Cache de ícones",
    icon_cache_help: "Remove logos de moedas salvos localmente.",
    clear_icon_cache: "Limpar ícones",
    custom_components: "Widgets personalizados",
    custom_components_help: "Abre a pasta de plugins de widgets locais.",
    open_custom_components_folder: "Abrir pasta",
    theme: "Tema",
    language: "Idioma",
    appearance_interface: "Interface",
    appearance_widget_defaults: "Padrões dos widgets",
    theme_help: "Atualiza as cores da janela principal e dos widgets.",
    language_help: "Atualiza os textos da interface imediatamente.",
    red_up_color_help: "Mostra altas em vermelho e quedas em verde.",
    default_opacity_help: "Define a transparência dos novos widgets.",
    default_always_on_top_help: "Mantém novos widgets acima de outras janelas.",
    system_startup: "Inicialização",
    system_tray: "Bandeja",
    system_app_info: "Informações do app",
    system_maintenance: "Manutenção",
    auto_start_help: "Executa o Crypto HUD depois que você entra.",
    show_main_window_on_startup_help: "Abre a janela principal ao iniciar o app.",
    shortcut_help: "Use Alt+C para ocultar ou restaurar widgets.",
    tray_icon_help: "Mantém acesso rápido para abrir a janela principal ou sair.",
    tray_hover_display_help: "Mostra temporariamente os widgets ao passar pelo ícone da bandeja.",
    apply: "Aplicar",
    widget_library: "Biblioteca de widgets",
    my_widgets: "Meus widgets",
    selected_widget: "Widget selecionado:",
    selected_widget_description: "As opções abaixo afetam apenas o widget selecionado.",
    widget_name: "Nome",
    lock_position_help: "Mantém este widget fixo na posição atual.",
    widget_scale_help: "Ajusta o tamanho geral do widget.",
    opacity_help: "Ajusta a transparência do widget.",
    widget_show_coin_logos: "Mostrar logos",
    widget_show_coin_logos_help: "Mostra ícones dos tokens ao lado dos pares.",
    widget_hide_quote_asset: "Ocultar ativo de cotação",
    widget_hide_quote_asset_help: "Mostra BTC em vez de BTC/USDC.",
    widget_theme: "Tema do widget",
    widget_theme_help: "Substitui este widget ou segue o sistema.",
    widget_topmost: "Sempre no topo",
    widget_topmost_help: "Mantém este widget acima de outras janelas.",
    advanced_options: "Opções avançadas",
    reset_widget_positions: "Redefinir",
    hide_all_widgets: "Ocultar todos",
    reset: "Redefinir",
    preview: "Prévia",
    preview_pairs: "Pares",
    preview_updated: "Atualizado há 3 s",
    preview_source_ok: "Fonte OK",
    app_settings: "Exibição",
    add_widget: "Adicionar",
    apply_widget: "Aplicar widget",
    no_widgets: "Sem widgets",
    quote_board_title: "Painel de cotações",
    mini_ticker_title: "Mini cotação",
    plugin_market_description: "Widgets integrados e locais disponíveis.",
    my_widgets_description: "Gerencie widgets adicionados, pares e exibição.",
    market_settings_description: "Configure fontes, atualização, pares padrão e alertas.",
    appearance_settings_description: "Ajuste tema, idioma e exibição padrão.",
    system_settings_description: "Gerencie inicialização, proxy, bandeja e informações do app.",
    quote_board_description: "Mostra até 20 pares para acompanhar mercados principais juntos.",
    mini_ticker_description: "Mostra 1 par em uma janela compacta para cantos da área de trabalho.",
    market_settings: "Ajustes de mercado",
    appearance_settings: "Ajustes de aparência",
    system_settings: "Ajustes do sistema",
    settings_path_label: "Arquivo de ajustes",
    close: "Fechar",
    source_prefix: "Feed ao vivo",
    runtime_no_pairs: "Sem pares",
    runtime_connecting: "Conectando",
    runtime_connection_error: "Falha na conexão",
    runtime_updated: "Atualizado",
    runtime_stale: "Desatualizado",
    runtime_source_error: "Problema na fonte",
    runtime_live_count_prefix: "",
    runtime_live_count_suffix: " ao vivo",
    runtime_elapsed_second_unit: " s",
    runtime_elapsed_minute_unit: " min",
    widget_visible: "Visível",
    widget_hidden: "Oculto",
    delete_widget: "Remover widget",
    status_alert_invalid: "A regra de alerta está incompleta",
    status_auto_start_failed: "Falha no início automático",
    status_shortcut_failed: "Falha ao registrar atalho",
    status_network_proxy_invalid: "Proxy de rede inválido",
    status_icon_cache_clear_failed: "Falha ao limpar cache de ícones",
    status_custom_components_folder_open_failed: "Não foi possível abrir a pasta de widgets",
    status_symbol_catalog_fallback: "Usando lista de pares populares",
};

const VI_TEXT: UiText = UiText {
    tray_tooltip: "Crypto HUD",
    tray_settings: "Cửa sổ chính",
    tray_show_widgets: "Hiện widget",
    tray_quit: "Thoát",
    settings_title: "Crypto HUD",
    tab_widgets: "Widget",
    tab_plugin_market: "Thư viện widget",
    tab_market_data: "Thị trường",
    tab_appearance: "Giao diện",
    tab_system: "Hệ thống",
    always_on_top: "Khóa vị trí",
    default_always_on_top: "Ghim widget mới lên trên",
    opacity: "Độ mờ",
    default_opacity: "Độ mờ mặc định",
    widget_scale: "Tỉ lệ",
    red_up_color: "Đỏ khi tăng",
    market_provider: "Nguồn đang bật",
    refresh_interval: "Chu kỳ cập nhật",
    seconds_unit: "giây",
    market_provider_help: "Hiển thị trong tìm kiếm cặp và dùng cho các cặp đã chọn.",
    refresh_interval_help: "Số giây giữa các lần cập nhật giá.",
    default_symbols: "Cặp mặc định cho widget mới",
    alert_settings: "Quy tắc cảnh báo",
    alert_enabled: "Bật cảnh báo",
    alert_symbol: "Cặp",
    alert_condition: "Điều kiện",
    alert_threshold: "Ngưỡng",
    alert_clear: "Xóa cảnh báo",
    symbols: "Cặp",
    symbols_help: "Tối đa 20; cặp đã chọn sẽ hiển thị dạng thẻ",
    empty_pairs: "Chưa cấu hình cặp",
    auto_start: "Mở khi đăng nhập",
    show_main_window_on_startup: "Hiện cửa sổ chính khi khởi động",
    shortcut: "Phím tắt hiện/ẩn",
    tray_icon: "Hiện biểu tượng khay",
    tray_hover_display: "Hiện khi rê chuột lên khay",
    network_proxy_settings: "Proxy mạng",
    network_proxy_enabled: "Bật proxy mạng",
    network_proxy_url: "Địa chỉ proxy",
    network_proxy_example_hint: "Ví dụ: http://127.0.0.1:7890  ·  socks5://127.0.0.1:1080",
    network_proxy_help: "Định tuyến giá, tìm kiếm cặp và cập nhật qua proxy.",
    app_version: "Phiên bản",
    about_us: "Giới thiệu",
    icon_cache: "Bộ nhớ đệm biểu tượng",
    icon_cache_help: "Xóa logo coin đã lưu cục bộ.",
    clear_icon_cache: "Xóa biểu tượng",
    custom_components: "Widget tùy chỉnh",
    custom_components_help: "Mở thư mục plugin widget cục bộ.",
    open_custom_components_folder: "Mở thư mục",
    theme: "Chủ đề",
    language: "Ngôn ngữ",
    appearance_interface: "Giao diện",
    appearance_widget_defaults: "Mặc định widget",
    theme_help: "Cập nhật màu cho cửa sổ chính và widget.",
    language_help: "Làm mới văn bản giao diện ngay lập tức.",
    red_up_color_help: "Hiển thị tăng giá màu đỏ và giảm giá màu xanh.",
    default_opacity_help: "Đặt độ trong suốt cho widget mới.",
    default_always_on_top_help: "Giữ widget mới nằm trên các cửa sổ khác.",
    system_startup: "Khởi động",
    system_tray: "Khay hệ thống",
    system_app_info: "Thông tin ứng dụng",
    system_maintenance: "Bảo trì",
    auto_start_help: "Chạy Crypto HUD sau khi bạn đăng nhập.",
    show_main_window_on_startup_help: "Mở cửa sổ chính khi khởi chạy ứng dụng.",
    shortcut_help: "Dùng Alt+C để ẩn hoặc khôi phục widget.",
    tray_icon_help: "Giữ lối tắt để mở cửa sổ chính hoặc thoát.",
    tray_hover_display_help: "Tạm thời hiện widget khi rê chuột lên biểu tượng khay.",
    apply: "Áp dụng",
    widget_library: "Thư viện widget",
    my_widgets: "Widget của tôi",
    selected_widget: "Widget đã chọn:",
    selected_widget_description: "Các thiết lập dưới đây chỉ ảnh hưởng widget đã chọn.",
    widget_name: "Tên",
    lock_position_help: "Giữ widget này cố định tại vị trí hiện tại.",
    widget_scale_help: "Điều chỉnh kích thước tổng thể của widget.",
    opacity_help: "Điều chỉnh độ trong suốt của widget.",
    widget_show_coin_logos: "Hiện logo coin",
    widget_show_coin_logos_help: "Hiển thị biểu tượng token cạnh cặp.",
    widget_hide_quote_asset: "Ẩn tài sản định giá",
    widget_hide_quote_asset_help: "Hiển thị BTC thay vì BTC/USDC.",
    widget_theme: "Chủ đề widget",
    widget_theme_help: "Ghi đè widget này hoặc theo hệ thống.",
    widget_topmost: "Luôn ở trên",
    widget_topmost_help: "Giữ widget này trên các cửa sổ khác.",
    advanced_options: "Tùy chọn nâng cao",
    reset_widget_positions: "Đặt lại",
    hide_all_widgets: "Ẩn tất cả",
    reset: "Đặt lại",
    preview: "Xem trước",
    preview_pairs: "Cặp",
    preview_updated: "Cập nhật 3 giây trước",
    preview_source_ok: "Nguồn OK",
    app_settings: "Hiển thị widget",
    add_widget: "Thêm",
    apply_widget: "Áp dụng widget",
    no_widgets: "Không có widget",
    quote_board_title: "Bảng giá",
    plugin_market_description: "Widget tích hợp và cục bộ có sẵn.",
    my_widgets_description: "Quản lý widget đã thêm, cặp và hiển thị cửa sổ.",
    market_settings_description: "Cấu hình nguồn giá, tần suất, cặp mặc định và cảnh báo.",
    appearance_settings_description: "Điều chỉnh chủ đề, ngôn ngữ và mặc định widget.",
    system_settings_description: "Quản lý khởi động, proxy, khay và thông tin ứng dụng.",
    quote_board_description: "Hiển thị tối đa 20 cặp để theo dõi các thị trường chính.",
    mini_ticker_title: "Ticker mini",
    mini_ticker_description: "Hiển thị 1 cặp trong cửa sổ nhỏ ở góc màn hình.",
    market_settings: "Thiết lập thị trường",
    appearance_settings: "Thiết lập giao diện",
    system_settings: "Thiết lập hệ thống",
    settings_path_label: "Tệp thiết lập",
    close: "Đóng",
    source_prefix: "Feed trực tiếp",
    runtime_no_pairs: "Không có cặp",
    runtime_connecting: "Đang kết nối",
    runtime_connection_error: "Kết nối thất bại",
    runtime_updated: "Đã cập nhật",
    runtime_stale: "Đã cũ",
    runtime_source_error: "Lỗi nguồn",
    runtime_live_count_prefix: "",
    runtime_live_count_suffix: " đang hoạt động",
    runtime_elapsed_second_unit: " giây",
    runtime_elapsed_minute_unit: " phút",
    widget_visible: "Đang hiện",
    widget_hidden: "Đã ẩn",
    delete_widget: "Xóa widget",
    status_alert_invalid: "Quy tắc cảnh báo chưa đầy đủ",
    status_auto_start_failed: "Không bật được tự khởi động",
    status_shortcut_failed: "Không đăng ký được phím tắt",
    status_network_proxy_invalid: "Proxy mạng không hợp lệ",
    status_icon_cache_clear_failed: "Không xóa được bộ nhớ đệm biểu tượng",
    status_custom_components_folder_open_failed: "Không mở được thư mục widget tùy chỉnh",
    status_symbol_catalog_fallback: "Đang dùng danh sách cặp phổ biến",
};

const ID_TEXT: UiText = UiText {
    tray_tooltip: "Crypto HUD",
    tray_settings: "Jendela utama",
    tray_show_widgets: "Tampilkan widget",
    tray_quit: "Keluar",
    settings_title: "Crypto HUD",
    tab_widgets: "Widget",
    tab_plugin_market: "Pustaka widget",
    tab_market_data: "Pasar",
    tab_appearance: "Tampilan",
    tab_system: "Sistem",
    always_on_top: "Kunci posisi",
    default_always_on_top: "Sematkan widget baru di atas",
    opacity: "Opasitas",
    default_opacity: "Opasitas default",
    widget_scale: "Skala",
    red_up_color: "Merah untuk naik",
    market_provider: "Sumber aktif",
    refresh_interval: "Interval refresh",
    seconds_unit: "dtk",
    market_provider_help: "Ditampilkan di pencarian pair dan dipakai oleh pair yang dipilih.",
    refresh_interval_help: "Detik antar pembaruan harga.",
    default_symbols: "Pair default untuk widget baru",
    alert_settings: "Aturan peringatan",
    alert_enabled: "Aktifkan peringatan",
    alert_symbol: "Pair",
    alert_condition: "Kondisi",
    alert_threshold: "Ambang",
    alert_clear: "Hapus peringatan",
    symbols: "Pair",
    symbols_help: "Maksimal 20; pair terpilih muncul sebagai tag",
    empty_pairs: "Belum ada pair",
    auto_start: "Jalankan saat masuk",
    show_main_window_on_startup: "Tampilkan jendela utama saat mulai",
    shortcut: "Pintasan tampil/sembunyi",
    tray_icon: "Tampilkan ikon tray",
    tray_hover_display: "Tampilkan saat hover tray",
    network_proxy_settings: "Proxy jaringan",
    network_proxy_enabled: "Aktifkan proxy jaringan",
    network_proxy_url: "Alamat proxy",
    network_proxy_example_hint: "Contoh: http://127.0.0.1:7890  ·  socks5://127.0.0.1:1080",
    network_proxy_help: "Mengalihkan harga, pencarian pair, dan pembaruan melalui proxy.",
    app_version: "Versi",
    about_us: "Tentang",
    icon_cache: "Cache ikon",
    icon_cache_help: "Menghapus logo coin yang tersimpan lokal.",
    clear_icon_cache: "Hapus ikon",
    custom_components: "Widget kustom",
    custom_components_help: "Membuka folder plugin widget lokal.",
    open_custom_components_folder: "Buka folder",
    theme: "Tema",
    language: "Bahasa",
    appearance_interface: "Antarmuka",
    appearance_widget_defaults: "Default widget",
    theme_help: "Memperbarui warna jendela utama dan widget.",
    language_help: "Menyegarkan teks antarmuka seketika.",
    red_up_color_help: "Menampilkan kenaikan merah dan penurunan hijau.",
    default_opacity_help: "Mengatur transparansi widget baru.",
    default_always_on_top_help: "Menjaga widget baru di atas jendela lain.",
    system_startup: "Mulai otomatis",
    system_tray: "Area tray",
    system_app_info: "Info aplikasi",
    system_maintenance: "Pemeliharaan",
    auto_start_help: "Menjalankan Crypto HUD setelah Anda masuk.",
    show_main_window_on_startup_help: "Membuka jendela utama saat aplikasi mulai.",
    shortcut_help: "Gunakan Alt+C untuk menyembunyikan atau memulihkan widget.",
    tray_icon_help: "Menyimpan akses cepat untuk membuka jendela utama atau keluar.",
    tray_hover_display_help: "Menampilkan widget sementara saat ikon tray di-hover.",
    apply: "Terapkan",
    widget_library: "Pustaka widget",
    my_widgets: "Widget saya",
    selected_widget: "Widget dipilih:",
    selected_widget_description: "Pengaturan di bawah hanya memengaruhi widget yang dipilih.",
    widget_name: "Nama",
    lock_position_help: "Menjaga widget ini tetap di posisi saat ini.",
    widget_scale_help: "Menyesuaikan ukuran keseluruhan widget.",
    opacity_help: "Menyesuaikan transparansi widget.",
    widget_show_coin_logos: "Tampilkan logo coin",
    widget_show_coin_logos_help: "Menampilkan ikon token di samping pair.",
    widget_hide_quote_asset: "Sembunyikan aset kuotasi",
    widget_hide_quote_asset_help: "Menampilkan BTC, bukan BTC/USDC.",
    widget_theme: "Tema widget",
    widget_theme_help: "Timpa widget ini atau ikuti sistem.",
    widget_topmost: "Selalu di atas",
    widget_topmost_help: "Menjaga widget ini di atas jendela lain.",
    advanced_options: "Opsi lanjutan",
    reset_widget_positions: "Atur ulang posisi",
    hide_all_widgets: "Sembunyikan semua",
    reset: "Atur ulang",
    preview: "Pratinjau",
    preview_pairs: "Pair",
    preview_updated: "Diperbarui 3 dtk",
    preview_source_ok: "Sumber OK",
    app_settings: "Tampilan widget",
    add_widget: "Tambah",
    apply_widget: "Terapkan widget",
    no_widgets: "Tidak ada widget",
    quote_board_title: "Papan harga",
    plugin_market_description: "Widget bawaan dan lokal yang tersedia.",
    my_widgets_description: "Kelola widget, pair, dan tampilan jendela.",
    market_settings_description: "Atur sumber harga, refresh, pair default, dan peringatan.",
    appearance_settings_description: "Atur tema, bahasa, dan tampilan default widget.",
    system_settings_description: "Kelola startup, proxy, tray, dan info aplikasi.",
    quote_board_description: "Menampilkan hingga 20 pair untuk memantau pasar utama.",
    mini_ticker_title: "Ticker mini",
    mini_ticker_description: "Menampilkan 1 pair dalam jendela kecil untuk sudut desktop.",
    market_settings: "Pengaturan pasar",
    appearance_settings: "Pengaturan tampilan",
    system_settings: "Pengaturan sistem",
    settings_path_label: "File pengaturan",
    close: "Tutup",
    source_prefix: "Feed live",
    runtime_no_pairs: "Tanpa pair",
    runtime_connecting: "Menghubungkan",
    runtime_connection_error: "Koneksi gagal",
    runtime_updated: "Diperbarui",
    runtime_stale: "Usang",
    runtime_source_error: "Masalah sumber",
    runtime_live_count_prefix: "",
    runtime_live_count_suffix: " aktif",
    runtime_elapsed_second_unit: " dtk",
    runtime_elapsed_minute_unit: " mnt",
    widget_visible: "Tampil",
    widget_hidden: "Tersembunyi",
    delete_widget: "Hapus widget",
    status_alert_invalid: "Aturan peringatan belum lengkap",
    status_auto_start_failed: "Gagal mengaktifkan auto-start",
    status_shortcut_failed: "Gagal mendaftarkan pintasan",
    status_network_proxy_invalid: "Proxy jaringan tidak valid",
    status_icon_cache_clear_failed: "Gagal menghapus cache ikon",
    status_custom_components_folder_open_failed: "Tidak dapat membuka folder widget kustom",
    status_symbol_catalog_fallback: "Memakai daftar pair populer",
};

const TR_TEXT: UiText = UiText {
    tray_tooltip: "Crypto HUD",
    tray_settings: "Ana pencere",
    tray_show_widgets: "Widget'ları göster",
    tray_quit: "Çıkış",
    settings_title: "Crypto HUD",
    tab_widgets: "Widget'lar",
    tab_plugin_market: "Widget kitaplığı",
    tab_market_data: "Piyasa",
    tab_appearance: "Görünüm",
    tab_system: "Sistem",
    always_on_top: "Konumu kilitle",
    default_always_on_top: "Yeni widget'ları üstte tut",
    opacity: "Opaklık",
    default_opacity: "Varsayılan opaklık",
    widget_scale: "Ölçek",
    red_up_color: "Yükselişler kırmızı",
    market_provider: "Etkin kaynaklar",
    refresh_interval: "Yenileme aralığı",
    seconds_unit: "sn",
    market_provider_help: "Çift aramasında gösterilir ve seçili çiftler için kullanılır.",
    refresh_interval_help: "Fiyat güncellemeleri arasındaki saniye.",
    default_symbols: "Yeni widget'lar için varsayılan çiftler",
    alert_settings: "Uyarı kuralı",
    alert_enabled: "Uyarıyı etkinleştir",
    alert_symbol: "Çift",
    alert_condition: "Koşul",
    alert_threshold: "Eşik",
    alert_clear: "Uyarıyı temizle",
    symbols: "Çiftler",
    symbols_help: "En fazla 20; seçilen çiftler etiket olarak görünür",
    empty_pairs: "Çift yapılandırılmadı",
    auto_start: "Oturum açınca başlat",
    show_main_window_on_startup: "Başlangıçta ana pencereyi göster",
    shortcut: "Göster/gizle kısayolu",
    tray_icon: "Tepsi simgesini göster",
    tray_hover_display: "Tepsi üzerine gelince göster",
    network_proxy_settings: "Ağ proxy'si",
    network_proxy_enabled: "Ağ proxy'sini etkinleştir",
    network_proxy_url: "Proxy adresi",
    network_proxy_example_hint: "Ornekler: http://127.0.0.1:7890  ·  socks5://127.0.0.1:1080",
    network_proxy_help: "Fiyatları, çift aramasını ve güncellemeleri proxy üzerinden yönlendirir.",
    app_version: "Sürüm",
    about_us: "Hakkında",
    icon_cache: "Simge önbelleği",
    icon_cache_help: "Yerelde önbelleğe alınmış coin logolarını kaldırır.",
    clear_icon_cache: "Simgeleri temizle",
    custom_components: "Özel widget'lar",
    custom_components_help: "Yerel widget plugin klasörünü açar.",
    open_custom_components_folder: "Klasörü aç",
    theme: "Tema",
    language: "Dil",
    appearance_interface: "Arayüz",
    appearance_widget_defaults: "Widget varsayılanları",
    theme_help: "Ana pencere ve widget renklerini günceller.",
    language_help: "Arayüz metinlerini hemen yeniler.",
    red_up_color_help: "Yükselişleri kırmızı, düşüşleri yeşil gösterir.",
    default_opacity_help: "Yeni widget'ların saydamlığını ayarlar.",
    default_always_on_top_help: "Yeni widget'ları diğer pencerelerin üstünde tutar.",
    system_startup: "Başlangıç",
    system_tray: "Tepsi",
    system_app_info: "Uygulama bilgisi",
    system_maintenance: "Bakım",
    auto_start_help: "Oturum açtıktan sonra Crypto HUD'u çalıştırır.",
    show_main_window_on_startup_help: "Uygulama açıldığında ana pencereyi açar.",
    shortcut_help: "Widget'ları gizlemek veya geri getirmek için Alt+C kullanın.",
    tray_icon_help: "Ana pencereyi açmak veya çıkmak için hızlı erişimi korur.",
    tray_hover_display_help: "Tepsi simgesi üzerindeyken widget'ları geçici gösterir.",
    apply: "Uygula",
    widget_library: "Widget kitaplığı",
    my_widgets: "Widget'larım",
    selected_widget: "Seçili widget:",
    selected_widget_description: "Aşağıdaki ayarlar yalnızca seçili widget'ı etkiler.",
    widget_name: "Ad",
    lock_position_help: "Bu widget'ı mevcut konumunda sabit tutar.",
    widget_scale_help: "Widget'ın genel boyutunu ayarlar.",
    opacity_help: "Widget saydamlığını ayarlar.",
    widget_show_coin_logos: "Coin logolarını göster",
    widget_show_coin_logos_help: "Çiftlerin yanında token simgelerini gösterir.",
    widget_hide_quote_asset: "Karşıt varlığı gizle",
    widget_hide_quote_asset_help: "BTC/USDC yerine BTC gösterir.",
    widget_theme: "Widget teması",
    widget_theme_help: "Bu widget'ı ayrı ayarla veya sistemi takip et.",
    widget_topmost: "Her zaman üstte",
    widget_topmost_help: "Bu widget'ı diğer pencerelerin üstünde tutar.",
    advanced_options: "Gelişmiş seçenekler",
    reset_widget_positions: "Sıfırla",
    hide_all_widgets: "Tümünü gizle",
    reset: "Sıfırla",
    preview: "Önizleme",
    preview_pairs: "Çiftler",
    preview_updated: "3 sn önce güncellendi",
    preview_source_ok: "Kaynak OK",
    app_settings: "Widget görünümü",
    add_widget: "Ekle",
    apply_widget: "Widget'ı uygula",
    no_widgets: "Widget yok",
    quote_board_title: "Fiyat panosu",
    plugin_market_description: "Kullanılabilir yerleşik ve yerel widget'lar.",
    my_widgets_description: "Eklenen widget'ları, çiftleri ve pencere görünümünü yönetin.",
    market_settings_description:
        "Fiyat kaynaklarını, yenilemeyi, varsayılan çiftleri ve uyarıları ayarlayın.",
    appearance_settings_description: "Tema, dil ve varsayılan widget görünümünü ayarlayın.",
    system_settings_description: "Başlangıç, proxy, tepsi ve uygulama bilgisini yönetin.",
    quote_board_description: "Ana piyasaları birlikte izlemek için en fazla 20 çift gösterir.",
    mini_ticker_title: "Mini gösterge",
    mini_ticker_description: "Masaüstü köşeleri için kompakt pencerede 1 çift gösterir.",
    market_settings: "Piyasa ayarları",
    appearance_settings: "Görünüm ayarları",
    system_settings: "Sistem ayarları",
    settings_path_label: "Ayar dosyası",
    close: "Kapat",
    source_prefix: "Canlı feed",
    runtime_no_pairs: "Çift yok",
    runtime_connecting: "Bağlanıyor",
    runtime_connection_error: "Bağlantı başarısız",
    runtime_updated: "Güncellendi",
    runtime_stale: "Eski",
    runtime_source_error: "Kaynak sorunu",
    runtime_live_count_prefix: "",
    runtime_live_count_suffix: " canlı",
    runtime_elapsed_second_unit: " sn",
    runtime_elapsed_minute_unit: " dk",
    widget_visible: "Gösteriliyor",
    widget_hidden: "Gizli",
    delete_widget: "Widget'ı kaldır",
    status_alert_invalid: "Uyarı kuralı eksik",
    status_auto_start_failed: "Otomatik başlatma başarısız",
    status_shortcut_failed: "Kısayol kaydı başarısız",
    status_network_proxy_invalid: "Ağ proxy'si geçersiz",
    status_icon_cache_clear_failed: "Simge önbelleği temizlenemedi",
    status_custom_components_folder_open_failed: "Özel widget klasörü açılamadı",
    status_symbol_catalog_fallback: "Popüler çiftler listesi kullanılıyor",
};

const KO_TEXT: UiText = UiText {
    tray_tooltip: "Crypto HUD",
    tray_settings: "메인 창",
    tray_show_widgets: "위젯 표시",
    tray_quit: "종료",
    settings_title: "Crypto HUD",
    tab_widgets: "위젯",
    tab_plugin_market: "위젯 라이브러리",
    tab_market_data: "시세",
    tab_appearance: "화면",
    tab_system: "시스템",
    always_on_top: "위치 잠금",
    default_always_on_top: "새 위젯 항상 위",
    opacity: "불투명도",
    default_opacity: "기본 불투명도",
    widget_scale: "크기",
    red_up_color: "상승 빨간색",
    market_provider: "활성 소스",
    refresh_interval: "새로고침 간격",
    seconds_unit: "초",
    market_provider_help: "페어 검색에 표시되며 선택한 페어에 사용됩니다.",
    refresh_interval_help: "시세를 새로고침하는 간격(초)입니다.",
    default_symbols: "새 위젯 기본 페어",
    alert_settings: "알림 규칙",
    alert_enabled: "알림 켜기",
    alert_symbol: "페어",
    alert_condition: "조건",
    alert_threshold: "기준값",
    alert_clear: "알림 지우기",
    symbols: "페어",
    symbols_help: "최대 20개, 선택한 페어는 태그로 표시됩니다",
    empty_pairs: "설정된 페어 없음",
    auto_start: "로그인 시 실행",
    show_main_window_on_startup: "시작 시 메인 창 표시",
    shortcut: "표시/숨김 단축키",
    tray_icon: "트레이 아이콘 표시",
    tray_hover_display: "트레이에 올리면 표시",
    network_proxy_settings: "네트워크 프록시",
    network_proxy_enabled: "네트워크 프록시 사용",
    network_proxy_url: "프록시 주소",
    network_proxy_example_hint: "예: http://127.0.0.1:7890  ·  socks5://127.0.0.1:1080",
    network_proxy_help: "시세, 페어 검색, 업데이트를 프록시로 라우팅합니다.",
    app_version: "버전",
    about_us: "정보",
    icon_cache: "아이콘 캐시",
    icon_cache_help: "로컬에 캐시된 코인 로고를 제거합니다.",
    clear_icon_cache: "아이콘 지우기",
    custom_components: "사용자 지정 위젯",
    custom_components_help: "로컬 위젯 플러그인 폴더를 엽니다.",
    open_custom_components_folder: "폴더 열기",
    theme: "테마",
    language: "언어",
    appearance_interface: "인터페이스",
    appearance_widget_defaults: "위젯 기본값",
    theme_help: "메인 창과 위젯 색상을 업데이트합니다.",
    language_help: "인터페이스 텍스트를 즉시 새로고침합니다.",
    red_up_color_help: "상승은 빨간색, 하락은 초록색으로 표시합니다.",
    default_opacity_help: "새 위젯의 투명도를 설정합니다.",
    default_always_on_top_help: "새 위젯을 다른 창 위에 유지합니다.",
    system_startup: "시작",
    system_tray: "트레이",
    system_app_info: "앱 정보",
    system_maintenance: "관리",
    auto_start_help: "로그인 후 Crypto HUD를 실행합니다.",
    show_main_window_on_startup_help: "앱 시작 시 메인 창을 엽니다.",
    shortcut_help: "Alt+C로 위젯을 숨기거나 복원합니다.",
    tray_icon_help: "메인 창 열기와 종료를 위한 빠른 접근을 유지합니다.",
    tray_hover_display_help: "트레이 아이콘에 마우스를 올리면 위젯을 잠시 표시합니다.",
    apply: "적용",
    widget_library: "위젯 라이브러리",
    my_widgets: "내 위젯",
    selected_widget: "선택한 위젯:",
    selected_widget_description: "아래 설정은 선택한 위젯에만 적용됩니다.",
    widget_name: "이름",
    lock_position_help: "이 위젯을 현재 위치에 고정합니다.",
    widget_scale_help: "위젯의 전체 크기를 조정합니다.",
    opacity_help: "위젯 투명도를 조정합니다.",
    widget_show_coin_logos: "코인 로고 표시",
    widget_show_coin_logos_help: "페어 옆에 토큰 아이콘을 표시합니다.",
    widget_hide_quote_asset: "견적 자산 숨기기",
    widget_hide_quote_asset_help: "BTC/USDC 대신 BTC만 표시합니다.",
    widget_theme: "위젯 테마",
    widget_theme_help: "이 위젯만 설정하거나 시스템을 따릅니다.",
    widget_topmost: "항상 위",
    widget_topmost_help: "이 위젯을 다른 창 위에 유지합니다.",
    advanced_options: "고급 옵션",
    reset_widget_positions: "초기화",
    hide_all_widgets: "모두 숨기기",
    reset: "초기화",
    preview: "미리보기",
    preview_pairs: "페어",
    preview_updated: "3초 전 업데이트",
    preview_source_ok: "소스 정상",
    app_settings: "위젯 표시",
    add_widget: "추가",
    apply_widget: "위젯 적용",
    no_widgets: "위젯 없음",
    quote_board_title: "시세 보드",
    plugin_market_description: "사용 가능한 기본 및 로컬 위젯입니다.",
    my_widgets_description: "추가된 위젯, 페어, 창 표시를 관리합니다.",
    market_settings_description: "시세 소스, 새로고침, 기본 페어와 알림을 설정합니다.",
    appearance_settings_description: "테마, 언어, 기본 위젯 표시를 조정합니다.",
    system_settings_description: "시작 동작, 프록시, 트레이와 앱 정보를 관리합니다.",
    quote_board_description: "주요 시장을 함께 추적하도록 최대 20개 페어를 표시합니다.",
    mini_ticker_title: "미니 티커",
    mini_ticker_description: "데스크톱 모서리용 작은 창에 1개 페어를 표시합니다.",
    market_settings: "시세 설정",
    appearance_settings: "화면 설정",
    system_settings: "시스템 설정",
    settings_path_label: "설정 파일",
    close: "닫기",
    source_prefix: "실시간 피드",
    runtime_no_pairs: "페어 없음",
    runtime_connecting: "연결 중",
    runtime_connection_error: "연결 실패",
    runtime_updated: "업데이트됨",
    runtime_stale: "오래됨",
    runtime_source_error: "소스 문제",
    runtime_live_count_prefix: "",
    runtime_live_count_suffix: "개 실시간",
    runtime_elapsed_second_unit: "초",
    runtime_elapsed_minute_unit: "분",
    widget_visible: "표시됨",
    widget_hidden: "숨김",
    delete_widget: "위젯 제거",
    status_alert_invalid: "알림 규칙이 완전하지 않습니다",
    status_auto_start_failed: "자동 시작 설정 실패",
    status_shortcut_failed: "단축키 등록 실패",
    status_network_proxy_invalid: "네트워크 프록시가 올바르지 않습니다",
    status_icon_cache_clear_failed: "아이콘 캐시를 지우지 못했습니다",
    status_custom_components_folder_open_failed: "사용자 지정 위젯 폴더를 열 수 없습니다",
    status_symbol_catalog_fallback: "인기 페어 목록 사용 중",
};

const JA_TEXT: UiText = UiText {
    tray_tooltip: "Crypto HUD",
    tray_settings: "メインウィンドウ",
    tray_show_widgets: "ウィジェットを表示",
    tray_quit: "終了",
    settings_title: "Crypto HUD",
    tab_widgets: "ウィジェット",
    tab_plugin_market: "ウィジェットライブラリ",
    tab_market_data: "マーケット",
    tab_appearance: "外観",
    tab_system: "システム",
    always_on_top: "位置を固定",
    default_always_on_top: "新規ウィジェットを最前面に固定",
    opacity: "不透明度",
    default_opacity: "既定の不透明度",
    widget_scale: "スケール",
    red_up_color: "上昇を赤で表示",
    market_provider: "有効なソース",
    refresh_interval: "更新間隔",
    seconds_unit: "秒",
    market_provider_help: "ペア検索に表示され、選択したペアに使われます。",
    refresh_interval_help: "価格を更新する間隔の秒数です。",
    default_symbols: "新規ウィジェットの既定ペア",
    alert_settings: "アラートルール",
    alert_enabled: "アラートを有効化",
    alert_symbol: "ペア",
    alert_condition: "条件",
    alert_threshold: "しきい値",
    alert_clear: "アラートをクリア",
    symbols: "ペア",
    symbols_help: "最大 20 件、選択したペアはタグとして表示されます",
    empty_pairs: "ペア未設定",
    auto_start: "ログイン時に起動",
    show_main_window_on_startup: "起動時にメインウィンドウを表示",
    shortcut: "表示/非表示ショートカット",
    tray_icon: "トレイアイコンを表示",
    tray_hover_display: "トレイにホバー時表示",
    network_proxy_settings: "ネットワークプロキシ",
    network_proxy_enabled: "ネットワークプロキシを有効化",
    network_proxy_url: "プロキシアドレス",
    network_proxy_example_hint: "例：http://127.0.0.1:7890  ·  socks5://127.0.0.1:1080",
    network_proxy_help: "価格、ペア検索、更新をプロキシ経由で送信します。",
    app_version: "バージョン",
    about_us: "このアプリについて",
    icon_cache: "アイコンキャッシュ",
    icon_cache_help: "ローカルに保存されたコインロゴを削除します。",
    clear_icon_cache: "アイコンをクリア",
    custom_components: "カスタムウィジェット",
    custom_components_help: "ローカルウィジェットプラグインのフォルダーを開きます。",
    open_custom_components_folder: "フォルダーを開く",
    theme: "テーマ",
    language: "言語",
    appearance_interface: "インターフェイス",
    appearance_widget_defaults: "ウィジェット既定値",
    theme_help: "メインウィンドウとウィジェットの色を更新します。",
    language_help: "インターフェイスのテキストをすぐに更新します。",
    red_up_color_help: "上昇を赤、下落を緑で表示します。",
    default_opacity_help: "新規ウィジェットの透明度を設定します。",
    default_always_on_top_help: "新規ウィジェットを他のウィンドウより前面に保ちます。",
    system_startup: "起動",
    system_tray: "トレイ",
    system_app_info: "アプリ情報",
    system_maintenance: "メンテナンス",
    auto_start_help: "サインイン後に Crypto HUD を実行します。",
    show_main_window_on_startup_help: "アプリ起動時にメインウィンドウを開きます。",
    shortcut_help: "Alt+C でウィジェットを非表示または復元します。",
    tray_icon_help: "メインウィンドウを開く、または終了するための入口を保持します。",
    tray_hover_display_help: "トレイアイコンにホバーしている間だけウィジェットを表示します。",
    apply: "適用",
    widget_library: "ウィジェットライブラリ",
    my_widgets: "マイウィジェット",
    selected_widget: "選択中のウィジェット:",
    selected_widget_description: "以下の設定は選択中のウィジェットにのみ適用されます。",
    widget_name: "名前",
    lock_position_help: "このウィジェットを現在位置に固定します。",
    widget_scale_help: "ウィジェット全体のサイズを調整します。",
    opacity_help: "ウィジェットの透明度を調整します。",
    widget_show_coin_logos: "コインロゴを表示",
    widget_show_coin_logos_help: "ペアの横にトークンアイコンを表示します。",
    widget_hide_quote_asset: "クォート資産を非表示",
    widget_hide_quote_asset_help: "BTC/USDC ではなく BTC と表示します。",
    widget_theme: "ウィジェットテーマ",
    widget_theme_help: "このウィジェットだけ上書きするか、システムに従います。",
    widget_topmost: "常に最前面",
    widget_topmost_help: "このウィジェットを他のウィンドウより前面に保ちます。",
    advanced_options: "詳細オプション",
    reset_widget_positions: "リセット",
    hide_all_widgets: "すべて非表示",
    reset: "リセット",
    preview: "プレビュー",
    preview_pairs: "ペア",
    preview_updated: "3秒前に更新",
    preview_source_ok: "ソース正常",
    app_settings: "ウィジェット表示",
    add_widget: "追加",
    apply_widget: "ウィジェットを適用",
    no_widgets: "ウィジェットなし",
    quote_board_title: "レートボード",
    plugin_market_description: "利用可能な組み込みおよびローカルウィジェットです。",
    my_widgets_description: "追加済みウィジェット、ペア、ウィンドウ表示を管理します。",
    market_settings_description: "価格ソース、更新間隔、既定ペア、アラートを設定します。",
    appearance_settings_description: "テーマ、言語、既定のウィジェット表示を調整します。",
    system_settings_description: "起動、プロキシ、トレイ、アプリ情報を管理します。",
    quote_board_description: "主要市場をまとめて追跡するため最大 20 ペアを表示します。",
    mini_ticker_title: "ミニティッカー",
    mini_ticker_description: "デスクトップ端に置ける小さなウィンドウで 1 ペアを表示します。",
    market_settings: "マーケット設定",
    appearance_settings: "外観設定",
    system_settings: "システム設定",
    settings_path_label: "設定ファイル",
    close: "閉じる",
    source_prefix: "ライブフィード",
    runtime_no_pairs: "ペアなし",
    runtime_connecting: "接続中",
    runtime_connection_error: "接続に失敗しました",
    runtime_updated: "更新済み",
    runtime_stale: "古い",
    runtime_source_error: "ソースの問題",
    runtime_live_count_prefix: "",
    runtime_live_count_suffix: " 件ライブ",
    runtime_elapsed_second_unit: "秒",
    runtime_elapsed_minute_unit: "分",
    widget_visible: "表示中",
    widget_hidden: "非表示",
    delete_widget: "ウィジェットを削除",
    status_alert_invalid: "アラートルールが未完成です",
    status_auto_start_failed: "自動起動の設定に失敗しました",
    status_shortcut_failed: "ショートカット登録に失敗しました",
    status_network_proxy_invalid: "ネットワークプロキシが無効です",
    status_icon_cache_clear_failed: "アイコンキャッシュの削除に失敗しました",
    status_custom_components_folder_open_failed: "カスタムウィジェットフォルダーを開けません",
    status_symbol_catalog_fallback: "人気ペア一覧を使用中",
};

const RU_TEXT: UiText = UiText {
    tray_tooltip: "Crypto HUD",
    tray_settings: "Главное окно",
    tray_show_widgets: "Показать виджеты",
    tray_quit: "Выход",
    settings_title: "Crypto HUD",
    tab_widgets: "Виджеты",
    tab_plugin_market: "Библиотека виджетов",
    tab_market_data: "Рынок",
    tab_appearance: "Вид",
    tab_system: "Система",
    always_on_top: "Закрепить позицию",
    default_always_on_top: "Новые виджеты поверх окон",
    opacity: "Непрозрачность",
    default_opacity: "Непрозрачность по умолчанию",
    widget_scale: "Масштаб",
    red_up_color: "Рост красным",
    market_provider: "Активные источники",
    refresh_interval: "Интервал обновления",
    seconds_unit: "с",
    market_provider_help: "Показывается в поиске пар и используется выбранными парами.",
    refresh_interval_help: "Секунды между обновлениями котировок.",
    default_symbols: "Пары по умолчанию для новых виджетов",
    alert_settings: "Правило оповещения",
    alert_enabled: "Включить оповещение",
    alert_symbol: "Пара",
    alert_condition: "Условие",
    alert_threshold: "Порог",
    alert_clear: "Очистить оповещение",
    symbols: "Пары",
    symbols_help: "До 20; выбранные пары отображаются как теги",
    empty_pairs: "Пары не настроены",
    auto_start: "Запуск при входе",
    show_main_window_on_startup: "Показывать главное окно при запуске",
    shortcut: "Горячая клавиша показа/скрытия",
    tray_icon: "Показывать значок в трее",
    tray_hover_display: "Показывать при наведении на трей",
    network_proxy_settings: "Сетевой прокси",
    network_proxy_enabled: "Включить сетевой прокси",
    network_proxy_url: "Адрес прокси",
    network_proxy_example_hint: "Примеры: http://127.0.0.1:7890  ·  socks5://127.0.0.1:1080",
    network_proxy_help: "Направляет котировки, поиск пар и обновления через прокси.",
    app_version: "Версия",
    about_us: "О приложении",
    icon_cache: "Кэш значков",
    icon_cache_help: "Удаляет локально сохраненные логотипы монет.",
    clear_icon_cache: "Очистить значки",
    custom_components: "Пользовательские виджеты",
    custom_components_help: "Открывает папку локальных плагинов виджетов.",
    open_custom_components_folder: "Открыть папку",
    theme: "Тема",
    language: "Язык",
    appearance_interface: "Интерфейс",
    appearance_widget_defaults: "Параметры виджетов по умолчанию",
    theme_help: "Обновляет цвета главного окна и виджетов.",
    language_help: "Сразу обновляет текст интерфейса.",
    red_up_color_help: "Показывает рост красным, а падение зеленым.",
    default_opacity_help: "Задает прозрачность новых виджетов.",
    default_always_on_top_help: "Держит новые виджеты поверх других окон.",
    system_startup: "Запуск",
    system_tray: "Трей",
    system_app_info: "Информация о приложении",
    system_maintenance: "Обслуживание",
    auto_start_help: "Запускает Crypto HUD после входа в систему.",
    show_main_window_on_startup_help: "Открывает главное окно при запуске приложения.",
    shortcut_help: "Используйте Alt+C, чтобы скрыть или вернуть виджеты.",
    tray_icon_help: "Оставляет быстрый доступ к главному окну и выходу.",
    tray_hover_display_help: "Временно показывает виджеты при наведении на значок в трее.",
    apply: "Применить",
    widget_library: "Библиотека виджетов",
    my_widgets: "Мои виджеты",
    selected_widget: "Выбранный виджет:",
    selected_widget_description: "Параметры ниже влияют только на выбранный виджет.",
    widget_name: "Имя",
    lock_position_help: "Фиксирует этот виджет в текущей позиции.",
    widget_scale_help: "Настраивает общий размер виджета.",
    opacity_help: "Настраивает прозрачность виджета.",
    widget_show_coin_logos: "Показывать логотипы монет",
    widget_show_coin_logos_help: "Показывает значки токенов рядом с парами.",
    widget_hide_quote_asset: "Скрыть котируемый актив",
    widget_hide_quote_asset_help: "Показывает BTC вместо BTC/USDC.",
    widget_theme: "Тема виджета",
    widget_theme_help: "Переопределяет тему виджета или следует системе.",
    widget_topmost: "Всегда поверх",
    widget_topmost_help: "Держит этот виджет поверх других окон.",
    advanced_options: "Дополнительно",
    reset_widget_positions: "Сброс",
    hide_all_widgets: "Скрыть все",
    reset: "Сброс",
    preview: "Предпросмотр",
    preview_pairs: "Пары",
    preview_updated: "Обновлено 3 с назад",
    preview_source_ok: "Источник OK",
    app_settings: "Отображение виджетов",
    add_widget: "Добавить",
    apply_widget: "Применить виджет",
    no_widgets: "Нет виджетов",
    quote_board_title: "Панель котировок",
    plugin_market_description: "Доступные встроенные и локальные виджеты.",
    my_widgets_description: "Управление добавленными виджетами, парами и отображением.",
    market_settings_description: "Настройте источники котировок, обновление, пары и оповещения.",
    appearance_settings_description: "Настройте тему, язык и отображение виджетов по умолчанию.",
    system_settings_description: "Управляйте запуском, прокси, треем и информацией приложения.",
    quote_board_description: "Показывает до 20 пар для совместного отслеживания рынков.",
    mini_ticker_title: "Мини-тикер",
    mini_ticker_description: "Показывает 1 пару в компактном окне для угла рабочего стола.",
    market_settings: "Настройки рынка",
    appearance_settings: "Настройки вида",
    system_settings: "Системные настройки",
    settings_path_label: "Файл настроек",
    close: "Закрыть",
    source_prefix: "Живой поток",
    runtime_no_pairs: "Нет пар",
    runtime_connecting: "Подключение",
    runtime_connection_error: "Ошибка подключения",
    runtime_updated: "Обновлено",
    runtime_stale: "Устарело",
    runtime_source_error: "Проблема источника",
    runtime_live_count_prefix: "",
    runtime_live_count_suffix: " активных",
    runtime_elapsed_second_unit: " с",
    runtime_elapsed_minute_unit: " мин",
    widget_visible: "Показан",
    widget_hidden: "Скрыт",
    delete_widget: "Удалить виджет",
    status_alert_invalid: "Правило оповещения заполнено не полностью",
    status_auto_start_failed: "Не удалось настроить автозапуск",
    status_shortcut_failed: "Не удалось зарегистрировать горячую клавишу",
    status_network_proxy_invalid: "Сетевой прокси недействителен",
    status_icon_cache_clear_failed: "Не удалось очистить кэш значков",
    status_custom_components_folder_open_failed: "Не удалось открыть папку виджетов",
    status_symbol_catalog_fallback: "Используется список популярных пар",
};

const AR_TEXT: UiText = UiText {
    tray_tooltip: "\u{2066}Crypto HUD\u{2069}",
    tray_settings: "النافذة الرئيسية",
    tray_show_widgets: "إظهار الأدوات",
    tray_quit: "إنهاء",
    settings_title: "\u{2066}Crypto HUD\u{2069}",
    tab_widgets: "الأدوات",
    tab_plugin_market: "مكتبة الأدوات",
    tab_market_data: "السوق",
    tab_appearance: "المظهر",
    tab_system: "النظام",
    always_on_top: "قفل الموضع",
    default_always_on_top: "إبقاء الأدوات الجديدة في الأعلى",
    opacity: "الشفافية",
    default_opacity: "الشفافية الافتراضية",
    widget_scale: "الحجم",
    red_up_color: "الأحمر للصعود",
    market_provider: "المصادر المفعلة",
    refresh_interval: "فاصل التحديث",
    seconds_unit: "ث",
    market_provider_help: "يظهر في بحث الأزواج ويستخدم للأزواج المحددة.",
    refresh_interval_help: "عدد الثواني بين تحديثات الأسعار.",
    default_symbols: "الأزواج الافتراضية للأدوات الجديدة",
    alert_settings: "قاعدة التنبيه",
    alert_enabled: "تفعيل التنبيه",
    alert_symbol: "الزوج",
    alert_condition: "الشرط",
    alert_threshold: "الحد",
    alert_clear: "مسح التنبيه",
    symbols: "الأزواج",
    symbols_help: "حتى \u{2066}20\u{2069} زوجا؛ تظهر الأزواج المحددة كوسوم",
    empty_pairs: "لا توجد أزواج مضبوطة",
    auto_start: "التشغيل عند تسجيل الدخول",
    show_main_window_on_startup: "إظهار النافذة الرئيسية عند البدء",
    shortcut: "اختصار الإظهار/الإخفاء",
    tray_icon: "إظهار أيقونة علبة النظام",
    tray_hover_display: "إظهار عند تمرير المؤشر على العلبة",
    network_proxy_settings: "وكيل الشبكة",
    network_proxy_enabled: "تفعيل وكيل الشبكة",
    network_proxy_url: "عنوان الوكيل",
    network_proxy_example_hint:
        "أمثلة: \u{2066}http://127.0.0.1:7890\u{2069}  ·  \u{2066}socks5://127.0.0.1:1080\u{2069}",
    network_proxy_help: "يوجه الأسعار وبحث الأزواج والتحديثات عبر الوكيل.",
    app_version: "الإصدار",
    about_us: "حول التطبيق",
    icon_cache: "ذاكرة الأيقونات",
    icon_cache_help: "يزيل شعارات العملات المخزنة محليا.",
    clear_icon_cache: "مسح الأيقونات",
    custom_components: "أدوات مخصصة",
    custom_components_help: "يفتح مجلد إضافات الأدوات المحلية.",
    open_custom_components_folder: "فتح المجلد",
    theme: "السمة",
    language: "اللغة",
    appearance_interface: "الواجهة",
    appearance_widget_defaults: "إعدادات الأدوات الافتراضية",
    theme_help: "يحدث ألوان النافذة الرئيسية والأدوات.",
    language_help: "يحدث نصوص الواجهة فورا.",
    red_up_color_help: "يعرض الصعود بالأحمر والهبوط بالأخضر.",
    default_opacity_help: "يضبط شفافية الأدوات الجديدة.",
    default_always_on_top_help: "يبقي الأدوات الجديدة فوق النوافذ الأخرى.",
    system_startup: "بدء التشغيل",
    system_tray: "علبة النظام",
    system_app_info: "معلومات التطبيق",
    system_maintenance: "الصيانة",
    auto_start_help: "يشغل \u{2066}Crypto HUD\u{2069} بعد تسجيل الدخول.",
    show_main_window_on_startup_help: "يفتح النافذة الرئيسية عند تشغيل التطبيق.",
    shortcut_help: "استخدم \u{2066}Alt+C\u{2069} لإخفاء الأدوات أو استعادتها.",
    tray_icon_help: "يبقي وصولا سريعا للنافذة الرئيسية والخروج.",
    tray_hover_display_help: "يعرض الأدوات مؤقتا عند تمرير المؤشر فوق أيقونة العلبة.",
    apply: "تطبيق",
    widget_library: "مكتبة الأدوات",
    my_widgets: "أدواتي",
    selected_widget: "الأداة المحددة:",
    selected_widget_description: "الإعدادات أدناه تؤثر في الأداة المحددة فقط.",
    widget_name: "الاسم",
    lock_position_help: "يبقي هذه الأداة ثابتة في موضعها الحالي.",
    widget_scale_help: "يضبط الحجم العام للأداة.",
    opacity_help: "يضبط شفافية الأداة.",
    widget_show_coin_logos: "إظهار شعارات العملات",
    widget_show_coin_logos_help: "يعرض أيقونات الرموز بجانب الأزواج.",
    widget_hide_quote_asset: "إخفاء أصل التسعير",
    widget_hide_quote_asset_help: "يعرض \u{2066}BTC\u{2069} بدلا من \u{2066}BTC/USDC\u{2069}.",
    widget_theme: "سمة الأداة",
    widget_theme_help: "تخصيص هذه الأداة أو اتباع النظام.",
    widget_topmost: "دائما في الأعلى",
    widget_topmost_help: "يبقي هذه الأداة فوق النوافذ الأخرى.",
    advanced_options: "خيارات متقدمة",
    reset_widget_positions: "إعادة ضبط",
    hide_all_widgets: "إخفاء الكل",
    reset: "إعادة ضبط",
    preview: "معاينة",
    preview_pairs: "الأزواج",
    preview_updated: "تم التحديث قبل \u{2066}3\u{2069} ثوان",
    preview_source_ok: "المصدر جيد",
    app_settings: "عرض الأداة",
    add_widget: "إضافة",
    apply_widget: "تطبيق الأداة",
    no_widgets: "لا توجد أدوات",
    quote_board_title: "لوحة الأسعار",
    plugin_market_description: "الأدوات المضمنة والمحلية المتاحة.",
    my_widgets_description: "إدارة الأدوات المضافة والأزواج وعرض النوافذ.",
    market_settings_description: "إعداد مصادر الأسعار والتحديث والأزواج الافتراضية والتنبيهات.",
    appearance_settings_description: "ضبط السمة واللغة والعرض الافتراضي للأدوات.",
    system_settings_description: "إدارة بدء التشغيل والوكيل والعلبة ومعلومات التطبيق.",
    quote_board_description: "يعرض حتى \u{2066}20\u{2069} زوجا لمتابعة الأسواق الرئيسية معا.",
    mini_ticker_title: "مؤشر مصغر",
    mini_ticker_description: "يعرض زوجا واحدا في نافذة صغيرة لزوايا سطح المكتب.",
    market_settings: "إعدادات السوق",
    appearance_settings: "إعدادات المظهر",
    system_settings: "إعدادات النظام",
    settings_path_label: "ملف الإعدادات",
    close: "إغلاق",
    source_prefix: "بث مباشر",
    runtime_no_pairs: "لا توجد أزواج",
    runtime_connecting: "جار الاتصال",
    runtime_connection_error: "فشل الاتصال",
    runtime_updated: "تم التحديث",
    runtime_stale: "قديم",
    runtime_source_error: "مشكلة في المصدر",
    runtime_live_count_prefix: "",
    runtime_live_count_suffix: " مباشر",
    runtime_elapsed_second_unit: " ث",
    runtime_elapsed_minute_unit: " د",
    widget_visible: "ظاهر",
    widget_hidden: "مخفي",
    delete_widget: "إزالة الأداة",
    status_alert_invalid: "قاعدة التنبيه غير مكتملة",
    status_auto_start_failed: "فشل التشغيل التلقائي",
    status_shortcut_failed: "فشل تسجيل الاختصار",
    status_network_proxy_invalid: "وكيل الشبكة غير صالح",
    status_icon_cache_clear_failed: "فشل مسح ذاكرة الأيقونات",
    status_custom_components_folder_open_failed: "تعذر فتح مجلد الأدوات المخصصة",
    status_symbol_catalog_fallback: "يتم استخدام قائمة الأزواج الشائعة",
};

pub fn resolve_locale(preference: LanguagePreference) -> Locale {
    match preference {
        LanguagePreference::System => locale_from_system(),
        _ => locale_from_language_preference(preference).unwrap_or(Locale::En),
    }
}

pub fn locale_from_system() -> Locale {
    sys_locale::get_locale()
        .as_deref()
        .map(locale_from_tag)
        .unwrap_or(Locale::En)
}

pub fn locale_from_tag(locale: &str) -> Locale {
    LanguagePreference::from_locale_tag(locale)
        .and_then(locale_from_language_preference)
        .unwrap_or(Locale::En)
}

fn locale_from_language_preference(preference: LanguagePreference) -> Option<Locale> {
    match preference {
        LanguagePreference::En => Some(Locale::En),
        LanguagePreference::ZhHans => Some(Locale::ZhHans),
        LanguagePreference::ZhHant => Some(Locale::ZhHant),
        LanguagePreference::Es419 => Some(Locale::Es419),
        LanguagePreference::PtBr => Some(Locale::PtBr),
        LanguagePreference::Vi => Some(Locale::Vi),
        LanguagePreference::Id => Some(Locale::Id),
        LanguagePreference::Tr => Some(Locale::Tr),
        LanguagePreference::Ko => Some(Locale::Ko),
        LanguagePreference::Ja => Some(Locale::Ja),
        LanguagePreference::Ru => Some(Locale::Ru),
        LanguagePreference::Ar => Some(Locale::Ar),
        LanguagePreference::System => None,
    }
}

pub fn text(locale: Locale) -> &'static UiText {
    match locale {
        Locale::En => &EN_TEXT,
        Locale::ZhHans => &ZH_HANS_TEXT,
        Locale::ZhHant => &ZH_HANT_TEXT,
        Locale::Es419 => &ES_419_TEXT,
        Locale::PtBr => &PT_BR_TEXT,
        Locale::Vi => &VI_TEXT,
        Locale::Id => &ID_TEXT,
        Locale::Tr => &TR_TEXT,
        Locale::Ko => &KO_TEXT,
        Locale::Ja => &JA_TEXT,
        Locale::Ru => &RU_TEXT,
        Locale::Ar => &AR_TEXT,
    }
}

pub fn is_rtl(locale: Locale) -> bool {
    matches!(locale, Locale::Ar)
}

pub fn shortcut_options(locale: Locale) -> Vec<&'static str> {
    match locale {
        Locale::En => vec!["Alt+C", "Disabled"],
        Locale::ZhHans => vec!["Alt+C", "禁用"],
        Locale::ZhHant => vec!["Alt+C", "停用"],
        Locale::Es419 => vec!["Alt+C", "Desactivado"],
        Locale::PtBr => vec!["Alt+C", "Desativado"],
        Locale::Vi => vec!["Alt+C", "Tắt"],
        Locale::Id => vec!["Alt+C", "Nonaktif"],
        Locale::Tr => vec!["Alt+C", "Devre dışı"],
        Locale::Ko => vec!["Alt+C", "사용 안 함"],
        Locale::Ja => vec!["Alt+C", "無効"],
        Locale::Ru => vec!["Alt+C", "Отключено"],
        Locale::Ar => vec!["\u{2066}Alt+C\u{2069}", "معطل"],
    }
}

pub fn language_options(locale: Locale) -> Vec<&'static str> {
    match locale {
        Locale::ZhHans => language_options_with_system("跟随系统"),
        Locale::ZhHant => language_options_with_system("跟隨系統"),
        Locale::Es419 => language_options_with_system("Sistema"),
        Locale::PtBr => language_options_with_system("Sistema"),
        Locale::Vi => language_options_with_system("Theo hệ thống"),
        Locale::Id => language_options_with_system("Sistem"),
        Locale::Tr => language_options_with_system("Sistem"),
        Locale::Ko => language_options_with_system("시스템"),
        Locale::Ja => language_options_with_system("システム"),
        Locale::Ru => language_options_with_system("Система"),
        Locale::Ar => language_options_for_arabic(),
        Locale::En => language_options_with_system("System"),
    }
}

fn language_options_with_system(system_label: &'static str) -> Vec<&'static str> {
    let mut options = Vec::with_capacity(Locale::ALL.len() + 1);
    options.extend(
        LanguagePreference::ALL
            .iter()
            .map(|preference| match preference {
                LanguagePreference::System => system_label,
                _ => language_preference_label(*preference),
            }),
    );
    options
}

fn language_options_for_arabic() -> Vec<&'static str> {
    let mut options = Vec::with_capacity(Locale::ALL.len() + 1);
    options.extend(
        LanguagePreference::ALL
            .iter()
            .map(|preference| match preference {
                LanguagePreference::System => "النظام",
                LanguagePreference::En => "\u{2066}English\u{2069}",
                LanguagePreference::ZhHans => "\u{2066}简体中文\u{2069}",
                LanguagePreference::ZhHant => "\u{2066}繁體中文\u{2069}",
                LanguagePreference::Es419 => "\u{2066}Español (LatAm)\u{2069}",
                LanguagePreference::PtBr => "\u{2066}Português (Brasil)\u{2069}",
                LanguagePreference::Vi => "\u{2066}Tiếng Việt\u{2069}",
                LanguagePreference::Id => "\u{2066}Bahasa Indonesia\u{2069}",
                LanguagePreference::Tr => "\u{2066}Türkçe\u{2069}",
                LanguagePreference::Ko => "\u{2066}한국어\u{2069}",
                LanguagePreference::Ja => "\u{2066}日本語\u{2069}",
                LanguagePreference::Ru => "\u{2066}Русский\u{2069}",
                LanguagePreference::Ar => "العربية",
            }),
    );
    options
}

fn language_preference_label(preference: LanguagePreference) -> &'static str {
    match preference {
        LanguagePreference::System => "System",
        LanguagePreference::En => "English",
        LanguagePreference::ZhHans => "简体中文",
        LanguagePreference::ZhHant => "繁體中文",
        LanguagePreference::Es419 => "Español (LatAm)",
        LanguagePreference::PtBr => "Português (Brasil)",
        LanguagePreference::Vi => "Tiếng Việt",
        LanguagePreference::Id => "Bahasa Indonesia",
        LanguagePreference::Tr => "Türkçe",
        LanguagePreference::Ko => "한국어",
        LanguagePreference::Ja => "日本語",
        LanguagePreference::Ru => "Русский",
        LanguagePreference::Ar => "العربية",
    }
}

pub fn theme_options(locale: Locale) -> Vec<&'static str> {
    match locale {
        Locale::En => vec!["System", "Light", "Dark"],
        Locale::ZhHans => vec!["跟随系统", "浅色", "深色"],
        Locale::ZhHant => vec!["跟隨系統", "淺色", "深色"],
        Locale::Es419 => vec!["Sistema", "Claro", "Oscuro"],
        Locale::PtBr => vec!["Sistema", "Claro", "Escuro"],
        Locale::Vi => vec!["Theo hệ thống", "Sáng", "Tối"],
        Locale::Id => vec!["Sistem", "Terang", "Gelap"],
        Locale::Tr => vec!["Sistem", "Açık", "Koyu"],
        Locale::Ko => vec!["시스템", "라이트", "다크"],
        Locale::Ja => vec!["システム", "ライト", "ダーク"],
        Locale::Ru => vec!["Система", "Светлая", "Темная"],
        Locale::Ar => vec!["النظام", "فاتح", "داكن"],
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
        Locale::ZhHant => vec!["價格高於", "價格低於", "24h 漲跌高於", "24h 漲跌低於"],
        Locale::Es419 => vec![
            "Precio por encima",
            "Precio por debajo",
            "Cambio 24h por encima",
            "Cambio 24h por debajo",
        ],
        Locale::PtBr => vec![
            "Preço acima",
            "Preço abaixo",
            "Variação 24h acima",
            "Variação 24h abaixo",
        ],
        Locale::Vi => vec![
            "Giá cao hơn",
            "Giá thấp hơn",
            "Biến động 24h cao hơn",
            "Biến động 24h thấp hơn",
        ],
        Locale::Id => vec![
            "Harga di atas",
            "Harga di bawah",
            "Perubahan 24j di atas",
            "Perubahan 24j di bawah",
        ],
        Locale::Tr => vec![
            "Fiyat üzerinde",
            "Fiyat altında",
            "24 sa değişim üzerinde",
            "24 sa değişim altında",
        ],
        Locale::Ko => vec![
            "가격 초과",
            "가격 미만",
            "24시간 변동 초과",
            "24시간 변동 미만",
        ],
        Locale::Ja => vec![
            "価格が上回る",
            "価格が下回る",
            "24h 変動が上回る",
            "24h 変動が下回る",
        ],
        Locale::Ru => vec!["Цена выше", "Цена ниже", "Изм. 24ч выше", "Изм. 24ч ниже"],
        Locale::Ar => vec![
            "السعر أعلى من",
            "السعر أقل من",
            "تغير \u{2066}24\u{2069} ساعة أعلى من",
            "تغير \u{2066}24\u{2069} ساعة أقل من",
        ],
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
    let count = count_text_for_locale(locale, count);
    match locale {
        Locale::En => format!("Used {count}"),
        Locale::ZhHans => format!("已使用 {count} 个"),
        Locale::ZhHant => format!("已使用 {count} 個"),
        Locale::Es419 => format!("{count} usados"),
        Locale::PtBr => format!("{count} em uso"),
        Locale::Vi => format!("Đã dùng {count}"),
        Locale::Id => format!("Dipakai {count}"),
        Locale::Tr => format!("{count} kullanıldı"),
        Locale::Ko => format!("{count}개 사용"),
        Locale::Ja => format!("{count} 使用中"),
        Locale::Ru => format!("Использовано: {count}"),
        Locale::Ar => format!("مستخدم {count}"),
    }
}

pub fn default_widget_name(locale: Locale, widget: WidgetText, number: u64) -> String {
    let number = ltr_isolate_for_locale(locale, &number.to_string());
    format!("{} {number}", widget_title(locale, widget))
}

pub fn status_failure_message(
    locale: Locale,
    summary: &str,
    error: impl std::fmt::Display,
) -> String {
    let error = ltr_isolate_for_locale(locale, &error.to_string());
    match locale {
        Locale::ZhHans | Locale::ZhHant | Locale::Ja => format!("{summary}：{error}"),
        _ => format!("{summary}: {error}"),
    }
}

pub fn localized_status_failure_message(locale: Locale, summary: &str, detail: &str) -> String {
    match locale {
        Locale::ZhHans | Locale::ZhHant | Locale::Ja => format!("{summary}：{detail}"),
        _ => format!("{summary}: {detail}"),
    }
}

pub fn save_failure_message(locale: Locale, error: impl std::fmt::Display) -> String {
    let summary = match locale {
        Locale::En => "Could not save settings",
        Locale::ZhHans => "无法保存设置",
        Locale::ZhHant => "無法儲存設定",
        Locale::Es419 => "No se pudo guardar la configuración",
        Locale::PtBr => "Não foi possível salvar as configurações",
        Locale::Vi => "Không thể lưu cài đặt",
        Locale::Id => "Tidak dapat menyimpan pengaturan",
        Locale::Tr => "Ayarlar kaydedilemedi",
        Locale::Ko => "설정을 저장할 수 없습니다",
        Locale::Ja => "設定を保存できませんでした",
        Locale::Ru => "Не удалось сохранить настройки",
        Locale::Ar => "تعذر حفظ الإعدادات",
    };
    status_failure_message(locale, summary, error)
}

pub fn network_proxy_empty_address_detail(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Enter a proxy address",
        Locale::ZhHans => "请输入代理地址",
        Locale::ZhHant => "請輸入代理位址",
        Locale::Es419 => "Ingresa una dirección de proxy",
        Locale::PtBr => "Informe um endereço de proxy",
        Locale::Vi => "Nhập địa chỉ proxy",
        Locale::Id => "Masukkan alamat proxy",
        Locale::Tr => "Proxy adresi girin",
        Locale::Ko => "프록시 주소를 입력하세요",
        Locale::Ja => "プロキシアドレスを入力してください",
        Locale::Ru => "Введите адрес прокси",
        Locale::Ar => "أدخل عنوان الوكيل",
    }
}

pub fn alert_notification_title(locale: Locale, symbol: &str) -> String {
    let symbol = format_market_pair_symbol(symbol);
    let symbol = ltr_isolate_for_locale(locale, &symbol);
    match locale {
        Locale::En => format!("{symbol} alert"),
        Locale::ZhHans => format!("{symbol} 告警"),
        Locale::ZhHant => format!("{symbol} 警示"),
        Locale::Es419 => format!("Alerta de {symbol}"),
        Locale::PtBr => format!("Alerta de {symbol}"),
        Locale::Vi => format!("Cảnh báo {symbol}"),
        Locale::Id => format!("Peringatan {symbol}"),
        Locale::Tr => format!("{symbol} uyarısı"),
        Locale::Ko => format!("{symbol} 알림"),
        Locale::Ja => format!("{symbol} アラート"),
        Locale::Ru => format!("Оповещение {symbol}"),
        Locale::Ar => format!("تنبيه {symbol}"),
    }
}

pub fn alert_notification_body(
    locale: Locale,
    symbol: &str,
    condition: AlertCondition,
    threshold: f64,
    current_value: f64,
) -> String {
    let symbol = format_market_pair_symbol(symbol);
    let threshold = format_alert_value(condition, threshold);
    let current_value = format_alert_value(condition, current_value);
    let symbol = ltr_isolate_for_locale(locale, &symbol);
    let threshold = ltr_isolate_for_locale(locale, &threshold);
    let current_value = ltr_isolate_for_locale(locale, &current_value);
    match condition {
        AlertCondition::PriceAbove => {
            alert_price_above_body(locale, &symbol, threshold.as_str(), current_value.as_str())
        }
        AlertCondition::PriceBelow => {
            alert_price_below_body(locale, &symbol, threshold.as_str(), current_value.as_str())
        }
        AlertCondition::ChangePercentAbove => {
            alert_change_above_body(locale, &symbol, threshold.as_str(), current_value.as_str())
        }
        AlertCondition::ChangePercentBelow => {
            alert_change_below_body(locale, &symbol, threshold.as_str(), current_value.as_str())
        }
    }
}

pub(crate) fn ltr_isolate_for_locale(locale: Locale, value: &str) -> String {
    if is_rtl(locale) {
        format!("\u{2066}{value}\u{2069}")
    } else {
        value.to_string()
    }
}

pub(crate) fn strip_bidi_isolate_marks(value: &str) -> String {
    value
        .chars()
        .filter(|character| !is_bidi_isolate_mark(*character))
        .collect()
}

pub(crate) fn is_bidi_isolate_mark(character: char) -> bool {
    matches!(character, '\u{2066}' | '\u{2067}' | '\u{2068}' | '\u{2069}')
}

fn count_text_for_locale(locale: Locale, count: usize) -> String {
    ltr_isolate_for_locale(locale, &count.to_string())
}

fn range_text_for_locale(locale: Locale, min: usize, max: usize) -> String {
    ltr_isolate_for_locale(locale, &format!("{min}-{max}"))
}

fn selected_count_text_for_locale(locale: Locale, selected_count: usize, max: usize) -> String {
    ltr_isolate_for_locale(locale, &format!("{selected_count}/{max}"))
}

fn format_alert_value(condition: AlertCondition, value: f64) -> String {
    match condition {
        AlertCondition::PriceAbove | AlertCondition::PriceBelow => format_price(value),
        AlertCondition::ChangePercentAbove | AlertCondition::ChangePercentBelow => {
            format_pair_change(value)
        }
    }
}

fn alert_price_above_body(
    locale: Locale,
    symbol: &str,
    threshold: &str,
    current_value: &str,
) -> String {
    match locale {
        Locale::En => format!("{symbol} price is above {threshold}: {current_value}"),
        Locale::ZhHans => format!("{symbol} 价格已高于 {threshold}：{current_value}"),
        Locale::ZhHant => format!("{symbol} 價格已高於 {threshold}：{current_value}"),
        Locale::Es419 => {
            format!("El precio de {symbol} está por encima de {threshold}: {current_value}")
        }
        Locale::PtBr => format!("O preço de {symbol} está acima de {threshold}: {current_value}"),
        Locale::Vi => format!("Giá {symbol} đã cao hơn {threshold}: {current_value}"),
        Locale::Id => format!("Harga {symbol} di atas {threshold}: {current_value}"),
        Locale::Tr => format!("{symbol} fiyatı {threshold} üzerinde: {current_value}"),
        Locale::Ko => format!("{symbol} 가격이 {threshold} 이상입니다: {current_value}"),
        Locale::Ja => format!("{symbol} の価格が {threshold} を上回りました: {current_value}"),
        Locale::Ru => format!("Цена {symbol} выше {threshold}: {current_value}"),
        Locale::Ar => format!("سعر {symbol} أعلى من {threshold}: {current_value}"),
    }
}

fn alert_price_below_body(
    locale: Locale,
    symbol: &str,
    threshold: &str,
    current_value: &str,
) -> String {
    match locale {
        Locale::En => format!("{symbol} price is below {threshold}: {current_value}"),
        Locale::ZhHans => format!("{symbol} 价格已低于 {threshold}：{current_value}"),
        Locale::ZhHant => format!("{symbol} 價格已低於 {threshold}：{current_value}"),
        Locale::Es419 => {
            format!("El precio de {symbol} está por debajo de {threshold}: {current_value}")
        }
        Locale::PtBr => format!("O preço de {symbol} está abaixo de {threshold}: {current_value}"),
        Locale::Vi => format!("Giá {symbol} đã thấp hơn {threshold}: {current_value}"),
        Locale::Id => format!("Harga {symbol} di bawah {threshold}: {current_value}"),
        Locale::Tr => format!("{symbol} fiyatı {threshold} altında: {current_value}"),
        Locale::Ko => format!("{symbol} 가격이 {threshold} 이하입니다: {current_value}"),
        Locale::Ja => format!("{symbol} の価格が {threshold} を下回りました: {current_value}"),
        Locale::Ru => format!("Цена {symbol} ниже {threshold}: {current_value}"),
        Locale::Ar => format!("سعر {symbol} أقل من {threshold}: {current_value}"),
    }
}

fn alert_change_above_body(
    locale: Locale,
    symbol: &str,
    threshold: &str,
    current_value: &str,
) -> String {
    match locale {
        Locale::En => format!("{symbol} 24h change is above {threshold}: {current_value}"),
        Locale::ZhHans => format!("{symbol} 24h 涨跌已高于 {threshold}：{current_value}"),
        Locale::ZhHant => format!("{symbol} 24h 漲跌已高於 {threshold}：{current_value}"),
        Locale::Es419 => {
            format!("El cambio 24h de {symbol} está por encima de {threshold}: {current_value}")
        }
        Locale::PtBr => {
            format!("A variação 24h de {symbol} está acima de {threshold}: {current_value}")
        }
        Locale::Vi => format!("Biến động 24h của {symbol} đã cao hơn {threshold}: {current_value}"),
        Locale::Id => format!("Perubahan 24j {symbol} di atas {threshold}: {current_value}"),
        Locale::Tr => {
            format!("{symbol} 24 saatlik değişimi {threshold} üzerinde: {current_value}")
        }
        Locale::Ko => format!("{symbol} 24시간 변동률이 {threshold} 이상입니다: {current_value}"),
        Locale::Ja => {
            format!("{symbol} の24時間変動率が {threshold} を上回りました: {current_value}")
        }
        Locale::Ru => format!("Изменение {symbol} за 24ч выше {threshold}: {current_value}"),
        Locale::Ar => {
            format!(
                "تغير {symbol} خلال \u{2066}24\u{2069} ساعة أعلى من {threshold}: {current_value}"
            )
        }
    }
}

fn alert_change_below_body(
    locale: Locale,
    symbol: &str,
    threshold: &str,
    current_value: &str,
) -> String {
    match locale {
        Locale::En => format!("{symbol} 24h change is below {threshold}: {current_value}"),
        Locale::ZhHans => format!("{symbol} 24h 涨跌已低于 {threshold}：{current_value}"),
        Locale::ZhHant => format!("{symbol} 24h 漲跌已低於 {threshold}：{current_value}"),
        Locale::Es419 => {
            format!("El cambio 24h de {symbol} está por debajo de {threshold}: {current_value}")
        }
        Locale::PtBr => {
            format!("A variação 24h de {symbol} está abaixo de {threshold}: {current_value}")
        }
        Locale::Vi => {
            format!("Biến động 24h của {symbol} đã thấp hơn {threshold}: {current_value}")
        }
        Locale::Id => format!("Perubahan 24j {symbol} di bawah {threshold}: {current_value}"),
        Locale::Tr => {
            format!("{symbol} 24 saatlik değişimi {threshold} altında: {current_value}")
        }
        Locale::Ko => format!("{symbol} 24시간 변동률이 {threshold} 이하입니다: {current_value}"),
        Locale::Ja => {
            format!("{symbol} の24時間変動率が {threshold} を下回りました: {current_value}")
        }
        Locale::Ru => format!("Изменение {symbol} за 24ч ниже {threshold}: {current_value}"),
        Locale::Ar => {
            format!("تغير {symbol} خلال \u{2066}24\u{2069} ساعة أقل من {threshold}: {current_value}")
        }
    }
}

pub fn update_available_notification_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Crypto HUD update available",
        Locale::ZhHans => "Crypto HUD 有可用更新",
        Locale::ZhHant => "Crypto HUD 有可用更新",
        Locale::Es419 => "Actualización de Crypto HUD disponible",
        Locale::PtBr => "Atualização do Crypto HUD disponível",
        Locale::Vi => "Có bản cập nhật Crypto HUD",
        Locale::Id => "Pembaruan Crypto HUD tersedia",
        Locale::Tr => "Crypto HUD güncellemesi var",
        Locale::Ko => "Crypto HUD 업데이트 사용 가능",
        Locale::Ja => "Crypto HUD の更新があります",
        Locale::Ru => "Доступно обновление Crypto HUD",
        Locale::Ar => "يتوفر تحديث \u{2066}Crypto HUD\u{2069}",
    }
}

pub fn update_available_notification_body(
    locale: Locale,
    tag_name: &str,
    asset_name: Option<&str>,
    checksum_asset_name: Option<&str>,
) -> String {
    let tag_name = ltr_isolate_for_locale(locale, tag_name);
    let asset_name = asset_name.map(|asset| ltr_isolate_for_locale(locale, asset));
    let checksum_asset_name =
        checksum_asset_name.map(|checksum| ltr_isolate_for_locale(locale, checksum));
    let release_channel = ltr_isolate_for_locale(locale, "GitHub Releases");

    match (locale, asset_name.as_deref(), checksum_asset_name.as_deref()) {
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
        (Locale::ZhHant, Some(asset), Some(checksum)) => format!(
            "已發布 {tag_name}。請在 GitHub Releases 下載 {asset}，並使用 {checksum} 校驗。"
        ),
        (Locale::ZhHant, Some(asset), None) => {
            format!("已發布 {tag_name}。請在 GitHub Releases 下載 {asset}。")
        }
        (Locale::ZhHant, None, _) => {
            format!("已發布 {tag_name}，可在 GitHub Releases 查看。")
        }
        (Locale::Es419, Some(asset), Some(checksum)) => format!(
            "{tag_name} está disponible. Descarga {asset} desde GitHub Releases y verifica con {checksum}."
        ),
        (Locale::Es419, Some(asset), None) => {
            format!("{tag_name} está disponible. Descarga {asset} desde GitHub Releases.")
        }
        (Locale::Es419, None, _) => {
            format!("{tag_name} está disponible en GitHub Releases.")
        }
        (Locale::PtBr, Some(asset), Some(checksum)) => format!(
            "{tag_name} está disponível. Baixe {asset} no GitHub Releases e verifique com {checksum}."
        ),
        (Locale::PtBr, Some(asset), None) => {
            format!("{tag_name} está disponível. Baixe {asset} no GitHub Releases.")
        }
        (Locale::PtBr, None, _) => {
            format!("{tag_name} está disponível no GitHub Releases.")
        }
        (Locale::Vi, Some(asset), Some(checksum)) => format!(
            "{tag_name} đã có. Tải {asset} từ GitHub Releases và xác minh bằng {checksum}."
        ),
        (Locale::Vi, Some(asset), None) => {
            format!("{tag_name} đã có. Tải {asset} từ GitHub Releases.")
        }
        (Locale::Vi, None, _) => {
            format!("{tag_name} đã có trên GitHub Releases.")
        }
        (Locale::Id, Some(asset), Some(checksum)) => format!(
            "{tag_name} tersedia. Unduh {asset} dari GitHub Releases dan verifikasi dengan {checksum}."
        ),
        (Locale::Id, Some(asset), None) => {
            format!("{tag_name} tersedia. Unduh {asset} dari GitHub Releases.")
        }
        (Locale::Id, None, _) => {
            format!("{tag_name} tersedia di GitHub Releases.")
        }
        (Locale::Tr, Some(asset), Some(checksum)) => format!(
            "{tag_name} hazır. {asset} dosyasını GitHub Releases üzerinden indirin ve {checksum} ile doğrulayın."
        ),
        (Locale::Tr, Some(asset), None) => {
            format!("{tag_name} hazır. {asset} dosyasını GitHub Releases üzerinden indirin.")
        }
        (Locale::Tr, None, _) => {
            format!("{tag_name} GitHub Releases üzerinde hazır.")
        }
        (Locale::Ko, Some(asset), Some(checksum)) => format!(
            "{tag_name}을 사용할 수 있습니다. GitHub Releases에서 {asset}을 다운로드하고 {checksum}(으)로 확인하세요."
        ),
        (Locale::Ko, Some(asset), None) => {
            format!("{tag_name}을 사용할 수 있습니다. GitHub Releases에서 {asset}을 다운로드하세요.")
        }
        (Locale::Ko, None, _) => {
            format!("{tag_name}을 GitHub Releases에서 사용할 수 있습니다.")
        }
        (Locale::Ja, Some(asset), Some(checksum)) => format!(
            "{tag_name} が利用可能です。GitHub Releases から {asset} をダウンロードし、{checksum} で検証してください。"
        ),
        (Locale::Ja, Some(asset), None) => {
            format!("{tag_name} が利用可能です。GitHub Releases から {asset} をダウンロードしてください。")
        }
        (Locale::Ja, None, _) => {
            format!("{tag_name} は GitHub Releases で利用可能です。")
        }
        (Locale::Ru, Some(asset), Some(checksum)) => format!(
            "{tag_name} доступен. Скачайте {asset} из GitHub Releases и проверьте с помощью {checksum}."
        ),
        (Locale::Ru, Some(asset), None) => {
            format!("{tag_name} доступен. Скачайте {asset} из GitHub Releases.")
        }
        (Locale::Ru, None, _) => {
            format!("{tag_name} доступен в GitHub Releases.")
        }
        (Locale::Ar, Some(asset), Some(checksum)) => format!(
            "{tag_name} متاح. نزّل {asset} من {release_channel} وتحقق منه باستخدام {checksum}."
        ),
        (Locale::Ar, Some(asset), None) => {
            format!("{tag_name} متاح. نزّل {asset} من {release_channel}.")
        }
        (Locale::Ar, None, _) => {
            format!("{tag_name} متاح على {release_channel}.")
        }
    }
}

pub fn default_theme_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Default",
        Locale::ZhHans => "默认",
        Locale::ZhHant => "預設",
        Locale::Es419 => "Predeterminado",
        Locale::PtBr => "Padrão",
        Locale::Vi => "Mặc định",
        Locale::Id => "Bawaan",
        Locale::Tr => "Varsayılan",
        Locale::Ko => "기본값",
        Locale::Ja => "既定",
        Locale::Ru => "По умолчанию",
        Locale::Ar => "افتراضي",
    }
}

pub fn plugin_unavailable_id(locale: Locale, plugin_id: &str) -> String {
    let plugin_id = ltr_isolate_for_locale(locale, plugin_id);
    match locale {
        Locale::En => format!("Plugin unavailable: {plugin_id}"),
        Locale::ZhHans => format!("插件不可用：{plugin_id}"),
        Locale::ZhHant => format!("外掛不可用：{plugin_id}"),
        Locale::Es419 => format!("Plugin no disponible: {plugin_id}"),
        Locale::PtBr => format!("Plugin indisponível: {plugin_id}"),
        Locale::Vi => format!("Plugin không khả dụng: {plugin_id}"),
        Locale::Id => format!("Plugin tidak tersedia: {plugin_id}"),
        Locale::Tr => format!("Plugin kullanılamıyor: {plugin_id}"),
        Locale::Ko => format!("플러그인을 사용할 수 없음: {plugin_id}"),
        Locale::Ja => format!("プラグインを利用できません: {plugin_id}"),
        Locale::Ru => format!("Плагин недоступен: {plugin_id}"),
        Locale::Ar => format!("الإضافة غير متاحة: {plugin_id}"),
    }
}

pub fn plugin_disabled_reason(locale: Locale, reason: &str) -> String {
    let reason = plugin_status_reason(locale, reason);
    match locale {
        Locale::En => format!("Plugin disabled: {reason}"),
        Locale::ZhHans => format!("插件已禁用：{reason}"),
        Locale::ZhHant => format!("外掛已停用：{reason}"),
        Locale::Es419 => format!("Plugin desactivado: {reason}"),
        Locale::PtBr => format!("Plugin desativado: {reason}"),
        Locale::Vi => format!("Plugin đã tắt: {reason}"),
        Locale::Id => format!("Plugin dinonaktifkan: {reason}"),
        Locale::Tr => format!("Plugin devre dışı: {reason}"),
        Locale::Ko => format!("플러그인 사용 안 함: {reason}"),
        Locale::Ja => format!("プラグインは無効です: {reason}"),
        Locale::Ru => format!("Плагин отключен: {reason}"),
        Locale::Ar => format!("الإضافة معطلة: {reason}"),
    }
}

pub fn plugin_unavailable_reason(locale: Locale, reason: &str) -> String {
    let reason = plugin_status_reason(locale, reason);
    match locale {
        Locale::En => format!("Plugin unavailable: {reason}"),
        Locale::ZhHans => format!("插件不可用：{reason}"),
        Locale::ZhHant => format!("外掛不可用：{reason}"),
        Locale::Es419 => format!("Plugin no disponible: {reason}"),
        Locale::PtBr => format!("Plugin indisponível: {reason}"),
        Locale::Vi => format!("Plugin không khả dụng: {reason}"),
        Locale::Id => format!("Plugin tidak tersedia: {reason}"),
        Locale::Tr => format!("Plugin kullanılamıyor: {reason}"),
        Locale::Ko => format!("플러그인을 사용할 수 없음: {reason}"),
        Locale::Ja => format!("プラグインを利用できません: {reason}"),
        Locale::Ru => format!("Плагин недоступен: {reason}"),
        Locale::Ar => format!("الإضافة غير متاحة: {reason}"),
    }
}

fn plugin_status_reason(locale: Locale, reason: &str) -> String {
    match reason {
        "prototype widget is disabled" => plugin_prototype_disabled_reason(locale).to_string(),
        reason if reason == crate::plugin::SLINT_RENDERER_UNCOMPILED_REASON => {
            plugin_renderer_uncompiled_reason(locale)
        }
        _ => {
            if let Some(detail) = reason.strip_prefix("Slint compilation failed: ") {
                return plugin_slint_compilation_failed_reason(locale, detail);
            }
            if let Some(rest) = reason.strip_prefix("renderer.component ") {
                if let Some((component, available_components)) =
                    rest.split_once(" was not exported; available components: ")
                {
                    return plugin_component_not_exported_reason(
                        locale,
                        component,
                        available_components,
                    );
                }
            }
            if let Some(name) = reason.strip_prefix("Slint component is missing required property ")
            {
                return plugin_missing_required_property_reason(locale, name);
            }
            if let Some(name) = reason.strip_prefix("Slint component is missing required callback ")
            {
                return plugin_missing_required_callback_reason(locale, name);
            }
            if let Some(rest) = reason.strip_prefix("Slint property ") {
                if let Some((name, detail)) = rest.split_once(" has type ") {
                    return plugin_property_type_mismatch_reason(locale, name, detail);
                }
            }
            ltr_isolate_for_locale(locale, reason)
        }
    }
}

fn plugin_prototype_disabled_reason(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "prototype widget is disabled",
        Locale::ZhHans => "原型小组件已禁用",
        Locale::ZhHant => "原型小工具已停用",
        Locale::Es419 => "widget prototipo desactivado",
        Locale::PtBr => "widget protótipo desativado",
        Locale::Vi => "widget nguyên mẫu đã tắt",
        Locale::Id => "widget prototipe dinonaktifkan",
        Locale::Tr => "prototip widget devre dışı",
        Locale::Ko => "프로토타입 위젯 사용 안 함",
        Locale::Ja => "プロトタイプウィジェットは無効です",
        Locale::Ru => "прототип виджета отключен",
        Locale::Ar => "أداة النموذج الأولي معطلة",
    }
}

fn plugin_renderer_uncompiled_reason(locale: Locale) -> String {
    let slint = ltr_isolate_for_locale(locale, "Slint");
    match locale {
        Locale::En => "Slint renderer has not been compiled".to_string(),
        Locale::ZhHans => format!("{slint} 渲染器尚未编译"),
        Locale::ZhHant => format!("{slint} 渲染器尚未編譯"),
        Locale::Es419 => format!("El renderizador de {slint} no se ha compilado"),
        Locale::PtBr => format!("O renderizador {slint} ainda não foi compilado"),
        Locale::Vi => format!("Bộ kết xuất {slint} chưa được biên dịch"),
        Locale::Id => format!("Renderer {slint} belum dikompilasi"),
        Locale::Tr => format!("{slint} oluşturucu henüz derlenmedi"),
        Locale::Ko => format!("{slint} 렌더러가 아직 컴파일되지 않았습니다"),
        Locale::Ja => format!("{slint} レンダラーはまだコンパイルされていません"),
        Locale::Ru => format!("рендерер {slint} еще не скомпилирован"),
        Locale::Ar => format!("لم يتم تجميع عارض {slint} بعد"),
    }
}

fn plugin_slint_compilation_failed_reason(locale: Locale, detail: &str) -> String {
    let slint = ltr_isolate_for_locale(locale, "Slint");
    let detail = ltr_isolate_for_locale(locale, detail);
    match locale {
        Locale::En => format!("Slint compilation failed: {detail}"),
        Locale::ZhHans => format!("{slint} 编译失败：{detail}"),
        Locale::ZhHant => format!("{slint} 編譯失敗：{detail}"),
        Locale::Es419 => format!("Falló la compilación de {slint}: {detail}"),
        Locale::PtBr => format!("Falha ao compilar {slint}: {detail}"),
        Locale::Vi => format!("Biên dịch {slint} thất bại: {detail}"),
        Locale::Id => format!("Kompilasi {slint} gagal: {detail}"),
        Locale::Tr => format!("{slint} derlemesi başarısız: {detail}"),
        Locale::Ko => format!("{slint} 컴파일 실패: {detail}"),
        Locale::Ja => format!("{slint} のコンパイルに失敗しました: {detail}"),
        Locale::Ru => format!("сбой компиляции {slint}: {detail}"),
        Locale::Ar => format!("فشل تجميع {slint}: {detail}"),
    }
}

fn plugin_component_not_exported_reason(
    locale: Locale,
    component: &str,
    available_components: &str,
) -> String {
    let component = ltr_isolate_for_locale(locale, component);
    let available_components = ltr_isolate_for_locale(locale, available_components);
    match locale {
        Locale::En => {
            format!("Component {component} was not exported; available: {available_components}")
        }
        Locale::ZhHans => format!("组件未导出：{component}；可用组件：{available_components}"),
        Locale::ZhHant => format!("元件未匯出：{component}；可用元件：{available_components}"),
        Locale::Es419 => {
            format!("El componente {component} no se exportó; disponibles: {available_components}")
        }
        Locale::PtBr => {
            format!("Componente {component} não exportado; disponíveis: {available_components}")
        }
        Locale::Vi => {
            format!("Component {component} chưa được export; hiện có: {available_components}")
        }
        Locale::Id => {
            format!("Komponen {component} tidak diekspor; tersedia: {available_components}")
        }
        Locale::Tr => format!("{component} bileşeni dışa aktarılmadı; mevcut: {available_components}"),
        Locale::Ko => format!("{component} 컴포넌트를 내보내지 않았습니다. 사용 가능: {available_components}"),
        Locale::Ja => format!("コンポーネント {component} はエクスポートされていません。利用可能: {available_components}"),
        Locale::Ru => format!("компонент {component} не экспортирован; доступны: {available_components}"),
        Locale::Ar => format!("لم يتم تصدير المكون {component}؛ المتاح: {available_components}"),
    }
}

fn plugin_missing_required_property_reason(locale: Locale, name: &str) -> String {
    let name = ltr_isolate_for_locale(locale, name);
    match locale {
        Locale::En => format!("Missing required property {name}"),
        Locale::ZhHans => format!("缺少必需属性：{name}"),
        Locale::ZhHant => format!("缺少必要屬性：{name}"),
        Locale::Es419 => format!("Falta la propiedad requerida {name}"),
        Locale::PtBr => format!("Propriedade obrigatória ausente: {name}"),
        Locale::Vi => format!("Thiếu thuộc tính bắt buộc {name}"),
        Locale::Id => format!("Properti wajib hilang: {name}"),
        Locale::Tr => format!("Gerekli özellik eksik: {name}"),
        Locale::Ko => format!("필수 속성 누락: {name}"),
        Locale::Ja => format!("必須プロパティがありません: {name}"),
        Locale::Ru => format!("отсутствует обязательное свойство: {name}"),
        Locale::Ar => format!("الخاصية المطلوبة مفقودة: {name}"),
    }
}

fn plugin_missing_required_callback_reason(locale: Locale, name: &str) -> String {
    let name = ltr_isolate_for_locale(locale, name);
    match locale {
        Locale::En => format!("Missing required callback {name}"),
        Locale::ZhHans => format!("缺少必需回调：{name}"),
        Locale::ZhHant => format!("缺少必要回呼：{name}"),
        Locale::Es419 => format!("Falta el callback requerido {name}"),
        Locale::PtBr => format!("Callback obrigatório ausente: {name}"),
        Locale::Vi => format!("Thiếu callback bắt buộc {name}"),
        Locale::Id => format!("Callback wajib hilang: {name}"),
        Locale::Tr => format!("Gerekli callback eksik: {name}"),
        Locale::Ko => format!("필수 콜백 누락: {name}"),
        Locale::Ja => format!("必須コールバックがありません: {name}"),
        Locale::Ru => format!("отсутствует обязательный callback: {name}"),
        Locale::Ar => format!("نداء callback المطلوب مفقود: {name}"),
    }
}

fn plugin_property_type_mismatch_reason(locale: Locale, name: &str, detail: &str) -> String {
    let name = ltr_isolate_for_locale(locale, name);
    let detail = ltr_isolate_for_locale(locale, detail);
    match locale {
        Locale::En => format!("Property {name} has type {detail}"),
        Locale::ZhHans => format!("属性类型不匹配：{name}（{detail}）"),
        Locale::ZhHant => format!("屬性型別不符：{name}（{detail}）"),
        Locale::Es419 => format!("Tipo de propiedad incorrecto: {name} ({detail})"),
        Locale::PtBr => format!("Tipo de propriedade incorreto: {name} ({detail})"),
        Locale::Vi => format!("Sai kiểu thuộc tính: {name} ({detail})"),
        Locale::Id => format!("Tipe properti tidak cocok: {name} ({detail})"),
        Locale::Tr => format!("Özellik türü eşleşmiyor: {name} ({detail})"),
        Locale::Ko => format!("속성 타입 불일치: {name} ({detail})"),
        Locale::Ja => format!("プロパティ型が一致しません: {name} ({detail})"),
        Locale::Ru => format!("несовпадение типа свойства: {name} ({detail})"),
        Locale::Ar => format!("نوع الخاصية غير مطابق: {name} ({detail})"),
    }
}

pub fn plugin_builtin_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Built-in",
        Locale::ZhHans => "内置",
        Locale::ZhHant => "內建",
        Locale::Es419 => "Integrado",
        Locale::PtBr => "Integrado",
        Locale::Vi => "Tích hợp",
        Locale::Id => "Bawaan",
        Locale::Tr => "Yerleşik",
        Locale::Ko => "기본 제공",
        Locale::Ja => "組み込み",
        Locale::Ru => "Встроенный",
        Locale::Ar => "مضمن",
    }
}

pub fn plugin_trusted_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Trusted",
        Locale::ZhHans => "已信任",
        Locale::ZhHant => "已信任",
        Locale::Es419 => "Confiable",
        Locale::PtBr => "Confiável",
        Locale::Vi => "Đã tin cậy",
        Locale::Id => "Tepercaya",
        Locale::Tr => "Güvenilir",
        Locale::Ko => "신뢰됨",
        Locale::Ja => "信頼済み",
        Locale::Ru => "Доверенный",
        Locale::Ar => "موثوق",
    }
}

pub fn builtin_plugin_title(locale: Locale, plugin_id: &str) -> Option<&'static str> {
    match plugin_id {
        "com.cryptohud.focus-ticker" => Some(match locale {
            Locale::En => "Focus Ticker",
            Locale::ZhHans => "焦点行情",
            Locale::ZhHant => "焦點行情",
            Locale::Es419 => "Ticker destacado",
            Locale::PtBr => "Ticker em foco",
            Locale::Vi => "Ticker trọng tâm",
            Locale::Id => "Ticker fokus",
            Locale::Tr => "Odak ticker",
            Locale::Ko => "포커스 티커",
            Locale::Ja => "フォーカスティッカー",
            Locale::Ru => "Фокусный тикер",
            Locale::Ar => "شريط التركيز",
        }),
        "com.cryptohud.market-board" => Some(match locale {
            Locale::En => "Market Board",
            Locale::ZhHans => "行情看板",
            Locale::ZhHant => "行情看板",
            Locale::Es419 => "Panel de mercado",
            Locale::PtBr => "Painel de mercado",
            Locale::Vi => "Bảng thị trường",
            Locale::Id => "Papan pasar",
            Locale::Tr => "Piyasa panosu",
            Locale::Ko => "시장 보드",
            Locale::Ja => "マーケットボード",
            Locale::Ru => "Доска рынка",
            Locale::Ar => "لوحة السوق",
        }),
        "com.cryptohud.market-compass" => Some(match locale {
            Locale::En => "Market Compass",
            Locale::ZhHans => "市场罗盘",
            Locale::ZhHant => "市場羅盤",
            Locale::Es419 => "Brújula de mercado",
            Locale::PtBr => "Bússola de mercado",
            Locale::Vi => "La bàn thị trường",
            Locale::Id => "Kompas pasar",
            Locale::Tr => "Piyasa pusulası",
            Locale::Ko => "시장 나침반",
            Locale::Ja => "マーケットコンパス",
            Locale::Ru => "Компас рынка",
            Locale::Ar => "بوصلة السوق",
        }),
        "com.cryptohud.trust-card" => Some(match locale {
            Locale::En => "Trust Card",
            Locale::ZhHans => "信任卡片",
            Locale::ZhHant => "信任卡片",
            Locale::Es419 => "Tarjeta de confianza",
            Locale::PtBr => "Cartão de confiança",
            Locale::Vi => "Thẻ niềm tin",
            Locale::Id => "Kartu kepercayaan",
            Locale::Tr => "Güven kartı",
            Locale::Ko => "신뢰 카드",
            Locale::Ja => "トラストカード",
            Locale::Ru => "Карточка доверия",
            Locale::Ar => "بطاقة الثقة",
        }),
        "com.cryptohud.status-strip" => Some(match locale {
            Locale::En => "Status Strip",
            Locale::ZhHans => "状态条",
            Locale::ZhHant => "狀態列",
            Locale::Es419 => "Franja de estado",
            Locale::PtBr => "Faixa de status",
            Locale::Vi => "Dải trạng thái",
            Locale::Id => "Strip status",
            Locale::Tr => "Durum şeridi",
            Locale::Ko => "상태 스트립",
            Locale::Ja => "ステータスストリップ",
            Locale::Ru => "Полоса состояния",
            Locale::Ar => "شريط الحالة",
        }),
        _ => None,
    }
}

pub fn builtin_plugin_description(locale: Locale, plugin_id: &str) -> Option<&'static str> {
    match plugin_id {
        "com.cryptohud.focus-ticker" => Some(match locale {
            Locale::En => "Large single-pair ticker with chart emphasis for focused monitoring.",
            Locale::ZhHans => "突出单个交易对的大号行情条，适合专注盯盘。",
            Locale::ZhHant => "突出單一交易對的大型行情條，適合專注盯盤。",
            Locale::Es419 => "Ticker grande de un par con gráfico para seguimiento enfocado.",
            Locale::PtBr => "Ticker grande de um par com gráfico para monitoramento focado.",
            Locale::Vi => "Ticker một cặp cỡ lớn có biểu đồ để theo dõi tập trung.",
            Locale::Id => "Ticker satu pair berukuran besar dengan grafik untuk pemantauan fokus.",
            Locale::Tr => "Odaklı izleme için grafik vurgulu büyük tek çift ticker.",
            Locale::Ko => "차트가 강조된 큰 단일 페어 티커로 집중 모니터링에 적합합니다.",
            Locale::Ja => "チャートを強調した大きな単一ペアティッカーで集中監視できます。",
            Locale::Ru => "Крупный тикер одной пары с акцентом на график для фокусного наблюдения.",
            Locale::Ar => "شريط كبير لزوج واحد مع إبراز الرسم البياني للمتابعة المركزة.",
        }),
        "com.cryptohud.market-board" => Some(match locale {
            Locale::En => "Board layout for scanning several market pairs together.",
            Locale::ZhHans => "看板式布局，适合同时扫视多个交易对。",
            Locale::ZhHant => "看板式版面，適合同時掃視多個交易對。",
            Locale::Es419 => "Diseño de panel para revisar varios pares de mercado juntos.",
            Locale::PtBr => "Layout em painel para acompanhar vários pares juntos.",
            Locale::Vi => "Bố cục bảng để quét nhiều cặp thị trường cùng lúc.",
            Locale::Id => "Tata letak papan untuk memantau beberapa pair sekaligus.",
            Locale::Tr => "Birden çok piyasa çiftini birlikte taramak için pano düzeni.",
            Locale::Ko => "여러 시장 페어를 함께 훑어보는 보드 레이아웃입니다.",
            Locale::Ja => "複数の市場ペアをまとめて確認するボード型レイアウトです。",
            Locale::Ru => "Макет доски для одновременного просмотра нескольких рыночных пар.",
            Locale::Ar => "تخطيط لوحة لمراجعة عدة أزواج سوق معا.",
        }),
        "com.cryptohud.market-compass" => Some(match locale {
            Locale::En => "Circular multi-pair view for quickly spotting market rotation.",
            Locale::ZhHans => "环形多交易对视图，方便快速观察市场轮动。",
            Locale::ZhHant => "環形多交易對視圖，方便快速觀察市場輪動。",
            Locale::Es419 => "Vista circular multipar para detectar rápido la rotación del mercado.",
            Locale::PtBr => "Visão circular multipar para perceber rápido a rotação do mercado.",
            Locale::Vi => "Góc nhìn vòng tròn nhiều cặp để nhanh chóng nhận ra luân chuyển thị trường.",
            Locale::Id => "Tampilan melingkar multi-pair untuk cepat melihat rotasi pasar.",
            Locale::Tr => "Piyasa rotasyonunu hızlı görmek için dairesel çok çift görünümü.",
            Locale::Ko => "시장 순환을 빠르게 파악하는 원형 다중 페어 보기입니다.",
            Locale::Ja => "市場ローテーションを素早く捉える円形の複数ペア表示です。",
            Locale::Ru => "Круговой обзор нескольких пар для быстрого поиска ротации рынка.",
            Locale::Ar => "عرض دائري لعدة أزواج لرصد دوران السوق بسرعة.",
        }),
        "com.cryptohud.trust-card" => Some(match locale {
            Locale::En => "Single-pair card with trend chart and source status for confidence checks.",
            Locale::ZhHans => "单交易对卡片，结合趋势图和数据源状态便于确认行情。",
            Locale::ZhHant => "單一交易對卡片，結合趨勢圖和資料源狀態便於確認行情。",
            Locale::Es419 => "Tarjeta de un par con gráfico de tendencia y estado de fuente.",
            Locale::PtBr => "Cartão de um par com gráfico de tendência e status da fonte.",
            Locale::Vi => "Thẻ một cặp có biểu đồ xu hướng và trạng thái nguồn để kiểm tra độ tin cậy.",
            Locale::Id => "Kartu satu pair dengan grafik tren dan status sumber untuk pengecekan keyakinan.",
            Locale::Tr => "Güven kontrolü için trend grafiği ve kaynak durumu olan tek çift kartı.",
            Locale::Ko => "신뢰 확인을 위한 추세 차트와 소스 상태가 있는 단일 페어 카드입니다.",
            Locale::Ja => "信頼確認用にトレンドチャートとソース状態を備えた単一ペアカードです。",
            Locale::Ru => "Карточка одной пары с графиком тренда и статусом источника для проверки уверенности.",
            Locale::Ar => "بطاقة لزوج واحد مع مخطط اتجاه وحالة المصدر للتحقق بثقة.",
        }),
        "com.cryptohud.status-strip" => Some(match locale {
            Locale::En => "Compact strip for scanning up to five pairs in a tight space.",
            Locale::ZhHans => "紧凑状态条，适合在小空间快速查看最多 5 个交易对。",
            Locale::ZhHant => "緊湊狀態列，適合在小空間快速查看最多 5 個交易對。",
            Locale::Es419 => "Franja compacta para revisar hasta cinco pares en poco espacio.",
            Locale::PtBr => "Faixa compacta para acompanhar até cinco pares em pouco espaço.",
            Locale::Vi => "Dải nhỏ gọn để quét tối đa năm cặp trong không gian hẹp.",
            Locale::Id => "Strip ringkas untuk memantau hingga lima pair di ruang sempit.",
            Locale::Tr => "Dar alanda en fazla beş çifti taramak için kompakt şerit.",
            Locale::Ko => "좁은 공간에서 최대 5개 페어를 훑어보는 컴팩트 스트립입니다.",
            Locale::Ja => "狭いスペースで最大 5 ペアを確認できるコンパクトなストリップです。",
            Locale::Ru => "Компактная полоса для просмотра до пяти пар в ограниченном пространстве.",
            Locale::Ar => "شريط مدمج لمتابعة حتى خمسة أزواج في مساحة ضيقة.",
        }),
        _ => None,
    }
}

pub fn provider_mixed_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Mixed",
        Locale::ZhHans => "多个源",
        Locale::ZhHant => "多個來源",
        Locale::Es419 => "Mixto",
        Locale::PtBr => "Misto",
        Locale::Vi => "Hỗn hợp",
        Locale::Id => "Campuran",
        Locale::Tr => "Karma",
        Locale::Ko => "혼합",
        Locale::Ja => "混在",
        Locale::Ru => "Смешано",
        Locale::Ar => "مختلط",
    }
}

pub fn symbol_bounds_description(locale: Locale, min: usize, max: usize) -> String {
    let raw_max = max;
    let min_max = range_text_for_locale(locale, min, max);
    let max = count_text_for_locale(locale, max);
    match locale {
        Locale::ZhHans => {
            symbol_bounds_zh(min, raw_max, max.as_str(), min_max.as_str(), "个交易对")
        }
        Locale::ZhHant => {
            symbol_bounds_zh(min, raw_max, max.as_str(), min_max.as_str(), "個交易對")
        }
        Locale::Es419 if min == raw_max => format!("{max} pares"),
        Locale::Es419 if min <= 1 => format!("hasta {max} pares"),
        Locale::Es419 => format!("{min_max} pares"),
        Locale::PtBr if min == raw_max => format!("{max} pares"),
        Locale::PtBr if min <= 1 => format!("até {max} pares"),
        Locale::PtBr => format!("{min_max} pares"),
        Locale::Vi if min == raw_max => format!("{max} cặp"),
        Locale::Vi if min <= 1 => format!("tối đa {max} cặp"),
        Locale::Vi => format!("{min_max} cặp"),
        Locale::Id if min == raw_max => format!("{max} pair"),
        Locale::Id if min <= 1 => format!("hingga {max} pair"),
        Locale::Id => format!("{min_max} pair"),
        Locale::Tr if min == raw_max => format!("{max} çift"),
        Locale::Tr if min <= 1 => format!("en fazla {max} çift"),
        Locale::Tr => format!("{min_max} çift"),
        Locale::Ko if min == raw_max => format!("{max}개 페어"),
        Locale::Ko if min <= 1 => format!("최대 {max}개 페어"),
        Locale::Ko => format!("{min_max}개 페어"),
        Locale::Ja if min == raw_max => format!("{max} ペア"),
        Locale::Ja if min <= 1 => format!("最大 {max} ペア"),
        Locale::Ja => format!("{min_max} ペア"),
        Locale::Ru if min == raw_max => format!("{max} пар"),
        Locale::Ru if min <= 1 => format!("до {max} пар"),
        Locale::Ru => format!("{min_max} пар"),
        Locale::Ar if min == raw_max => format!("{max} أزواج"),
        Locale::Ar if min <= 1 => format!("حتى {max} أزواج"),
        Locale::Ar => format!("{min_max} أزواج"),
        Locale::En if min == raw_max => format!("{max} pairs"),
        Locale::En if min <= 1 => format!("up to {max} pairs"),
        Locale::En => format!("{min_max} pairs"),
    }
}

fn symbol_bounds_zh(
    raw_min: usize,
    raw_max: usize,
    max: &str,
    min_max: &str,
    unit: &'static str,
) -> String {
    if raw_min == raw_max {
        format!("{max} {unit}")
    } else if raw_min <= 1 {
        format!("最多 {max} {unit}")
    } else {
        format!("{min_max} {unit}")
    }
}

pub fn plugin_capabilities_description(locale: Locale, capabilities: &[&str]) -> String {
    capabilities
        .iter()
        .copied()
        .filter(|capability| !capability.trim().is_empty())
        .map(|capability| plugin_capability_label(locale, capability))
        .collect::<Vec<_>>()
        .join(", ")
}

fn plugin_capability_label(locale: Locale, capability: &str) -> String {
    match capability {
        "market.price" => match locale {
            Locale::En => "prices",
            Locale::ZhHans => "价格",
            Locale::ZhHant => "價格",
            Locale::Es419 => "precios",
            Locale::PtBr => "preços",
            Locale::Vi => "giá",
            Locale::Id => "harga",
            Locale::Tr => "fiyatlar",
            Locale::Ko => "가격",
            Locale::Ja => "価格",
            Locale::Ru => "цены",
            Locale::Ar => "الأسعار",
        }
        .to_string(),
        "market.candles" => match locale {
            Locale::En => "candles",
            Locale::ZhHans => "K 线",
            Locale::ZhHant => "K 線",
            Locale::Es419 => "velas",
            Locale::PtBr => "candles",
            Locale::Vi => "nến",
            Locale::Id => "candle",
            Locale::Tr => "mumlar",
            Locale::Ko => "캔들",
            Locale::Ja => "ローソク足",
            Locale::Ru => "свечи",
            Locale::Ar => "الشموع",
        }
        .to_string(),
        _ => ltr_isolate_for_locale(locale, capability),
    }
}

pub fn local_slint_plugin_description(
    locale: Locale,
    version: &semver::Version,
    width: i32,
    height: i32,
    symbol_bounds: &str,
    capabilities: &str,
) -> String {
    let capabilities = capabilities.trim();
    let capability_suffix = if capabilities.is_empty() {
        String::new()
    } else {
        format!(" · {capabilities}")
    };

    match locale {
        Locale::En => format!(
            "Local Slint plugin v{version} · {width}x{height} · {symbol_bounds}{capability_suffix}"
        ),
        Locale::ZhHans => format!(
            "本地 Slint 插件 v{version} · {width}x{height} · {symbol_bounds}{capability_suffix}"
        ),
        Locale::ZhHant => format!(
            "本機 Slint 外掛 v{version} · {width}x{height} · {symbol_bounds}{capability_suffix}"
        ),
        Locale::Es419 => format!(
            "Plugin Slint local v{version} · {width}x{height} · {symbol_bounds}{capability_suffix}"
        ),
        Locale::PtBr => format!(
            "Plugin Slint local v{version} · {width}x{height} · {symbol_bounds}{capability_suffix}"
        ),
        Locale::Vi => format!(
            "Plugin Slint cục bộ v{version} · {width}x{height} · {symbol_bounds}{capability_suffix}"
        ),
        Locale::Id => format!(
            "Plugin Slint lokal v{version} · {width}x{height} · {symbol_bounds}{capability_suffix}"
        ),
        Locale::Tr => format!(
            "Yerel Slint plugini v{version} · {width}x{height} · {symbol_bounds}{capability_suffix}"
        ),
        Locale::Ko => format!(
            "로컬 Slint 플러그인 v{version} · {width}x{height} · {symbol_bounds}{capability_suffix}"
        ),
        Locale::Ja => format!(
            "ローカル Slint プラグイン v{version} · {width}x{height} · {symbol_bounds}{capability_suffix}"
        ),
        Locale::Ru => format!(
            "Локальный плагин Slint v{version} · {width}x{height} · {symbol_bounds}{capability_suffix}"
        ),
        Locale::Ar => {
            let slint = ltr_isolate_for_locale(locale, "Slint");
            let version = ltr_isolate_for_locale(locale, &format!("v{version}"));
            let dimensions = ltr_isolate_for_locale(locale, &format!("{width}x{height}"));
            format!(
                "إضافة {slint} محلية {version} · {dimensions} · {symbol_bounds}{capability_suffix}"
            )
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolPickerCopyMode {
    DefaultAdd,
    DefaultReplace,
    WidgetAdd,
    WidgetReplace,
}

pub fn icon_cache_cleared(locale: Locale, deleted: usize) -> String {
    let deleted = count_text_for_locale(locale, deleted);
    match locale {
        Locale::En => format!("Icon cache cleared ({deleted} files removed)"),
        Locale::ZhHans => format!("图标缓存已清空（已移除 {deleted} 个文件）"),
        Locale::ZhHant => format!("圖示快取已清空（已移除 {deleted} 個檔案）"),
        Locale::Es419 => format!("Caché de íconos borrada ({deleted} archivos eliminados)"),
        Locale::PtBr => format!("Cache de ícones limpo ({deleted} arquivos removidos)"),
        Locale::Vi => format!("Đã xóa bộ nhớ đệm biểu tượng ({deleted} tệp đã xóa)"),
        Locale::Id => format!("Cache ikon dibersihkan ({deleted} file dihapus)"),
        Locale::Tr => format!("Simge önbelleği temizlendi ({deleted} dosya kaldırıldı)"),
        Locale::Ko => format!("아이콘 캐시를 지웠습니다({deleted}개 파일 제거)"),
        Locale::Ja => format!("アイコンキャッシュを削除しました（{deleted} ファイルを削除）"),
        Locale::Ru => format!("Кэш значков очищен (удалено файлов: {deleted})"),
        Locale::Ar => format!("تم مسح ذاكرة الأيقونات المؤقتة (تمت إزالة {deleted} ملفات)"),
    }
}

pub fn symbols_help_text(locale: Locale, min: usize, max: usize) -> String {
    let raw_max = max;
    let max = count_text_for_locale(locale, max);
    match locale {
        Locale::En if min == raw_max => format!("Exactly {max} pairs"),
        Locale::En => format!("Up to {max} pairs"),
        Locale::ZhHans if min == raw_max => format!("必须选择 {max} 个交易对"),
        Locale::ZhHans => format!("最多选择 {max} 个交易对"),
        Locale::ZhHant if min == raw_max => format!("必須選擇 {max} 個交易對"),
        Locale::ZhHant => format!("最多選擇 {max} 個交易對"),
        Locale::Es419 if min == raw_max => format!("Exactamente {max} pares"),
        Locale::Es419 => format!("Hasta {max} pares"),
        Locale::PtBr if min == raw_max => format!("Exatamente {max} pares"),
        Locale::PtBr => format!("Até {max} pares"),
        Locale::Vi if min == raw_max => format!("Chọn đúng {max} cặp"),
        Locale::Vi => format!("Tối đa {max} cặp"),
        Locale::Id if min == raw_max => format!("Tepat {max} pair"),
        Locale::Id => format!("Hingga {max} pair"),
        Locale::Tr if min == raw_max => format!("Tam {max} çift"),
        Locale::Tr => format!("En fazla {max} çift"),
        Locale::Ko if min == raw_max => format!("정확히 {max}개 페어"),
        Locale::Ko => format!("최대 {max}개 페어"),
        Locale::Ja if min == raw_max => format!("{max} ペアを選択"),
        Locale::Ja => format!("最大 {max} ペア"),
        Locale::Ru if min == raw_max => format!("Ровно {max} пар"),
        Locale::Ru => format!("До {max} пар"),
        Locale::Ar if min == raw_max => format!("اختر {max} أزواج بالضبط"),
        Locale::Ar => format!("حتى {max} أزواج"),
    }
}

pub fn symbol_search_placeholder(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Search pair, name, or BTCUSDT",
        Locale::ZhHans => "搜索代码、名称或 BTCUSDT",
        Locale::ZhHant => "搜尋代碼、名稱或 BTCUSDT",
        Locale::Es419 => "Buscar par, nombre o BTCUSDT",
        Locale::PtBr => "Buscar par, nome ou BTCUSDT",
        Locale::Vi => "Tìm cặp, tên hoặc BTCUSDT",
        Locale::Id => "Cari pair, nama, atau BTCUSDT",
        Locale::Tr => "Çift, ad veya BTCUSDT ara",
        Locale::Ko => "페어, 이름 또는 BTCUSDT 검색",
        Locale::Ja => "ペア、名前、BTCUSDT を検索",
        Locale::Ru => "Поиск пары, названия или BTCUSDT",
        Locale::Ar => "ابحث عن الزوج أو الاسم أو \u{2066}BTCUSDT\u{2069}",
    }
}

pub fn symbol_picker_title_text(locale: Locale, mode: SymbolPickerCopyMode) -> &'static str {
    match (locale, mode) {
        (Locale::En, SymbolPickerCopyMode::DefaultAdd) => "Add new-widget default pair",
        (Locale::En, SymbolPickerCopyMode::DefaultReplace) => "Replace new-widget default pair",
        (Locale::En, SymbolPickerCopyMode::WidgetAdd) => "Add current widget pair",
        (Locale::En, SymbolPickerCopyMode::WidgetReplace) => "Replace current widget pair",
        (Locale::ZhHans, SymbolPickerCopyMode::DefaultAdd) => "添加新建默认交易对",
        (Locale::ZhHans, SymbolPickerCopyMode::DefaultReplace) => "更换新建默认交易对",
        (Locale::ZhHans, SymbolPickerCopyMode::WidgetAdd) => "添加当前组件交易对",
        (Locale::ZhHans, SymbolPickerCopyMode::WidgetReplace) => "更换当前组件交易对",
        (Locale::ZhHant, SymbolPickerCopyMode::DefaultAdd) => "新增新小工具預設交易對",
        (Locale::ZhHant, SymbolPickerCopyMode::DefaultReplace) => "更換新小工具預設交易對",
        (Locale::ZhHant, SymbolPickerCopyMode::WidgetAdd) => "新增目前小工具交易對",
        (Locale::ZhHant, SymbolPickerCopyMode::WidgetReplace) => "更換目前小工具交易對",
        (Locale::Es419, SymbolPickerCopyMode::DefaultAdd) => "Agregar par predeterminado",
        (Locale::Es419, SymbolPickerCopyMode::DefaultReplace) => "Reemplazar par predeterminado",
        (Locale::Es419, SymbolPickerCopyMode::WidgetAdd) => "Agregar par al widget",
        (Locale::Es419, SymbolPickerCopyMode::WidgetReplace) => "Reemplazar par del widget",
        (Locale::PtBr, SymbolPickerCopyMode::DefaultAdd) => "Adicionar par padrão",
        (Locale::PtBr, SymbolPickerCopyMode::DefaultReplace) => "Substituir par padrão",
        (Locale::PtBr, SymbolPickerCopyMode::WidgetAdd) => "Adicionar par ao widget",
        (Locale::PtBr, SymbolPickerCopyMode::WidgetReplace) => "Substituir par do widget",
        (Locale::Vi, SymbolPickerCopyMode::DefaultAdd) => "Thêm cặp mặc định",
        (Locale::Vi, SymbolPickerCopyMode::DefaultReplace) => "Thay cặp mặc định",
        (Locale::Vi, SymbolPickerCopyMode::WidgetAdd) => "Thêm cặp cho widget",
        (Locale::Vi, SymbolPickerCopyMode::WidgetReplace) => "Thay cặp của widget",
        (Locale::Id, SymbolPickerCopyMode::DefaultAdd) => "Tambah pair default",
        (Locale::Id, SymbolPickerCopyMode::DefaultReplace) => "Ganti pair default",
        (Locale::Id, SymbolPickerCopyMode::WidgetAdd) => "Tambah pair widget",
        (Locale::Id, SymbolPickerCopyMode::WidgetReplace) => "Ganti pair widget",
        (Locale::Tr, SymbolPickerCopyMode::DefaultAdd) => "Varsayılan çift ekle",
        (Locale::Tr, SymbolPickerCopyMode::DefaultReplace) => "Varsayılan çifti değiştir",
        (Locale::Tr, SymbolPickerCopyMode::WidgetAdd) => "Widget çifti ekle",
        (Locale::Tr, SymbolPickerCopyMode::WidgetReplace) => "Widget çiftini değiştir",
        (Locale::Ko, SymbolPickerCopyMode::DefaultAdd) => "기본 페어 추가",
        (Locale::Ko, SymbolPickerCopyMode::DefaultReplace) => "기본 페어 교체",
        (Locale::Ko, SymbolPickerCopyMode::WidgetAdd) => "위젯 페어 추가",
        (Locale::Ko, SymbolPickerCopyMode::WidgetReplace) => "위젯 페어 교체",
        (Locale::Ja, SymbolPickerCopyMode::DefaultAdd) => "既定ペアを追加",
        (Locale::Ja, SymbolPickerCopyMode::DefaultReplace) => "既定ペアを置換",
        (Locale::Ja, SymbolPickerCopyMode::WidgetAdd) => "ウィジェットのペアを追加",
        (Locale::Ja, SymbolPickerCopyMode::WidgetReplace) => "ウィジェットのペアを置換",
        (Locale::Ru, SymbolPickerCopyMode::DefaultAdd) => "Добавить пару по умолчанию",
        (Locale::Ru, SymbolPickerCopyMode::DefaultReplace) => "Заменить пару по умолчанию",
        (Locale::Ru, SymbolPickerCopyMode::WidgetAdd) => "Добавить пару в виджет",
        (Locale::Ru, SymbolPickerCopyMode::WidgetReplace) => "Заменить пару виджета",
        (Locale::Ar, SymbolPickerCopyMode::DefaultAdd) => "إضافة زوج افتراضي",
        (Locale::Ar, SymbolPickerCopyMode::DefaultReplace) => "استبدال الزوج الافتراضي",
        (Locale::Ar, SymbolPickerCopyMode::WidgetAdd) => "إضافة زوج للأداة",
        (Locale::Ar, SymbolPickerCopyMode::WidgetReplace) => "استبدال زوج الأداة",
    }
}

pub fn symbol_picker_confirm_text(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Confirm",
        Locale::ZhHans => "确定",
        Locale::ZhHant => "確認",
        Locale::Es419 => "Confirmar",
        Locale::PtBr => "Confirmar",
        Locale::Vi => "Xác nhận",
        Locale::Id => "Konfirmasi",
        Locale::Tr => "Onayla",
        Locale::Ko => "확인",
        Locale::Ja => "確定",
        Locale::Ru => "Подтвердить",
        Locale::Ar => "تأكيد",
    }
}

pub fn symbol_picker_cancel_text(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Cancel",
        Locale::ZhHans => "取消",
        Locale::ZhHant => "取消",
        Locale::Es419 => "Cancelar",
        Locale::PtBr => "Cancelar",
        Locale::Vi => "Hủy",
        Locale::Id => "Batal",
        Locale::Tr => "İptal",
        Locale::Ko => "취소",
        Locale::Ja => "キャンセル",
        Locale::Ru => "Отмена",
        Locale::Ar => "إلغاء",
    }
}

pub fn symbol_picker_empty_status_text(locale: Locale, mode: SymbolPickerCopyMode) -> &'static str {
    match (locale, mode) {
        (Locale::En, SymbolPickerCopyMode::DefaultAdd) => {
            "No new-widget default pairs are available"
        }
        (Locale::En, SymbolPickerCopyMode::DefaultReplace) => {
            "No new-widget default pairs are available to replace"
        }
        (Locale::En, SymbolPickerCopyMode::WidgetAdd) => "No current widget pairs are available",
        (Locale::En, SymbolPickerCopyMode::WidgetReplace) => {
            "No current widget pairs are available to replace"
        }
        (Locale::ZhHans, SymbolPickerCopyMode::DefaultAdd) => "没有可添加的新建默认交易对",
        (Locale::ZhHans, SymbolPickerCopyMode::DefaultReplace) => "没有可更换的新建默认交易对",
        (Locale::ZhHans, SymbolPickerCopyMode::WidgetAdd) => "没有可添加的当前组件交易对",
        (Locale::ZhHans, SymbolPickerCopyMode::WidgetReplace) => "没有可更换的当前组件交易对",
        (Locale::ZhHant, SymbolPickerCopyMode::DefaultAdd) => "沒有可新增的新小工具預設交易對",
        (Locale::ZhHant, SymbolPickerCopyMode::DefaultReplace) => "沒有可更換的新小工具預設交易對",
        (Locale::ZhHant, SymbolPickerCopyMode::WidgetAdd) => "沒有可新增的目前小工具交易對",
        (Locale::ZhHant, SymbolPickerCopyMode::WidgetReplace) => "沒有可更換的目前小工具交易對",
        (Locale::Es419, SymbolPickerCopyMode::DefaultAdd) => {
            "No hay pares predeterminados disponibles"
        }
        (Locale::Es419, SymbolPickerCopyMode::DefaultReplace) => {
            "No hay pares predeterminados para reemplazar"
        }
        (Locale::Es419, SymbolPickerCopyMode::WidgetAdd) => {
            "No hay pares disponibles para este widget"
        }
        (Locale::Es419, SymbolPickerCopyMode::WidgetReplace) => {
            "No hay pares del widget para reemplazar"
        }
        (Locale::PtBr, SymbolPickerCopyMode::DefaultAdd) => "Não há pares padrão disponíveis",
        (Locale::PtBr, SymbolPickerCopyMode::DefaultReplace) => {
            "Não há pares padrão para substituir"
        }
        (Locale::PtBr, SymbolPickerCopyMode::WidgetAdd) => {
            "Não há pares disponíveis para este widget"
        }
        (Locale::PtBr, SymbolPickerCopyMode::WidgetReplace) => {
            "Não há pares do widget para substituir"
        }
        (Locale::Vi, SymbolPickerCopyMode::DefaultAdd) => "Không có cặp mặc định để thêm",
        (Locale::Vi, SymbolPickerCopyMode::DefaultReplace) => "Không có cặp mặc định để thay",
        (Locale::Vi, SymbolPickerCopyMode::WidgetAdd) => "Không có cặp cho widget hiện tại",
        (Locale::Vi, SymbolPickerCopyMode::WidgetReplace) => {
            "Không có cặp để thay cho widget hiện tại"
        }
        (Locale::Id, SymbolPickerCopyMode::DefaultAdd) => "Tidak ada pair default yang tersedia",
        (Locale::Id, SymbolPickerCopyMode::DefaultReplace) => {
            "Tidak ada pair default untuk diganti"
        }
        (Locale::Id, SymbolPickerCopyMode::WidgetAdd) => "Tidak ada pair untuk widget ini",
        (Locale::Id, SymbolPickerCopyMode::WidgetReplace) => "Tidak ada pair widget untuk diganti",
        (Locale::Tr, SymbolPickerCopyMode::DefaultAdd) => "Kullanılabilir varsayılan çift yok",
        (Locale::Tr, SymbolPickerCopyMode::DefaultReplace) => "Değiştirilecek varsayılan çift yok",
        (Locale::Tr, SymbolPickerCopyMode::WidgetAdd) => "Bu widget için kullanılabilir çift yok",
        (Locale::Tr, SymbolPickerCopyMode::WidgetReplace) => "Değiştirilecek widget çifti yok",
        (Locale::Ko, SymbolPickerCopyMode::DefaultAdd) => "추가할 기본 페어가 없습니다",
        (Locale::Ko, SymbolPickerCopyMode::DefaultReplace) => "교체할 기본 페어가 없습니다",
        (Locale::Ko, SymbolPickerCopyMode::WidgetAdd) => "현재 위젯에 추가할 페어가 없습니다",
        (Locale::Ko, SymbolPickerCopyMode::WidgetReplace) => "현재 위젯에서 교체할 페어가 없습니다",
        (Locale::Ja, SymbolPickerCopyMode::DefaultAdd) => "追加できる既定ペアがありません",
        (Locale::Ja, SymbolPickerCopyMode::DefaultReplace) => "置換できる既定ペアがありません",
        (Locale::Ja, SymbolPickerCopyMode::WidgetAdd) => {
            "現在のウィジェットに追加できるペアがありません"
        }
        (Locale::Ja, SymbolPickerCopyMode::WidgetReplace) => {
            "現在のウィジェットで置換できるペアがありません"
        }
        (Locale::Ru, SymbolPickerCopyMode::DefaultAdd) => "Нет доступных пар по умолчанию",
        (Locale::Ru, SymbolPickerCopyMode::DefaultReplace) => "Нет пар по умолчанию для замены",
        (Locale::Ru, SymbolPickerCopyMode::WidgetAdd) => "Нет пар для текущего виджета",
        (Locale::Ru, SymbolPickerCopyMode::WidgetReplace) => "Нет пар текущего виджета для замены",
        (Locale::Ar, SymbolPickerCopyMode::DefaultAdd) => "لا توجد أزواج افتراضية متاحة",
        (Locale::Ar, SymbolPickerCopyMode::DefaultReplace) => "لا توجد أزواج افتراضية للاستبدال",
        (Locale::Ar, SymbolPickerCopyMode::WidgetAdd) => "لا توجد أزواج متاحة لهذه الأداة",
        (Locale::Ar, SymbolPickerCopyMode::WidgetReplace) => "لا توجد أزواج في الأداة للاستبدال",
    }
}

pub fn symbol_picker_status_text(
    locale: Locale,
    mode: SymbolPickerCopyMode,
    selected_count: usize,
    max: usize,
    candidate_count: usize,
    query: &str,
    fallback_only: bool,
) -> String {
    let has_query = !query.trim().is_empty();
    match mode {
        SymbolPickerCopyMode::DefaultReplace | SymbolPickerCopyMode::WidgetReplace
            if has_query && candidate_count == 0 =>
        {
            no_matching_pairs(locale)
        }
        SymbolPickerCopyMode::DefaultReplace | SymbolPickerCopyMode::WidgetReplace
            if candidate_count == 0 =>
        {
            no_replacement_pairs(locale)
        }
        SymbolPickerCopyMode::DefaultReplace | SymbolPickerCopyMode::WidgetReplace
            if fallback_only =>
        {
            local_candidates_found(locale, candidate_count)
        }
        SymbolPickerCopyMode::DefaultReplace => {
            replacement_pairs_found(locale, candidate_count, false)
        }
        SymbolPickerCopyMode::WidgetReplace => {
            replacement_pairs_found(locale, candidate_count, true)
        }
        _ if selected_count >= max => selected_limit_reached(locale, selected_count, max),
        _ if has_query && candidate_count == 0 => selected_no_matching(locale, selected_count, max),
        _ if candidate_count == 0 => selected_no_available(locale, selected_count, max),
        _ if fallback_only => local_candidates_found(locale, candidate_count),
        SymbolPickerCopyMode::DefaultAdd => add_pairs_found(locale, candidate_count, false),
        SymbolPickerCopyMode::WidgetAdd => add_pairs_found(locale, candidate_count, true),
    }
}

pub fn widget_symbol_status_text(locale: Locale, selected_count: usize, max: usize) -> String {
    selected_count_text(locale, selected_count, max)
}

pub fn default_symbol_status_text(
    locale: Locale,
    selected_count: usize,
    max: usize,
    candidate_count: usize,
    query: &str,
) -> String {
    if selected_count >= max {
        return default_symbols_full(locale, selected_count, max);
    }
    if !query.trim().is_empty() && candidate_count == 0 {
        return selected_no_matching(locale, selected_count, max);
    }
    default_symbols_future_only(locale, selected_count, max)
}

fn selected_count_text(locale: Locale, selected_count: usize, max: usize) -> String {
    let selected = selected_count_text_for_locale(locale, selected_count, max);
    match locale {
        Locale::En => format!("Selected {selected}"),
        Locale::ZhHans => format!("已选择 {selected}"),
        Locale::ZhHant => format!("已選擇 {selected}"),
        Locale::Es419 => format!("Seleccionado {selected}"),
        Locale::PtBr => format!("Selecionado {selected}"),
        Locale::Vi => format!("Đã chọn {selected}"),
        Locale::Id => format!("Dipilih {selected}"),
        Locale::Tr => format!("{selected} seçildi"),
        Locale::Ko => format!("{selected} 선택됨"),
        Locale::Ja => format!("{selected} 選択済み"),
        Locale::Ru => format!("Выбрано {selected}"),
        Locale::Ar => format!("تم اختيار {selected}"),
    }
}

fn no_matching_pairs(locale: Locale) -> String {
    match locale {
        Locale::En => "No matching pairs".to_string(),
        Locale::ZhHans => "没有匹配交易对".to_string(),
        Locale::ZhHant => "沒有符合的交易對".to_string(),
        Locale::Es419 => "No hay pares coincidentes".to_string(),
        Locale::PtBr => "Nenhum par correspondente".to_string(),
        Locale::Vi => "Không có cặp phù hợp".to_string(),
        Locale::Id => "Tidak ada pair yang cocok".to_string(),
        Locale::Tr => "Eşleşen çift yok".to_string(),
        Locale::Ko => "일치하는 페어가 없습니다".to_string(),
        Locale::Ja => "一致するペアがありません".to_string(),
        Locale::Ru => "Нет совпадающих пар".to_string(),
        Locale::Ar => "لا توجد أزواج مطابقة".to_string(),
    }
}

fn no_replacement_pairs(locale: Locale) -> String {
    match locale {
        Locale::En => "No pairs available to replace".to_string(),
        Locale::ZhHans => "没有可更换的交易对".to_string(),
        Locale::ZhHant => "沒有可更換的交易對".to_string(),
        Locale::Es419 => "No hay pares para reemplazar".to_string(),
        Locale::PtBr => "Não há pares para substituir".to_string(),
        Locale::Vi => "Không có cặp để thay".to_string(),
        Locale::Id => "Tidak ada pair untuk diganti".to_string(),
        Locale::Tr => "Değiştirilecek çift yok".to_string(),
        Locale::Ko => "교체할 페어가 없습니다".to_string(),
        Locale::Ja => "置換できるペアがありません".to_string(),
        Locale::Ru => "Нет пар для замены".to_string(),
        Locale::Ar => "لا توجد أزواج للاستبدال".to_string(),
    }
}

fn local_candidates_found(locale: Locale, candidate_count: usize) -> String {
    let candidate_count = count_text_for_locale(locale, candidate_count);
    match locale {
        Locale::En => {
            format!("Catalog is unavailable; using local candidates. Found {candidate_count}")
        }
        Locale::ZhHans => format!("候选目录暂不可用，已使用本地候选，找到 {candidate_count} 个"),
        Locale::ZhHant => format!("候選目錄暫不可用，已使用本機候選，找到 {candidate_count} 個"),
        Locale::Es419 => {
            format!("El catálogo no está disponible; usando candidatos locales. {candidate_count} encontrados")
        }
        Locale::PtBr => {
            format!(
                "Catálogo indisponível; usando candidatos locais. {candidate_count} encontrados"
            )
        }
        Locale::Vi => {
            format!("Danh mục chưa khả dụng; dùng danh sách cục bộ. Tìm thấy {candidate_count}")
        }
        Locale::Id => {
            format!("Katalog tidak tersedia; memakai kandidat lokal. Ditemukan {candidate_count}")
        }
        Locale::Tr => {
            format!("Katalog kullanılamıyor; yerel adaylar kullanılıyor. {candidate_count} bulundu")
        }
        Locale::Ko => {
            format!("카탈로그를 사용할 수 없어 로컬 후보를 사용합니다. {candidate_count}개 발견")
        }
        Locale::Ja => {
            format!("カタログを利用できないためローカル候補を使用中。{candidate_count} 件見つかりました")
        }
        Locale::Ru => {
            format!(
                "Каталог недоступен; используются локальные варианты. Найдено: {candidate_count}"
            )
        }
        Locale::Ar => {
            format!(
                "الفهرس غير متاح؛ يتم استخدام المرشحات المحلية. تم العثور على {candidate_count}"
            )
        }
    }
}

fn replacement_pairs_found(locale: Locale, candidate_count: usize, current_widget: bool) -> String {
    let candidate_count = count_text_for_locale(locale, candidate_count);
    match (locale, current_widget) {
        (Locale::En, false) => {
            format!("Found {candidate_count} replacement pairs. Only affects newly created widgets")
        }
        (Locale::En, true) => {
            format!("Found {candidate_count} replacement pairs. Applies to this widget immediately")
        }
        (Locale::ZhHans, false) => format!("找到 {candidate_count} 个可更换交易对，只影响以后新建的小组件"),
        (Locale::ZhHans, true) => format!("找到 {candidate_count} 个可更换交易对，会立即影响当前小组件"),
        (Locale::ZhHant, false) => format!("找到 {candidate_count} 個可更換交易對，只影響之後新建的小工具"),
        (Locale::ZhHant, true) => format!("找到 {candidate_count} 個可更換交易對，會立即影響目前小工具"),
        (Locale::Es419, false) => format!("Se encontraron {candidate_count} pares de reemplazo. Solo afecta widgets nuevos"),
        (Locale::Es419, true) => format!("Se encontraron {candidate_count} pares de reemplazo. Aplica a este widget de inmediato"),
        (Locale::PtBr, false) => format!("{candidate_count} pares para substituir. Afeta apenas novos widgets"),
        (Locale::PtBr, true) => format!("{candidate_count} pares para substituir. Aplica a este widget imediatamente"),
        (Locale::Vi, false) => format!("Tìm thấy {candidate_count} cặp thay thế. Chỉ ảnh hưởng widget mới"),
        (Locale::Vi, true) => format!("Tìm thấy {candidate_count} cặp thay thế. Áp dụng ngay cho widget này"),
        (Locale::Id, false) => format!("Ditemukan {candidate_count} pair pengganti. Hanya untuk widget baru"),
        (Locale::Id, true) => format!("Ditemukan {candidate_count} pair pengganti. Langsung berlaku untuk widget ini"),
        (Locale::Tr, false) => format!("{candidate_count} değiştirme çifti bulundu. Yalnızca yeni widget'ları etkiler"),
        (Locale::Tr, true) => format!("{candidate_count} değiştirme çifti bulundu. Bu widget'a hemen uygulanır"),
        (Locale::Ko, false) => format!("교체 페어 {candidate_count}개 발견. 새 위젯에만 적용됩니다"),
        (Locale::Ko, true) => format!("교체 페어 {candidate_count}개 발견. 이 위젯에 즉시 적용됩니다"),
        (Locale::Ja, false) => format!("置換ペアが {candidate_count} 件見つかりました。新しいウィジェットのみに適用"),
        (Locale::Ja, true) => format!("置換ペアが {candidate_count} 件見つかりました。このウィジェットに即時適用"),
        (Locale::Ru, false) => format!("Найдено пар для замены: {candidate_count}. Влияет только на новые виджеты"),
        (Locale::Ru, true) => format!("Найдено пар для замены: {candidate_count}. Сразу применяется к этому виджету"),
        (Locale::Ar, false) => format!("تم العثور على {candidate_count} أزواج للاستبدال. يؤثر فقط في الأدوات الجديدة"),
        (Locale::Ar, true) => format!("تم العثور على {candidate_count} أزواج للاستبدال. يطبق على هذه الأداة فورًا"),
    }
}

fn selected_limit_reached(locale: Locale, selected_count: usize, max: usize) -> String {
    let selected = selected_count_text_for_locale(locale, selected_count, max);
    match locale {
        Locale::En => {
            format!("Selected {selected}. Limit reached; remove one pair first")
        }
        Locale::ZhHans => format!("已选 {selected}，已达上限，先移除一个交易对"),
        Locale::ZhHant => format!("已選 {selected}，已達上限，請先移除一個交易對"),
        Locale::Es419 => {
            format!("Seleccionado {selected}. Límite alcanzado; elimina un par")
        }
        Locale::PtBr => {
            format!("Selecionado {selected}. Limite atingido; remova um par")
        }
        Locale::Vi => format!("Đã chọn {selected}. Đã đạt giới hạn; hãy bỏ một cặp"),
        Locale::Id => format!("Dipilih {selected}. Batas tercapai; hapus satu pair"),
        Locale::Tr => {
            format!("{selected} seçildi. Sınıra ulaşıldı; önce bir çift kaldırın")
        }
        Locale::Ko => {
            format!("{selected} 선택됨. 한도에 도달했습니다. 페어 하나를 제거하세요")
        }
        Locale::Ja => format!("{selected} 選択済み。上限に達しました。先に 1 ペア削除してください"),
        Locale::Ru => format!("Выбрано {selected}. Достигнут лимит; удалите одну пару"),
        Locale::Ar => format!("تم اختيار {selected}. تم بلوغ الحد؛ أزل زوجًا أولًا"),
    }
}

fn selected_no_matching(locale: Locale, selected_count: usize, max: usize) -> String {
    let selected = selected_count_text_for_locale(locale, selected_count, max);
    match locale {
        Locale::En => format!("Selected {selected}. No matching pairs"),
        Locale::ZhHans => format!("已选 {selected}，没有匹配交易对"),
        Locale::ZhHant => format!("已選 {selected}，沒有符合的交易對"),
        Locale::Es419 => format!("Seleccionado {selected}. No hay pares coincidentes"),
        Locale::PtBr => format!("Selecionado {selected}. Nenhum par correspondente"),
        Locale::Vi => format!("Đã chọn {selected}. Không có cặp phù hợp"),
        Locale::Id => format!("Dipilih {selected}. Tidak ada pair yang cocok"),
        Locale::Tr => format!("{selected} seçildi. Eşleşen çift yok"),
        Locale::Ko => format!("{selected} 선택됨. 일치하는 페어가 없습니다"),
        Locale::Ja => format!("{selected} 選択済み。一致するペアがありません"),
        Locale::Ru => format!("Выбрано {selected}. Нет совпадающих пар"),
        Locale::Ar => format!("تم اختيار {selected}. لا توجد أزواج مطابقة"),
    }
}

fn selected_no_available(locale: Locale, selected_count: usize, max: usize) -> String {
    let selected = selected_count_text_for_locale(locale, selected_count, max);
    match locale {
        Locale::En => format!("Selected {selected}. No pairs available to add"),
        Locale::ZhHans => format!("已选 {selected}，没有可添加的交易对"),
        Locale::ZhHant => format!("已選 {selected}，沒有可新增的交易對"),
        Locale::Es419 => format!("Seleccionado {selected}. No hay pares para agregar"),
        Locale::PtBr => format!("Selecionado {selected}. Não há pares para adicionar"),
        Locale::Vi => format!("Đã chọn {selected}. Không có cặp để thêm"),
        Locale::Id => format!("Dipilih {selected}. Tidak ada pair untuk ditambahkan"),
        Locale::Tr => format!("{selected} seçildi. Eklenecek çift yok"),
        Locale::Ko => format!("{selected} 선택됨. 추가할 페어가 없습니다"),
        Locale::Ja => format!("{selected} 選択済み。追加できるペアがありません"),
        Locale::Ru => format!("Выбрано {selected}. Нет пар для добавления"),
        Locale::Ar => format!("تم اختيار {selected}. لا توجد أزواج للإضافة"),
    }
}

fn add_pairs_found(locale: Locale, candidate_count: usize, current_widget: bool) -> String {
    let candidate_count = count_text_for_locale(locale, candidate_count);
    match (locale, current_widget) {
        (Locale::En, false) => {
            format!("Found {candidate_count} pairs. Only affects newly created widgets")
        }
        (Locale::En, true) => {
            format!("Found {candidate_count} pairs. Applies to this widget immediately")
        }
        (Locale::ZhHans, false) => {
            format!("找到 {candidate_count} 个可添加交易对，只影响以后新建的小组件")
        }
        (Locale::ZhHans, true) => {
            format!("找到 {candidate_count} 个可添加交易对，会立即影响当前小组件")
        }
        (Locale::ZhHant, false) => {
            format!("找到 {candidate_count} 個可新增交易對，只影響之後新建的小工具")
        }
        (Locale::ZhHant, true) => {
            format!("找到 {candidate_count} 個可新增交易對，會立即影響目前小工具")
        }
        (Locale::Es419, false) => {
            format!("Se encontraron {candidate_count} pares. Solo afecta widgets nuevos")
        }
        (Locale::Es419, true) => {
            format!("Se encontraron {candidate_count} pares. Aplica a este widget de inmediato")
        }
        (Locale::PtBr, false) => {
            format!("{candidate_count} pares encontrados. Afeta apenas novos widgets")
        }
        (Locale::PtBr, true) => {
            format!("{candidate_count} pares encontrados. Aplica a este widget imediatamente")
        }
        (Locale::Vi, false) => format!("Tìm thấy {candidate_count} cặp. Chỉ ảnh hưởng widget mới"),
        (Locale::Vi, true) => {
            format!("Tìm thấy {candidate_count} cặp. Áp dụng ngay cho widget này")
        }
        (Locale::Id, false) => format!("Ditemukan {candidate_count} pair. Hanya untuk widget baru"),
        (Locale::Id, true) => {
            format!("Ditemukan {candidate_count} pair. Langsung berlaku untuk widget ini")
        }
        (Locale::Tr, false) => {
            format!("{candidate_count} çift bulundu. Yalnızca yeni widget'ları etkiler")
        }
        (Locale::Tr, true) => {
            format!("{candidate_count} çift bulundu. Bu widget'a hemen uygulanır")
        }
        (Locale::Ko, false) => format!("페어 {candidate_count}개 발견. 새 위젯에만 적용됩니다"),
        (Locale::Ko, true) => format!("페어 {candidate_count}개 발견. 이 위젯에 즉시 적용됩니다"),
        (Locale::Ja, false) => {
            format!("ペアが {candidate_count} 件見つかりました。新しいウィジェットのみに適用")
        }
        (Locale::Ja, true) => {
            format!("ペアが {candidate_count} 件見つかりました。このウィジェットに即時適用")
        }
        (Locale::Ru, false) => {
            format!("Найдено пар: {candidate_count}. Влияет только на новые виджеты")
        }
        (Locale::Ru, true) => {
            format!("Найдено пар: {candidate_count}. Сразу применяется к этому виджету")
        }
        (Locale::Ar, false) => {
            format!("تم العثور على {candidate_count} أزواج. يؤثر فقط في الأدوات الجديدة")
        }
        (Locale::Ar, true) => {
            format!("تم العثور على {candidate_count} أزواج. يطبق على هذه الأداة فورًا")
        }
    }
}

fn default_symbols_full(locale: Locale, selected_count: usize, max: usize) -> String {
    let selected = selected_count_text_for_locale(locale, selected_count, max);
    match locale {
        Locale::En => format!("Selected {selected}. New-widget defaults are full"),
        Locale::ZhHans => format!("已选 {selected}，新建小组件默认交易对已满"),
        Locale::ZhHant => format!("已選 {selected}，新小工具預設交易對已滿"),
        Locale::Es419 => format!("Seleccionado {selected}. Predeterminados llenos"),
        Locale::PtBr => format!("Selecionado {selected}. Padrões completos"),
        Locale::Vi => format!("Đã chọn {selected}. Cặp mặc định đã đầy"),
        Locale::Id => format!("Dipilih {selected}. Default widget baru penuh"),
        Locale::Tr => format!("{selected} seçildi. Yeni widget varsayılanları dolu"),
        Locale::Ko => format!("{selected} 선택됨. 새 위젯 기본값이 가득 찼습니다"),
        Locale::Ja => {
            format!("{selected} 選択済み。新規ウィジェットの既定ペアは満杯です")
        }
        Locale::Ru => format!("Выбрано {selected}. Пары по умолчанию заполнены"),
        Locale::Ar => format!("تم اختيار {selected}. الأزواج الافتراضية ممتلئة"),
    }
}

fn default_symbols_future_only(locale: Locale, selected_count: usize, max: usize) -> String {
    let selected = selected_count_text_for_locale(locale, selected_count, max);
    match locale {
        Locale::En => {
            format!("Selected {selected}. Only affects newly created widgets")
        }
        Locale::ZhHans => format!("已选 {selected}，只影响以后新建的小组件"),
        Locale::ZhHant => format!("已選 {selected}，只影響之後新建的小工具"),
        Locale::Es419 => format!("Seleccionado {selected}. Solo afecta widgets nuevos"),
        Locale::PtBr => format!("Selecionado {selected}. Afeta apenas novos widgets"),
        Locale::Vi => format!("Đã chọn {selected}. Chỉ ảnh hưởng widget mới"),
        Locale::Id => format!("Dipilih {selected}. Hanya untuk widget baru"),
        Locale::Tr => format!("{selected} seçildi. Yalnızca yeni widget'ları etkiler"),
        Locale::Ko => format!("{selected} 선택됨. 새 위젯에만 적용됩니다"),
        Locale::Ja => format!("{selected} 選択済み。新しいウィジェットのみに適用"),
        Locale::Ru => format!("Выбрано {selected}. Влияет только на новые виджеты"),
        Locale::Ar => format!("تم اختيار {selected}. يؤثر فقط في الأدوات الجديدة"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_tags_resolve_to_supported_locales() {
        for (tag, expected_locale) in [
            ("zh-CN", Locale::ZhHans),
            ("zh-Hans", Locale::ZhHans),
            ("zh-Hans-HK", Locale::ZhHans),
            ("zh-SG", Locale::ZhHans),
            ("zh-TW", Locale::ZhHant),
            ("zh-Hant-CN", Locale::ZhHant),
            ("zh_Hant_HK", Locale::ZhHant),
            ("zh_TW.UTF-8", Locale::ZhHant),
            ("zh_Hant_HK@calendar=gregorian", Locale::ZhHant),
            ("cmn-Hans-CN", Locale::ZhHans),
            ("cmn-Hant-TW", Locale::ZhHant),
            ("yue-HK", Locale::ZhHant),
            ("en-US", Locale::En),
            ("en-CA", Locale::En),
            ("en-GB", Locale::En),
            ("en-IN", Locale::En),
            ("en-PK", Locale::En),
            ("en-NG", Locale::En),
            ("es-419", Locale::Es419),
            ("es-MX", Locale::Es419),
            ("es_AR.UTF-8", Locale::Es419),
            ("es-CO", Locale::Es419),
            ("es-US", Locale::Es419),
            ("es-PE", Locale::Es419),
            ("es", Locale::En),
            ("es-ES", Locale::En),
            ("pt-BR", Locale::PtBr),
            ("pt_BR.UTF-8", Locale::PtBr),
            ("pt", Locale::En),
            ("pt-PT", Locale::En),
            ("vi-VN", Locale::Vi),
            ("id-ID", Locale::Id),
            ("in_ID.UTF-8", Locale::Id),
            ("tr-TR", Locale::Tr),
            ("ko-KR", Locale::Ko),
            ("ja-JP", Locale::Ja),
            ("ru-RU", Locale::Ru),
            ("ru-KZ", Locale::Ru),
            ("ar-SA", Locale::Ar),
            ("ar-AE", Locale::Ar),
            ("ar_EG.UTF-8", Locale::Ar),
            ("C.UTF-8", Locale::En),
        ] {
            assert_eq!(
                locale_from_tag(tag),
                expected_locale,
                "{tag} should resolve to {expected_locale:?}"
            );
        }
    }

    #[test]
    fn locale_tag_resolution_reuses_language_preference_parser() {
        for tag in [
            "en-US",
            "zh-Hans-SG",
            "zh-Hant-HK",
            "es-419",
            "es-MX",
            "pt-BR",
            "vi-VN",
            "id-ID",
            "tr-TR",
            "ko-KR",
            "ja-JP",
            "ru-KZ",
            "ar-SA",
        ] {
            let preference = LanguagePreference::from_locale_tag(tag)
                .unwrap_or_else(|| panic!("{tag} should map to a language preference"));
            assert_eq!(
                locale_from_tag(tag),
                locale_from_language_preference(preference).unwrap(),
                "{tag} should resolve through the shared language-preference parser"
            );
        }

        for unsupported_tag in ["es", "es-ES", "pt", "pt-PT", "C.UTF-8"] {
            assert!(LanguagePreference::from_locale_tag(unsupported_tag).is_none());
            assert_eq!(locale_from_tag(unsupported_tag), Locale::En);
        }
    }

    #[test]
    fn supported_locale_lists_stay_in_sync_with_language_preferences() {
        let explicit_locales = LanguagePreference::ALL
            .into_iter()
            .filter(|preference| *preference != LanguagePreference::System)
            .map(resolve_locale)
            .collect::<Vec<_>>();
        assert_eq!(explicit_locales, Locale::ALL);
        assert_eq!(
            Locale::NON_ENGLISH,
            Locale::ALL
                .into_iter()
                .filter(|locale| *locale != Locale::En)
                .collect::<Vec<_>>()
                .as_slice()
        );
    }

    #[test]
    fn locale_all_is_available_outside_test_only_code() {
        let source = include_str!("i18n.rs");
        let all_index = source
            .find("pub const ALL: [Self; 12]")
            .expect("Locale::ALL should exist");
        let impl_index = source[..all_index]
            .rfind("impl Locale")
            .expect("Locale::ALL should live inside a Locale impl");
        let test_cfg_index = source[..all_index].rfind("#[cfg(test)]");

        assert!(
            test_cfg_index.is_none_or(|index| index < impl_index),
            "Locale::ALL is documented as a shared locale contract and should not be test-only"
        );
    }

    #[test]
    fn localization_guide_lists_every_supported_locale() {
        let guide = read_workspace_file("LOCALIZATION.md");
        let documented_rows = documented_locale_rows(&guide);
        let expected_rows = Locale::ALL
            .into_iter()
            .map(|locale| {
                (
                    locale_documentation_tag(locale).to_string(),
                    format!("{:?}", language_preference_for_locale(locale)),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            documented_rows, expected_rows,
            "LOCALIZATION.md should document exactly the supported locales in preference order"
        );
    }

    #[test]
    fn readmes_advertise_the_exact_supported_locale_tags() {
        let supported_language_count = Locale::ALL.len();
        let readme_en = read_workspace_file("README.md");
        let readme_zh = read_workspace_file("README.zh-CN.md");
        let expected_tags = Locale::ALL
            .into_iter()
            .map(locale_documentation_tag)
            .collect::<Vec<_>>();

        assert!(
            ["UI languages", "interface languages"]
                .into_iter()
                .any(|label| readme_en.contains(&format!("{supported_language_count} {label}"))),
            "README.md should advertise the current supported language count"
        );
        assert!(
            readme_zh.contains(&format!("{supported_language_count} 种界面语言")),
            "README.zh-CN.md should advertise the current supported language count"
        );

        assert_eq!(
            documented_readme_locale_tags(&readme_en, "Supported UI languages:"),
            expected_tags,
            "README.md should list exactly the supported UI locale tags in preference order"
        );
        assert_eq!(
            documented_readme_locale_tags(&readme_zh, "支持的界面语言："),
            expected_tags,
            "README.zh-CN.md should list exactly the supported UI locale tags in preference order"
        );
    }

    #[test]
    fn localization_guide_documents_identical_english_term_policy() {
        let guide = read_workspace_file("LOCALIZATION.md");

        for required_phrase in [
            "primary_static_ui_copy_is_localized_for_every_non_english_locale",
            "non_english_ui_text_does_not_match_english_unless_intentional",
            "ALLOWED_IDENTICAL_PRIMARY_UI_FIELDS",
            "allowed_identical_primary_ui_fields_are_still_needed",
            "ALLOWED_IDENTICAL_TECHNICAL_UI_FIELDS",
            "allowed_identical_technical_ui_fields_are_still_needed",
        ] {
            assert!(
                guide.contains(required_phrase),
                "LOCALIZATION.md should document {required_phrase}"
            );
        }
    }

    #[test]
    fn localization_guide_documents_locale_detection_edges() {
        let guide = read_workspace_file("LOCALIZATION.md");

        for required_phrase in [
            "`LanguagePreference::from_locale_tag` is the shared parsing source",
            "locale detection and persisted language preference compatibility",
            "The app-layer `locale_from_tag` must reuse that parser",
            "`es-419`, `es-MX`, `es-AR`",
            "`es-CO`, and `es-US` map to `es-419`",
            "Do not map `es-ES` to `es-419`",
            "does not maintain European Spanish copy",
            "Do not infer `es`",
            "without a market region as Latin American Spanish",
            "`pt-BR` and `pt_BR.UTF-8` map to `pt-BR`",
            "Do not map `pt-PT` to `pt-BR`",
            "does not maintain generic",
            "Do not infer `pt`",
            "without a Brazil region as Brazilian",
        ] {
            assert!(
                guide.contains(required_phrase),
                "LOCALIZATION.md should document locale detection edge: {required_phrase}"
            );
        }
    }

    #[test]
    fn localization_guide_keeps_alert_formatting_in_the_locale_layer() {
        let guide = read_workspace_file("LOCALIZATION.md");

        for required_phrase in [
            "Core/runtime alert evaluation should return structured data only",
            "notification titles and bodies are formatted in the app layer",
            "active locale",
        ] {
            assert!(
                guide.contains(required_phrase),
                "LOCALIZATION.md should document alert notification locale boundary: {required_phrase}"
            );
        }
    }

    #[test]
    fn localization_guide_lists_focused_i18n_checks() {
        let guide = read_workspace_file("LOCALIZATION.md");

        for required_command in [
            "cargo +1.96.0 test -p crypto-hud non_english_ui_text_constants_explicitly_maintain_every_field",
            "cargo +1.96.0 test -p crypto-hud non_english_ui_text_does_not_match_english_unless_intentional",
            "cargo +1.96.0 test -p crypto-hud readmes_advertise_the_exact_supported_locale_tags",
            "cargo +1.96.0 test -p crypto-hud-shell-state enum_serialization_uses_stable_config_tags",
            "cargo +1.96.0 test -p crypto-hud-shell-state language_preference_deserializes_common_locale_tags",
            "cargo +1.96.0 test -p crypto-hud-shell-state language_preference_error_lists_canonical_and_legacy_config_tags",
            "cargo +1.96.0 test -p crypto-hud locale_tags_resolve_to_supported_locales",
            "cargo +1.96.0 test -p crypto-hud locale_tag_resolution_reuses_language_preference_parser",
            "cargo +1.96.0 test -p crypto-hud supported_locale_lists_stay_in_sync_with_language_preferences",
            "cargo +1.96.0 test -p crypto-hud locale_all_is_available_outside_test_only_code",
            "cargo +1.96.0 test -p crypto-hud arabic_ui_text_constant_keeps_ltr_literals_isolated",
            "cargo +1.96.0 test -p crypto-hud dynamic_option_sets_are_localized_for_every_non_english_locale",
            "cargo +1.96.0 test -p crypto-hud key_settings_help_copy_is_localized_for_every_non_english_locale",
            "cargo +1.96.0 test -p crypto-hud settings_and_market_copy_follow_locale",
            "cargo +1.96.0 test -p crypto-hud slint_user_facing_text_literals_are_limited_to_non_localized_tokens",
            "cargo +1.96.0 test -p crypto-hud tray_menu_exposes_localized_show_widgets_action",
            "cargo +1.96.0 test -p crypto-hud refresh_tray_text_sets_every_localized_tray_label",
            "cargo +1.96.0 test -p crypto-hud settings_window_shared_text_controls_follow_rtl_layout",
            "cargo +1.96.0 test -p crypto-hud plugin_market_text_rows_follow_rtl_layout",
            "cargo +1.96.0 test -p crypto-hud my_widgets_list_text_rows_follow_rtl_layout",
            "cargo +1.96.0 test -p crypto-hud selected_widget_detail_text_rows_follow_rtl_layout",
            "cargo +1.96.0 test -p crypto-hud network_proxy_text_rows_follow_rtl_layout",
            "cargo +1.96.0 test -p crypto-hud system_app_info_labels_follow_rtl_layout",
            "cargo +1.96.0 test -p crypto-hud symbol_picker_text_rows_follow_rtl_layout",
            "cargo +1.96.0 test -p crypto-hud settings_refresh_preserves_open_symbol_picker",
            "cargo +1.96.0 test -p crypto-hud app_signature_version_keeps_prefix_inside_ltr_isolate",
            "cargo +1.96.0 test -p crypto-hud primary_alert_input_accepts_rtl_isolated_symbol_display",
            "cargo +1.96.0 test -p crypto-hud update_notification_body_is_localized_for_every_non_english_locale",
            "cargo +1.96.0 test -p crypto-hud arabic_update_notification_isolates_ltr_release_values",
            "cargo +1.96.0 test -p crypto-hud alert_notification_copy_is_localized_for_every_non_english_locale",
            "cargo +1.96.0 test -p crypto-hud arabic_alert_notification_isolates_ltr_market_values",
            "cargo +1.96.0 test -p crypto-hud alert_24h_terms_are_locale_appropriate",
            "cargo +1.96.0 test -p crypto-hud runtime_text_bridge_uses_selected_locale_labels",
            "cargo +1.96.0 test -p crypto-hud runtime_refresh_tracks_locale_sensitive_widget_labels",
            "cargo +1.96.0 test -p crypto-hud initial_widget_apply_sets_locale_sensitive_widget_labels",
            "cargo +1.96.0 test -p crypto-hud network_proxy_empty_address_detail_is_localized_for_every_non_english_locale",
            "cargo +1.96.0 test -p crypto-hud status_failure_message_is_localized_for_every_non_english_locale",
            "cargo +1.96.0 test -p crypto-hud icon_cache_cleared_status_is_localized_for_every_non_english_locale",
            "cargo +1.96.0 test -p crypto-hud builtin_plugin_market_copy_is_localized_for_every_non_english_locale",
            "cargo +1.96.0 test -p crypto-hud bundled_builtin_slint_plugins_have_localized_market_copy",
            "cargo +1.96.0 test -p crypto-hud plugin_unavailable_id_is_localized_for_every_non_english_locale",
            "cargo +1.96.0 test -p crypto-hud plugin_market_dynamic_descriptions_are_localized_for_every_non_english_locale",
            "cargo +1.96.0 test -p crypto-hud dynamic_short_labels_are_localized_for_every_non_english_locale",
            "cargo +1.96.0 test -p crypto-hud known_plugin_status_reasons_are_localized_for_every_non_english_locale",
            "cargo +1.96.0 test -p crypto-hud symbol_picker_control_copy_is_localized_for_every_non_english_locale",
            "cargo +1.96.0 test -p crypto-hud dynamic_plugin_and_symbol_copy_follow_locale",
            "cargo +1.96.0 test -p crypto-hud plugin_market_item_preserves_local_plugin_metadata_in_every_locale",
            "cargo +1.96.0 test -p crypto-hud custom_widget_display_names_preserve_user_text_in_every_locale",
            "cargo +1.96.0 test -p crypto-hud custom_widget_name_input_preserves_user_text_in_every_locale",
            "cargo +1.96.0 test -p crypto-hud local_plugin_market_titles_preserve_manifest_names_in_every_locale",
            "cargo +1.96.0 test -p crypto-hud custom_plugin_theme_labels_preserve_manifest_names_in_every_locale",
            "cargo +1.96.0 test -p crypto-hud plugin_development_guides_document_manifest_name_boundary",
            "cargo +1.96.0 test -p crypto-hud repo_plugins_accept_host_supplied_rtl_layout",
            "cargo +1.96.0 test -p crypto-hud repo_plugin_visible_text_literals_are_limited_to_non_localized_tokens",
            "cargo +1.96.0 test -p crypto-hud localization_guide_lists_every_supported_locale",
        ] {
            assert!(
                guide.contains(required_command),
                "LOCALIZATION.md should list focused i18n check: {required_command}"
            );
        }
    }

    #[test]
    fn localization_guide_documents_plugin_i18n_boundaries() {
        let guide = read_workspace_file("LOCALIZATION.md");

        for required_phrase in [
            "Built-in plugin-market titles and descriptions live in",
            "Repo-bundled Slint plugins must accept the host-provided `rtl-layout` property",
            "`pairs-heading-text`, `source-text`, `source-name-text`, `updated-text`, and",
            "Only `PluginSource::Builtin` may use",
            "Manifest-provided display names",
            "User-entered display names such as custom widget names follow the same rule",
            "Do not add visible English fallback strings in plugin Slint",
        ] {
            assert!(
                guide.contains(required_phrase),
                "LOCALIZATION.md should document plugin i18n boundary: {required_phrase}"
            );
        }
    }

    #[test]
    fn plugin_development_guides_document_manifest_name_boundary() {
        let english_guide = read_workspace_file("CUSTOM_UI_PLUGIN_DEVELOPMENT.md");
        let repo_plugin_guide = read_workspace_file("crates/crypto-hud/plugins/README.md");

        for required_phrase in [
            "`name` is the author-provided display name",
            "Crypto HUD shows it exactly as written and does not localize it",
        ] {
            assert!(
                english_guide.contains(required_phrase),
                "CUSTOM_UI_PLUGIN_DEVELOPMENT.md should document manifest name boundary: {required_phrase}"
            );
        }

        for required_phrase in [
            "`name` 是作者提供的展示名",
            "Crypto HUD 会按原样显示，不做本地化",
        ] {
            assert!(
                repo_plugin_guide.contains(required_phrase),
                "plugins/README.md should document manifest name boundary: {required_phrase}"
            );
        }
    }

    #[test]
    fn localization_guide_documents_market_review_notes() {
        let guide = read_workspace_file("LOCALIZATION.md");

        for required_phrase in [
            "## Market Review Notes",
            "`zh-TW`: review crypto-community terminology separately from Simplified",
            "`es-419`: avoid Spain-specific wording",
            "`pt-BR`: use Brazilian Portuguese conventions",
            "`ko` and `ja`: keep UI copy precise, restrained",
            "`ru`: review sanctions/compliance-sensitive wording before release",
            "`ar`: test RTL layout manually in addition to string checks",
            "viewing public market data only",
        ] {
            assert!(
                guide.contains(required_phrase),
                "LOCALIZATION.md should document market review note: {required_phrase}"
            );
        }
    }

    #[test]
    fn explicit_language_preference_overrides_system() {
        assert_eq!(resolve_locale(LanguagePreference::En), Locale::En);
        assert_eq!(resolve_locale(LanguagePreference::ZhHans), Locale::ZhHans);
        assert_eq!(resolve_locale(LanguagePreference::ZhHant), Locale::ZhHant);
        assert_eq!(resolve_locale(LanguagePreference::Es419), Locale::Es419);
        assert_eq!(resolve_locale(LanguagePreference::PtBr), Locale::PtBr);
        assert_eq!(resolve_locale(LanguagePreference::Ar), Locale::Ar);
    }

    #[test]
    fn language_options_include_all_supported_locales_in_preference_order() {
        let english_options = language_options(Locale::En);
        assert_eq!(english_options.len(), LanguagePreference::ALL.len());
        assert_eq!(
            english_options,
            vec![
                "System",
                "English",
                "简体中文",
                "繁體中文",
                "Español (LatAm)",
                "Português (Brasil)",
                "Tiếng Việt",
                "Bahasa Indonesia",
                "Türkçe",
                "한국어",
                "日本語",
                "Русский",
                "العربية"
            ]
        );
        for (index, preference) in LanguagePreference::ALL.into_iter().enumerate() {
            assert_eq!(LanguagePreference::from_index(index as i32), preference);
            assert_eq!(preference.index(), index as i32);
            if preference != LanguagePreference::System {
                assert_eq!(
                    english_options[index],
                    language_preference_label(preference)
                );
            }
        }
        assert_eq!(language_options(Locale::ZhHans)[0], "跟随系统");
        assert_eq!(language_options(Locale::Ar)[0], "النظام");
        assert_eq!(
            language_options(Locale::Ar)[LanguagePreference::En.index() as usize],
            "\u{2066}English\u{2069}"
        );
        assert_eq!(
            language_options(Locale::Ar)[LanguagePreference::Ar.index() as usize],
            "العربية"
        );
    }

    #[test]
    fn locale_direction_marks_arabic_as_rtl() {
        assert!(is_rtl(Locale::Ar));
        assert!(!is_rtl(Locale::En));
        assert!(!is_rtl(Locale::ZhHant));
        assert!(!is_rtl(Locale::Ru));
    }

    #[test]
    fn arabic_static_ui_text_isolates_ltr_tokens() {
        let ar_text = text(Locale::Ar);

        assert_eq!(ar_text.tray_tooltip, "\u{2066}Crypto HUD\u{2069}");
        assert_eq!(ar_text.settings_title, "\u{2066}Crypto HUD\u{2069}");
        assert_eq!(
            ar_text.auto_start_help,
            "يشغل \u{2066}Crypto HUD\u{2069} بعد تسجيل الدخول."
        );
        assert_eq!(
            update_available_notification_title(Locale::Ar),
            "يتوفر تحديث \u{2066}Crypto HUD\u{2069}"
        );
        assert_eq!(
            ar_text.network_proxy_example_hint,
            "أمثلة: \u{2066}http://127.0.0.1:7890\u{2069}  ·  \u{2066}socks5://127.0.0.1:1080\u{2069}"
        );
        assert_eq!(
            text(Locale::En).network_proxy_example_hint,
            "Examples: http://127.0.0.1:7890  ·  socks5://127.0.0.1:1080"
        );
        assert_eq!(
            ltr_isolate_for_locale(Locale::Ar, "C:\\Users\\me\\crypto-hud.json"),
            "\u{2066}C:\\Users\\me\\crypto-hud.json\u{2069}"
        );
        assert_eq!(ltr_isolate_for_locale(Locale::En, "0.9.1"), "0.9.1");
        assert_eq!(
            ar_text.symbols_help,
            "حتى \u{2066}20\u{2069} زوجا؛ تظهر الأزواج المحددة كوسوم"
        );
        assert_eq!(
            ar_text.shortcut_help,
            "استخدم \u{2066}Alt+C\u{2069} لإخفاء الأدوات أو استعادتها."
        );
        assert_eq!(
            ar_text.widget_hide_quote_asset_help,
            "يعرض \u{2066}BTC\u{2069} بدلا من \u{2066}BTC/USDC\u{2069}."
        );
        assert_eq!(
            ar_text.preview_updated,
            "تم التحديث قبل \u{2066}3\u{2069} ثوان"
        );
        assert_eq!(
            ar_text.quote_board_description,
            "يعرض حتى \u{2066}20\u{2069} زوجا لمتابعة الأسواق الرئيسية معا."
        );
        assert_eq!(
            symbol_search_placeholder(Locale::Ar),
            "ابحث عن الزوج أو الاسم أو \u{2066}BTCUSDT\u{2069}"
        );
        assert_eq!(
            shortcut_options(Locale::Ar),
            vec!["\u{2066}Alt+C\u{2069}", "معطل"]
        );
        let language_options = language_options(Locale::Ar);
        for preference in LanguagePreference::ALL {
            let label = language_options[preference.index() as usize];
            if preference != LanguagePreference::System && preference != LanguagePreference::Ar {
                assert_eq!(
                    super::strip_bidi_isolate_marks(label),
                    language_preference_label(preference)
                );
                assert!(label.starts_with('\u{2066}'));
                assert!(label.ends_with('\u{2069}'));
            }
        }
        assert_eq!(shortcut_options(Locale::En), vec!["Alt+C", "Disabled"]);
    }

    #[test]
    fn dynamic_option_sets_are_localized_for_every_non_english_locale() {
        let english_language_options = language_options(Locale::En);
        let english_theme_options = theme_options(Locale::En);
        let english_shortcut_options = shortcut_options(Locale::En);
        let english_alert_options = alert_condition_options(Locale::En);

        for locale in Locale::NON_ENGLISH {
            let language_options = language_options(locale);
            assert_eq!(language_options.len(), LanguagePreference::ALL.len());
            assert_ne!(
                language_options, english_language_options,
                "language options should localize the system label for {locale:?}"
            );

            let theme_options = theme_options(locale);
            assert_eq!(theme_options.len(), 3);
            assert_ne!(
                theme_options, english_theme_options,
                "theme options should be localized for {locale:?}"
            );

            let shortcut_options = shortcut_options(locale);
            assert_eq!(shortcut_options.len(), 2);
            assert_eq!(
                super::strip_bidi_isolate_marks(shortcut_options[0]),
                "Alt+C"
            );
            assert_ne!(
                shortcut_options, english_shortcut_options,
                "shortcut disabled label should be localized for {locale:?}"
            );

            let alert_options = alert_condition_options(locale);
            assert_eq!(alert_options.len(), 4);
            assert_ne!(
                alert_options, english_alert_options,
                "alert condition options should be localized for {locale:?}"
            );
        }
    }

    #[test]
    fn network_proxy_empty_address_detail_is_localized_for_every_non_english_locale() {
        let english_detail = network_proxy_empty_address_detail(Locale::En);

        for locale in Locale::NON_ENGLISH {
            let detail = network_proxy_empty_address_detail(locale);
            assert!(!detail.trim().is_empty());
            assert_ne!(
                detail, english_detail,
                "empty proxy address detail should be localized for {locale:?}"
            );
        }
    }

    #[test]
    fn status_failure_message_is_localized_for_every_non_english_locale() {
        let english_status = status_failure_message(
            Locale::En,
            text(Locale::En).status_shortcut_failed,
            "denied",
        );

        for locale in Locale::NON_ENGLISH {
            let status =
                status_failure_message(locale, text(locale).status_shortcut_failed, "denied");
            assert!(!status.trim().is_empty());
            assert!(
                status.contains(text(locale).status_shortcut_failed),
                "status failure should include the localized summary for {locale:?}"
            );
            assert_ne!(
                status, english_status,
                "status failure should be localized for {locale:?}"
            );
        }

        assert_eq!(
            status_failure_message(
                Locale::Ar,
                text(Locale::Ar).status_shortcut_failed,
                "denied"
            ),
            "فشل تسجيل الاختصار: \u{2066}denied\u{2069}"
        );
    }

    #[test]
    fn save_failure_message_is_localized_and_isolates_rtl_details() {
        assert_eq!(
            save_failure_message(Locale::En, "access denied"),
            "Could not save settings: access denied"
        );
        assert_eq!(
            save_failure_message(Locale::ZhHans, "access denied"),
            "无法保存设置：access denied"
        );
        assert_eq!(
            save_failure_message(Locale::Ar, "C:\\state\\layouts.json"),
            "تعذر حفظ الإعدادات: \u{2066}C:\\state\\layouts.json\u{2069}"
        );
    }

    #[test]
    fn icon_cache_cleared_status_is_localized_for_every_non_english_locale() {
        let english_status = icon_cache_cleared(Locale::En, 3);

        for locale in Locale::NON_ENGLISH {
            let status = icon_cache_cleared(locale, 3);
            assert!(!status.trim().is_empty());
            assert_ne!(
                status, english_status,
                "icon cache cleared status should be localized for {locale:?}"
            );
        }

        assert_eq!(
            icon_cache_cleared(Locale::Ar, 3),
            "تم مسح ذاكرة الأيقونات المؤقتة (تمت إزالة \u{2066}3\u{2069} ملفات)"
        );
    }

    #[test]
    fn builtin_plugin_market_copy_is_localized_for_every_non_english_locale() {
        let plugin_ids = [
            "com.cryptohud.focus-ticker",
            "com.cryptohud.market-board",
            "com.cryptohud.market-compass",
            "com.cryptohud.trust-card",
            "com.cryptohud.status-strip",
        ];

        for plugin_id in plugin_ids {
            let english_title = builtin_plugin_title(Locale::En, plugin_id)
                .unwrap_or_else(|| panic!("{plugin_id} should have an English title"));
            let english_description = builtin_plugin_description(Locale::En, plugin_id)
                .unwrap_or_else(|| panic!("{plugin_id} should have an English description"));

            for locale in Locale::NON_ENGLISH {
                let title = builtin_plugin_title(locale, plugin_id)
                    .unwrap_or_else(|| panic!("{plugin_id} should have a {locale:?} title"));
                let description = builtin_plugin_description(locale, plugin_id)
                    .unwrap_or_else(|| panic!("{plugin_id} should have a {locale:?} description"));

                assert!(!title.trim().is_empty());
                assert!(!description.trim().is_empty());
                assert_ne!(
                    title, english_title,
                    "{plugin_id} title should be localized for {locale:?}"
                );
                assert_ne!(
                    description, english_description,
                    "{plugin_id} description should be localized for {locale:?}"
                );
            }
        }

        assert!(builtin_plugin_title(Locale::En, "com.example.local").is_none());
        assert!(builtin_plugin_description(Locale::En, "com.example.local").is_none());
    }

    #[test]
    fn plugin_unavailable_id_is_localized_for_every_non_english_locale() {
        let plugin_id = "custom.plugin";
        let english_status = plugin_unavailable_id(Locale::En, plugin_id);

        for locale in Locale::NON_ENGLISH {
            let status = plugin_unavailable_id(locale, plugin_id);
            assert!(!status.trim().is_empty());
            assert!(
                status.contains(plugin_id),
                "plugin unavailable status should preserve the plugin id for {locale:?}"
            );
            assert_ne!(
                status, english_status,
                "plugin unavailable status should be localized for {locale:?}"
            );
        }

        assert_eq!(
            plugin_unavailable_id(Locale::Ar, plugin_id),
            "الإضافة غير متاحة: \u{2066}custom.plugin\u{2069}"
        );
    }

    #[test]
    fn plugin_market_dynamic_descriptions_are_localized_for_every_non_english_locale() {
        let version = semver::Version::new(1, 2, 3);
        let capabilities = ["market.price", "market.candles", "custom.feed"];
        let english_bounds = symbol_bounds_description(Locale::En, 1, 5);
        let english_capabilities = plugin_capabilities_description(Locale::En, &capabilities);
        let english_description = local_slint_plugin_description(
            Locale::En,
            &version,
            320,
            180,
            &english_bounds,
            &english_capabilities,
        );

        for locale in Locale::NON_ENGLISH {
            let bounds = symbol_bounds_description(locale, 1, 5);
            let capability_text = plugin_capabilities_description(locale, &capabilities);
            let description = local_slint_plugin_description(
                locale,
                &version,
                320,
                180,
                &bounds,
                &capability_text,
            );

            assert!(!bounds.trim().is_empty());
            assert!(!capability_text.trim().is_empty());
            assert!(!description.trim().is_empty());
            assert_ne!(
                description, english_description,
                "local Slint plugin description should be localized for {locale:?}"
            );
            assert!(
                capability_text.contains("custom.feed"),
                "unknown capability ids should remain exact technical tokens for {locale:?}"
            );
            assert!(
                !capability_text.contains("market.price"),
                "known capability ids should use localized labels for {locale:?}"
            );
            assert!(
                !capability_text.contains("market.candles"),
                "known capability ids should use localized labels for {locale:?}"
            );
        }

        assert_eq!(
            plugin_capabilities_description(Locale::Ar, &capabilities),
            "الأسعار, الشموع, \u{2066}custom.feed\u{2069}"
        );
        assert_eq!(
            local_slint_plugin_description(
                Locale::Ar,
                &version,
                320,
                180,
                &symbol_bounds_description(Locale::Ar, 1, 5),
                &plugin_capabilities_description(Locale::Ar, &capabilities),
            ),
            "إضافة \u{2066}Slint\u{2069} محلية \u{2066}v1.2.3\u{2069} · \u{2066}320x180\u{2069} · حتى \u{2066}5\u{2069} أزواج · الأسعار, الشموع, \u{2066}custom.feed\u{2069}"
        );
    }

    #[test]
    fn dynamic_short_labels_are_localized_for_every_non_english_locale() {
        let english_labels = [
            default_theme_label(Locale::En),
            plugin_builtin_label(Locale::En),
            plugin_trusted_label(Locale::En),
            provider_mixed_label(Locale::En),
        ];

        for locale in Locale::NON_ENGLISH {
            let localized_labels = [
                default_theme_label(locale),
                plugin_builtin_label(locale),
                plugin_trusted_label(locale),
                provider_mixed_label(locale),
            ];

            for (localized, english) in localized_labels.iter().zip(english_labels.iter()) {
                assert_ne!(
                    localized, english,
                    "dynamic short label should be localized for {locale:?}: {english}"
                );
            }
        }

        assert_eq!(default_theme_label(Locale::Id), "Bawaan");
    }

    #[test]
    fn known_plugin_status_reasons_are_localized_for_every_non_english_locale() {
        let prototype_reason = "prototype widget is disabled";
        let renderer_reason = crate::plugin::SLINT_RENDERER_UNCOMPILED_REASON;
        let compilation_reason = "Slint compilation failed: syntax error at line 1";
        let component_reason =
            "renderer.component PriceCard was not exported; available components: Root, Demo";
        let property_reason = "Slint component is missing required property source-text";
        let callback_reason = "Slint component is missing required callback drag-move";
        let type_reason = "Slint property widget-width has type String, expected Number";
        let english_prototype = plugin_disabled_reason(Locale::En, prototype_reason);
        let english_renderer = plugin_unavailable_reason(Locale::En, renderer_reason);
        let english_dynamic_reasons = [
            plugin_unavailable_reason(Locale::En, compilation_reason),
            plugin_unavailable_reason(Locale::En, component_reason),
            plugin_unavailable_reason(Locale::En, property_reason),
            plugin_unavailable_reason(Locale::En, callback_reason),
            plugin_unavailable_reason(Locale::En, type_reason),
        ];

        for locale in Locale::NON_ENGLISH {
            let prototype = plugin_disabled_reason(locale, prototype_reason);
            let renderer = plugin_unavailable_reason(locale, renderer_reason);
            let localized_dynamic_reasons = [
                plugin_unavailable_reason(locale, compilation_reason),
                plugin_unavailable_reason(locale, component_reason),
                plugin_unavailable_reason(locale, property_reason),
                plugin_unavailable_reason(locale, callback_reason),
                plugin_unavailable_reason(locale, type_reason),
            ];

            assert_ne!(
                prototype, english_prototype,
                "prototype plugin disabled reason should be localized for {locale:?}"
            );
            assert!(
                !prototype.to_ascii_lowercase().contains(prototype_reason),
                "prototype plugin disabled reason should not keep raw English for {locale:?}"
            );
            assert_ne!(
                renderer, english_renderer,
                "uncompiled Slint renderer reason should be localized for {locale:?}"
            );
            assert!(
                !renderer.contains("has not been compiled"),
                "uncompiled Slint renderer reason should not keep raw English for {locale:?}"
            );
            for (localized, english) in localized_dynamic_reasons
                .iter()
                .zip(english_dynamic_reasons.iter())
            {
                assert_ne!(
                    localized, english,
                    "Slint plugin diagnostic should be localized for {locale:?}: {localized}"
                );
                for raw_prefix in [
                    "Slint compilation failed",
                    "was not exported",
                    "missing required property",
                    "missing required callback",
                    "has type",
                ] {
                    assert!(
                        !localized.contains(raw_prefix),
                        "Slint plugin diagnostic should not keep raw prefix {raw_prefix:?} for {locale:?}: {localized}"
                    );
                }
            }
        }

        assert_eq!(
            plugin_disabled_reason(Locale::ZhHans, prototype_reason),
            "插件已禁用：原型小组件已禁用"
        );
        assert_eq!(
            plugin_unavailable_reason(Locale::Ar, renderer_reason),
            "الإضافة غير متاحة: لم يتم تجميع عارض \u{2066}Slint\u{2069} بعد"
        );
        assert_eq!(
            plugin_unavailable_reason(Locale::Ar, property_reason),
            "الإضافة غير متاحة: الخاصية المطلوبة مفقودة: \u{2066}source-text\u{2069}"
        );
        assert_eq!(
            plugin_unavailable_reason(Locale::Ar, component_reason),
            "الإضافة غير متاحة: لم يتم تصدير المكون \u{2066}PriceCard\u{2069}؛ المتاح: \u{2066}Root, Demo\u{2069}"
        );
    }

    #[test]
    fn symbol_picker_control_copy_is_localized_for_every_non_english_locale() {
        let modes = [
            SymbolPickerCopyMode::DefaultAdd,
            SymbolPickerCopyMode::DefaultReplace,
            SymbolPickerCopyMode::WidgetAdd,
            SymbolPickerCopyMode::WidgetReplace,
        ];

        for locale in Locale::NON_ENGLISH {
            assert_ne!(
                symbol_search_placeholder(locale),
                symbol_search_placeholder(Locale::En),
                "symbol search placeholder should be localized for {locale:?}"
            );
            assert_ne!(
                symbol_picker_confirm_text(locale),
                symbol_picker_confirm_text(Locale::En),
                "symbol picker confirm text should be localized for {locale:?}"
            );
            assert_ne!(
                symbol_picker_cancel_text(locale),
                symbol_picker_cancel_text(Locale::En),
                "symbol picker cancel text should be localized for {locale:?}"
            );

            for mode in modes {
                assert_ne!(
                    symbol_picker_title_text(locale, mode),
                    symbol_picker_title_text(Locale::En, mode),
                    "symbol picker title should be localized for {locale:?} {mode:?}"
                );
                assert_ne!(
                    symbol_picker_empty_status_text(locale, mode),
                    symbol_picker_empty_status_text(Locale::En, mode),
                    "symbol picker empty status should be localized for {locale:?} {mode:?}"
                );
            }
        }
    }

    #[test]
    fn key_settings_help_copy_is_localized_for_every_non_english_locale() {
        let english_fields = localized_settings_help_fields(text(Locale::En));
        for locale in Locale::NON_ENGLISH {
            let fields = localized_settings_help_fields(text(locale));
            for ((field_name, localized), (_, english)) in fields.iter().zip(english_fields.iter())
            {
                assert_ne!(
                    localized, english,
                    "{field_name} should be localized for {locale:?}"
                );
            }
        }
    }

    #[test]
    fn primary_static_ui_copy_is_localized_for_every_non_english_locale() {
        let english_fields = primary_static_ui_fields(text(Locale::En));
        for locale in Locale::NON_ENGLISH {
            let fields = primary_static_ui_fields(text(locale));
            for ((field_name, localized), (_, english)) in fields.iter().zip(english_fields.iter())
            {
                if primary_ui_text_matches_english(localized, english) {
                    assert!(
                        is_allowed_identical_primary_ui_field(locale, field_name),
                        "{field_name} should not remain English for {locale:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn non_english_ui_text_does_not_match_english_unless_intentional() {
        let source = std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join("i18n.rs"),
        )
        .unwrap();
        let english_fields = ui_text_constant_fields(&source, "EN");

        for locale in Locale::NON_ENGLISH {
            let constant = locale_ui_text_constant(locale);
            let fields = ui_text_constant_fields(&source, constant);
            for (field_name, localized) in fields {
                let Some(english) = english_fields
                    .iter()
                    .find(|(english_field, _)| english_field == &field_name)
                    .map(|(_, value)| value)
                else {
                    panic!("EN_TEXT should include {field_name}");
                };
                if localized.eq_ignore_ascii_case(english) {
                    assert!(
                        is_allowed_identical_ui_text_field(locale, &field_name),
                        "{constant}_TEXT.{field_name} unexpectedly matches EN_TEXT"
                    );
                }
            }
        }
    }

    #[test]
    fn allowed_identical_primary_ui_fields_are_still_needed() {
        let english_fields = primary_static_ui_fields(text(Locale::En));
        for entry in ALLOWED_IDENTICAL_PRIMARY_UI_FIELDS {
            let fields = primary_static_ui_fields(text(entry.locale));
            let Some((_, localized)) = fields
                .iter()
                .find(|(field_name, _)| field_name == &entry.field_name)
            else {
                panic!("{} should be a primary static UI field", entry.field_name);
            };
            let Some((_, english)) = english_fields
                .iter()
                .find(|(field_name, _)| field_name == &entry.field_name)
            else {
                panic!(
                    "{} should be an English primary static UI field",
                    entry.field_name
                );
            };

            assert!(
                primary_ui_text_matches_english(localized, english),
                "{} allowlist entry is stale for {:?}: {}",
                entry.field_name,
                entry.locale,
                entry.reason
            );
            assert!(
                !entry.reason.trim().is_empty(),
                "{} allowlist entry should explain why identical text is acceptable",
                entry.field_name
            );
        }
    }

    #[test]
    fn allowed_identical_technical_ui_fields_are_still_needed() {
        let source = std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join("i18n.rs"),
        )
        .unwrap();
        let english_fields = ui_text_constant_fields(&source, "EN");

        for entry in ALLOWED_IDENTICAL_TECHNICAL_UI_FIELDS {
            let Some(english) = english_fields
                .iter()
                .find(|(field_name, _)| field_name == entry.field_name)
                .map(|(_, value)| value)
            else {
                panic!("EN_TEXT should include {}", entry.field_name);
            };
            let still_needed = Locale::NON_ENGLISH.into_iter().any(|locale| {
                let constant = locale_ui_text_constant(locale);
                ui_text_constant_fields(&source, constant).into_iter().any(
                    |(field_name, localized)| {
                        field_name == entry.field_name && localized.eq_ignore_ascii_case(english)
                    },
                )
            });

            assert!(
                still_needed,
                "{} technical allowlist entry is stale: {}",
                entry.field_name, entry.reason
            );
            assert!(
                !entry.reason.trim().is_empty(),
                "{} technical allowlist entry should explain why identical text is acceptable",
                entry.field_name
            );
        }
    }

    #[test]
    fn non_english_ui_text_constants_explicitly_maintain_every_field() {
        let source = std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join("i18n.rs"),
        )
        .unwrap();
        let fields = source
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                line.strip_prefix("pub ")
                    .and_then(|line| line.split_once(": &'static str,"))
                    .map(|(field, _)| field.to_string())
            })
            .collect::<Vec<_>>();

        for locale in Locale::NON_ENGLISH {
            let constant = locale_ui_text_constant(locale);
            let body = ui_text_constant_body(&source, constant);
            for field in &fields {
                assert!(
                    body.lines()
                        .any(|line| line.trim_start().starts_with(&format!("{field}:"))),
                    "{constant}_TEXT should explicitly set {field}"
                );
            }
        }
    }

    #[test]
    fn arabic_ui_text_constant_keeps_ltr_literals_isolated() {
        let source = std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join("i18n.rs"),
        )
        .unwrap();
        let body = ui_text_constant_body(&source, "AR");
        let high_risk_literals = [
            "Crypto HUD",
            "Alt+C",
            "BTC",
            "BTC/USDC",
            "BTCUSDT",
            "http://",
            "socks5://",
            "127.0.0.1",
        ];

        for line in body.lines().map(str::trim) {
            let Some(value) = ui_text_field_literal_source(line) else {
                continue;
            };
            let has_high_risk_literal = high_risk_literals
                .iter()
                .any(|literal| value.contains(literal));
            let has_ascii_digit = value.chars().any(|character| character.is_ascii_digit());
            if has_high_risk_literal || has_ascii_digit {
                assert!(
                    value.contains("\\u{2066}"),
                    "AR_TEXT literal should isolate LTR fragment: {line}"
                );
            }
        }
    }

    fn ui_text_field_literal_source(line: &str) -> Option<&str> {
        let (_, value) = line.split_once(": \"")?;
        let end = value.rfind('"')?;
        Some(&value[..end])
    }

    fn standalone_ui_text_literal_source(line: &str) -> Option<&str> {
        let value = line.strip_prefix('"')?;
        let end = value.rfind('"')?;
        Some(&value[..end])
    }

    fn ui_text_constant_fields(source: &str, constant: &str) -> Vec<(String, String)> {
        let body = ui_text_constant_body(source, constant);
        let mut fields = Vec::new();
        let mut lines = body.lines().peekable();
        while let Some(line) = lines.next() {
            let line = line.trim();
            if let Some((field_name, _)) = line.split_once(": \"") {
                if let Some(value) = ui_text_field_literal_source(line) {
                    fields.push((field_name.to_string(), value.to_string()));
                }
                continue;
            }

            let Some(field_name) = line.strip_suffix(':') else {
                continue;
            };
            let Some(value_line) = lines.next().map(str::trim) else {
                continue;
            };
            if let Some(value) = standalone_ui_text_literal_source(value_line) {
                fields.push((field_name.to_string(), value.to_string()));
            }
        }
        fields
    }

    fn read_workspace_file(path: &str) -> String {
        std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join(path),
        )
        .unwrap_or_else(|error| panic!("failed to read {path}: {error}"))
    }

    fn documented_locale_rows(markdown: &str) -> Vec<(String, String)> {
        markdown
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if !line.starts_with("| `") {
                    return None;
                }
                let columns = line.split('|').map(str::trim).collect::<Vec<_>>();
                let locale = markdown_code_cell(columns.get(1)?)?;
                let preference = markdown_code_cell(columns.get(2)?)?;
                Some((locale.to_string(), preference.to_string()))
            })
            .collect()
    }

    fn markdown_code_cell(cell: &str) -> Option<&str> {
        cell.strip_prefix('`')?.strip_suffix('`')
    }

    fn documented_readme_locale_tags(markdown: &str, marker: &str) -> Vec<String> {
        let start = markdown
            .find(marker)
            .unwrap_or_else(|| panic!("README should contain locale list marker {marker:?}"));
        let paragraph = markdown[start + marker.len()..]
            .lines()
            .take_while(|line| !line.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        markdown_code_spans(&paragraph)
    }

    fn markdown_code_spans(markdown: &str) -> Vec<String> {
        let mut spans = Vec::new();
        let mut rest = markdown;
        while let Some(start) = rest.find('`') {
            rest = &rest[start + 1..];
            let Some(end) = rest.find('`') else {
                break;
            };
            spans.push(rest[..end].to_string());
            rest = &rest[end + 1..];
        }
        spans
    }

    fn ui_text_constant_body<'a>(source: &'a str, constant: &str) -> &'a str {
        let start_marker = format!("const {constant}_TEXT: UiText = UiText {{");
        let start = source.find(&start_marker).unwrap_or_else(|| {
            panic!("{constant}_TEXT should exist");
        }) + start_marker.len();
        let rest = &source[start..];
        let end = rest.find("\n};").unwrap_or_else(|| {
            panic!("{constant}_TEXT should end with }};");
        });
        &rest[..end]
    }

    fn locale_ui_text_constant(locale: Locale) -> &'static str {
        match locale {
            Locale::En => "EN",
            Locale::ZhHans => "ZH_HANS",
            Locale::ZhHant => "ZH_HANT",
            Locale::Es419 => "ES_419",
            Locale::PtBr => "PT_BR",
            Locale::Vi => "VI",
            Locale::Id => "ID",
            Locale::Tr => "TR",
            Locale::Ko => "KO",
            Locale::Ja => "JA",
            Locale::Ru => "RU",
            Locale::Ar => "AR",
        }
    }

    fn locale_documentation_tag(locale: Locale) -> &'static str {
        match locale {
            Locale::En => "en",
            Locale::ZhHans => "zh-CN",
            Locale::ZhHant => "zh-TW",
            Locale::Es419 => "es-419",
            Locale::PtBr => "pt-BR",
            Locale::Vi => "vi",
            Locale::Id => "id",
            Locale::Tr => "tr",
            Locale::Ko => "ko",
            Locale::Ja => "ja",
            Locale::Ru => "ru",
            Locale::Ar => "ar",
        }
    }

    fn language_preference_for_locale(locale: Locale) -> LanguagePreference {
        match locale {
            Locale::En => LanguagePreference::En,
            Locale::ZhHans => LanguagePreference::ZhHans,
            Locale::ZhHant => LanguagePreference::ZhHant,
            Locale::Es419 => LanguagePreference::Es419,
            Locale::PtBr => LanguagePreference::PtBr,
            Locale::Vi => LanguagePreference::Vi,
            Locale::Id => LanguagePreference::Id,
            Locale::Tr => LanguagePreference::Tr,
            Locale::Ko => LanguagePreference::Ko,
            Locale::Ja => LanguagePreference::Ja,
            Locale::Ru => LanguagePreference::Ru,
            Locale::Ar => LanguagePreference::Ar,
        }
    }

    fn localized_settings_help_fields(text: &'static UiText) -> Vec<(&'static str, &'static str)> {
        vec![
            ("market_provider_help", text.market_provider_help),
            ("refresh_interval_help", text.refresh_interval_help),
            ("symbols_help", text.symbols_help),
            ("network_proxy_help", text.network_proxy_help),
            ("theme_help", text.theme_help),
            ("language_help", text.language_help),
            ("red_up_color_help", text.red_up_color_help),
            ("default_opacity_help", text.default_opacity_help),
            (
                "default_always_on_top_help",
                text.default_always_on_top_help,
            ),
            ("auto_start_help", text.auto_start_help),
            (
                "show_main_window_on_startup_help",
                text.show_main_window_on_startup_help,
            ),
            ("shortcut_help", text.shortcut_help),
            ("tray_icon_help", text.tray_icon_help),
            ("tray_hover_display_help", text.tray_hover_display_help),
            (
                "selected_widget_description",
                text.selected_widget_description,
            ),
            ("lock_position_help", text.lock_position_help),
            ("widget_scale_help", text.widget_scale_help),
            ("opacity_help", text.opacity_help),
            (
                "widget_show_coin_logos_help",
                text.widget_show_coin_logos_help,
            ),
            (
                "widget_hide_quote_asset_help",
                text.widget_hide_quote_asset_help,
            ),
            ("widget_theme_help", text.widget_theme_help),
            ("widget_topmost_help", text.widget_topmost_help),
            ("plugin_market_description", text.plugin_market_description),
            ("my_widgets_description", text.my_widgets_description),
            (
                "market_settings_description",
                text.market_settings_description,
            ),
            (
                "appearance_settings_description",
                text.appearance_settings_description,
            ),
            (
                "system_settings_description",
                text.system_settings_description,
            ),
            ("quote_board_description", text.quote_board_description),
            ("mini_ticker_description", text.mini_ticker_description),
        ]
    }

    fn primary_static_ui_fields(text: &'static UiText) -> Vec<(&'static str, &'static str)> {
        vec![
            ("tray_settings", text.tray_settings),
            ("tray_show_widgets", text.tray_show_widgets),
            ("tray_quit", text.tray_quit),
            ("tab_widgets", text.tab_widgets),
            ("tab_plugin_market", text.tab_plugin_market),
            ("tab_market_data", text.tab_market_data),
            ("tab_appearance", text.tab_appearance),
            ("tab_system", text.tab_system),
            ("always_on_top", text.always_on_top),
            ("default_always_on_top", text.default_always_on_top),
            ("opacity", text.opacity),
            ("default_opacity", text.default_opacity),
            ("widget_scale", text.widget_scale),
            ("red_up_color", text.red_up_color),
            ("market_provider", text.market_provider),
            ("refresh_interval", text.refresh_interval),
            ("default_symbols", text.default_symbols),
            ("alert_settings", text.alert_settings),
            ("alert_enabled", text.alert_enabled),
            ("alert_symbol", text.alert_symbol),
            ("alert_condition", text.alert_condition),
            ("alert_threshold", text.alert_threshold),
            ("alert_clear", text.alert_clear),
            ("symbols", text.symbols),
            ("empty_pairs", text.empty_pairs),
            ("auto_start", text.auto_start),
            (
                "show_main_window_on_startup",
                text.show_main_window_on_startup,
            ),
            ("shortcut", text.shortcut),
            ("tray_icon", text.tray_icon),
            ("tray_hover_display", text.tray_hover_display),
            ("network_proxy_settings", text.network_proxy_settings),
            ("network_proxy_enabled", text.network_proxy_enabled),
            ("network_proxy_url", text.network_proxy_url),
            ("app_version", text.app_version),
            ("about_us", text.about_us),
            ("icon_cache", text.icon_cache),
            ("clear_icon_cache", text.clear_icon_cache),
            ("custom_components", text.custom_components),
            (
                "open_custom_components_folder",
                text.open_custom_components_folder,
            ),
            ("theme", text.theme),
            ("language", text.language),
            ("appearance_interface", text.appearance_interface),
            (
                "appearance_widget_defaults",
                text.appearance_widget_defaults,
            ),
            ("system_startup", text.system_startup),
            ("system_tray", text.system_tray),
            ("system_app_info", text.system_app_info),
            ("system_maintenance", text.system_maintenance),
            ("apply", text.apply),
            ("widget_library", text.widget_library),
            ("my_widgets", text.my_widgets),
            ("selected_widget", text.selected_widget),
            ("widget_name", text.widget_name),
            ("advanced_options", text.advanced_options),
            ("reset_widget_positions", text.reset_widget_positions),
            ("hide_all_widgets", text.hide_all_widgets),
            ("reset", text.reset),
            ("preview", text.preview),
            ("preview_pairs", text.preview_pairs),
            ("preview_source_ok", text.preview_source_ok),
            ("app_settings", text.app_settings),
            ("add_widget", text.add_widget),
            ("apply_widget", text.apply_widget),
            ("no_widgets", text.no_widgets),
            ("quote_board_title", text.quote_board_title),
            ("mini_ticker_title", text.mini_ticker_title),
            ("market_settings", text.market_settings),
            ("appearance_settings", text.appearance_settings),
            ("system_settings", text.system_settings),
            ("settings_path_label", text.settings_path_label),
            ("close", text.close),
            ("source_prefix", text.source_prefix),
            ("runtime_no_pairs", text.runtime_no_pairs),
            ("runtime_connecting", text.runtime_connecting),
            ("runtime_connection_error", text.runtime_connection_error),
            ("runtime_updated", text.runtime_updated),
            ("runtime_stale", text.runtime_stale),
            ("runtime_source_error", text.runtime_source_error),
            ("widget_visible", text.widget_visible),
            ("widget_hidden", text.widget_hidden),
            ("delete_widget", text.delete_widget),
            ("status_alert_invalid", text.status_alert_invalid),
            ("status_auto_start_failed", text.status_auto_start_failed),
            ("status_shortcut_failed", text.status_shortcut_failed),
            (
                "status_network_proxy_invalid",
                text.status_network_proxy_invalid,
            ),
            (
                "status_icon_cache_clear_failed",
                text.status_icon_cache_clear_failed,
            ),
            (
                "status_custom_components_folder_open_failed",
                text.status_custom_components_folder_open_failed,
            ),
            (
                "status_symbol_catalog_fallback",
                text.status_symbol_catalog_fallback,
            ),
        ]
    }

    fn is_allowed_identical_primary_ui_field(locale: Locale, field_name: &str) -> bool {
        ALLOWED_IDENTICAL_PRIMARY_UI_FIELDS
            .iter()
            .any(|entry| entry.locale == locale && entry.field_name == field_name)
    }

    fn is_allowed_identical_ui_text_field(locale: Locale, field_name: &str) -> bool {
        is_allowed_identical_primary_ui_field(locale, field_name)
            || ALLOWED_IDENTICAL_TECHNICAL_UI_FIELDS
                .iter()
                .any(|entry| entry.field_name == field_name)
    }

    fn primary_ui_text_matches_english(localized: &str, english: &str) -> bool {
        localized.eq_ignore_ascii_case(english)
    }

    struct AllowedIdenticalPrimaryUiField {
        locale: Locale,
        field_name: &'static str,
        reason: &'static str,
    }

    struct AllowedIdenticalTechnicalUiField {
        field_name: &'static str,
        reason: &'static str,
    }

    const ALLOWED_IDENTICAL_PRIMARY_UI_FIELDS: &[AllowedIdenticalPrimaryUiField] = &[
        AllowedIdenticalPrimaryUiField {
            locale: Locale::Es419,
            field_name: "tab_widgets",
            reason: "Widget is a common Latin American Spanish UI borrowing.",
        },
        AllowedIdenticalPrimaryUiField {
            locale: Locale::PtBr,
            field_name: "tab_widgets",
            reason: "Widget is a common Brazilian Portuguese UI borrowing.",
        },
        AllowedIdenticalPrimaryUiField {
            locale: Locale::PtBr,
            field_name: "appearance_interface",
            reason: "Interface is spelled the same in Brazilian Portuguese.",
        },
        AllowedIdenticalPrimaryUiField {
            locale: Locale::Id,
            field_name: "alert_symbol",
            reason: "Pair is used consistently as a trading-product borrowing in Indonesian.",
        },
    ];

    const ALLOWED_IDENTICAL_TECHNICAL_UI_FIELDS: &[AllowedIdenticalTechnicalUiField] = &[
        AllowedIdenticalTechnicalUiField {
            field_name: "tray_tooltip",
            reason: "The product name stays unchanged across locales.",
        },
        AllowedIdenticalTechnicalUiField {
            field_name: "settings_title",
            reason: "The product name stays unchanged across locales.",
        },
        AllowedIdenticalTechnicalUiField {
            field_name: "runtime_live_count_prefix",
            reason: "Some languages intentionally use an empty prefix before the live count.",
        },
    ];

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
            update_available_notification_body(
                Locale::En,
                "v1.2.3",
                Some("crypto-hud.exe"),
                Some("checksums.txt")
            ),
            "v1.2.3 is available. Download crypto-hud.exe and verify it with checksums.txt from GitHub Releases."
        );
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
            update_available_notification_body(
                Locale::ZhHans,
                "v1.2.3",
                Some("crypto-hud.exe"),
                Some("checksums.txt")
            ),
            "已发布 v1.2.3。请在 GitHub Releases 下载 crypto-hud.exe，并使用 checksums.txt 校验。"
        );
        assert_eq!(shortcut_options(Locale::ZhHans), vec!["Alt+C", "禁用"]);
        assert_eq!(
            alert_condition_options(Locale::ZhHans),
            vec!["价格高于", "价格低于", "24h 涨跌高于", "24h 涨跌低于"]
        );
    }

    #[test]
    fn update_notification_body_is_localized_for_every_non_english_locale() {
        let english_with_checksum = update_available_notification_body(
            Locale::En,
            "v1.2.3",
            Some("crypto-hud.exe"),
            Some("checksums.txt"),
        );
        let english_with_asset =
            update_available_notification_body(Locale::En, "v1.2.3", Some("crypto-hud.exe"), None);
        let english_release_only =
            update_available_notification_body(Locale::En, "v1.2.3", None, None);

        for locale in Locale::NON_ENGLISH {
            let with_checksum = update_available_notification_body(
                locale,
                "v1.2.3",
                Some("crypto-hud.exe"),
                Some("checksums.txt"),
            );
            assert_ne!(with_checksum, english_with_checksum);
            assert!(with_checksum.contains("v1.2.3"));
            assert!(with_checksum.contains("crypto-hud.exe"));
            assert!(with_checksum.contains("checksums.txt"));

            let with_asset =
                update_available_notification_body(locale, "v1.2.3", Some("crypto-hud.exe"), None);
            assert_ne!(with_asset, english_with_asset);
            assert!(with_asset.contains("v1.2.3"));
            assert!(with_asset.contains("crypto-hud.exe"));

            let release_only = update_available_notification_body(locale, "v1.2.3", None, None);
            assert_ne!(release_only, english_release_only);
            assert!(release_only.contains("v1.2.3"));
        }
    }

    #[test]
    fn arabic_update_notification_isolates_ltr_release_values() {
        let with_checksum = update_available_notification_body(
            Locale::Ar,
            "v1.2.3",
            Some("crypto-hud.exe"),
            Some("checksums.txt"),
        );
        let with_asset =
            update_available_notification_body(Locale::Ar, "v1.2.3", Some("crypto-hud.exe"), None);
        let release_only = update_available_notification_body(Locale::Ar, "v1.2.3", None, None);

        assert_eq!(
            with_checksum,
            "\u{2066}v1.2.3\u{2069} متاح. نزّل \u{2066}crypto-hud.exe\u{2069} من \u{2066}GitHub Releases\u{2069} وتحقق منه باستخدام \u{2066}checksums.txt\u{2069}."
        );
        assert!(with_asset.contains("\u{2066}v1.2.3\u{2069}"));
        assert!(with_asset.contains("\u{2066}crypto-hud.exe\u{2069}"));
        assert!(with_asset.contains("\u{2066}GitHub Releases\u{2069}"));
        assert!(release_only.contains("\u{2066}v1.2.3\u{2069}"));
        assert!(release_only.contains("\u{2066}GitHub Releases\u{2069}"));
        assert_eq!(
            update_available_notification_body(
                Locale::En,
                "v1.2.3",
                Some("crypto-hud.exe"),
                Some("checksums.txt"),
            ),
            "v1.2.3 is available. Download crypto-hud.exe and verify it with checksums.txt from GitHub Releases."
        );
    }

    #[test]
    fn alert_notification_copy_is_localized_for_every_non_english_locale() {
        let english = [
            alert_notification_body(
                Locale::En,
                "binance:spot:BTC/USDT",
                AlertCondition::PriceAbove,
                100000.0,
                101250.5,
            ),
            alert_notification_body(
                Locale::En,
                "binance:spot:BTC/USDT",
                AlertCondition::PriceBelow,
                100000.0,
                99000.0,
            ),
            alert_notification_body(
                Locale::En,
                "binance:spot:BTC/USDT",
                AlertCondition::ChangePercentAbove,
                5.0,
                6.25,
            ),
            alert_notification_body(
                Locale::En,
                "binance:spot:BTC/USDT",
                AlertCondition::ChangePercentBelow,
                -5.0,
                -6.25,
            ),
        ];

        for locale in Locale::NON_ENGLISH {
            let title = alert_notification_title(locale, "binance:spot:BTC/USDT");
            assert_ne!(
                title,
                alert_notification_title(Locale::En, "binance:spot:BTC/USDT")
            );
            assert!(title.contains("BTC/USDT"));

            for (index, condition) in [
                AlertCondition::PriceAbove,
                AlertCondition::PriceBelow,
                AlertCondition::ChangePercentAbove,
                AlertCondition::ChangePercentBelow,
            ]
            .into_iter()
            .enumerate()
            {
                let (threshold, current_value) = match condition {
                    AlertCondition::PriceAbove => (100000.0, 101250.5),
                    AlertCondition::PriceBelow => (100000.0, 99000.0),
                    AlertCondition::ChangePercentAbove => (5.0, 6.25),
                    AlertCondition::ChangePercentBelow => (-5.0, -6.25),
                };
                let body = alert_notification_body(
                    locale,
                    "binance:spot:BTC/USDT",
                    condition,
                    threshold,
                    current_value,
                );
                assert_ne!(body, english[index]);
                assert!(body.contains("BTC/USDT"));
            }
        }
    }

    #[test]
    fn arabic_alert_notification_isolates_ltr_market_values() {
        let title = alert_notification_title(Locale::Ar, "binance:spot:BTC/USDT");
        let body = alert_notification_body(
            Locale::Ar,
            "binance:spot:BTC/USDT",
            AlertCondition::ChangePercentAbove,
            5.0,
            6.25,
        );

        assert_eq!(title, "تنبيه \u{2066}BTC/USDT\u{2069}");
        assert!(body.contains("\u{2066}BTC/USDT\u{2069}"));
        assert!(body.contains("\u{2066}+5.00%\u{2069}"));
        assert!(body.contains("\u{2066}+6.25%\u{2069}"));
        assert_eq!(
            alert_notification_title(Locale::En, "binance:spot:BTC/USDT"),
            "BTC/USDT alert"
        );
        assert!(!alert_notification_body(
            Locale::En,
            "binance:spot:BTC/USDT",
            AlertCondition::ChangePercentAbove,
            5.0,
            6.25,
        )
        .contains('\u{2066}'));
    }

    #[test]
    fn alert_24h_terms_are_locale_appropriate() {
        assert_eq!(
            alert_condition_options(Locale::Tr),
            vec![
                "Fiyat üzerinde",
                "Fiyat altında",
                "24 sa değişim üzerinde",
                "24 sa değişim altında"
            ]
        );
        assert_eq!(
            alert_notification_body(
                Locale::Tr,
                "binance:spot:BTC/USDT",
                AlertCondition::ChangePercentAbove,
                5.0,
                6.25,
            ),
            "BTC/USDT 24 saatlik değişimi +5.00% üzerinde: +6.25%"
        );
        assert_eq!(
            alert_condition_options(Locale::Ar),
            vec![
                "السعر أعلى من",
                "السعر أقل من",
                "تغير \u{2066}24\u{2069} ساعة أعلى من",
                "تغير \u{2066}24\u{2069} ساعة أقل من"
            ]
        );
        assert_eq!(
            alert_notification_body(
                Locale::Ar,
                "binance:spot:BTC/USDT",
                AlertCondition::ChangePercentAbove,
                5.0,
                6.25,
            ),
            "تغير \u{2066}BTC/USDT\u{2069} خلال \u{2066}24\u{2069} ساعة أعلى من \u{2066}+5.00%\u{2069}: \u{2066}+6.25%\u{2069}"
        );
    }

    #[test]
    fn dynamic_plugin_and_symbol_copy_follow_locale() {
        assert_eq!(
            plugin_unavailable_id(Locale::Es419, "custom.plugin"),
            "Plugin no disponible: custom.plugin"
        );
        assert_eq!(plugin_builtin_label(Locale::Ja), "組み込み");
        assert_eq!(
            plugin_disabled_reason(Locale::Ar, "policy"),
            "الإضافة معطلة: \u{2066}policy\u{2069}"
        );
        assert_eq!(
            plugin_unavailable_id(Locale::Ar, "custom.plugin"),
            "الإضافة غير متاحة: \u{2066}custom.plugin\u{2069}"
        );
        assert_eq!(
            plugin_unavailable_reason(Locale::Ar, "missing manifest.json"),
            "الإضافة غير متاحة: \u{2066}missing manifest.json\u{2069}"
        );
        assert_eq!(provider_mixed_label(Locale::Ru), "Смешано");
        assert_eq!(default_theme_label(Locale::PtBr), "Padrão");
        assert_eq!(default_theme_label(Locale::Id), "Bawaan");
        assert_eq!(symbol_bounds_description(Locale::Ko, 1, 5), "최대 5개 페어");
        assert_eq!(
            plugin_capabilities_description(
                Locale::Ar,
                &["market.price", "market.candles", "custom.feed"]
            ),
            "الأسعار, الشموع, \u{2066}custom.feed\u{2069}"
        );
        assert_eq!(widget_usage_text(Locale::Ar, 2), "مستخدم \u{2066}2\u{2069}");
        assert_eq!(
            default_widget_name(Locale::Ar, WidgetText::QuoteBoard, 3),
            "لوحة الأسعار \u{2066}3\u{2069}"
        );

        let version = semver::Version::new(1, 2, 3);
        assert_eq!(
            local_slint_plugin_description(
                Locale::Vi,
                &version,
                320,
                180,
                "tối đa 5 cặp",
                "ticker"
            ),
            "Plugin Slint cục bộ v1.2.3 · 320x180 · tối đa 5 cặp · ticker"
        );
        assert_eq!(
            local_slint_plugin_description(Locale::En, &version, 320, 180, "up to 5 pairs", ""),
            "Local Slint plugin v1.2.3 · 320x180 · up to 5 pairs"
        );
        let arabic_symbol_bounds = symbol_bounds_description(Locale::Ar, 1, 5);
        let arabic_capabilities =
            plugin_capabilities_description(Locale::Ar, &["market.price", "market.candles"]);
        assert_eq!(
            local_slint_plugin_description(
                Locale::Ar,
                &version,
                320,
                180,
                &arabic_symbol_bounds,
                &arabic_capabilities
            ),
            "إضافة \u{2066}Slint\u{2069} محلية \u{2066}v1.2.3\u{2069} · \u{2066}320x180\u{2069} · حتى \u{2066}5\u{2069} أزواج · الأسعار, الشموع"
        );
        assert_eq!(
            local_slint_plugin_description(Locale::Ar, &version, 320, 180, &arabic_symbol_bounds, " "),
            "إضافة \u{2066}Slint\u{2069} محلية \u{2066}v1.2.3\u{2069} · \u{2066}320x180\u{2069} · حتى \u{2066}5\u{2069} أزواج"
        );

        assert_eq!(
            symbol_search_placeholder(Locale::Ru),
            "Поиск пары, названия или BTCUSDT"
        );
        assert_eq!(
            symbol_picker_title_text(Locale::Tr, SymbolPickerCopyMode::WidgetReplace),
            "Widget çiftini değiştir"
        );
        assert_eq!(
            symbol_picker_empty_status_text(Locale::ZhHant, SymbolPickerCopyMode::DefaultAdd),
            "沒有可新增的新小工具預設交易對"
        );
        assert_eq!(
            symbol_picker_status_text(
                Locale::Es419,
                SymbolPickerCopyMode::DefaultAdd,
                5,
                5,
                12,
                "",
                false
            ),
            "Seleccionado 5/5. Límite alcanzado; elimina un par"
        );
        assert_eq!(
            default_symbol_status_text(Locale::Ar, 2, 5, 0, "btc"),
            "تم اختيار \u{2066}2/5\u{2069}. لا توجد أزواج مطابقة"
        );
        assert_eq!(
            symbol_picker_status_text(
                Locale::Ar,
                SymbolPickerCopyMode::DefaultAdd,
                2,
                5,
                12,
                "",
                false
            ),
            "تم العثور على \u{2066}12\u{2069} أزواج. يؤثر فقط في الأدوات الجديدة"
        );
        assert_eq!(
            symbol_bounds_description(Locale::Ar, 2, 5),
            "\u{2066}2-5\u{2069} أزواج"
        );
        assert_eq!(
            symbols_help_text(Locale::Ar, 5, 5),
            "اختر \u{2066}5\u{2069} أزواج بالضبط"
        );
        assert_eq!(
            icon_cache_cleared(Locale::Ar, 3),
            "تم مسح ذاكرة الأيقونات المؤقتة (تمت إزالة \u{2066}3\u{2069} ملفات)"
        );
        assert_eq!(
            status_failure_message(
                Locale::En,
                text(Locale::En).status_network_proxy_invalid,
                "bad scheme"
            ),
            "Network proxy is invalid: bad scheme"
        );
        assert_eq!(
            status_failure_message(
                Locale::ZhHans,
                text(Locale::ZhHans).status_network_proxy_invalid,
                "bad scheme"
            ),
            "网络代理无效：bad scheme"
        );
        assert_eq!(
            status_failure_message(
                Locale::Ar,
                text(Locale::Ar).status_shortcut_failed,
                "denied"
            ),
            "فشل تسجيل الاختصار: \u{2066}denied\u{2069}"
        );
    }
}
