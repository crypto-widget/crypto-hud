use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};

use slint::{
    platform::{
        software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
        Platform, PointerEventButton, WindowAdapter, WindowEvent,
    },
    ComponentHandle, LogicalPosition, Rgb8Pixel,
};
use slint_interpreter::{Compiler, ComponentInstance, Value};

struct TestPlatform(Rc<RefCell<Option<Rc<MinimalSoftwareWindow>>>>);

impl Platform for TestPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        self.0.replace(Some(window.clone()));
        Ok(window)
    }
}

#[derive(Clone, Copy, Debug)]
enum State {
    Fresh,
    Loading,
    Failed,
    Stale,
    Partial,
}

fn view(
    count: usize,
    state: State,
    locale: crate::i18n::Locale,
) -> crypto_hud_runtime::WidgetRuntimeView {
    // Keep old fixtures valid even immediately after Windows boots.
    let now = Instant::now() + Duration::from_secs(7200);
    let symbols = ["BTC", "ETH", "SOL", "BNB", "PEPE", "DOGE", "ADA", "XRP"]
        .into_iter()
        .take(count)
        .map(|asset| format!("binance:spot:{asset}/USDT"))
        .collect::<Vec<_>>();
    let prices = [
        68000.12, 3420.56, 152.3478, 602.45, 0.00000411, 0.12, 0.33, 0.51,
    ];
    let mut cache = crypto_hud_runtime::QuoteCache::new();
    for (index, symbol) in symbols.iter().enumerate() {
        if matches!(state, State::Loading | State::Failed)
            || (matches!(state, State::Partial) && index == count - 1)
        {
            continue;
        }
        cache.insert(
            symbol.clone(),
            crypto_hud_runtime::QuoteState::new_with_chart_status(
                prices[index],
                if index % 2 == 0 { -4.86 } else { 3.24 },
                Vec::new(),
                Vec::new(),
                crypto_hud_core::MarketDataSource::Binance,
                now - Duration::from_secs(if matches!(state, State::Stale) {
                    3600
                } else {
                    2
                }),
                None,
                None,
            ),
        );
    }
    let text = crate::i18n::text(locale);
    crypto_hud_runtime::build_widget_runtime_view(crypto_hud_runtime::WidgetRuntimeViewParams {
        widget_id: "status-strip-test",
        symbols: &symbols,
        quote_cache: &cache,
        source_prefix: text.source_prefix,
        provider_labels: Default::default(),
        labels: crypto_hud_runtime::RuntimeTextLabels {
            no_pairs: text.runtime_no_pairs,
            connecting: text.runtime_connecting,
            connection_error: text.runtime_connection_error,
            updated: text.runtime_updated,
            stale: text.runtime_stale,
            source_error: text.runtime_source_error,
            live_count_prefix: text.runtime_live_count_prefix,
            live_count_suffix: text.runtime_live_count_suffix,
            elapsed_second_unit: text.runtime_elapsed_second_unit,
            elapsed_minute_unit: text.runtime_elapsed_minute_unit,
            isolate_numeric_values: crate::i18n::is_rtl(locale),
        },
        has_market_error: matches!(state, State::Failed | State::Stale | State::Partial),
        now,
        display_options: Default::default(),
    })
}

fn number(ui: &ComponentInstance, name: &str) -> f64 {
    let Value::Number(value) = ui.get_property(name).unwrap() else {
        panic!("{name} is not numeric");
    };
    value
}

fn boolean(ui: &ComponentInstance, name: &str) -> bool {
    let Value::Bool(value) = ui.get_property(name).unwrap() else {
        panic!("{name} is not boolean");
    };
    value
}

#[derive(Clone, Debug)]
struct Row {
    text: String,
    visible: bool,
    actual: f64,
    available: f64,
    color: Value,
    arrow: Value,
    arrow_visible: bool,
    arrow_color: Value,
    change_actual: f64,
    change_available: f64,
}

