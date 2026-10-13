use std::{
    cell::{Cell, RefCell},
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, Instant},
};

use slint::{
    platform::{
        software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
        Platform, PointerEventButton, WindowAdapter, WindowEvent,
    },
    ComponentHandle, Image, LogicalPosition, Rgb8Pixel,
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

#[derive(Clone, Copy)]
enum DataState {
    Ready,
    Loading,
    Failed,
    Cached,
    ChartLoading,
    ChartFailed,
}

fn runtime_view(
    asset: &str,
    price: f64,
    state: DataState,
    locale: crate::i18n::Locale,
) -> crypto_hud_runtime::WidgetRuntimeView {
    runtime_view_with_change(asset, price, -4.86, state, locale)
}

fn runtime_view_with_change(
    asset: &str,
    price: f64,
    change: f64,
    state: DataState,
    locale: crate::i18n::Locale,
) -> crypto_hud_runtime::WidgetRuntimeView {
    // Instant's platform epoch can be younger than the stale fixture age.
    let now = Instant::now() + Duration::from_secs(3600);
    let symbols = if asset.is_empty() {
        Vec::new()
    } else {
        vec![format!("binance:spot:{asset}/USDT")]
    };
    let mut cache = crypto_hud_runtime::QuoteCache::new();
    if !matches!(state, DataState::Loading | DataState::Failed) {
        let chart_ready = matches!(state, DataState::Ready | DataState::Cached);
        let timestamp = now
            - Duration::from_secs(if matches!(state, DataState::Cached) {
                3600
            } else {
                2
            });
        for symbol in &symbols {
            cache.insert(
                symbol.clone(),
                crypto_hud_runtime::QuoteState::new_with_chart_status(
                    price,
                    change,
                    if chart_ready {
                        [
                            128.0, 142.0, 138.0, 145.0, 131.0, 135.0, 125.0, 118.0, 121.0, 108.0,
                            100.0,
                        ]
                        .map(|close| {
                            (if change >= 0.0 { 250.0 - close } else { close }) * price / 100.0
                        })
                        .to_vec()
                    } else {
                        Vec::new()
                    },
                    Vec::new(),
                    crypto_hud_core::MarketDataSource::Binance,
                    timestamp,
                    chart_ready.then_some(timestamp),
                    matches!(state, DataState::ChartFailed)
                        .then(|| "candle request timed out".to_string()),
                ),
            );
        }
    }
    let text = crate::i18n::text(locale);
    crypto_hud_runtime::build_widget_runtime_view(crypto_hud_runtime::WidgetRuntimeViewParams {
        widget_id: "trust-card-test",
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
        has_market_error: matches!(state, DataState::Failed | DataState::Cached),
        now,
        display_options: Default::default(),
    })
}

fn instrument(source: String) -> String {
    let anchor = "export component TrustCard inherits Window {";
    assert!(source.contains(anchor));
    // Inspection properties are compiled only for these tests.
    source
        .replacen(
            anchor,
            &format!(
                r#"{anchor}
    in property <int> probe-generation;
    out property <image> inspected-icon-source: root.displayed-quote-icon;
    in-out property <length> inspected-price-width;
    in-out property <length> inspected-price-font;
    in-out property <length> fitted-price-width;
    in-out property <string> inspected-price;
    in-out property <string> inspected-change;
    in-out property <color> inspected-change-color;
    in-out property <string> inspected-period;
    out property <bool> inspected-status-visible: status_text.visible;
    out property <string> inspected-status: status_text.text;
    out property <string> inspected-source: status_source.text;
    in-out property <bool> inspected-chart-visible;
    out property <bool> inspected-lock-visible: lock_button.opacity > 0;
    out property <bool> inspected-lock-focused: lock_focus.has-focus;
    out property <bool> inspected-lock-focus-visible: root.show-lock-focus && lock_focus.has-focus;
    out property <color> inspected-chart-color: root.chart-color;
    out property <color> inspected-gain-color: root.gain-color;
    out property <color> inspected-loss-color: root.loss-color;
"#
            ),
            1,
        )
        .replacen(
            "price_text := Text {",
            r#"price_text := Text {
                property <int> inspected-generation: root.probe-generation;
                changed inspected-generation => {
                    root.inspected-price-width = self.width;
                    root.inspected-price-font = self.font-size;
                    root.fitted-price-width = fitted_measure.preferred-width;
                    root.inspected-price = self.text;
                    root.inspected-change = change_text.text;
                    root.inspected-change-color = change_text.color;
                    root.inspected-period = change_period.text;
                    root.inspected-chart-visible = chart.visible;
                }
                fitted_measure := Text {
                    text: parent.text;
                    font-size: parent.font-size;
                    font-weight: parent.font-weight;
                    visible: false;
                }
"#,
            1,
        )
        .replacen("background: transparent;", "background: #eceae5;", 1)
}

// Deterministic existing artwork, cropped only in this test fixture. No network
// or machine-specific icon cache is required to reproduce plugin previews.
fn fixture_icon(asset: &str) -> Image {
    let (x, y, width, height): (u32, u32, u32, u32) = match asset {
        "BTC" => (39, 52, 177, 176),
        "ETH" => (326, 48, 117, 181),
        "SOL" => (556, 69, 170, 142),
        "BNB" => (806, 48, 181, 180),
        "PEPE" => (808, 308, 168, 155),
        _ => return Image::default(),
    };
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("plugins/com.cryptohud.market-compass/ui/champagne-coin-atlas-200.png");
    let source = Image::load_from_path(&path).unwrap().to_rgba8().unwrap();
    let (x, y, width, height) = (
        x.div_ceil(2),
        y.div_ceil(2),
        width.div_ceil(2),
        height.div_ceil(2),
    );
    let mut pixels = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(width, height);
    for row in 0..height {
        let from = ((y + row) * source.width() + x) as usize;
        let to = (row * width) as usize;
        pixels.make_mut_slice()[to..to + width as usize]
            .copy_from_slice(&source.as_slice()[from..from + width as usize]);
    }
    Image::from_rgba8(pixels)
}

struct Harness {
    ui: ComponentInstance,
    widget: crate::widget_host::WidgetUi,
    window: Rc<MinimalSoftwareWindow>,
    capture_dir: Option<PathBuf>,
}

impl Harness {
    fn new() -> Self {
        let adapter = Rc::new(RefCell::new(None));
        slint::platform::set_platform(Box::new(TestPlatform(adapter.clone()))).unwrap();
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/com.cryptohud.trust-card/ui/main.slint");
        let compiled = spin_on::spin_on(
            Compiler::default()
                .build_from_source(instrument(std::fs::read_to_string(&path).unwrap()), path),
        );
        let ui = compiled
            .component("TrustCard")
            .unwrap_or_else(|| {
                panic!(
                    "Trust Card should compile: {:?}",
                    compiled.diagnostics().collect::<Vec<_>>()
                )
            })
            .create()
            .unwrap();
        crate::image_resampling::install_image_resampling(&ui).unwrap();
        let widget =
            crate::widget_host::WidgetUi::DynamicSlint(crate::widget_host::DynamicWidgetUi {
                instance: ui.clone_strong(),
            });
        ui.set_property("content-opacity", Value::Number(100.0))
            .unwrap();
        ui.show().unwrap();
        ui.window().set_size(slint::LogicalSize::new(260.0, 180.0));
        let window = adapter.borrow().as_ref().unwrap().clone();
        Self {
            ui,
            widget,
            window,
            capture_dir: std::env::var_os("CRYPTO_HUD_TRUST_CARD_CAPTURE_DIR").map(PathBuf::from),
        }
    }

    fn scale(&self, scale: f64, dpi: f32) {
        self.ui
            .window()
            .dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor: dpi });
        for (property, value) in [
            ("widget-scale", scale),
            ("widget-width", 260.0 * scale),
            ("widget-height", 180.0 * scale),
        ] {
            self.ui
                .set_property(property, Value::Number(value))
                .unwrap();
        }
        self.ui.window().set_size(slint::LogicalSize::new(
            260.0 * scale as f32,
            180.0 * scale as f32,
        ));
    }

    fn apply(
        &self,
        view: &crypto_hud_runtime::WidgetRuntimeView,
        asset: &str,
        scale: f32,
        locale: crate::i18n::Locale,
    ) {
        self.apply_pending(view, asset, scale, locale);
        // Repeater rows update lazily; lay out the new host model before probes
        // inspect the Text instances from the previous data state.
        let size = self.window.size();
        let mut pixels = vec![Rgb8Pixel { r: 0, g: 0, b: 0 }; (size.width * size.height) as usize];
        self.paint(&mut pixels, RepaintBufferType::NewBuffer);
    }

    fn apply_pending(
        &self,
        view: &crypto_hud_runtime::WidgetRuntimeView,
        asset: &str,
        scale: f32,
        locale: crate::i18n::Locale,
    ) {
        let text = crate::i18n::text(locale);
        self.widget
            .set_change_period_text(crate::i18n::change_period_label(locale).into());
        self.widget.set_layout_lock_text(text.always_on_top.into());
        self.widget.set_rtl_layout(crate::i18n::is_rtl(locale));
        self.ui
            .set_property("empty-text", Value::String(text.runtime_no_pairs.into()))
            .unwrap();
        let icons = if asset.is_empty() {
            Vec::new()
        } else {
            vec![fixture_icon(asset)]
        };
        crate::widget_host::apply_runtime_view_to_widget(
            &self.widget,
            view,
            &icons,
            &[!icons.is_empty()],
            !icons.is_empty(),
            Default::default(),
            scale,
        );
        slint::platform::update_timers_and_animations();
    }

    fn boolean(&self, name: &str) -> bool {
        self.refresh_measurements();
        let Value::Bool(value) = self.ui.get_property(name).unwrap() else {
            panic!("{name} must be boolean")
        };
        value
    }

    fn number(&self, name: &str) -> f64 {
        self.refresh_measurements();
        let Value::Number(value) = self.ui.get_property(name).unwrap() else {
            panic!("{name} must be numeric")
        };
        value
    }

    fn string(&self, name: &str) -> String {
        self.refresh_measurements();
        let Value::String(value) = self.ui.get_property(name).unwrap() else {
            panic!("{name} must be text")
        };
        value.to_string()
    }

    fn refresh_measurements(&self) {
        let Value::Number(value) = self.ui.get_property("probe-generation").unwrap() else {
            unreachable!()
        };
        self.ui
            .set_property("probe-generation", Value::Number(value + 1.0))
            .unwrap();
        slint::platform::update_timers_and_animations();
    }

    fn paint(&self, pixels: &mut [Rgb8Pixel], mode: RepaintBufferType) {
        self.ui.window().request_redraw();
        assert!(self.window.draw_if_needed(|renderer| {
            renderer.set_repaint_buffer_type(mode);
            renderer.render(pixels, self.window.size().width as usize);
        }));
    }

    fn capture(&self, name: &str) {
        let size = self.window.size();
        let mut pixels = vec![Rgb8Pixel { r: 0, g: 0, b: 0 }; (size.width * size.height) as usize];
        self.paint(&mut pixels, RepaintBufferType::NewBuffer);
        if let Some(folder) = &self.capture_dir {
            self.save(folder, name, &pixels);
        }
    }

    fn save(&self, folder: &Path, name: &str, pixels: &[Rgb8Pixel]) {
        std::fs::create_dir_all(folder).unwrap();
        let size = self.window.size();
        let mut bytes = format!("P6\n{} {}\n255\n", size.width, size.height).into_bytes();
        for pixel in pixels {
            bytes.extend_from_slice(&[pixel.r, pixel.g, pixel.b]);
        }
        std::fs::write(folder.join(format!("{name}.ppm")), bytes).unwrap();
    }

    fn move_pointer(&self, x: f32, y: f32) {
        self.ui.window().dispatch_event(WindowEvent::PointerMoved {
            position: LogicalPosition::new(x, y),
        });
    }
}

