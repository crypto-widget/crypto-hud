# 插件开发规范

本目录存放 Crypto HUD 的仓库内置插件。每个插件都是一个独立目录，包含
`widget.json` 清单和一个 Slint 入口组件。

## 目录结构

每个插件使用以下结构：

```text
com.example.my-widget/
  widget.json
  ui/
    main.slint
    optional-asset.png
```

插件 id 必须稳定，使用小写 ASCII、数字、点号或连字符，例如
`com.cryptohud.focus-ticker`。

## 清单文件规范

清单文件必须命名为 `widget.json`。

```json
{
  "schemaVersion": 3,
  "id": "com.example.my-widget",
  "name": "My Widget",
  "version": "1.0.0",
  "hostApiVersion": ">=0.2.0, <1.0.0",
  "renderer": {
    "kind": "slint",
    "entry": "ui/main.slint",
    "component": "MyWidget"
  },
  "permissions": {
    "network": false,
    "filesystem": false
  },
  "defaultSize": {
    "width": 300,
    "height": 180
  },
  "sizePolicy": {
    "kind": "fixed"
  },
  "minSymbolLimit": 1,
  "symbolLimit": 1,
  "defaultSymbols": [
    "binance:spot:BTC/USDT"
  ],
  "dataRequirements": [
    { "capability": "market.price" }
  ]
}
```

宿主会强制校验：

- `schemaVersion` 必须为 `3`。
- `name` 是作者提供的展示名，Crypto HUD 会按原样显示，不做本地化。
- `version` 必须是合法 SemVer。
- `hostApiVersion` 必须匹配当前宿主 API `0.2.0`。只使用整数参数的旧插件可继续声明
  `>=0.1.0, <1.0.0`；使用扩展参数类型的插件必须包含 `0.2.0` 且排除 `0.1.x`，建议写成
  `>=0.2.0, <1.0.0`。
- `renderer.kind` 必须为 `slint`。
- `renderer.entry` 只能包含普通相对路径组件；绝对路径、根路径、盘符相对路径以及
  `.`、`..` 组件都会被拒绝。
- `permissions.network` 和 `permissions.filesystem` 当前都必须是 `false`。
- `defaultSize` 必须在 `120x80` 到 `1200x900` 之间。
- `sizePolicy` 可省略，默认 `{ "kind": "fixed" }`。
- `minSymbolLimit` 可省略，默认 `1`，必须在 `1` 到 `symbolLimit` 之间。
- `symbolLimit` 是最大币种数量，必须在 `1` 到 `8` 之间。
- `defaultSymbols` 可省略；填写时必须是有效交易对，数量不能超过 `symbolLimit`，
  也不能少于 `minSymbolLimit`。未写数据源的输入会按 Binance 现货和 USDT
  默认报价规范化，例如 `BTC` 等价于 `binance:spot:BTC/USDT`。
- `dataRequirements` 当前只支持 `market.price` 和 `market.candles`。
  `market.candles` 是显式启用能力：只有至少一个已配置且可用的组件为该交易对声明此能力时，
  宿主才会请求 K 线。共享同一交易对的多个组件会合并成一条行情订阅。
- 允许的文件扩展名为 `json`、`slint`、`png`、`jpg`、`jpeg`、`svg`。
- `previewImages` 与 `renderer.entry` 使用相同的严格相对路径规则，且必须解析到插件
  目录内部。
- 所有 `.slint` import 和 `@image-url()` 资源都必须解析到插件目录内部。图片资源仅支持
  `png`、`jpg`、`jpeg`、`svg`，单个文件最大 1 MiB。除 `.slint` 外的文件 import 会被
  拒绝；当前不支持自定义字体或其他外部文件系统资源。
- SVG（包括 SVG data URL）必须是自包含的 UTF-8 XML，不能包含 DTD。图片和滤镜图片
  引用必须内嵌 PNG 或 JPEG 数据；外部路径、URL 和嵌套 SVG 图片会被拒绝。
  `<use href="#shape">` 等内部矢量引用仍受支持。内联图片 data URL 遵循相同类型和大小限制。

内容尺寸随币种数量变化的插件建议声明 `symbolGrid`：

