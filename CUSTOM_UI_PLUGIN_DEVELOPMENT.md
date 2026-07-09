# Custom UI Plugin Development Guide

This guide explains how to build custom UI widget plugins for Crypto HUD. On startup, Crypto HUD copies this file into the custom component directory:

```text
<Crypto HUD state dir>/plugins/CUSTOM_UI_PLUGIN_DEVELOPMENT.md
```

Open System Settings and click `Custom components / Open Folder` to open that directory. Each plugin must live in its own subdirectory under the custom component directory. Crypto HUD scans those subdirectories during startup.

## Directory Layout

Each plugin should use a stable directory name and must contain `widget.json` plus a Slint entry file:

```text
com.example.my-widget/
  widget.json
  ui/
    main.slint
    preview-light.png
```

Allowed file extensions inside a plugin directory are `json`, `slint`, `png`, `jpg`, `jpeg`, and `svg`. Keep the plugin directory within the host size limits.

## widget.json

The manifest file must be named `widget.json`. Minimal example:

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
  "previewImages": [
    "ui/preview-light.png"
  ],
  "dataRequirements": [
    { "capability": "market.price" }
  ]
}
```

Manifest requirements:

- `schemaVersion` must be `3`.
- `id` must use lowercase ASCII letters, digits, dots, or hyphens, for example `com.example.my-widget`.
- `version` must be valid SemVer.
- `hostApiVersion` must match the host API, for example `>=0.1.0, <1.0.0`.
- `renderer.kind` must be `slint`.
- `renderer.entry` must be a relative path and must not contain `..`.
- `renderer.component` must be the exported component name in the entry Slint file.
- `permissions.network` and `permissions.filesystem` must both be `false`.
- `defaultSize` must be between `120x80` and `1200x900`.
- `minSymbolLimit` and `symbolLimit` must be between `1` and `8`, with `minSymbolLimit <= symbolLimit`.
- `defaultSymbols` is optional. If present, its length must satisfy the symbol limits and every entry must be a valid market pair.
- `previewImages` is optional. It may contain up to 5 images and only supports `png`, `jpg`, and `jpeg`.
- `dataRequirements` currently supports `market.price` and `market.candles`.

Pairs without an explicit source are normalized as Binance spot pairs quoted in USDT. For example, `BTC` is equivalent to `binance:spot:BTC/USDT`.

## Size Policies

Fixed-size plugins use:

```json
"sizePolicy": {
  "kind": "fixed"
}
```

Grid plugins that resize with the number of pairs should use `symbolGrid`:

```json
"sizePolicy": {
  "kind": "symbolGrid",
  "cellSize": {
    "width": 122,
    "height": 84
  },
  "contentPadding": {
    "width": 8,
    "height": 8
  },
  "columns": 5
}
```

`defaultSize` must equal the natural size when rendering `symbolLimit` pairs. Older horizontal-list plugins may use `symbolBlocks`, but new plugins should prefer `fixed` or `symbolGrid`.

## Slint Component Contract

The component referenced by `renderer.component` must inherit `Window` and expose these properties:

```slint
struct QuoteRow {
    symbol: string,
    price: string,
    change: string,
    positive: bool,
}