#[test]
fn trust_card_fits_eight_slot_prices_and_preserves_localized_data_states() {
    use crate::i18n::Locale;
    let h = Harness::new();
    crate::icon_sampling_tests::verify_card_icon_sampling(
        &h.ui,
        &h.widget,
        &runtime_view("BTC", 68000.12, DataState::Ready, Locale::En),
        (260.0, 180.0),
        38.0,
        |name| h.capture(&format!("trust-card-{name}")),
    );
    for scale in [0.3, 1.0, 1.25, 2.0] {
        h.scale(scale, 1.0);
        for theme in ["light", "dark"] {
            h.ui.set_property("theme-name", Value::String(theme.into()))
                .unwrap();
            for (asset, price) in [
                ("BTC", 68000.12),
                ("ETH", 3420.56),
                ("SOL", 152.3478),
                ("BNB", 602.45),
                ("PEPE", 0.00000411),
                ("BTC", 88888888.0),
            ] {
                let view = runtime_view(asset, price, DataState::Ready, Locale::En);
                h.apply(&view, asset, scale as f32, Locale::En);
                h.ui.window().dispatch_event(WindowEvent::PointerExited);
                let name = if price == 88888888.0 {
                    "eight-digits"
                } else {
                    asset
                };
                h.capture(&format!(
                    "trust-card-{name}-{theme}-{}",
                    (scale * 100.0) as i32
                ));
                assert_eq!(h.string("inspected-price"), view.quote_rows[0].price);
                assert!(
                    h.number("fitted-price-width") > 0.0
                        && h.number("fitted-price-width")
                            <= h.number("inspected-price-width") + 0.1,
                    "{name}/{theme}/{scale}: price must fit without ellipsis"
                );
                assert_eq!(
                    h.string("inspected-period"),
                    crate::i18n::change_period_label(Locale::En)
                );
                assert!(!h.boolean("inspected-lock-visible"));
                assert!(!h.boolean("inspected-status-visible"));
                assert!(h.boolean("inspected-chart-visible"));
            }
        }
    }
    h.scale(1.0, 1.0);
    for theme in ["light", "dark"] {
        h.ui.set_property("theme-name", Value::String(theme.into()))
            .unwrap();
        for locale in Locale::ALL {
            for (name, state) in [
                ("loading", DataState::Loading),
                ("failure", DataState::Failed),
            ] {
                let view = runtime_view("BTC", 68000.12, state, locale);
                h.apply(&view, "BTC", 1.0, locale);
                assert_eq!(h.string("inspected-price"), "—");
                assert!(!h.boolean("inspected-chart-visible"));
                assert!(h.boolean("inspected-status-visible"));
                assert!(h.string("inspected-status").contains(&view.updated_text));
                if [Locale::En, Locale::ZhHans, Locale::Ar].contains(&locale) {
                    h.capture(&format!("trust-card-{name}-{theme}-{locale:?}"));
                }
            }
        }
        let ready = runtime_view("BTC", 68000.12, DataState::Ready, Locale::En);
        h.apply(&ready, "BTC", 1.0, Locale::En);
        let ready_font = h.number("inspected-price-font");
        for (name, state) in [
            ("cached", DataState::Cached),
            ("chart-loading", DataState::ChartLoading),
            ("chart-failure", DataState::ChartFailed),
        ] {
            let view = runtime_view("BTC", 68000.12, state, Locale::En);
            h.apply(&view, "BTC", 1.0, Locale::En);
            assert_eq!(h.string("inspected-price"), ready.quote_rows[0].price);
            assert_eq!(h.number("inspected-price-font"), ready_font);
            assert!(h.boolean("inspected-status-visible"));
            assert!(!h.boolean("inspected-chart-visible"));
            assert_eq!(h.string("inspected-source"), view.source_text);
            assert_eq!(
                h.ui.get_property("updated-text").unwrap(),
                Value::String(view.updated_text.into())
            );
            h.capture(&format!("trust-card-{name}-{theme}"));
        }
        let empty = runtime_view("", 0.0, DataState::Ready, Locale::En);
        h.apply(&empty, "", 1.0, Locale::En);
        h.capture(&format!("trust-card-empty-{theme}"));
        h.apply(&ready, "", 1.0, Locale::En);
        h.capture(&format!("trust-card-no-logo-{theme}"));
        let long = runtime_view("VERYLONGTOKEN", 68000.12, DataState::Ready, Locale::En);
        h.apply(&long, "", 1.0, Locale::En);
        h.capture(&format!("trust-card-long-symbol-{theme}"));
        for (asset, price, change) in [("BTC", 68000.12, -4.86), ("PEPE", 0.000012, 2.41)] {
            let view = runtime_view_with_change(asset, price, change, DataState::Ready, Locale::En);
            h.apply(&view, asset, 1.0, Locale::En);
            for red_up in [false, true] {
                h.ui.set_property("red-up-enabled", Value::Bool(red_up))
                    .unwrap();
                h.refresh_measurements();
                let expected =
                    h.ui.get_property(if (change >= 0.0) != red_up {
                        "inspected-gain-color"
                    } else {
                        "inspected-loss-color"
                    })
                    .unwrap();
                assert_eq!(
                    h.ui.get_property("inspected-change-color").unwrap(),
                    expected
                );
                assert_eq!(
                    h.ui.get_property("inspected-chart-color").unwrap(),
                    expected
                );
                h.capture(&format!("trust-card-{asset}-red-up-{red_up}-{theme}"));
            }
        }
        h.ui.set_property("red-up-enabled", Value::Bool(false))
            .unwrap();
        h.apply(&ready, "BTC", 1.0, Locale::En);
        h.scale(1.0, 1.25);
        h.capture(&format!("trust-card-BTC-{theme}-dpi-125"));
        assert_eq!(h.window.size(), slint::PhysicalSize::new(325, 225));
        assert!(h.number("fitted-price-width") <= h.number("inspected-price-width") + 0.1);
        h.scale(1.0, 1.0);
    }
    h.scale(2.0, 1.0);
    h.ui.set_property("theme-name", Value::String("dark".into()))
        .unwrap();
    h.ui.set_property("layout-locked", Value::Bool(true))
        .unwrap();
    h.move_pointer(458.0, 70.0);
    assert!(!h.boolean("inspected-status-visible"));
    assert!(h.boolean("inspected-chart-visible"));
    h.capture("trust-card-BTC-dark-200-locked-hover");
    h.ui.hide().unwrap();
}