```json
"sizePolicy": {
  "kind": "symbolGrid",
  "cellSize": {
    "width": 136,
    "height": 84
  },
  "contentPadding": {
    "width": 8,
    "height": 8
  },
  "columns": 5
}
```

此时自然宽度为 `cellSize.width * min(当前币种数, columns) + contentPadding.width`，
自然高度为 `cellSize.height * ceil(当前币种数 / columns) + contentPadding.height`。
也可以用 `rows` 声明固定行数，由宿主自动计算列数。`defaultSize` 必须等于
`symbolLimit` 个币种时的自然尺寸。

旧版横向列表插件仍可声明 `symbolBlocks`：

```json
"sizePolicy": {
  "kind": "symbolBlocks",
  "blockSize": {
    "width": 136,
    "height": 84
  },
  "padding": {
    "width": 8,
    "height": 8
  }
}
```

此时自然宽度为 `blockSize.width * 当前币种数 + padding.width`，自然高度为
`blockSize.height + padding.height`。`defaultSize` 必须等于 `symbolLimit` 个币种
时的自然尺寸。

## Slint 组件合同

`renderer.component` 指向的组件必须继承 `Window`，并暴露以下必需属性：

```slint
in property <string> widget-id;
in property <[QuoteRow]> quote-rows;
in property <string> pairs-heading-text;
in property <string> source-text;
in property <string> updated-text;
in property <string> empty-text;
in property <bool> pin-to-top;
in property <bool> layout-locked;
in property <int> widget-width;
in property <int> widget-height;
in property <string> theme-name;
in property <bool> red-up-enabled;
in property <int> content-opacity;
```

仓库内置插件还应暴露以下宿主兼容属性。宿主会在属性存在时自动下发对应值，
用来保持插件市场预览、桌面小组件、多语言和 RTL 布局一致：

```slint
in property <string> source-name-text;
in property <string> layout-lock-text;
in property <string> change-period-text;
in property <[string]> quote-base-labels;
in property <[string]> quote-quote-labels;
in property <[bool]> quote-price-ready;
in property <bool> market-status-warning: false;
in property <[string]> quote-compact-prices;
in property <[string]> quote-full-prices;
in property <bool> rtl-layout: false;
in property <float> widget-scale: 1.0;
in property <[image]> quote-icons;
in property <string> chart-line-path;
in property <string> chart-fill-path;
in property <bool> chart-ready;
in property <bool> chart-positive;
in property <[float]> quote-chart-end-y-ratios;
```

`layout-lock-text` 是宿主提供的本地化“锁定位置”标签，属于可选属性，旧插件仍可继续运行。
锁定控件应提供可勾选的无障碍角色、`layout-locked` 状态、可见焦点，以及空格/回车操作。

`quote-chart-end-y-ratios` 与 `quote-rows` 顺序一致，表示每个币种最后一个图表点的纵坐标，
单位为绘图区高度的千分之一（顶部 0，底部 1000），没有图表时为 500。
它是可选输入，轮播组件可用它放置所选币种的尾点，旧插件保持兼容。

`change-period-text` 是本地化的 24 小时涨跌周期标签。上述 `quote-*` 数组
与 `quote-rows` 一一对应：基础币标签只含基础币；报价币标签包含分隔符（如 `/ USDT`），
隐藏报价币时为空，两个标签均保留 RTL 隔离。`quote-price-ready` 表示存在可显示的缓存价格，
不代表数据新鲜；新鲜度和错误提示使用 `updated-text`，图表可用性使用 `chart-ready`。
这些属性均可选，旧插件继续兼容，不应通过比较翻译字符串推断状态。

`market-status-warning` 是可选结构化状态：所选币种缺少行情、缓存过期，或受数据源/图表错误
影响时为真；空选择和不影响这些币种的错误不设置该标记。使用已有 `source-text`、
`updated-text` 展示本地化说明，不能解析其文案判断异常。旧运行时 JSON 默认该字段为假，
旧插件可以不声明这个输入。

