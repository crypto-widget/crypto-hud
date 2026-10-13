use std::{cell::RefCell, collections::BTreeMap, path::PathBuf, rc::Rc, time::Instant};

use slint::{
    platform::{
        software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
        Platform, WindowAdapter,
    },
    ComponentHandle, Rgb8Pixel,
};
use slint_interpreter::{Compiler, Value};

struct PriceTestPlatform(Rc<RefCell<Option<Rc<MinimalSoftwareWindow>>>>);

impl Platform for PriceTestPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        self.0.replace(Some(window.clone()));
        Ok(window)
    }
}

// Instrument the actual price Text items without adding test properties to plugins.
fn instrument_prices(mut source: String, paper_backdrop: bool) -> String {
    // Give exported previews a neutral canvas without changing production transparency.
    let window_start = source.find("inherits Window {").unwrap();
    if let Some(background) = source[window_start..].find("background: transparent;") {
        let at = window_start + background;
        source.replace_range(
            at..at + "background: transparent;".len(),
            if paper_backdrop {
                "background: #ebf0f6;"
            } else {
                "background: root.theme-name == \"light\" ? #ebf0f6 : #272e37;"
            },
        );
    }
    let mut offset = 0;
    let mut probe_id = 0;
    while let Some(found) = [
        "text: quote.price;",
        "text: root.price-ready ? quote.price : \"—\";",
    ]
    .iter()
    .filter_map(|binding| source[offset..].find(binding))
    .min()
    {
        let at = offset + found;
        let start = source[..at].rfind("Text {").unwrap();
        let replacement = format!(
            "PriceProbe {{\n                probe-generation: root.price-probe-generation;\n\
             measured(text, actual, available) => {{ root.price-measured(index * 10 + {probe_id}, text, actual, available); }}"
        );
        source.replace_range(start..start + "Text {".len(), &replacement);
        offset = at + replacement.len();
        probe_id += 1;
    }
    assert!(probe_id > 0);
    let root = source.find("inherits Window {").unwrap() + "inherits Window {".len();
    source.insert_str(root, "\n    in property <int> price-probe-generation;\n    callback price-measured(int, string, length, length);\n");
    source.insert_str(
        source.find("export component").unwrap(),
        r#"
component PriceProbe inherits Text {
    in property <int> probe-generation;
    callback measured(string, length, length);
    glyphs := Text {
        text: root.text;
        font-size: root.font-size;
        font-weight: root.font-weight;
        font-family: root.font-family;
        visible: false;
    }
    changed probe-generation => { root.measured(root.text, glyphs.preferred-width, root.width); }
}
"#,
    );
    source
}

