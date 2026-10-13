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
    ComponentHandle, LogicalPosition, Rgb8Pixel,
};
use slint_interpreter::{Compiler, ComponentInstance, Value};

struct OffscreenPlatform {
    adapter: Rc<RefCell<Option<Rc<MinimalSoftwareWindow>>>>,
}

impl Platform for OffscreenPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        self.adapter.replace(Some(window.clone()));
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

#[derive(Clone, Copy)]
struct QuoteFixture {
    price: f64,
    change: f64,
    state: DataState,
}

impl Default for QuoteFixture {
    fn default() -> Self {
        Self {
            price: 68000.12,
            change: -4.86,
            state: DataState::Ready,
        }
    }
}

fn runtime_view(
    symbols: &[String],
    fixture: QuoteFixture,
    locale: crate::i18n::Locale,
) -> crypto_hud_runtime::WidgetRuntimeView {
    // Leave room for the stale fixture even immediately after a Windows reboot.
    let now = Instant::now() + Duration::from_secs(3600);
    let mut cache = crypto_hud_runtime::QuoteCache::new();
    if !matches!(fixture.state, DataState::Loading | DataState::Failed) {
        let chart_ready = matches!(fixture.state, DataState::Ready | DataState::Cached);
        let age = Duration::from_secs(if matches!(fixture.state, DataState::Cached) {
            3600
        } else {
            2
        });
        let closes = [
            128.0, 133.0, 130.0, 138.0, 142.0, 138.0, 145.0, 138.0, 141.0, 137.0, 139.0, 131.0,
            135.0, 127.0, 129.0, 123.0, 125.0, 122.0, 124.0, 118.0, 121.0, 110.0, 108.0, 100.0,
            103.0, 100.0, 104.0, 101.0, 109.0, 106.0, 110.0, 105.0, 102.0, 100.0,
        ];
        for symbol in symbols {
            cache.insert(
                symbol.clone(),
                crypto_hud_runtime::QuoteState::new_with_chart_status(
                    fixture.price,
                    fixture.change,
                    if chart_ready {
                        closes
                            .iter()
                            .map(|close| close * fixture.price / 100.0)
                            .collect()
                    } else {
                        Vec::new()
                    },
                    Vec::new(),
                    crypto_hud_core::MarketDataSource::Binance,
                    now - age,
                    chart_ready.then_some(now - age),
                    matches!(fixture.state, DataState::ChartFailed)
                        .then(|| "candle request timed out".to_string()),
                ),
            );
        }
    }
    let text = crate::i18n::text(locale);
    crypto_hud_runtime::build_widget_runtime_view(crypto_hud_runtime::WidgetRuntimeViewParams {
        widget_id: "mint-tile-test",
        symbols,
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
        has_market_error: matches!(fixture.state, DataState::Failed | DataState::Cached),
        now,
        display_options: Default::default(),
    })
}

fn boolean(ui: &ComponentInstance, property: &str) -> bool {
    let Value::Bool(value) = ui.get_property(property).unwrap() else {
        panic!("{property} must be boolean");
    };
    value
}

fn number(ui: &ComponentInstance, property: &str) -> f64 {
    let Value::Number(value) = ui.get_property(property).unwrap() else {
        panic!("{property} must be numeric");
    };
    value
}

fn render(window: &MinimalSoftwareWindow, capture: Option<(&Path, &str)>) {
    let size = window.size();
    let mut pixels = vec![
        Rgb8Pixel {
            r: 236,
            g: 234,
            b: 229
        };
        (size.width * size.height) as usize
    ];
    window.window().request_redraw();
    assert!(window.draw_if_needed(|renderer| {
        renderer.render(&mut pixels, size.width as usize);
    }));
    if let Some((folder, name)) = capture {
        std::fs::create_dir_all(folder).unwrap();
        let mut bytes = format!("P6\n{} {}\n255\n", size.width, size.height).into_bytes();
        for pixel in pixels {
            bytes.extend_from_slice(&[pixel.r, pixel.g, pixel.b]);
        }
        std::fs::write(folder.join(format!("{name}.ppm")), bytes).unwrap();
    }
}

#[derive(Clone, Debug)]
struct QuoteMeasurement {
    price: String,
    available_width: f64,
    font_size: f64,
    change: String,
    change_color: Value,
    period: String,
}