`quote-compact-prices` 将价格限制在 8 个显示字符以内，小数点、正负号和每个下标数字
均按 1 位计算。连续 4 个及以上前导小数零使用下标压缩，例如 `0.000004110` 显示为
`0.0₅4110`，下标表示首个非零数字前的零总数。其他超长价格调整小数精度，超长整数
使用科学计数法。`quote-full-prices` 保留缓存 `f64` 的数值精度，不进行显示舍入，
极端数量级使用科学计数法；它不是交易所返回的原始字符串。两个数组均保留 RTL 隔离，
方向控制字符不占显示位；没有缓存价格时为空字符串。
所有内置组件和插件的 `quote-rows[index].price` 数值统一采用同一套最多 8 位的格式，
只使用原有 `quote-rows` 的旧插件也会收到紧凑价格。短价格不补零；加载和错误文案
继续本地化，不受数值 8 位限制。缓存行情、告警计算和完整价格数值保持不变。
价格栏应按 8 个数字预留宽度，不能依靠省略号截断数值；字符数不能按 UTF-8 字节
或 RTL 隔离控制符计数。

`cargo +1.96.0 test --locked -p crypto-hud widget_price_tests` 使用真实 Slint 字体
测量各组件的价格栏，覆盖浅深主题、30%–200% 缩放、主流币种、PEPE 和超长数值。
`scripts/gui-eight-slot-prices-smoke.ps1` 在隔离状态下验证全部组件的原生窗口；
可用 `-SymbolCount 1` / `-SymbolCount 8` 和 `-ScalePercent 30` / `200` 检查边界。