struct Harness {
    ui: ComponentInstance,
    window: Rc<MinimalSoftwareWindow>,
    rows: Rc<RefCell<BTreeMap<i32, Row>>>,
    generation: Cell<i32>,
}

impl Harness {
    fn new() -> Self {
        let adapter = Rc::new(RefCell::new(None));
        slint::platform::set_platform(Box::new(TestPlatform(adapter.clone()))).unwrap();
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/com.cryptohud.status-strip/ui/main.slint");
        let source = std::fs::read_to_string(&path).unwrap().replace("\r\n", "\n")
            .replacen("export component StatusStrip inherits Window {", r#"export component StatusStrip inherits Window {
    in property <int> probe-generation;
    callback row-probed(int, string, bool, length, length, color, image, bool, color, length, length);
    out property <string> inspected-status: detail_text.text;
    out property <color> inspected-status-color: status_warning.background;
    out property <bool> inspected-warning-visible: status_warning.visible;
    out property <color> inspected-gain: root.gain;
    out property <color> inspected-loss: root.loss;
    out property <color> inspected-gain-contrast: root.gain-contrast;
    out property <color> inspected-loss-contrast: root.loss-contrast;
    out property <color> inspected-warning: root.warning;
    out property <bool> inspected-empty: empty_label.visible;
    out property <bool> inspected-lock: lock_button.opacity > 0;
    out property <bool> inspected-focus: root.show-lock-focus && lock_focus.has-focus;
    out property <bool> inspected-details: status_details.visible;
    out property <length> inspected-lock-x: lock_button.x + lock_button.width / 2;
    out property <length> inspected-lock-y: lock_button.y + lock_button.height / 2;
"#, 1)
            .replacen("price_text := Text {", r#"price_text := Text {
                property <int> observed-generation: root.probe-generation;
                changed observed-generation => {
                    root.row-probed(index, self.text, self.visible, glyphs.preferred-width, self.width, change_text.color,
                        change_arrow.source, change_arrow.visible, change_arrow.colorize, change_glyphs.preferred-width, change_text.width);
                }
                glyphs := Text {
                    text: price_text.text; font-size: price_text.font-size;
                    font-weight: price_text.font-weight; visible: false;
                }
                change_glyphs := Text {
                    text: change_text.text; font-size: change_text.font-size;
                    font-weight: change_text.font-weight; visible: false;
                }
"#, 1)
            .replacen("background: transparent;\n    default-font-family:", "background: #eceae5;\n    default-font-family:", 1);
        let compiled = spin_on::spin_on(Compiler::default().build_from_source(source, path));
        let definition = compiled
            .component("StatusStrip")
            .unwrap_or_else(|| panic!("{:?}", compiled.diagnostics().collect::<Vec<_>>()));
        let ui = definition.create().unwrap();
        let rows = Rc::new(RefCell::new(BTreeMap::new()));
        let captured = rows.clone();
        ui.set_callback("row-probed", move |values| {
            let [Value::Number(index), Value::String(text), Value::Bool(visible), Value::Number(actual), Value::Number(available), color,
                arrow, Value::Bool(arrow_visible), arrow_color, Value::Number(change_actual), Value::Number(change_available)] = values else { panic!("invalid probe"); };
            captured.borrow_mut().insert(*index as i32, Row { text: text.to_string(), visible: *visible, actual: *actual,
                available: *available, color: color.clone(), arrow: arrow.clone(), arrow_visible: *arrow_visible,
                arrow_color: arrow_color.clone(), change_actual: *change_actual, change_available: *change_available });
            Value::Void
        }).unwrap();
        ui.set_property("content-opacity", Value::Number(100.0))
            .unwrap();
        ui.show().unwrap();
        let window = adapter.borrow().as_ref().unwrap().clone();
        Self {
            ui,
            window,
            rows,
            generation: Cell::new(0),
        }
    }

    fn apply(
        &self,
        count: usize,
        state: State,
        locale: crate::i18n::Locale,
        scale: f64,
        theme: &str,
    ) -> crypto_hud_runtime::WidgetRuntimeView {
        let width = ((122 * count.clamp(1, 5) + 8) as f64 * scale).round();
        let height = (92.0 * scale).round();
        self.ui
            .set_property("widget-scale", Value::Number(scale))
            .unwrap();
        self.ui
            .set_property("widget-width", Value::Number(width))
            .unwrap();
        self.ui
            .set_property("widget-height", Value::Number(height))
            .unwrap();
        self.ui
            .set_property("theme-name", Value::String(theme.into()))
            .unwrap();
        self.ui
            .window()
            .set_size(slint::LogicalSize::new(width as f32, height as f32));
        let widget =
            crate::widget_host::WidgetUi::DynamicSlint(crate::widget_host::DynamicWidgetUi {
                instance: self.ui.clone_strong(),
            });
        widget.set_rtl_layout(crate::i18n::is_rtl(locale));
        widget.set_change_period_text(crate::i18n::change_period_label(locale).into());
        widget.set_layout_lock_text(crate::i18n::text(locale).always_on_top.into());
        widget.set_empty_text(crate::i18n::text(locale).runtime_no_pairs.into());
        let view = view(count, state, locale);
        crate::widget_host::apply_runtime_view_to_widget(
            &widget,
            &view,
            &[],
            &[],
            false,
            Default::default(),
            scale as f32,
        );
        view
    }

    fn measure(&self) -> BTreeMap<i32, Row> {
        self.rows.borrow_mut().clear();
        self.generation.set(self.generation.get() + 1);
        self.ui
            .set_property(
                "probe-generation",
                Value::Number(self.generation.get().into()),
            )
            .unwrap();
        slint::platform::update_timers_and_animations();
        self.rows.borrow().clone()
    }

    fn render(&self, name: &str) {
        let size = self.window.size();
        let mut pixels = vec![
            Rgb8Pixel {
                r: 236,
                g: 234,
                b: 229
            };
            (size.width * size.height) as usize
        ];
        self.window.window().request_redraw();
        assert!(self.window.draw_if_needed(|renderer| {
            renderer.render(&mut pixels, size.width as usize);
        }));
        if let Some(folder) = std::env::var_os("CRYPTO_HUD_STATUS_STRIP_CAPTURE_DIR") {
            let folder = PathBuf::from(folder);
            std::fs::create_dir_all(&folder).unwrap();
            let mut bytes = format!("P6\n{} {}\n255\n", size.width, size.height).into_bytes();
            for pixel in pixels {
                bytes.extend_from_slice(&[pixel.r, pixel.g, pixel.b]);
            }
            std::fs::write(folder.join(format!("{name}.ppm")), bytes).unwrap();
        }
    }
}

#[test]
fn status_strip_prices_themes_readiness_and_localized_status() {
    let h = Harness::new();
    for theme in ["light", "dark"] {
        for count in 1..=8 {
            for scale in [0.3, 1.0, 1.25, 2.0] {
                let data = h.apply(count, State::Fresh, crate::i18n::Locale::En, scale, theme);
                h.render(&format!("strip-{theme}-{count}-{}", (scale * 100.0) as i32));
                let rows = h.measure();
                assert_eq!(rows.len(), count);
                for (index, row) in &rows {
                    assert_eq!(row.text, data.quote_rows[*index as usize].price);
                    assert!(
                        row.actual <= row.available + 0.1,
                        "{theme}/{count}/{scale}: {row:?}"
                    );
                    let expected =
                        h.ui.get_property(if index % 2 == 0 {
                            "inspected-loss"
                        } else {
                            if theme == "light" && *index == 3 {
                                "inspected-gain-contrast"
                            } else {
                                "inspected-gain"
                            }
                        })
                        .unwrap();
                    assert_eq!(row.color, expected);
                    assert_eq!(row.arrow_color, row.color);
                    assert!(row.arrow_visible);
                    assert!(
                        row.change_actual <= row.change_available + 0.1,
                        "{theme}/{count}/{scale}: change should fit beside its arrow and period: {row:?}"
                    );
                }
                if count > 1 {
                    assert_ne!(rows[&0].arrow, rows[&1].arrow);
                }
                assert!(!boolean(&h.ui, "inspected-lock"));
                assert!(!boolean(&h.ui, "inspected-details"));
                assert!(!boolean(&h.ui, "inspected-warning-visible"));
            }
        }
        for locale in crate::i18n::Locale::ALL {
            for state in [State::Loading, State::Failed, State::Stale, State::Partial] {
                let data = h.apply(3, state, locale, 1.0, theme);
                h.render(&format!("strip-{theme}-{state:?}-{locale:?}"));
                assert_eq!(
                    h.ui.get_property("inspected-status").unwrap(),
                    Value::String(format!("{}\n{}", data.updated_text, data.source_text).into())
                );
                let rows = h.measure();
                for (index, row) in rows {
                    assert_eq!(row.visible, data.quote_price_ready[index as usize]);
                    assert_eq!(row.arrow_visible, data.quote_price_ready[index as usize]);
                }
                assert_eq!(
                    h.ui.get_property("inspected-status-color").unwrap(),
                    h.ui.get_property("inspected-warning").unwrap()
                );
                assert!(boolean(&h.ui, "inspected-warning-visible"));
                h.ui.window().dispatch_event(WindowEvent::PointerMoved {
                    position: LogicalPosition::new(
                        number(&h.ui, "widget-width") as f32 - 10.0,
                        12.0,
                    ),
                });
                assert!(boolean(&h.ui, "inspected-details"));
                h.ui.window().dispatch_event(WindowEvent::PointerExited);
            }
        }
        h.apply(0, State::Fresh, crate::i18n::Locale::En, 1.0, theme);
        h.render(&format!("strip-{theme}-empty"));
        assert!(boolean(&h.ui, "inspected-empty"));
        h.apply(5, State::Fresh, crate::i18n::Locale::En, 1.0, theme);
        h.render(&format!("strip-{theme}-standard-colors"));
        let normal_colors = h.measure();
        h.ui.set_property("red-up-enabled", Value::Bool(true))
            .unwrap();
        h.render(&format!("strip-{theme}-red-up"));
        let rows = h.measure();
        assert_eq!(rows[&0].color, h.ui.get_property("inspected-gain").unwrap());
        assert_eq!(rows[&1].color, h.ui.get_property("inspected-loss").unwrap());
        assert_eq!(
            rows[&3].color,
            h.ui.get_property(if theme == "light" {
                "inspected-loss-contrast"
            } else {
                "inspected-loss"
            })
            .unwrap()
        );
        for (index, row) in &rows {
            assert_eq!(row.arrow, normal_colors[index].arrow);
            assert_eq!(row.arrow_color, row.color);
        }
        h.ui.set_property("red-up-enabled", Value::Bool(false))
            .unwrap();
        for dpi in [1.25, 1.5, 2.0] {
            h.ui.window()
                .dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor: dpi });
            h.ui.window().set_size(slint::LogicalSize::new(618.0, 92.0));
            h.render(&format!("strip-{theme}-dpi-{}", (dpi * 100.0) as i32));
            assert_eq!(
                h.window.size(),
                slint::LogicalSize::new(618.0, 92.0).to_physical(dpi)
            );
        }
        h.ui.window()
            .dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor: 1.0 });
    }
    h.ui.hide().unwrap();
}