fn instrument(source: String) -> String {
    let root_marker = "export component MintTile inherits Window {";
    let price_marker = "price_text := Text {";
    for marker in [root_marker, price_marker, "    price_measure := Text {"] {
        assert!(
            source.contains(marker),
            "missing inspection anchor {marker}"
        );
    }
    // All probes live only in the test compilation, not in the plugin contract.
    source
        .replacen(
            root_marker,
            &format!(
                r#"{root_marker}
    in property <int> probe-generation;
    out property <image> inspected-icon-source: root.displayed-quote-icon;
    out property <length> fitted-price-width: fitted_measure.preferred-width;
    out property <string> inspected-chart-status: chart_status.text;
    out property <bool> inspected-chart-status-visible: chart_status.visible;
    out property <bool> inspected-lock-focused: lock_focus.has-focus;
    out property <bool> inspected-lock-visible: lock_button.opacity > 0;
    out property <bool> inspected-lock-focus-visible: root.show-lock-focus && lock_focus.has-focus;
    out property <color> inspected-neutral-color: root.secondary-text-color;
    out property <color> inspected-chart-color: root.chart-color;
    out property <color> inspected-gain-color: root.gain-color;
    out property <color> inspected-loss-color: root.loss-color;
    callback quote-measured(string, length, length, string, color, string);
"#
            ),
            1,
        )
        .replacen(
            "    price_measure := Text {",
            r#"    fitted_measure := Text {
        text: root.measured-price;
        font-size: root.price-font-size;
        font-weight: 700;
        visible: false;
    }
    price_measure := Text {"#,
            1,
        )
        .replacen(
            price_marker,
            &format!(
                r#"{price_marker}
                property <int> inspected-generation: root.probe-generation;
                changed inspected-generation => {{
                    root.quote-measured(self.text, self.width, self.font-size,
                        change_text.text, change_text.color, change_period.text);
                }}
"#
            ),
            1,
        )
        .replacen("background: transparent;", "background: #eceae5;", 1)
}