export component MyWidget inherits Window {
    in property <string> widget-id;
    in property <[QuoteRow]> quote-rows;
    in property <string> pairs-heading-text;
    in property <string> source-text;
    in property <string> updated-text;
    in property <string> empty-text;
    in property <bool> pin-to-top;
    in property <bool> layout-locked;
    in property <float> widget-width;
    in property <float> widget-height;
    in property <string> theme-name;
    in property <bool> red-up-enabled;
    in property <float> content-opacity;
    in property <float> widget-scale: 1.0;

    callback drag-move(length, length);
    callback toggle-layout-lock();
}
```

Optional properties:

```slint
in property <[image]> quote-icons;
in property <string> chart-line-path;
in property <string> chart-fill-path;
in property <bool> chart-ready;
in property <bool> chart-positive;
```

`quote-icons[index]` aligns with `quote-rows[index]`. If the icon array is empty or too short, hide icons or render an empty placeholder.

## Theme And Change Colors

The host passes the selected widget theme through `theme-name`. Declare supported themes in `widget.json`; if omitted, the widget has one `default` theme and the settings window will not show a theme selector.

```json
"themes": [
  { "id": "light", "name": "Light", "role": "light" },
  { "id": "dark", "name": "Dark", "role": "dark", "default": true }
]
```

When the user chooses System, the host sends the theme with role `light` or `dark` to match the OS. If that role is not declared, it sends the widget's default theme.

```slint
property <bool> light-theme: root.theme-name == "light";
property <color> card-background: root.light-theme ? #f8fafc : #111827;
property <color> text-color: root.light-theme ? #0f172a : #f8fafc;
property <color> gain-color: root.light-theme ? #16a34a : #22c55e;
property <color> loss-color: root.light-theme ? #dc2626 : #f87171;
```

`red-up-enabled` controls market color direction:

- `false`: green means up, red means down.
- `true`: red means up, green means down.

All price changes, charts, and status colors should use the same mapping:

```slint
property <color> change-color: row.positive
    ? (root.red-up-enabled ? root.loss-color : root.gain-color)
    : (root.red-up-enabled ? root.gain-color : root.loss-color);
```

## Scaling And Dragging

The host stores widget scale and passes the real window state through `widget-scale`, `widget-width`, and `widget-height`. Do not infer scale from `root.width` or `root.height`.

Recommended structure:

```slint
width: root.widget-width * 1px;
height: root.widget-height * 1px;

drag_area := TouchArea {
    width: root.widget-width * 1px;
    height: root.widget-height * 1px;
    mouse-cursor: root.layout-locked ? default : move;

    moved => {
        if (!root.layout-locked && self.pressed) {
            root.drag-move(self.mouse-x - self.pressed-x, self.mouse-y - self.pressed-y);
        }
    }
}

card := Rectangle {
    x: (root.widget-width * 1px - 300px * root.widget-scale) / 2;
    y: (root.widget-height * 1px - 180px * root.widget-scale) / 2;
    width: 300px;
    height: 180px;
    transform-origin: { x: 0px, y: 0px };
    transform-scale-x: root.widget-scale;
    transform-scale-y: root.widget-scale;
}
```

Requirements:

- Bind the root window size to `widget-width` and `widget-height`.
- Keep the drag layer at the unscaled root level.
- Put visual content inside the scaled content layer.
- Allow dragging when `layout-locked == false`.
- Provide a visible or discoverable lock control that calls `toggle-layout-lock()`.
- Give fixed-layout text explicit widths and use `overflow: elide` when needed.

## Data Rendering

- Render prices and changes from `quote-rows`.
- Show `empty-text` when `quote-rows` is empty.
- Use `pairs-heading-text`, `source-text`, and `updated-text` as host-provided localized strings.
- Single-pair chart plugins usually use the first `quote-rows` item plus optional chart path properties.
- Show a static fallback or lightweight placeholder while chart data is not ready.

## Development Flow

1. Create a plugin subdirectory inside the custom component directory.
2. Write `widget.json`.
3. Write `ui/main.slint` and satisfy the required properties and callbacks.
4. Restart Crypto HUD, or reopen the app, so the host scans plugins again.
5. Check whether the plugin appears in `Widget Library`.
6. Test light theme, dark theme, scaling, dragging, locking, and different pair counts.

## Troubleshooting

- Plugin does not appear: check that it is in a subdirectory under the custom component directory and contains `widget.json`.
- Plugin is unavailable: check `renderer.component`, required properties, and required callbacks.
- Preview images do not render: check that `previewImages` paths are relative and use `png`, `jpg`, or `jpeg`.
- Size is wrong: check that `defaultSize` matches the natural size implied by `sizePolicy`.
- Change colors are wrong: check that all gain/loss colors respect `red-up-enabled`.

## Verification Commands

In a source checkout, you can run:

```powershell
cargo test -p crypto-hud discovers_valid_local_plugin
cargo test -p crypto-hud discovers_repo_local_plugins
cargo check -p crypto-hud
```