#[test]
fn trust_card_quote_refresh_partial_repaint_preserves_an_unchanged_chart() {
    use crate::i18n::Locale;
    let h = Harness::new();
    h.scale(1.0, 1.25);
    let before = runtime_view("BTC", 68000.12, DataState::Ready, Locale::En);
    let after = runtime_view_with_change("BTC", 68012.34, -4.82, DataState::Ready, Locale::ZhHans);
    assert_ne!(before.quote_rows[0].price, after.quote_rows[0].price);
    assert_ne!(before.quote_rows[0].change, after.quote_rows[0].change);
    assert_eq!(before.chart_line_path, after.chart_line_path);
    assert_eq!(before.chart_end_y_ratio, after.chart_end_y_ratio);
    assert_ne!(
        crate::i18n::change_period_label(Locale::En),
        crate::i18n::change_period_label(Locale::ZhHans)
    );
    for theme in ["light", "dark"] {
        h.ui.set_property("theme-name", Value::String(theme.into()))
            .unwrap();
        for opacity in [100.0, 70.0] {
            h.ui.set_property("content-opacity", Value::Number(opacity))
                .unwrap();
            h.apply_pending(&before, "BTC", 1.0, Locale::En);
            let size = h.window.size();
            let mut pixels =
                vec![Rgb8Pixel { r: 0, g: 0, b: 0 }; (size.width * size.height) as usize];
            h.paint(&mut pixels, RepaintBufferType::ReusedBuffer);
            // Use the actual host bridge and runtime rows without a full redraw
            // between the old quote and the retained-buffer refresh.
            h.apply_pending(&after, "BTC", 1.0, Locale::ZhHans);
            h.paint(&mut pixels, RepaintBufferType::ReusedBuffer);
            let partial = pixels.clone();
            h.paint(&mut pixels, RepaintBufferType::NewBuffer);
            let different = partial.iter().zip(&pixels).filter(|(a, b)| a != b).count();
            if different != 0 {
                if let Some(folder) = &h.capture_dir {
                    h.save(
                        folder,
                        &format!("quote-refresh-{theme}-{opacity}-partial"),
                        &partial,
                    );
                    h.save(
                        folder,
                        &format!("quote-refresh-{theme}-{opacity}-full"),
                        &pixels,
                    );
                }
            }
            assert_eq!(
                different, 0,
                "quote refresh partial redraw {theme}/{opacity}"
            );
            assert_eq!(h.string("inspected-price"), after.quote_rows[0].price);
            assert_eq!(h.string("inspected-change"), after.quote_rows[0].change);
            assert_eq!(
                h.string("inspected-period"),
                crate::i18n::change_period_label(Locale::ZhHans)
            );
            assert!(h.boolean("inspected-chart-visible"));
        }
    }
    h.ui.hide().unwrap();
}