Status Strip 保持每格 122 × 84、窗口四周 4px 留白及 1–5 个币对的横条布局。
保留旧配置的 `default` 主题 ID 并赋予浅色角色，新增 `dark` 深色主题。
价格按最多 8 位预留固定宽度；未就绪行情用中性占位，涨跌幅带本地化 24 小时周期。
保留原版青、蓝、橙、黄、绿的立体彩色分块、高光及暗边，深色主题采用同色系深色渐变。
背景色用于装饰和分块识别；底行直接显示方向三角和涨跌幅，周期标签右对齐。
箭头方向遵循实际涨跌，箭头与文字颜色共同遵守 `red-up-enabled`；浅色黄色块使用深色变体。
行情未就绪时隐藏箭头。方向图标来自 [Bootstrap Icons 1.11.3](https://github.com/twbs/icons/tree/v1.11.3)，
每个 SVG 保留 MIT 许可证。
渐变采用对齐物理像素的圆角切片，避免透明窗口悬浮重绘产生细缝。
正常状态仅显示币对、价格和涨跌幅，保留完整色块；来源和更新时间收进底部边缘的悬浮详情。
宿主的 `market-status-warning` 控制右上角的小异常提示点，悬浮或无障碍读取可查看详情。
详情不拦截拖动。锁按钮仅在悬浮或键盘焦点时浮现于右下角，临时覆盖最后一个周期标签，
不遮挡币对、价格或涨跌幅，支持空格、回车及无障碍操作。
运行 `cargo +1.96.0 test --locked -p crypto-hud status_strip` 验证，设置
`CRYPTO_HUD_STATUS_STRIP_CAPTURE_DIR` 可输出真实 Slint PPM 渲染图。
`scripts/gui-status-strip-runtime-smoke.ps1` 用隔离离线状态验证原生窗口、主题、缩放、
DPI 和数量规范化，接受 1–8 个请求币对并限制显示为最多 5 个，不调用桌面截图或输入。

Focus Ticker 提供浅色和深色主题，自然窗口尺寸为 490 × 100，卡片为 470 × 80。
币对按可用宽度调整字号，折线宽 146px、无底色。锁定按钮默认隐藏，
鼠标悬浮于组件或使用键盘聚焦时浮现在右上角，不预留横向空间，也不挤压图表。
价格上边距为 6px，底行区域下边距为 8px，保持原有字号与图表高度。
交易对栏宽 112px，交易对、价格和图表之间均留 8px 间距。
价格栏宽 168px，以 8 个显示位适配，统一采用 38px 字号。悬浮价格显示完整缓存数值，
提示浮层不改变卡片宽度，也不拦截价格区域的拖动。
辅助信息对齐在同一底行；两套主题统一采用 Segoe UI，价格字重减轻，锁图标为 17px。
该插件只显示一个币对，超过数量限制的配置由宿主规范化。
锁图标采用 [Feather 4.29.2](https://github.com/feathericons/feather/tree/v4.29.2)，
每个 SVG 内保留 MIT 许可证。

可运行 `scripts/gui-focus-ticker-smoke.ps1` 验证隔离状态下的两个原生窗口，支持
`-ScalePercent`、`-OpacityPercent`、`-Language`、`-RedUp` 和 `-SymbolCount 1..8`。
`-KeepOpen` 将窗口保留一小时供检查。
`-PriceMatrix` 可用固定离线价格同时展示 BTC、ETH、SOL、BNB、PEPE 的两套主题，
建议使用 100% 缩放查看十个窗口。运行
`cargo +1.96.0 test --locked -p crypto-hud focus_ticker` 检查文字适配、空数据、
加载/失败状态及键盘/鼠标锁定；设置 `CRYPTO_HUD_FOCUS_CAPTURE_DIR` 后会额外输出
真实 Slint 组件的两套 PPM 渲染图，可转换为插件 PNG 预览。

Mint Tile 使用 360 × 360 的单币种原生窗口。浅色主题采用低饱和薄荷色，深色主题采用深灰绿；
圆角渐变、细边框和柔和阴影由 Slint 原生绘制，避免单倍率材质图片在缩放和高 DPI 下变糊。
价格保持最高信息层级，币对、来源、更新信息减轻字重，涨跌胶囊缩小并显示本地化 24 小时周期。
价格栏按最多 8 个显示字符适配，包括 PEPE 压缩价格；没有就绪价格时，价格与涨跌均使用中性占位，
图表就绪状态独立判断。无行情时在图表区域显示宿主 `updated-text` 状态；有行情时来源与更新
状态在底部各占一行，不使用始终绿色的连接指示点。软件渲染器不支持圆角渐变裁剪和 BoxShadow，
因此使用原生圆角分层实现渐变与阴影，保持缩放时的清晰边缘。
锁定控件采用锁图标，只在悬浮或键盘聚焦时显示，已锁定也不常显；它不占布局空间，
支持空格、回车和可见焦点。主题 ID 与单币种限制保持兼容。

Trust Card 使用 260 × 180 的紧凑窗口，卡片为 240 × 160；上方为宿主币种图标和币对，
中间价格按最多 8 位适配，下方并排显示本地化 24 小时涨跌与迷你趋势图。浅色采用奶白薄荷，
深色采用石墨绿，保留既有主题 ID、默认深色和单币种限制。Mint Tile 继续保持现有的
360 × 360 完整走势图与来源页脚。
锁定控件仅在悬浮或键盘聚焦时浮现，支持空格/回车；来源与更新时间在悬浮币种/币对区域或键盘聚焦时
替换底行显示，不改变尺寸或阻碍拖动。未就绪价格显示中性占位；加载、图表不可用或
`market-status-warning` 为真时持续显示本地化状态，缓存价格仍然可见。原生渐变的全局起点
和裁剪带都按物理像素对齐，避免分数 DPI 下局部重绘产生横缝。迷你折线不使用面积填充；
锁显示切换时，其外层透明裁剪余量改变 1 个物理像素，让整个绘图区失效重绘，折线与尾点
位置保持不变，以兼容 Slint 1.17 软件 Path 超出脏区域重复混色的问题。
运行 `cargo +1.96.0 test --locked -p crypto-hud trust_card` 检查真实 Slint 渲染、文字适配、
行情状态、锁定/拖动和旧尺寸规范化；设置 `CRYPTO_HUD_TRUST_CARD_CAPTURE_DIR` 可输出 PPM。
运行 `scripts/gui-trust-card-runtime-smoke.ps1` 验证隔离原生窗口、离线数据和 DPI，支持主题、
缩放、透明度、语言、配色方向、锁定、图标与 1–8 个请求币对（规范化为 1 个）。
`-PreviewMatrix` 展示 BTC/PEPE 两套主题，`-IconCacheDir` 可提供离线币种图稿，
`-KeepOpen` 保留窗口一小时；脚本不截取桌面或自动操作桌面输入。

Market Compass 的默认浅色主题采用香槟金表盘、陶瓷徽章和独立币种图稿。
同款深色主题使用石墨黑陶瓷、烟金拉丝边框和暖灰白文字，保持相同几何、交互和预滤波素材规格；
深色 XRP、ADA 图标采用更亮色调以保持辨识度。既有主题 ID 保持兼容，新实例默认使用浅色。
自然尺寸为 480 × 480，支持 1–8 个币种；价格栏宽 300px，采用固定 44px 字号适配最多 8 位，
价格就绪和图表就绪相互独立。BTC、ETH、SOL、BNB、XRP、DOGE、ADA、PEPE 使用随插件提供的
参考图稿；其他币种继续读取宿主图标，缺少图标时显示币种名称。
点击徽章以最短弧线旋转至正上方，标签始终正立；悬浮或键盘聚焦时暂停自动轮播，方向键切换币种。
锁定按钮悬浮显示，不占布局宽度，并支持键盘聚焦、空格和回车。
软件渲染器采用最近邻纹理采样、不支持 Path 渐变，因此素材按 100/125/150/200% 预滤波，
结合系统 DPI 和组件缩放选择；浅色表盘最高 896px，深色在 150% 以上复用 720px 素材，
以满足单素材 1 MiB 和整个双主题插件目录 5 MiB 限制。
动态图表填充使用物理像素对齐的裁剪带衰减透明度，价格、路径、尾点和涨跌方向来自真实宿主数据。
运行 `scripts/gui-market-compass-smoke.ps1` 检查隔离原生窗口，支持主题、数量、PEPE、缩放、
透明度、语言、涨跌配色、图标和锁定参数；`-KeepOpen` 保留窗口一小时。
运行 `cargo +1.96.0 test --locked -p crypto-hud market_compass` 验证轮播、输入、渲染和素材限制；
设置 `CRYPTO_HUD_MARKET_COMPASS_CAPTURE_DIR` 可输出真实 Slint 参考渲染图。

插件 Slint 文件中需要定义 `QuoteRow`：

```slint
struct QuoteRow {
    symbol: string,
    price: string,
    change: string,
    positive: bool,
}
```

组件还必须暴露以下回调：

```slint
callback drag-move(length, length);
callback toggle-layout-lock();
```

回调语义：

- `drag-move(dx, dy)`：移动桌面小组件窗口。
- `toggle-layout-lock()`：切换布局锁定状态。

## 本地化和 RTL

可见 UI 文案应来自宿主下发的本地化属性，避免在 Slint 文件里硬编码英文文本。
常用属性包括 `pairs-heading-text`、`source-text`、`source-name-text`、`updated-text`
和 `empty-text`。行情符号、价格、百分比、交易所名和协议 token 可以保持原文，
因为这些内容本身就是市场数据或技术标识。

渲染本地化标签的插件必须声明 `rtl-layout`，并在阿拉伯语等 RTL locale 下调整
文本对齐：

```slint
Text {
    text: root.source-text;
    horizontal-alignment: root.rtl-layout ? right : left;
    overflow: elide;
}
```

行情符号、价格和百分比通常不做整体镜像，便于用户快速扫盘。不要用本地化文案
判断布局或数据状态，例如不要比较 `Connecting`；应使用 `chart-ready`、数据数量
或宿主下发的结构化状态。

## 主题和颜色适配

宿主只下发主题名称，不下发具体颜色。插件可以在 `widget.json` 中声明多套主题；
如果省略 `themes`，则视为只有一套 `default` 主题，设置窗口不会显示主题切换项。

```json
"themes": [
  { "id": "light", "name": "Light", "role": "light" },
  { "id": "dark", "name": "Dark", "role": "dark", "default": true }
]
```

用户选择“跟随系统”时，宿主会根据系统浅色/深色寻找对应 `role` 的主题；如果不存在，
则下发该小组件的默认主题。

```slint
in property <string> theme-name: "dark";

property <bool> light-theme: root.theme-name == "light";
property <color> card-background: root.light-theme ? #f8fafcec : #111827ee;
property <color> card-primary-text-color: root.light-theme ? #0f172a : #f8fafc;
property <color> gain-color: root.light-theme ? #16a34a : #22c55e;
property <color> loss-color: root.light-theme ? #dc2626 : #f87171;
```

`theme-name` 的值来自 `themes[].id`，例如 `light`、`dark` 或自定义 id。

## 插件自定义参数

插件最多可以声明 8 个参数。宿主会校验清单、在所选小组件设置中按类型渲染控件、按小组件实例持久化参数值，并通过 `config-<key>` 属性传给 Slint。参数不会给插件增加网络或文件系统权限。

| 清单 `kind` | Slint 属性类型 | 设置控件 |
| --- | --- | --- |
| `integer` | `int` 或 `float` | 整数步进器 |
| `boolean` | `bool` | 开关 |
| `choice` | `string` | 前后选项器 |
| `decimal` | `float` | 小数步进器 |
| `color` | `color` 或 `brush` | 十六进制颜色输入 |
| `string` | `string` | 有长度上限的单行输入 |

```json
"parameters": [
  {
    "kind": "integer",
    "key": "switch-interval-seconds",
    "name": "Switch interval",
    "nameZhHans": "切换时间",
    "description": "Time between automatic pair switches.",
    "descriptionZhHans": "币种自动切换的时间间隔。",
    "default": 5,
    "minimum": 1,
    "maximum": 60,
    "step": 1,
    "unit": "s",
    "unitZhHans": "秒"
  },
  {
    "kind": "boolean",
    "key": "show-caption",
    "name": "Show caption",
    "nameZhHans": "显示标题",
    "default": true
  },
  {
    "kind": "choice",
    "key": "density",
    "name": "Density",
    "nameZhHans": "密度",
    "default": "compact",
    "options": [
      { "value": "compact", "name": "Compact", "nameZhHans": "紧凑" },
      { "value": "comfortable", "name": "Comfortable", "nameZhHans": "舒适" }
    ]
  },
  {
    "kind": "decimal",
    "key": "line-width",
    "name": "Line width",
    "nameZhHans": "线宽",
    "default": 1.5,
    "minimum": 0.5,
    "maximum": 4.0,
    "step": 0.25,
    "precision": 2,
    "unit": "px",
    "unitZhHans": "像素"
  },
  {
    "kind": "color",
    "key": "accent-color",
    "name": "Accent color",
    "nameZhHans": "强调色",
    "default": "#3366ff"
  },
  {
    "kind": "string",
    "key": "caption",
    "name": "Caption",
    "nameZhHans": "标题",
    "default": "Market",
    "minLength": 1,
    "maxLength": 24
  }
]
```

参数 key 只能使用小写 ASCII 字母、数字和内部连字符。每个参数都必须暴露类型完全匹配的 Slint 属性：

```slint
in property <int> config-switch-interval-seconds: 5;
in property <bool> config-show-caption: true;
in property <string> config-density: "compact";
in property <float> config-line-width: 1.5;
in property <color> config-accent-color: #3366ff;
in property <string> config-caption: "Market";

Timer {
    interval: root.config-switch-interval-seconds * 1s;
}
```

约束规则：

- 公共的 `name`、`nameZhHans`、`description` 和 `descriptionZhHans` 用于宿主设置控件；简体中文以外的语言使用英文文案。
- `choice` 必须包含 2–32 个选项。`value` 是稳定持久化 token，只能包含小写 ASCII 字母、数字、点、下划线或内部连字符；`default` 必须命中一个选项。
- `decimal` 的数值必须有限，满足 `minimum < maximum` 且 `step > 0`；`precision` 默认为 2，范围为 0–6。
- `color` 使用 `#RRGGBB` 或 `#RRGGBBAA`；宿主会把十六进制规范为小写，并向 Slint 注入 brush。
- `string.maxLength` 必填，范围为 1–256；`minLength` 默认为 0。只接受不含控制字符的单行 Unicode 文本。
- 参数未保存或持久化类型不兼容时使用 `default`；数值会限制在声明范围内。未知配置键继续原样保留，便于兼容迁移。

扩展类型 `boolean`、`choice`、`decimal`、`color`、`string` 要求 Host API 0.2.0；清单 schema 仍保持 v3。

## 行情颜色方向

`red-up-enabled` 是宿主下发给所有插件的全局行情颜色方向开关。

- `false`：默认，绿涨红跌。
- `true`：红涨绿跌。

插件渲染涨跌文字、K 线、折线、面积图和状态色时，都应使用同一映射：

```slint
property <color> chart-color: root.chart-positive
    ? (root.red-up-enabled ? root.loss-color : root.gain-color)
    : (root.red-up-enabled ? root.gain-color : root.loss-color);
```

需要渲染 K 线或折线图的插件，可以暴露以下可选行情路径属性：

```slint
in property <string> chart-line-path;
in property <string> chart-fill-path;
in property <bool> chart-ready;
in property <bool> chart-positive;
```

需要渲染币种图标的插件，可以暴露以下可选属性：

```slint
in property <[image]> quote-icons;
```

`quote-icons[index]` 与 `quote-rows[index]` 对齐。宿主会先读取本地缓存，
缓存缺失时按 `spothq/cryptocurrency-icons`、Iconify `cryptocurrency`、
Trust Wallet assets 的顺序后台查找；Trust Wallet 会先尝试原生链图标，再按常见
链的 `tokenlist.json` 匹配 `logoURI`。命中后缓存到本地，后续刷新直接复用。
这三个来源都不需要 API key 或授权。插件应在数组长度不足时隐藏图标或使用空
`image` 兜底。

## 尺寸和等比缩放

`defaultSize` 和可选 `sizePolicy` 描述插件的自然尺寸。宿主会保存 `scale_percent`
作为缩放真值，再用“当前自然尺寸 × scale_percent”派生真实桌面窗口宽高，并通过
`widget-scale` 把内容缩放比例下发给插件。插件内部应保持标准自然尺寸，不再根据
`root.width` / `root.height` 自己反推缩放。
新插件建议暴露 `widget-scale` 属性；宿主仍兼容旧插件，但旧插件无法获得最稳定的
拖拽和缩放行为。

推荐 Slint 写法：

```slint
in property <float> widget-scale: 1.0;
property <float> content-scale: root.widget-scale;

drag_area := TouchArea {
    width: root.widget-width * 1px;
    height: root.widget-height * 1px;

    moved => {
        if (!root.layout-locked && self.pressed) {
            root.drag-move(self.mouse-x - self.pressed-x, self.mouse-y - self.pressed-y);
        }
    }
}

card := Rectangle {
    x: (root.widget-width * 1px - 300px * root.content-scale) / 2;
    y: (root.widget-height * 1px - 180px * root.content-scale) / 2;
    width: 300px;
    height: 180px;
    transform-origin: { x: 0px, y: 0px };
    transform-scale-x: root.content-scale;
    transform-scale-y: root.content-scale;
}
```

注意事项：

- 根窗口 `width`、`height` 应绑定到 `widget-width`、`widget-height`。
- 拖拽层放在未缩放的根层，视觉内容放进 `transform-scale` 的内容层。
- 内容缩放应直接使用宿主下发的 `widget-scale`。
- 固定版式里的文本必须设置明确宽度，必要时使用 `overflow: elide`。

## 拖拽和锁定

每个桌面插件应支持：

- `layout-locked == false` 时可拖动。
- 有可见或可发现的锁定入口，调用 `toggle-layout-lock()`。
- 拖拽区域有明确 cursor 反馈。

装饰层和输入区域应分离，避免 hover 或 pressed 状态改变整体布局尺寸。

## 行情数据

- 使用 `quote-rows` 渲染价格数据。
- 宿主会按 `minSymbolLimit` 和 `symbolLimit` 规范化币种数量。
- 单币种图表插件通常使用第一条 `quote-rows` 和可选 chart path 属性。
- chart 数据未准备好时，应显示静态兜底或轻量占位。
- 涨跌颜色需要尊重 `red-up-enabled`。
- 可见状态和空态文案使用宿主下发的本地化属性，不在插件里写英文兜底。

## 新增插件流程

1. 在 `crates/crypto-hud/plugins` 下创建插件目录。
2. 添加 `widget.json`。
3. 添加 `ui/main.slint`，满足必需属性和回调合同。
4. 在插件内部定义 `light` 和 `dark` 两套颜色，不依赖宿主下发颜色。
5. 如果设置页市场需要专属缩略图，在 `settings_window.rs` 增加 preview kind 映射，
   并在 `price-card.slint` 增加对应缩略绘制。
6. 涨跌文字和 K 线颜色必须尊重 `red-up-enabled`。
7. 本地化标签必须使用宿主下发的文本属性，并用 `rtl-layout` 检查阿拉伯语布局。
8. 保存插件文件后等待文件树稳定，宿主会自动重扫、重编译并重建插件窗口；也可以在
   `系统设置 / 插件诊断` 中手动点击“重新加载”。诊断面板会展示清单、兼容版本、Slint
   编译、属性和回调错误，但不会暴露用户状态目录的绝对路径。

设置页会同时显示清单 schema 与 Host API 兼容范围。若已保存的小组件引用了缺失或失效插件，
恢复相同插件 id 后热重载即可自动找回；也可以在所选小组件中选择替代插件并“重新定位”。
重新定位会保留不透明配置、名称和布局元数据，再按替代插件合同规范化币种、尺寸、主题和参数。

宿主会在有界、可取消的后台扫描器中为每个插件目录独立计算指纹，连续两次观察到稳定内容后只编译
发生变化的候选。快速连续保存会产生递增 generation；若编译或实例创建完成前文件再次变化，旧结果
会被丢弃。临时扫描或访问错误会进入诊断，但不会被当作已确认删除，因此 last-known-good 候选仍会
继续运行。清单、资源、合同或 Slint 编译失败时，已成功运行的定义和窗口继续工作，诊断面板显示
新错误；修复后会先完整创建该插件的全部实例，再在同一 UI turn 中切换。
未变化插件的窗口和动画状态不会重建。被替换插件自身的私有 Slint 状态会重新开始，因为 Host
API 0.2 尚未提供插件内部状态迁移合同。

删除插件目录后，宿主只移除该插件窗口，仍保留布局和配置；恢复相同插件 id 后会从宿主状态重建。
行情订阅只在成功提交新定义后刷新，插件的网络和文件系统权限始终保持关闭。

## 验证清单

提交前至少运行：

```powershell
cargo test -p crypto-hud discovers_repo_local_plugins
cargo test -p crypto-hud discovers_plugin_with_all_extended_parameter_property_types
cargo test --workspace
mise run check
powershell -File scripts/gui-plugin-hot-reload-smoke.ps1
```

Mint Tile 的聚焦测试检查真实 Slint 渲染、文字适配、数据状态及交互。
运行时 smoke 使用隔离状态和离线行情，检查插件加载、数据行、锁定状态以及当前桌面 DPI 下的
窗口尺寸，通过应用计时器退出，不调用桌面截图、输入或 UI Automation：

```powershell
cargo +1.96.0 test --locked -p crypto-hud mint_tile
powershell -File scripts/gui-mint-tile-runtime-smoke.ps1 -Theme both
powershell -File scripts/gui-mint-tile-runtime-smoke.ps1 -FirstAsset PEPE -ScalePercent 30
powershell -File scripts/gui-mint-tile-runtime-smoke.ps1 -Theme dark -ScalePercent 200 -Language ar -RedUp -Locked -NoLogos
```

运行时 smoke 还支持 `-OpacityPercent` 和 `-KeepOpen`；后者保留预览窗口一小时。
在可使用桌面截图与输入自动化的会话中，可以额外运行交互 smoke，检查鼠标锁定/解锁、拖动和
无障碍操作：

```powershell
powershell -File scripts/gui-mint-tile-smoke.ps1 -PaperBackdrop
powershell -File scripts/gui-mint-tile-smoke.ps1 -PaperBackdrop -ScalePercent 30
powershell -File scripts/gui-mint-tile-smoke.ps1 -PaperBackdrop -Language ar -RedUp -LockLight
```

需要同步浅色和深色预览图时，以默认 100% 组件缩放运行 `-PaperBackdrop -UpdatePreviews`；
交互脚本会从完整原生窗口截图生成 360 × 360 预览，避免 DPI 裁切和相邻卡片透入透明边缘。

手动 GUI smoke：

- 运行 `mise run run-app`。
- 检查浅色、深色、系统主题下的显示。
- 拖拽缩放小组件，确认窗口、内容、K 线和命中区域同步缩放。
- 切换布局锁定，确认拖拽和缩放手柄状态正确。
- 打开设置页市场，确认插件左侧预览形态和真实插件一致。
- 修改 `.slint` 并确认热重载；故意制造编译错误，确认诊断面板显示错误，修复后可恢复窗口。
- 对每种参数控件修改值、重启应用并确认按小组件实例持久化。
- 切换到阿拉伯语，确认标签对齐、空态和源信息没有混排错位。
