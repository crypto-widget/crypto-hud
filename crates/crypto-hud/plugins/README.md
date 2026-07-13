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
  "hostApiVersion": ">=0.1.0, <1.0.0",
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
- `hostApiVersion` 必须匹配当前宿主 API。
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
- 允许的文件扩展名为 `json`、`slint`、`png`、`jpg`、`jpeg`、`svg`。
- `previewImages` 与 `renderer.entry` 使用相同的严格相对路径规则，且必须解析到插件
  目录内部。
- 所有 `.slint` import 和 `@image-url()` 资源都必须解析到插件目录内部。图片资源仅支持
  `png`、`jpg`、`jpeg`、`svg`，单个文件最大 1 MiB。除 `.slint` 外的文件 import 会被
  拒绝；当前不支持自定义字体或其他外部文件系统资源。

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
in property <bool> rtl-layout: false;
in property <float> widget-scale: 1.0;
in property <[image]> quote-icons;
in property <string> chart-line-path;
in property <string> chart-fill-path;
in property <bool> chart-ready;
in property <bool> chart-positive;
```

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

插件最多可以声明 8 个整数参数。宿主会校验清单、在所选小组件设置中自动渲染步进控件、按小组件实例持久化参数值，并通过 `config-<key>` 属性传给 Slint。

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
  }
]
```

参数 key 只能使用小写 ASCII 字母、数字和内部连字符。Slint 必须暴露对应的数值属性：

```slint
in property <int> config-switch-interval-seconds: 5;

Timer {
    interval: root.config-switch-interval-seconds * 1s;
}
```

未保存过的参数使用 `default`，已保存的值会被限制在 `minimum` 到 `maximum` 范围内。

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

## 验证清单

提交前至少运行：

```powershell
cargo test -p crypto-hud discovers_repo_local_plugins
cargo test --workspace
mise run check
```

手动 GUI smoke：

- 运行 `mise run run-app`。
- 检查浅色、深色、系统主题下的显示。
- 拖拽缩放小组件，确认窗口、内容、K 线和命中区域同步缩放。
- 切换布局锁定，确认拖拽和缩放手柄状态正确。
- 打开设置页市场，确认插件左侧预览形态和真实插件一致。
- 切换到阿拉伯语，确认标签对齐、空态和源信息没有混排错位。