#[test]
fn trust_card_hover_keyboard_locking_and_partial_repaint_are_stable() {
    let h = Harness::new();
    let view = runtime_view("BTC", 68000.12, DataState::Ready, crate::i18n::Locale::En);
    h.apply(&view, "BTC", 1.0, crate::i18n::Locale::En);
    let toggles = Rc::new(Cell::new(0));
    let observed = toggles.clone();
    let weak = h.ui.as_weak();
    h.ui.set_callback("toggle-layout-lock", move |_| {
        let ui = weak.upgrade().unwrap();
        let Value::Bool(locked) = ui.get_property("layout-locked").unwrap() else {
            unreachable!()
        };
        ui.set_property("layout-locked", Value::Bool(!locked))
            .unwrap();
        observed.set(observed.get() + 1);
        Value::Void
    })
    .unwrap();
    let drags = Rc::new(Cell::new(0));
    let observed = drags.clone();
    h.ui.set_callback("drag-move", move |_| {
        observed.set(observed.get() + 1);
        Value::Void
    })
    .unwrap();
    let pointer_button = |down, x, y| {
        let position = LogicalPosition::new(x, y);
        h.ui.window().dispatch_event(if down {
            WindowEvent::PointerPressed {
                position,
                button: PointerEventButton::Left,
            }
        } else {
            WindowEvent::PointerReleased {
                position,
                button: PointerEventButton::Left,
            }
        });
    };
    let click = || {
        h.move_pointer(229.0, 35.0);
        pointer_button(true, 229.0, 35.0);
        pointer_button(false, 229.0, 35.0);
    };
    let drag = || {
        pointer_button(true, 120.0, 135.0);
        h.move_pointer(145.0, 140.0);
        pointer_button(false, 145.0, 140.0);
    };
    for theme in ["light", "dark"] {
        h.ui.set_property("theme-name", Value::String(theme.into()))
            .unwrap();
        h.ui.window().dispatch_event(WindowEvent::PointerExited);
        assert!(!h.boolean("inspected-lock-visible"));
        h.move_pointer(120.0, 135.0);
        assert!(h.boolean("inspected-lock-visible"));
        assert!(!h.boolean("inspected-status-visible"));
        assert!(h.boolean("inspected-chart-visible"));
        h.capture(&format!("trust-card-hover-{theme}"));
        h.move_pointer(80.0, 35.0);
        assert!(h.boolean("inspected-status-visible"));
        assert!(!h.boolean("inspected-chart-visible"));
        h.capture(&format!("trust-card-details-{theme}"));
        let before_header_drag = drags.get();
        pointer_button(true, 80.0, 35.0);
        h.move_pointer(100.0, 38.0);
        pointer_button(false, 100.0, 38.0);
        assert!(drags.get() > before_header_drag);
        click();
        assert!(!h.boolean("inspected-status-visible"));
        assert!(h.boolean("inspected-chart-visible"));
        assert!(h.boolean("layout-locked"));
        assert!(!h.boolean("inspected-lock-focus-visible"));
        h.ui.window().dispatch_event(WindowEvent::PointerExited);
        assert!(!h.boolean("inspected-lock-visible"));
        let before = drags.get();
        drag();
        assert_eq!(drags.get(), before);
        click();
        assert!(!h.boolean("layout-locked"));
        drag();
        assert!(
            drags.get() > before,
            "the passive hover status must not block dragging"
        );
    }
    assert_eq!(toggles.get(), 4);

    // Exercise native retained-buffer damage at fractional scale/DPI. The
    // hover footer and overlay must match a freshly rendered frame exactly.
    for scale in [0.3, 1.0, 1.25] {
        for dpi in [1.0, 1.25, 1.5] {
            h.scale(scale, dpi);
            let size = h.window.size();
            let mut pixels =
                vec![Rgb8Pixel { r: 0, g: 0, b: 0 }; (size.width * size.height) as usize];
            for theme in ["light", "dark"] {
                h.ui.set_property("theme-name", Value::String(theme.into()))
                    .unwrap();
                for opacity in [100.0, 70.0] {
                    h.ui.set_property("content-opacity", Value::Number(opacity))
                        .unwrap();
                    h.ui.window().dispatch_event(WindowEvent::PointerExited);
                    for (state, point) in [
                        ("enter", Some((120.0, 135.0))),
                        ("header", Some((80.0, 35.0))),
                        ("lock", Some((229.0, 35.0))),
                        ("leave", None),
                    ] {
                        h.paint(&mut pixels, RepaintBufferType::ReusedBuffer);
                        if let Some((x, y)) = point {
                            h.move_pointer(x * scale as f32, y * scale as f32);
                        } else {
                            h.ui.window().dispatch_event(WindowEvent::PointerExited);
                        }
                        h.paint(&mut pixels, RepaintBufferType::ReusedBuffer);
                        let partial = pixels.clone();
                        h.paint(&mut pixels, RepaintBufferType::NewBuffer);
                        let different = partial.iter().zip(&pixels).filter(|(a, b)| a != b).count();
                        if different != 0 {
                            if let Some(folder) = &h.capture_dir {
                                h.save(
                                    folder,
                                    &format!(
                                        "seam-{theme}-{scale}-{dpi}-{opacity}-{state}-partial"
                                    ),
                                    &partial,
                                );
                                h.save(
                                    folder,
                                    &format!("seam-{theme}-{scale}-{dpi}-{opacity}-{state}-full"),
                                    &pixels,
                                );
                            }
                        }
                        assert_eq!(
                            different, 0,
                            "partial redraw {theme}/{scale}/{dpi}/{opacity}/{state}"
                        );
                    }
                }
            }
        }
    }
    h.scale(1.0, 1.0);
    h.ui.set_property("content-opacity", Value::Number(100.0))
        .unwrap();
    h.ui.window().dispatch_event(WindowEvent::PointerExited);
    h.ui.window()
        .dispatch_event(WindowEvent::WindowActiveChanged(true));
    for (index, key) in [
        slint::platform::Key::Tab,
        slint::platform::Key::Space,
        slint::platform::Key::Return,
    ]
    .into_iter()
    .enumerate()
    {
        h.ui.window()
            .dispatch_event(WindowEvent::KeyPressed { text: key.into() });
        h.ui.window()
            .dispatch_event(WindowEvent::KeyReleased { text: key.into() });
        assert_eq!(toggles.get(), 4 + index);
    }
    assert!(h.boolean("inspected-lock-focused"));
    assert!(h.boolean("inspected-lock-visible"));
    h.capture("trust-card-keyboard-focus-dark");
    h.ui.window().dispatch_event(WindowEvent::KeyPressRepeated {
        text: slint::platform::Key::Space.into(),
    });
    assert_eq!(toggles.get(), 6);
    assert!(!h.boolean("layout-locked"));
    h.ui.hide().unwrap();
}