#[test]
fn status_strip_floating_lock_and_status_details_preserve_dragging() {
    let h = Harness::new();
    h.apply(1, State::Fresh, crate::i18n::Locale::En, 1.0, "light");
    let toggles = Rc::new(Cell::new(0));
    let counted = toggles.clone();
    let weak = h.ui.as_weak();
    h.ui.set_callback("toggle-layout-lock", move |_| {
        let ui = weak.upgrade().unwrap();
        ui.set_property("layout-locked", Value::Bool(!boolean(&ui, "layout-locked")))
            .unwrap();
        counted.set(counted.get() + 1);
        Value::Void
    })
    .unwrap();
    let drags = Rc::new(Cell::new(0));
    let counted = drags.clone();
    h.ui.set_callback("drag-move", move |_| {
        counted.set(counted.get() + 1);
        Value::Void
    })
    .unwrap();
    for theme in ["light", "dark"] {
        h.apply(1, State::Fresh, crate::i18n::Locale::En, 1.0, theme);
        h.ui.window().dispatch_event(WindowEvent::PointerExited);
        assert!(!boolean(&h.ui, "inspected-lock"));
        h.ui.window().dispatch_event(WindowEvent::PointerMoved {
            position: LogicalPosition::new(20.0, 30.0),
        });
        assert!(boolean(&h.ui, "inspected-lock"));
        h.render(&format!("strip-{theme}-hover"));
        let position = LogicalPosition::new(
            number(&h.ui, "inspected-lock-x") as f32,
            number(&h.ui, "inspected-lock-y") as f32,
        );
        assert!(
            position.y >= 66.0,
            "lock must stay below the price and pair"
        );
        for locked in [true, false] {
            h.ui.window()
                .dispatch_event(WindowEvent::PointerMoved { position });
            h.ui.window().dispatch_event(WindowEvent::PointerPressed {
                position,
                button: PointerEventButton::Left,
            });
            h.ui.window().dispatch_event(WindowEvent::PointerReleased {
                position,
                button: PointerEventButton::Left,
            });
            assert_eq!(boolean(&h.ui, "layout-locked"), locked);
            h.ui.window().dispatch_event(WindowEvent::PointerExited);
            assert!(
                !boolean(&h.ui, "inspected-lock"),
                "locked overlay must disappear after pointer leaves"
            );
            let before = drags.get();
            let position = LogicalPosition::new(20.0, 84.0);
            h.ui.window()
                .dispatch_event(WindowEvent::PointerMoved { position });
            assert!(boolean(&h.ui, "inspected-details"));
            h.render(&format!("strip-{theme}-details"));
            h.ui.window().dispatch_event(WindowEvent::PointerPressed {
                position,
                button: PointerEventButton::Left,
            });
            h.ui.window().dispatch_event(WindowEvent::PointerMoved {
                position: LogicalPosition::new(40.0, 84.0),
            });
            h.ui.window().dispatch_event(WindowEvent::PointerReleased {
                position,
                button: PointerEventButton::Left,
            });
            assert_eq!(drags.get() > before, !locked);
        }
    }
    assert_eq!(toggles.get(), 4);
    h.ui.window().dispatch_event(WindowEvent::PointerExited);
    h.ui.window()
        .dispatch_event(WindowEvent::WindowActiveChanged(true));
    for key in [
        slint::platform::Key::Tab,
        slint::platform::Key::Space,
        slint::platform::Key::Return,
    ] {
        h.ui.window()
            .dispatch_event(WindowEvent::KeyPressed { text: key.into() });
        h.ui.window()
            .dispatch_event(WindowEvent::KeyReleased { text: key.into() });
    }
    assert_eq!(toggles.get(), 6);
    assert!(boolean(&h.ui, "inspected-focus"));
    h.ui.window().dispatch_event(WindowEvent::KeyPressRepeated {
        text: slint::platform::Key::Space.into(),
    });
    assert_eq!(toggles.get(), 6);
    h.render("strip-keyboard-focus");
    h.ui.hide().unwrap();
}