#[test]
fn mint_tile_fits_prices_and_supports_mouse_and_keyboard_locking() {
    let adapter = Rc::new(RefCell::new(None));
    slint::platform::set_platform(Box::new(OffscreenPlatform {
        adapter: adapter.clone(),
    }))
    .unwrap();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("plugins/com.cryptohud.mint-tile/ui/main.slint");
    let source = instrument(std::fs::read_to_string(&path).unwrap());
    let result = spin_on::spin_on(Compiler::default().build_from_source(source, path));
    let definition = result.component("MintTile").unwrap_or_else(|| {
        panic!(
            "Mint Tile should compile: {:?}",
            result.diagnostics().collect::<Vec<_>>()
        )
    });
    let ui = definition.create().unwrap();
    crate::image_resampling::install_image_resampling(&ui).unwrap();
    let widget = crate::widget_host::WidgetUi::DynamicSlint(crate::widget_host::DynamicWidgetUi {
        instance: ui.clone_strong(),
    });
    let measurement = Rc::new(RefCell::new(None));
    let observed = measurement.clone();
    ui.set_callback("quote-measured", move |args| {
        let [Value::String(price), Value::Number(available_width), Value::Number(font_size), Value::String(change), change_color, Value::String(period)] = args else {
            panic!("invalid quote measurement: {args:?}");
        };
        observed.replace(Some(QuoteMeasurement {
            price: price.to_string(), available_width: *available_width, font_size: *font_size,
            change: change.to_string(), change_color: change_color.clone(), period: period.to_string(),
        }));
        Value::Void
    }).unwrap();
    let generation = Cell::new(0);
    let measure = || {
        measurement.replace(None);
        generation.set(generation.get() + 1);
        ui.set_property(
            "probe-generation",
            Value::Number(f64::from(generation.get())),
        )
        .unwrap();
        slint::platform::update_timers_and_animations();
        measurement.borrow().as_ref().unwrap().clone()
    };
    let set_scale = |scale| {
        ui.set_property("widget-scale", Value::Number(scale))
            .unwrap();
        ui.set_property("widget-width", Value::Number(360.0 * scale))
            .unwrap();
        ui.set_property("widget-height", Value::Number(360.0 * scale))
            .unwrap();
        ui.window().set_size(slint::LogicalSize::new(
            360.0 * scale as f32,
            360.0 * scale as f32,
        ));
    };
    let apply = |view: &crypto_hud_runtime::WidgetRuntimeView, scale, locale| {
        let text = crate::i18n::text(locale);
        widget.set_change_period_text(crate::i18n::change_period_label(locale).into());
        widget.set_layout_lock_text(text.always_on_top.into());
        widget.set_rtl_layout(crate::i18n::is_rtl(locale));
        ui.set_property("empty-text", Value::String(text.runtime_no_pairs.into()))
            .unwrap();
        crate::widget_host::apply_runtime_view_to_widget(
            &widget,
            view,
            &[],
            &[],
            false,
            Default::default(),
            scale,
        );
    };
    ui.set_property("content-opacity", Value::Number(100.0))
        .unwrap();
    ui.show().unwrap();
    let window = adapter.borrow().as_ref().unwrap().clone();
    let capture_dir = std::env::var_os("CRYPTO_HUD_MINT_TILE_CAPTURE_DIR").map(PathBuf::from);
    let capture = |name: &str| {
        render(&window, capture_dir.as_deref().map(|folder| (folder, name)));
    };
    let btc = ["binance:spot:BTC/USDT".to_string()];

    crate::icon_sampling_tests::verify_card_icon_sampling(
        &ui,
        &widget,
        &runtime_view(&btc, QuoteFixture::default(), crate::i18n::Locale::En),
        (360.0, 360.0),
        32.0,
        |name| capture(&format!("mint-tile-{name}")),
    );

    for scale in [0.3, 1.0, 2.0] {
        set_scale(scale);
        for theme in ["light", "dark"] {
            ui.set_property("theme-name", Value::String(theme.into()))
                .unwrap();
            for (name, asset, price) in [
                ("btc", "BTC", 68000.12),
                ("pepe", "PEPE", 0.00000411),
                ("eight-digits", "BTC", 88888888.0),
            ] {
                let symbols = [format!("binance:spot:{asset}/USDT")];
                let view = runtime_view(
                    &symbols,
                    QuoteFixture {
                        price,
                        ..Default::default()
                    },
                    crate::i18n::Locale::En,
                );
                apply(&view, scale as f32, crate::i18n::Locale::En);
                ui.window().dispatch_event(WindowEvent::PointerExited);
                capture(&format!(
                    "mint-tile-{name}-{theme}-{}",
                    (scale * 100.0) as i32
                ));
                let observed = measure();
                let fitted_width = number(&ui, "fitted-price-width");
                assert_eq!(observed.price, view.quote_rows[0].price);
                assert!(
                    fitted_width > 0.0 && fitted_width <= observed.available_width + 0.1,
                    "{name}/{theme}/{scale}: {} needs {fitted_width}px, has {}px",
                    observed.price,
                    observed.available_width
                );
                assert_eq!(
                    observed.period,
                    crate::i18n::change_period_label(crate::i18n::Locale::En)
                );
                assert!(
                    !boolean(&ui, "inspected-lock-visible"),
                    "the lock is hidden at rest"
                );
            }
        }
    }
    set_scale(1.0);

    for theme in ["light", "dark"] {
        ui.set_property("theme-name", Value::String(theme.into()))
            .unwrap();
        for locale in crate::i18n::Locale::ALL {
            for (name, state) in [
                ("loading", DataState::Loading),
                ("failure", DataState::Failed),
            ] {
                let view = runtime_view(
                    &btc,
                    QuoteFixture {
                        state,
                        ..Default::default()
                    },
                    locale,
                );
                apply(&view, 1.0, locale);
                render(&window, None);
                let observed = measure();
                assert_eq!(view.quote_price_ready, vec![false]);
                assert_eq!(
                    observed.price, "—",
                    "loading copy must not occupy the numeric slot"
                );
                assert_eq!(observed.change, "—");
                assert_eq!(
                    observed.change_color,
                    ui.get_property("inspected-neutral-color").unwrap()
                );
                assert!(boolean(&ui, "inspected-chart-status-visible"));
                assert_eq!(
                    ui.get_property("inspected-chart-status").unwrap(),
                    Value::String(view.updated_text.clone().into())
                );
                assert_eq!(observed.period, crate::i18n::change_period_label(locale));
                if [
                    crate::i18n::Locale::En,
                    crate::i18n::Locale::ZhHans,
                    crate::i18n::Locale::Ar,
                ]
                .contains(&locale)
                {
                    capture(&format!("mint-tile-{name}-{theme}-{locale:?}"));
                }
            }
        }
        for (name, state) in [
            ("cached", DataState::Cached),
            ("chart-loading", DataState::ChartLoading),
            ("chart-failure", DataState::ChartFailed),
        ] {
            let ready = runtime_view(&btc, QuoteFixture::default(), crate::i18n::Locale::En);
            apply(&ready, 1.0, crate::i18n::Locale::En);
            render(&window, None);
            let ready_measurement = measure();
            let view = runtime_view(
                &btc,
                QuoteFixture {
                    state,
                    ..Default::default()
                },
                crate::i18n::Locale::En,
            );
            apply(&view, 1.0, crate::i18n::Locale::En);
            capture(&format!("mint-tile-{name}-{theme}"));
            let observed = measure();
            assert_eq!(view.quote_price_ready, vec![true]);
            assert_eq!(observed.price, ready_measurement.price);
            assert_eq!(
                observed.font_size, ready_measurement.font_size,
                "chart readiness must not resize a cached price"
            );
            assert_eq!(observed.change, view.quote_rows[0].change);
            assert_eq!(
                ui.get_property("updated-text").unwrap(),
                Value::String(view.updated_text.clone().into())
            );
            if matches!(state, DataState::Cached) {
                assert!(view
                    .updated_text
                    .contains(crate::i18n::text(crate::i18n::Locale::En).runtime_stale));
            } else {
                assert!(boolean(&ui, "inspected-chart-status-visible"));
                assert_eq!(
                    ui.get_property("inspected-chart-status").unwrap(),
                    Value::String(view.source_text.clone().into())
                );
                assert_eq!(
                    view.updated_text, ready.updated_text,
                    "chart failure must preserve quote freshness"
                );
            }
        }
        let empty = runtime_view(&[], QuoteFixture::default(), crate::i18n::Locale::En);
        apply(&empty, 1.0, crate::i18n::Locale::En);
        capture(&format!("mint-tile-empty-{theme}"));
        assert!(!boolean(&ui, "inspected-chart-status-visible"));

        let long = runtime_view(
            &["binance:spot:VERYLONGTOKEN/FDUSD".to_string()],
            QuoteFixture {
                change: 123456.78,
                ..Default::default()
            },
            crate::i18n::Locale::En,
        );
        apply(&long, 1.0, crate::i18n::Locale::En);
        capture(&format!("mint-tile-long-pair-and-change-{theme}"));
        assert_eq!(measure().price, long.quote_rows[0].price);

        let view = runtime_view(&btc, QuoteFixture::default(), crate::i18n::Locale::En);
        apply(&view, 1.0, crate::i18n::Locale::En);
        for red_up in [false, true] {
            ui.set_property("red-up-enabled", Value::Bool(red_up))
                .unwrap();
            render(&window, None);
            let expected = ui
                .get_property(if red_up {
                    "inspected-gain-color"
                } else {
                    "inspected-loss-color"
                })
                .unwrap();
            assert_eq!(measure().change_color, expected);
            assert_eq!(ui.get_property("inspected-chart-color").unwrap(), expected);
        }
        ui.set_property("red-up-enabled", Value::Bool(false))
            .unwrap();
        ui.window()
            .dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor: 1.5 });
        ui.window().set_size(slint::LogicalSize::new(360.0, 360.0));
        capture(&format!("mint-tile-btc-{theme}-dpi-150"));
        assert_eq!(window.size(), slint::PhysicalSize::new(540, 540));
        assert!(number(&ui, "fitted-price-width") <= measure().available_width + 0.1);
        ui.window()
            .dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor: 1.0 });
        ui.window().set_size(slint::LogicalSize::new(360.0, 360.0));
    }

    let toggles = Rc::new(Cell::new(0));
    let weak = ui.as_weak();
    let callback_toggles = toggles.clone();
    ui.set_callback("toggle-layout-lock", move |_| {
        let ui = weak.upgrade().unwrap();
        ui.set_property("layout-locked", Value::Bool(!boolean(&ui, "layout-locked")))
            .unwrap();
        callback_toggles.set(callback_toggles.get() + 1);
        Value::Void
    })
    .unwrap();
    let drags = Rc::new(Cell::new(0));
    let callback_drags = drags.clone();
    ui.set_callback("drag-move", move |_| {
        callback_drags.set(callback_drags.get() + 1);
        Value::Void
    })
    .unwrap();
    let click = |x, y| {
        let position = LogicalPosition::new(x, y);
        ui.window()
            .dispatch_event(WindowEvent::PointerMoved { position });
        ui.window().dispatch_event(WindowEvent::PointerPressed {
            position,
            button: PointerEventButton::Left,
        });
        ui.window().dispatch_event(WindowEvent::PointerReleased {
            position,
            button: PointerEventButton::Left,
        });
    };
    let drag = || {
        ui.window().dispatch_event(WindowEvent::PointerPressed {
            position: LogicalPosition::new(120.0, 220.0),
            button: PointerEventButton::Left,
        });
        ui.window().dispatch_event(WindowEvent::PointerMoved {
            position: LogicalPosition::new(160.0, 240.0),
        });
        ui.window().dispatch_event(WindowEvent::PointerReleased {
            position: LogicalPosition::new(160.0, 240.0),
            button: PointerEventButton::Left,
        });
    };
    ui.set_property("layout-locked", Value::Bool(false))
        .unwrap();
    for theme in ["light", "dark"] {
        ui.set_property("theme-name", Value::String(theme.into()))
            .unwrap();
        ui.window().dispatch_event(WindowEvent::PointerExited);
        assert!(!boolean(&ui, "inspected-lock-visible"));
        ui.window().dispatch_event(WindowEvent::PointerMoved {
            position: LogicalPosition::new(120.0, 220.0),
        });
        assert!(boolean(&ui, "inspected-lock-visible"));
        capture(&format!("mint-tile-hover-{theme}"));
        click(312.0, 48.0);
        assert!(boolean(&ui, "layout-locked"));
        assert!(
            !boolean(&ui, "inspected-lock-focus-visible"),
            "pointer clicks must not leave a keyboard focus ring"
        );
        capture(&format!("mint-tile-locked-hover-{theme}"));
        ui.window().dispatch_event(WindowEvent::PointerExited);
        assert!(
            !boolean(&ui, "inspected-lock-visible"),
            "locking must not keep the overlay permanently visible"
        );
        capture(&format!("mint-tile-locked-leave-{theme}"));
        let before = drags.get();
        drag();
        assert_eq!(drags.get(), before, "locked cards must reject dragging");
        click(312.0, 48.0);
        assert!(!boolean(&ui, "layout-locked"));
        drag();
        assert!(
            drags.get() > before,
            "unlocked cards must send drag callbacks"
        );
    }
    assert_eq!(toggles.get(), 4, "dragging must never toggle the lock");

    // Keyboard navigation reveals the lock without any pointer hover.
    ui.window().dispatch_event(WindowEvent::PointerExited);
    ui.window()
        .dispatch_event(WindowEvent::WindowActiveChanged(true));
    for (index, key) in [
        slint::platform::Key::Tab,
        slint::platform::Key::Space,
        slint::platform::Key::Return,
    ]
    .into_iter()
    .enumerate()
    {
        ui.window()
            .dispatch_event(WindowEvent::KeyPressed { text: key.into() });
        ui.window()
            .dispatch_event(WindowEvent::KeyReleased { text: key.into() });
        assert_eq!(
            toggles.get(),
            4 + index,
            "keyboard activation toggles exactly once per press"
        );
    }
    assert!(boolean(&ui, "inspected-lock-focused"));
    assert!(boolean(&ui, "inspected-lock-visible"));
    capture("mint-tile-keyboard-focus-dark");
    ui.window().dispatch_event(WindowEvent::KeyPressRepeated {
        text: slint::platform::Key::Space.into(),
    });
    assert_eq!(toggles.get(), 6, "holding Space must not repeatedly toggle");
    assert!(!boolean(&ui, "layout-locked"));
    ui.hide().unwrap();
}

