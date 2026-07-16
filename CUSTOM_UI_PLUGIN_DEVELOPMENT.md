# Custom UI Plugin Development Guide

This guide explains how to build custom UI widget plugins for Crypto HUD. On startup, Crypto HUD copies this file into the custom component directory:

```text
<Crypto HUD state dir>/plugins/CUSTOM_UI_PLUGIN_DEVELOPMENT.md
```

Open System Settings and click `Custom components / Open Folder` to open that directory. Each plugin must live in its own subdirectory under the custom component directory. Crypto HUD scans those subdirectories during startup and watches them for stable file changes while the app is running.

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
- `name` is the author-provided display name. Crypto HUD shows it exactly as written and does not localize it.
- `version` must be valid SemVer.
- `hostApiVersion` must match the current host API (`0.2.0`). Integer-only plugins may retain a compatible range such as `>=0.1.0, <1.0.0`; plugins that use the extended parameter kinds must use a range that includes `0.2.0` and excludes `0.1.x`, for example `>=0.2.0, <1.0.0`.
- `renderer.kind` must be `slint`.
- `renderer.entry` must contain only ordinary relative path components. Absolute, rooted,
  drive-relative, `.`, and `..` components are rejected.
- `renderer.component` must be the exported component name in the entry Slint file.
- `permissions.network` and `permissions.filesystem` must both be `false`.
- `defaultSize` must be between `120x80` and `1200x900`.
- `minSymbolLimit` and `symbolLimit` must be between `1` and `8`, with `minSymbolLimit <= symbolLimit`.
- `defaultSymbols` is optional. If present, its length must satisfy the symbol limits and every entry must be a valid market pair.
- `previewImages` is optional. It may contain up to 5 images and only supports `png`, `jpg`, and `jpeg`.
  Each entry follows the same strict relative-path rule as `renderer.entry` and must resolve inside
  the plugin directory.
- `dataRequirements` currently supports `market.price` and `market.candles`.
  `market.candles` is opt-in: the host requests candles for a pair only when at least one
  configured, available widget for that pair declares the capability. Requirements from widgets
  that share a pair are merged into one market subscription.

All `.slint` imports and `@image-url()` resources must resolve inside the plugin directory. Image
resources support `png`, `jpg`, `jpeg`, and `svg`, with a 1 MiB limit per asset. File imports other
than `.slint` are rejected; custom fonts and other external filesystem resources are not supported.

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
in property <bool> rtl-layout: false;
in property <string> source-name-text;
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

## Plugin Parameters

Plugins can declare up to 8 parameters. The host validates the manifest, renders a type-appropriate control in the selected widget settings, persists values per widget instance, and injects them into the Slint component as `config-<key>`. Plugins never receive network or filesystem access through parameters.

| Manifest kind | Slint property type | Settings control |
| --- | --- | --- |
| `integer` | `int` or `float` | Integer stepper |
| `boolean` | `bool` | Switch |
| `choice` | `string` | Previous/next selector |
| `decimal` | `float` | Decimal stepper |
| `color` | `color` or `brush` | Hex color input |
| `string` | `string` | Bounded single-line input |

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

The parameter key must use lowercase ASCII letters, digits, and internal hyphens. Every declared parameter requires a matching Slint property with the exact host type:

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

Parameter constraints:

- Common `name`, `nameZhHans`, `description`, and `descriptionZhHans` fields describe the host-rendered control. English text is used outside Simplified Chinese.
- `choice` requires 2–32 options. Option `value` is a stable persisted token using lowercase ASCII letters, digits, dots, underscores, or internal hyphens; `default` must match one option.
- `decimal` values must be finite, `minimum < maximum`, and `step > 0`. `precision` defaults to 2 and may be 0–6.
- `color` defaults and saved values use `#RRGGBB` or `#RRGGBBAA`; the host canonicalizes hex digits to lowercase and injects a Slint brush.
- `string.maxLength` is required and must be 1–256. `minLength` defaults to 0. Values are single-line Unicode text with no control characters.
- Missing or type-incompatible persisted values use `default`; numeric values are clamped to their declared range. Unknown configuration keys remain preserved for compatibility.