#[test]
fn status_strip_hover_has_no_partial_repaint_seams() {
    let h = Harness::new();
    let mut failures = Vec::new();
    for scale in [0.3, 1.0, 1.25] {
        for dpi in [1.0, 1.25, 1.5, 2.0] {
            h.ui.window()
                .dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor: dpi });
            for theme in ["light", "dark"] {
                h.apply(3, State::Fresh, crate::i18n::Locale::En, scale, theme);
                for opacity in [100.0, 70.0] {
                    h.ui.set_property("content-opacity", Value::Number(opacity))
                        .unwrap();
                    let size = h.window.size();
                    let mut pixels =
                        vec![Rgb8Pixel { r: 0, g: 0, b: 0 }; (size.width * size.height) as usize];
                    let paint = |pixels: &mut [Rgb8Pixel], mode| {
                        h.window.window().request_redraw();
                        assert!(h.window.draw_if_needed(|renderer| {
                            renderer.set_repaint_buffer_type(mode);
                            renderer.render(pixels, size.width as usize);
                        }));
                    };
                    h.ui.window().dispatch_event(WindowEvent::PointerExited);
                    paint(&mut pixels, RepaintBufferType::NewBuffer);
                    for point in [Some((30.0, 30.0)), Some((30.0, 84.0)), None] {
                        paint(&mut pixels, RepaintBufferType::ReusedBuffer);
                        if let Some((x, y)) = point {
                            h.ui.window().dispatch_event(WindowEvent::PointerMoved {
                                position: LogicalPosition::new(x * scale as f32, y * scale as f32),
                            });
                        } else {
                            h.ui.window().dispatch_event(WindowEvent::PointerExited);
                        }
                        paint(&mut pixels, RepaintBufferType::ReusedBuffer);
                        let partial = pixels.clone();
                        paint(&mut pixels, RepaintBufferType::NewBuffer);
                        let different = partial
                            .iter()
                            .zip(&pixels)
                            .enumerate()
                            .filter(|(_, (a, b))| a != b)
                            .map(|(i, (a, b))| {
                                (i % size.width as usize, i / size.width as usize, *a, *b)
                            })
                            .collect::<Vec<_>>();
                        if !different.is_empty() {
                            failures.push(format!(
                                "{theme}/{scale}/{dpi}/{opacity}/{point:?}: {} pixels; {:?}",
                                different.len(),
                                &different[..different.len().min(2)]
                            ));
                        }
                    }
                }
            }
        }
    }
    h.ui.hide().unwrap();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn status_strip_preserves_legacy_theme_layout_and_symbol_limit() {
    let catalog =
        crate::plugin::PluginCatalog::discover(vec![
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("plugins")
        ]);
    let plugin = catalog.find("com.cryptohud.status-strip").unwrap();
    assert!(plugin.is_available());
    assert_eq!(crate::plugin::default_theme_id(&plugin), "default");
    assert_eq!(plugin.preview_images.len(), 2);
    let definitions = crate::state_bridge::widget_definitions_from_catalog(&catalog);
    for count in 1..=8 {
        let symbols = ["BTC", "ETH", "SOL", "BNB", "PEPE", "DOGE", "ADA", "XRP"]
            .into_iter()
            .take(count)
            .map(|asset| format!("binance:spot:{asset}/USDT"))
            .collect::<Vec<_>>();
        let mut store: crypto_hud_shell_state::LayoutStore = serde_json::from_value(serde_json::json!({
            "widgets": [{
                "id":"status-strip-legacy", "plugin_id":plugin.id, "name":"My Strip",
                "layout":{"x":90,"y":100,"width":(122*count.min(5)+8),"height":92,
                    "scale_percent":100,"opacity_percent":75,"locked":true,"always_on_top":false},
                "symbols":symbols, "config":{"theme":"default"}
            }]
        })).unwrap();
        let mut expected = store.widgets[0].clone();
        expected.symbols.truncate(5);
        crypto_hud_shell_state::normalize_store_with_catalog(
            &mut store,
            0,
            &definitions,
            (1920, 1080),
        );
        assert_eq!(store.widgets[0], expected);
    }
}