#[test]
fn mint_tile_hover_partial_repaint_matches_a_fresh_frame() {
    let adapter = Rc::new(RefCell::new(None));
    slint::platform::set_platform(Box::new(OffscreenPlatform {
        adapter: adapter.clone(),
    }))
    .unwrap();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("plugins/com.cryptohud.mint-tile/ui/main.slint");
    let result = spin_on::spin_on(
        Compiler::default()
            .build_from_source(instrument(std::fs::read_to_string(&path).unwrap()), path),
    );
    let ui = result.component("MintTile").unwrap().create().unwrap();
    crate::image_resampling::install_image_resampling(&ui).unwrap();
    let widget = crate::widget_host::WidgetUi::DynamicSlint(crate::widget_host::DynamicWidgetUi {
        instance: ui.clone_strong(),
    });
    let view = runtime_view(
        &["binance:spot:BTC/USDT".to_string()],
        QuoteFixture::default(),
        crate::i18n::Locale::En,
    );
    crate::widget_host::apply_runtime_view_to_widget(
        &widget,
        &view,
        &[],
        &[],
        false,
        Default::default(),
        1.0,
    );
    widget.set_change_period_text("24h".into());
    ui.show().unwrap();
    let window = adapter.borrow().as_ref().unwrap().clone();
    let capture_dir = std::env::var_os("CRYPTO_HUD_MINT_TILE_CAPTURE_DIR").map(PathBuf::from);
    let mut failures = Vec::new();
    for scale in [0.3, 1.0, 1.25] {
        ui.set_property("widget-scale", Value::Number(scale))
            .unwrap();
        ui.set_property("widget-width", Value::Number(360.0 * scale))
            .unwrap();
        ui.set_property("widget-height", Value::Number(360.0 * scale))
            .unwrap();
        for dpi in [1.0, 1.25, 1.5, 1.75, 2.0] {
            ui.window()
                .dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor: dpi });
            ui.window().set_size(slint::LogicalSize::new(
                360.0 * scale as f32,
                360.0 * scale as f32,
            ));
            let size = window.size();
            let mut pixels =
                vec![Rgb8Pixel { r: 0, g: 0, b: 0 }; (size.width * size.height) as usize];
            let paint = |pixels: &mut [Rgb8Pixel], mode| {
                window.window().request_redraw();
                assert!(window.draw_if_needed(|renderer| {
                    renderer.set_repaint_buffer_type(mode);
                    renderer.render(pixels, size.width as usize);
                }));
            };
            for theme in ["light", "dark"] {
                ui.set_property("theme-name", Value::String(theme.into()))
                    .unwrap();
                for opacity in [100.0, 70.0] {
                    ui.set_property("content-opacity", Value::Number(opacity))
                        .unwrap();
                    ui.window().dispatch_event(WindowEvent::PointerExited);
                    for (state, point) in [
                        ("enter", Some((120.0, 220.0))),
                        ("lock", Some((312.0, 48.0))),
                        ("leave", None),
                    ] {
                        // Prime the retained buffer, then repaint only the hover damage.
                        paint(&mut pixels, RepaintBufferType::ReusedBuffer);
                        if let Some((x, y)) = point {
                            ui.window().dispatch_event(WindowEvent::PointerMoved {
                                position: LogicalPosition::new(x * scale as f32, y * scale as f32),
                            });
                        } else {
                            ui.window().dispatch_event(WindowEvent::PointerExited);
                        }
                        paint(&mut pixels, RepaintBufferType::ReusedBuffer);
                        let partial = pixels.clone();
                        paint(&mut pixels, RepaintBufferType::NewBuffer);
                        let different = partial.iter().zip(&pixels).filter(|(a, b)| a != b).count();
                        let name = format!("seam-{theme}-{scale}-{dpi}-{opacity}-{state}");
                        if different != 0 {
                            failures.push(format!("{name}: {different} changed pixels"));
                        }
                        if different != 0
                            || (scale == 1.0 && dpi == 1.25 && opacity == 100.0 && state == "enter")
                        {
                            if let Some(folder) = capture_dir.as_deref() {
                                std::fs::create_dir_all(folder).unwrap();
                                for (suffix, image) in [("partial", &partial), ("full", &pixels)] {
                                    let mut bytes =
                                        format!("P6\n{} {}\n255\n", size.width, size.height)
                                            .into_bytes();
                                    for pixel in image {
                                        bytes.extend_from_slice(&[pixel.r, pixel.g, pixel.b]);
                                    }
                                    std::fs::write(
                                        folder.join(format!("{name}-{suffix}.ppm")),
                                        bytes,
                                    )
                                    .unwrap();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    ui.hide().unwrap();
    assert!(
        failures.is_empty(),
        "Partial repaint seams:\n{}",
        failures.join("\n")
    );
}