Extended kinds (`boolean`, `choice`, `decimal`, `color`, and `string`) require Host API 0.2.0. The manifest schema remains version 3.

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
- Use `pairs-heading-text`, `source-text`, `source-name-text`, and `updated-text` as host-provided localized strings.
- Single-pair chart plugins usually use the first `quote-rows` item plus optional chart path properties.
- Show a static fallback or lightweight placeholder while chart data is not ready.

## Localization And RTL

Visible UI copy should come from host-provided properties instead of hardcoded English text. Declare `rtl-layout` when a plugin renders localized labels, and use it to align those labels for Arabic:

```slint
in property <bool> rtl-layout: false;

Text {
    text: root.source-text;
    horizontal-alignment: root.rtl-layout ? right : left;
    overflow: elide;
}
```

Keep market symbols, prices, and percent changes visually stable for scanning unless the whole layout is intentionally mirrored. Do not compare against localized strings such as `Connecting`; use host state or data readiness properties instead.

## Development Flow

1. Create a plugin subdirectory inside the custom component directory.
2. Write `widget.json`.
3. Write `ui/main.slint` and satisfy the required properties and callbacks.
4. Save the files and wait for the watcher to observe a stable tree, or click `Plugin diagnostics / Reload` in System Settings.
5. Check the diagnostics panel. It shows manifest, compatibility, Slint compilation, missing property, and callback errors without exposing the absolute state path.
6. Check whether the plugin appears in `Widget Library`.
7. Test light theme, dark theme, scaling, dragging, locking, parameter persistence, and different pair counts.

Crypto HUD fingerprints each plugin directory independently on a bounded, cancellable background scanner. After two stable observations, it compiles only changed candidates. Rapid consecutive saves receive increasing generations; a result is discarded if the source tree changes again before it can be committed. A temporary scan or access error is reported but is not treated as a confirmed deletion, so the last-known-good candidate remains active.

The last successfully compiled definition remains active when a changed manifest, resource, contract, or Slint source fails validation. Existing widget windows keep running while the new diagnostic is shown. Once the files compile again, every instance of that plugin is staged first and then replaced in one UI turn. Unchanged plugins keep their existing windows and local animation state. A replaced plugin starts with fresh private Slint state because Host API 0.2 does not expose plugin-state migration.

Deleting a plugin removes only that plugin's runtime windows after the tree is stable; saved layout and configuration remain intact. Recreating the same plugin ID restores those instances from the preserved host state. Crypto HUD displays both manifest schema and Host API compatibility in the diagnostics panel and on local plugin descriptions, and refreshes market subscriptions only after a successful reload without granting additional permissions.

## Troubleshooting

- Plugin does not appear: check that it is in a subdirectory under the custom component directory and contains `widget.json`.
- Plugin is unavailable: check `renderer.component`, required properties, and required callbacks.
- A broken edit still shows the previous design: this is the last-known-good definition. Fix the diagnostic and save again to replace it.
- Parameter property error: check that every `config-<key>` property exists and uses the type in the table above.
- Preview images do not render: check that `previewImages` paths are relative and use `png`, `jpg`, or `jpeg`.
- Size is wrong: check that `defaultSize` matches the natural size implied by `sizePolicy`.
- Change colors are wrong: check that all gain/loss colors respect `red-up-enabled`.
- A saved widget references a missing or invalid plugin: restore the original plugin ID and reload, or use the selected widget's `Replacement plugin / Relink` control. Relinking preserves opaque widget configuration, name, and layout metadata, then normalizes symbols, size, theme, and parameters against the replacement contract.

## Verification Commands

In a source checkout, you can run:

```powershell
cargo test -p crypto-hud discovers_valid_local_plugin
cargo test -p crypto-hud discovers_plugin_with_all_extended_parameter_property_types
cargo test -p crypto-hud discovers_repo_local_plugins
cargo check -p crypto-hud
powershell -File scripts/gui-plugin-hot-reload-smoke.ps1
```