#[test]
fn bundled_widgets_render_eight_slot_prices_without_elision() {
    let adapter = Rc::new(RefCell::new(None));
    slint::platform::set_platform(Box::new(PriceTestPlatform(adapter.clone()))).unwrap();
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut specs = vec![
        (
            "quote-board".to_string(),
            manifest_dir.join("ui/price-card-window.slint"),
            "PriceCardWindow".to_string(),
            286,
            194,
            5,
        ),
        (
            "mini-ticker".to_string(),
            manifest_dir.join("ui/price-card-window.slint"),
            "PriceCardWindow".to_string(),
            236,
            112,
            1,
        ),
    ];
    for entry in std::fs::read_dir(manifest_dir.join("plugins")).unwrap() {
        let path = entry.unwrap().path();
        if !path.join("widget.json").is_file() {
            continue;
        }
        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path.join("widget.json")).unwrap()).unwrap();
        let id = manifest["id"].as_str().unwrap();
        // Focus Ticker has its own end-to-end price fitting and hover regression.
        if id == "com.cryptohud.focus-ticker" {
            continue;
        }
        specs.push((
            id.to_string(),
            path.join(manifest["renderer"]["entry"].as_str().unwrap()),
            manifest["renderer"]["component"]
                .as_str()
                .unwrap()
                .to_string(),
            manifest["defaultSize"]["width"].as_i64().unwrap() as i32,
            manifest["defaultSize"]["height"].as_i64().unwrap() as i32,
            manifest["symbolLimit"].as_u64().unwrap() as usize,
        ));
    }
    let prices = [
        68000.12,
        3420.56,
        152.3478,
        602.45,
        0.000004110,
        88888888.0,
        99999999.0,
        99999999.9,
        -0.000004110,
        f64::MAX,
    ];
    let mut failures = Vec::new();
    for (id, path, component, width, height, count) in specs {
        // Mint Tile previews use a paper backdrop in both themes so the native
        // translucent drop shadow remains visible below the card.
        let source = instrument_prices(
            std::fs::read_to_string(&path).unwrap(),
            id == "com.cryptohud.mint-tile",
        );
        let compiled = spin_on::spin_on(Compiler::default().build_from_source(source, path));
        let definition = compiled
            .component(&component)
            .unwrap_or_else(|| panic!("{id}: {:?}", compiled.diagnostics().collect::<Vec<_>>()));
        let ui = definition.create().unwrap();
        crate::image_resampling::install_image_resampling(&ui).unwrap();
        let widget =
            crate::widget_host::WidgetUi::DynamicSlint(crate::widget_host::DynamicWidgetUi {
                instance: ui.clone_strong(),
            });
        if id == "mini-ticker" {
            ui.set_property("compact-mode", Value::Bool(true)).unwrap();
        }
        ui.set_property("content-opacity", Value::Number(100.0))
            .unwrap();
        ui.set_property("pairs-heading-text", Value::String("Pairs".into()))
            .unwrap();
        ui.set_property("empty-text", Value::String("No pairs".into()))
            .unwrap();
        let measurements = Rc::new(RefCell::new(BTreeMap::new()));
        let captured = measurements.clone();
        ui.set_callback("price-measured", move |args| {
            let [Value::Number(index), Value::String(text), Value::Number(actual), Value::Number(available)] = args else { panic!("invalid price measurement"); };
            captured.borrow_mut().insert(*index as i32, (text.to_string(), *actual, *available));
            Value::Void
        }).unwrap();
        ui.show().unwrap();
        let window = adapter.borrow().as_ref().unwrap().clone();
        let now = Instant::now();
        let base_symbols = ["BTC", "ETH", "SOL", "BNB", "PEPE", "DOGE", "ADA", "XRP"]
            .into_iter()
            .take(count.min(8))
            .map(|base| format!("binance:spot:{base}/USDT"))
            .collect::<Vec<_>>();
        let mut generation = 0;
        for scale in [0.3, 1.0, 1.5, 2.0] {
            ui.set_property("widget-scale", Value::Number(scale))
                .unwrap();
            ui.set_property(
                "widget-width",
                Value::Number((f64::from(width) * scale).round()),
            )
            .unwrap();
            ui.set_property(
                "widget-height",
                Value::Number((f64::from(height) * scale).round()),
            )
            .unwrap();
            ui.window().set_size(slint::LogicalSize::new(
                (width as f32 * scale as f32).round(),
                (height as f32 * scale as f32).round(),
            ));
            for theme in ["light", "dark"] {
                ui.set_property("theme-name", Value::String(theme.into()))
                    .unwrap();
                for (case, price) in prices.into_iter().enumerate() {
                    let mut symbols = base_symbols.clone();
                    if case == 4 {
                        symbols[0] = "binance:spot:PEPE/USDT".to_string();
                    }
                    let mut cache = crypto_hud_runtime::QuoteCache::new();
                    for (index, symbol) in symbols.iter().enumerate() {
                        cache.insert(
                            symbol.clone(),
                            crypto_hud_runtime::QuoteState::new(
                                if case == 0 {
                                    prices[index.min(4)]
                                } else {
                                    price
                                },
                                -4.86,
                                vec![110.0, 103.0, 104.0, 100.0],
                                crypto_hud_core::MarketDataSource::Binance,
                                now,
                            ),
                        );
                    }
                    let view = crypto_hud_runtime::build_widget_runtime_view(
                        crypto_hud_runtime::WidgetRuntimeViewParams {
                            widget_id: &id,
                            symbols: &symbols,
                            quote_cache: &cache,
                            source_prefix: "",
                            provider_labels: Default::default(),
                            labels: crypto_hud_runtime::RuntimeTextLabels {
                                isolate_numeric_values: case % 2 == 0,
                                ..Default::default()
                            },
                            has_market_error: false,
                            now,
                            display_options: Default::default(),
                        },
                    );
                    let display_options = crypto_hud_runtime::WidgetDisplayOptions {
                        hide_quote_asset: case % 3 == 1,
                        ..Default::default()
                    };
                    crate::widget_host::apply_runtime_view_to_widget(
                        &widget,
                        &view,
                        &[],
                        &[],
                        case % 2 == 1,
                        display_options,
                        scale as f32,
                    );
                    measurements.borrow_mut().clear();
                    let size = window.size();
                    let mut pixels = vec![
                        Rgb8Pixel {
                            r: 32,
                            g: 38,
                            b: 46
                        };
                        (size.width * size.height) as usize
                    ];
                    window.window().request_redraw();
                    window.draw_if_needed(|renderer| {
                        renderer.render(&mut pixels, size.width as usize);
                    });
                    for _ in 0..2 {
                        generation += 1;
                        ui.set_property("price-probe-generation", Value::Number(generation.into()))
                            .unwrap();
                        slint::platform::update_timers_and_animations();
                    }
                    assert!(
                        !measurements.borrow().is_empty(),
                        "{id}: no price text was measured"
                    );
                    for (text, actual, available) in measurements.borrow().values() {
                        if *actual > *available + 0.1 {
                            failures.push(format!("{id}/{theme}/{scale}: {text} needs {actual:.2}px, has {available:.2}px"));
                            break;
                        }
                    }
                    if scale == 1.0 && [0, 4, 5].contains(&case) {
                        if let Some(folder) =
                            std::env::var_os("CRYPTO_HUD_WIDGET_PRICE_CAPTURE_DIR")
                        {
                            let folder = PathBuf::from(folder);
                            std::fs::create_dir_all(&folder).unwrap();
                            let mut bytes =
                                format!("P6\n{} {}\n255\n", size.width, size.height).into_bytes();
                            for pixel in &pixels {
                                bytes.extend_from_slice(&[pixel.r, pixel.g, pixel.b]);
                            }
                            std::fs::write(folder.join(format!("{id}-{theme}-{case}.ppm")), bytes)
                                .unwrap();
                        }
                    }
                }
            }
        }
        ui.hide().unwrap();
    }
    assert!(
        failures.is_empty(),
        "Price clipping:\n{}",
        failures.join("\n")
    );
}
