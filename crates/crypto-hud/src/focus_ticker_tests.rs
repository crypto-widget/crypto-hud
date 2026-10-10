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
    ComponentHandle, LogicalPosition, Rgb8Pixel, VecModel,
};
use slint_interpreter::{Compiler, Value};

struct OffscreenPlatform(Rc<RefCell<Option<Rc<MinimalSoftwareWindow>>>>);

impl Platform for OffscreenPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        self.0.replace(Some(window.clone()));
        Ok(window)
    }
}

fn number(ui: &slint_interpreter::ComponentInstance, property: &str) -> f64 {
    let Value::Number(value) = ui.get_property(property).unwrap() else {
        panic!("{property} should be numeric");
    };
    value
}

fn boolean(ui: &slint_interpreter::ComponentInstance, property: &str) -> bool {
    let Value::Bool(value) = ui.get_property(property).unwrap() else {
        panic!("{property} should be a boolean");
    };
    value
}

fn capture(window: &MinimalSoftwareWindow, folder: &Path, name: &str, light: bool) {
    std::fs::create_dir_all(folder).unwrap();
    let size = window.size();
    let background = if light { [235, 240, 246] } else { [39, 46, 55] };
    let mut pixels = vec![
        Rgb8Pixel {
            r: background[0],
            g: background[1],
            b: background[2]
        };
        (size.width * size.height) as usize
    ];
    window.window().request_redraw();
    assert!(window.draw_if_needed(|renderer| {
        renderer.render(&mut pixels, size.width as usize);
    }));
    let mut bytes = format!("P6\n{} {}\n255\n", size.width, size.height).into_bytes();
    for pixel in pixels {
        bytes.extend_from_slice(&[pixel.r, pixel.g, pixel.b]);
    }
    std::fs::write(folder.join(format!("{name}.ppm")), bytes).unwrap();
}

#[test]
fn focus_ticker_preserves_precision_states_and_lock_interactions() {
    let adapter = Rc::new(RefCell::new(None));
    slint::platform::set_platform(Box::new(OffscreenPlatform(adapter.clone()))).unwrap();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("plugins/com.cryptohud.focus-ticker/ui/main.slint");
    // Inspect measured results and state without expanding the production plugin API.
    let source = std::fs::read_to_string(&path).unwrap()
        .replacen("    background: transparent;", "    background: root.light-theme ? #ebf0f6 : #272e37;", 1)
        .replace("    base_measure := Text {", r#"
    out property <length> fitted-price-width: fitted_measure.preferred-width;
    out property <length> fitted-pair-width: (base_measure.preferred-width + quote_measure.preferred-width + root.s(6px)) * root.pair-fit;
    out property <string> inspected-price: root.price-label;
    out property <string> inspected-full-price: root.price-full-label;
    out property <length> inspected-price-font-size: root.price-font-size;
    out property <bool> inspected-price-details: root.price-details-visible;
    out property <bool> inspected-price-ready: root.price-ready;
    out property <bool> inspected-empty: !root.has-quote;
    out property <bool> inspected-chart: root.chart-ready && root.price-ready;
    out property <bool> inspected-lock-focused: lock_focus.has-focus;
    out property <bool> inspected-lock-revealed: root.lock-revealed;
    out property <float> inspected-lock-opacity: lock_control.opacity;
    out property <length> inspected-plot-width: root.plot-width;
    fitted_measure := Text { text: root.price-label; font-size: root.price-font-size; font-weight: 500; visible: false; }
    base_measure := Text {"#);
    let result = spin_on::spin_on(Compiler::default().build_from_source(source, path));
    let definition = result.component("FocusTicker").unwrap_or_else(|| {
        panic!(
            "Focus Ticker should compile: {:?}",
            result.diagnostics().collect::<Vec<_>>()
        )
    });
    let ui = definition.create().unwrap();
    let widget = crate::widget_host::WidgetUi::DynamicSlint(crate::widget_host::DynamicWidgetUi {
        instance: ui.clone_strong(),
    });
    ui.set_property("content-opacity", Value::Number(100.0))
        .unwrap();
    ui.show().unwrap();
    let symbols = vec!["binance:spot:PEPE/USDT".to_string()];
    let now = Instant::now();
    let mut cache = crypto_hud_runtime::QuoteCache::new();
    let display_options = Default::default();
    let build_view = |symbols: &[String],
                      cache: &crypto_hud_runtime::QuoteCache,
                      failed,
                      locale| {
        crypto_hud_runtime::build_widget_runtime_view(crypto_hud_runtime::WidgetRuntimeViewParams {
            widget_id: "focus-ticker-test",
            symbols,
            quote_cache: cache,
            source_prefix: "",
            provider_labels: Default::default(),
            labels: crypto_hud_runtime::RuntimeTextLabels {
                connecting: crate::i18n::text(locale).runtime_connecting,
                connection_error: crate::i18n::text(locale).runtime_connection_error,
                isolate_numeric_values: crate::i18n::is_rtl(locale),
                ..Default::default()
            },
            has_market_error: failed,
            now,
            display_options,
        })
    };
    let apply = |view: &crypto_hud_runtime::WidgetRuntimeView, scale| {
        crate::widget_host::apply_runtime_view_to_widget(
            &widget,
            view,
            &[],
            &[],
            false,
            display_options,
            scale,
        );
    };

    for scale in [0.3, 1.0, 1.5, 2.0] {
        ui.window().set_size(slint::LogicalSize::new(
            490.0 * scale as f32,
            100.0 * scale as f32,
        ));
        ui.set_property("widget-scale", Value::Number(scale))
            .unwrap();
        ui.set_property("widget-width", Value::Number(490.0 * scale))
            .unwrap();
        ui.set_property("widget-height", Value::Number(100.0 * scale))
            .unwrap();
        ui.window().dispatch_event(WindowEvent::PointerExited);
        assert!(!boolean(&ui, "inspected-lock-revealed"));
        assert_eq!(number(&ui, "inspected-lock-opacity"), 0.0);
        let plot_width = number(&ui, "inspected-plot-width");
        assert!((plot_width - 146.0 * scale).abs() < 0.001);
        ui.window().dispatch_event(WindowEvent::PointerMoved {
            position: LogicalPosition::new(80.0 * scale as f32, 70.0 * scale as f32),
        });
        assert!(boolean(&ui, "inspected-lock-revealed"));
        assert_eq!(number(&ui, "inspected-plot-width"), plot_width);
        ui.window().dispatch_event(WindowEvent::PointerMoved {
            position: LogicalPosition::new(459.0 * scale as f32, 27.0 * scale as f32),
        });
        assert!(
            boolean(&ui, "inspected-lock-revealed"),
            "hover must remain visible when crossing onto the overlay at {scale}"
        );
        assert_eq!(number(&ui, "inspected-plot-width"), plot_width);
        ui.window().dispatch_event(WindowEvent::PointerExited);
        assert!(!boolean(&ui, "inspected-lock-revealed"));
        for price in [
            68000.12,
            3420.56,
            152.3478,
            602.45,
            0.000004110,
            0.000000000001,
            0.000000000000123456,
            -0.000000012345,
            106800.0,
            88888888.0,
            99999999.0,
            987654321.0,
        ] {
            cache.insert(
                symbols[0].clone(),
                crypto_hud_runtime::QuoteState::new(
                    price,
                    -4.86,
                    vec![price * 1.06, price * 1.03, price],
                    crypto_hud_core::MarketDataSource::Binance,
                    now,
                ),
            );
            let view = build_view(&symbols, &cache, false, crate::i18n::Locale::ZhHans);
            apply(&view, scale as f32);
            assert_eq!(
                ui.get_property("inspected-price").unwrap(),
                Value::String(view.quote_compact_prices[0].clone().into())
            );
            assert_eq!(
                ui.get_property("inspected-full-price").unwrap(),
                Value::String(view.quote_full_prices[0].clone().into())
            );
            assert!(view.quote_compact_prices[0].chars().count() <= 8);
            assert!(
                (number(&ui, "inspected-price-font-size") - 38.0 * scale).abs() < 0.001,
                "eight display slots should keep a consistent font for {price} at {scale}: font {}, width {}",
                number(&ui, "inspected-price-font-size"),
                number(&ui, "fitted-price-width")
            );
            let width = number(&ui, "fitted-price-width");
            assert!(
                width > 0.0 && width <= 168.0 * scale,
                "compact price {price} must fit at {scale}: {width}"
            );
        }
        for base in ["PEPE", "1000BONK", "VERYLONGASSETCODE"] {
            let long_symbols = vec![format!("binance:spot:{base}/USDT")];
            let view = build_view(&long_symbols, &cache, false, crate::i18n::Locale::En);
            apply(&view, scale as f32);
            assert!(number(&ui, "fitted-pair-width") <= 112.0 * scale);
        }
        for locale in crate::i18n::Locale::ALL {
            widget.set_change_period_text(crate::i18n::change_period_label(locale).into());
            widget.set_layout_lock_text(crate::i18n::text(locale).always_on_top.into());
            widget.set_rtl_layout(crate::i18n::is_rtl(locale));
            for failed in [false, true] {
                let view = build_view(&symbols, &Default::default(), failed, locale);
                apply(&view, scale as f32);
                assert_eq!(
                    ui.get_property("inspected-price").unwrap(),
                    Value::String(view.updated_text.into())
                );
                assert_eq!(
                    ui.get_property("inspected-price-ready").unwrap(),
                    Value::Bool(false)
                );
                assert_eq!(
                    ui.get_property("inspected-chart").unwrap(),
                    Value::Bool(false)
                );
                assert!(number(&ui, "fitted-price-width") <= 168.0 * scale);
                ui.window().dispatch_event(WindowEvent::PointerMoved {
                    position: LogicalPosition::new(220.0 * scale as f32, 35.0 * scale as f32),
                });
                assert!(!boolean(&ui, "inspected-price-details"));
                ui.window().dispatch_event(WindowEvent::PointerExited);
            }
        }
    }
    let empty = build_view(&[], &Default::default(), false, crate::i18n::Locale::En);
    apply(&empty, 1.0);
    assert_eq!(
        ui.get_property("inspected-empty").unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        ui.get_property("inspected-chart").unwrap(),
        Value::Bool(false)
    );

    ui.set_property("widget-scale", Value::Number(1.0)).unwrap();
    ui.window().set_size(slint::LogicalSize::new(490.0, 100.0));
    ui.set_property("widget-width", Value::Number(490.0))
        .unwrap();
    ui.set_property("widget-height", Value::Number(100.0))
        .unwrap();
    widget.set_rtl_layout(false);
    widget.set_change_period_text(
        crate::i18n::change_period_label(crate::i18n::Locale::ZhHans).into(),
    );
    widget.set_layout_lock_text(
        crate::i18n::text(crate::i18n::Locale::ZhHans)
            .always_on_top
            .into(),
    );
    cache.insert(
        symbols[0].clone(),
        crypto_hud_runtime::QuoteState::new(
            0.000004110,
            -4.86,
            vec![
                106.0, 107.0, 106.5, 107.8, 107.2, 108.0, 107.0, 107.5, 106.3, 106.9, 105.2, 105.7,
                105.3, 105.6, 105.1, 105.5, 105.0, 105.6, 104.9, 105.4, 101.0, 101.3, 100.4, 101.1,
                100.7, 101.6, 101.9, 101.5, 102.0, 101.1, 100.7,
            ],
            crypto_hud_core::MarketDataSource::Binance,
            now - Duration::from_secs(2),
        ),
    );
    let mut view = build_view(&symbols, &cache, false, crate::i18n::Locale::ZhHans);
    view.updated_text = "已更新 2秒".into();
    apply(&view, 1.0);
    ui.set_property("layout-locked", Value::Bool(true)).unwrap();
    ui.window().dispatch_event(WindowEvent::PointerMoved {
        position: LogicalPosition::new(220.0, 35.0),
    });
    assert!(boolean(&ui, "inspected-price-details"));
    assert_eq!(
        ui.get_property("inspected-full-price").unwrap(),
        Value::String("0.00000411".into())
    );
    ui.window().dispatch_event(WindowEvent::PointerExited);
    assert!(!boolean(&ui, "inspected-price-details"));
    if let Some(folder) = std::env::var_os("CRYPTO_HUD_FOCUS_CAPTURE_DIR") {
        let folder = PathBuf::from(folder);
        let window = adapter.borrow().as_ref().unwrap().clone();
        for theme in ["light", "dark"] {
            ui.set_property("theme-name", Value::String(theme.into()))
                .unwrap();
            capture(
                &window,
                &folder,
                &format!("focus-{theme}"),
                theme == "light",
            );
            ui.window().dispatch_event(WindowEvent::PointerMoved {
                position: LogicalPosition::new(100.0, 70.0),
            });
            capture(
                &window,
                &folder,
                &format!("focus-{theme}-hover"),
                theme == "light",
            );
            ui.window().dispatch_event(WindowEvent::PointerExited);
        }
    }

    let toggles = Rc::new(Cell::new(0));
    let callback_toggles = toggles.clone();
    let weak = ui.as_weak();
    ui.set_callback("toggle-layout-lock", move |_| {
        let ui = weak.upgrade().unwrap();
        let locked = matches!(ui.get_property("layout-locked").unwrap(), Value::Bool(true));
        ui.set_property("layout-locked", Value::Bool(!locked))
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
    ui.window()
        .dispatch_event(WindowEvent::WindowActiveChanged(true));
    for key in [
        slint::platform::Key::Tab,
        slint::platform::Key::Space,
        slint::platform::Key::Return,
    ] {
        ui.window()
            .dispatch_event(WindowEvent::KeyPressed { text: key.into() });
        ui.window()
            .dispatch_event(WindowEvent::KeyReleased { text: key.into() });
    }
    assert_eq!(
        ui.get_property("inspected-lock-focused").unwrap(),
        Value::Bool(true)
    );
    assert_eq!(toggles.get(), 2);
    assert!(
        boolean(&ui, "inspected-lock-revealed"),
        "keyboard focus reveals the lock without pointer hover"
    );
    ui.window().dispatch_event(WindowEvent::KeyPressRepeated {
        text: slint::platform::Key::Space.into(),
    });
    assert_eq!(toggles.get(), 2);
    let drag = || {
        ui.window().dispatch_event(WindowEvent::PointerPressed {
            position: LogicalPosition::new(220.0, 35.0),
            button: PointerEventButton::Left,
        });
        ui.window().dispatch_event(WindowEvent::PointerMoved {
            position: LogicalPosition::new(250.0, 45.0),
        });
        if matches!(
            ui.get_property("layout-locked").unwrap(),
            Value::Bool(false)
        ) {
            assert!(
                !boolean(&ui, "inspected-price-details"),
                "hide details during price-area drag"
            );
        }
        ui.window().dispatch_event(WindowEvent::PointerReleased {
            position: LogicalPosition::new(250.0, 45.0),
            button: PointerEventButton::Left,
        });
    };
    drag();
    assert_eq!(drags.get(), 0, "locked windows reject drag");
    let position = LogicalPosition::new(459.0, 27.0);
    ui.window()
        .dispatch_event(WindowEvent::PointerMoved { position });
    assert!(
        boolean(&ui, "inspected-lock-revealed"),
        "moving from the card onto the overlay must keep it visible"
    );
    ui.window().dispatch_event(WindowEvent::PointerPressed {
        position,
        button: PointerEventButton::Left,
    });
    ui.window().dispatch_event(WindowEvent::PointerReleased {
        position,
        button: PointerEventButton::Left,
    });
    assert_eq!(toggles.get(), 3, "mouse locking toggles exactly once");
    assert_eq!(
        drags.get(),
        0,
        "clicking the overlay must not drag the window"
    );
    ui.window().dispatch_event(WindowEvent::PointerExited);
    assert!(
        !boolean(&ui, "inspected-lock-revealed"),
        "mouse use clears keyboard reveal and hides the lock on exit"
    );
    drag();
    assert!(drags.get() > 0, "unlocked windows remain draggable");
    ui.window().dispatch_event(WindowEvent::PointerExited);
    ui.set_property(
        "quote-rows",
        Value::Model(Rc::new(VecModel::<Value>::default()).into()),
    )
    .unwrap();
    assert_eq!(
        ui.get_property("inspected-empty").unwrap(),
        Value::Bool(true)
    );
    if let Some(folder) = std::env::var_os("CRYPTO_HUD_FOCUS_CAPTURE_DIR") {
        let folder = PathBuf::from(folder);
        let window = adapter.borrow().as_ref().unwrap().clone();
        widget.set_empty_text(
            crate::i18n::text(crate::i18n::Locale::ZhHans)
                .empty_pairs
                .into(),
        );
        for theme in ["light", "dark"] {
            ui.set_property("theme-name", Value::String(theme.into()))
                .unwrap();
            for (name, failed, empty) in [
                ("loading", false, false),
                ("error", true, false),
                ("empty", false, true),
            ] {
                let rows = if empty { &[][..] } else { &symbols };
                let view = build_view(
                    rows,
                    &Default::default(),
                    failed,
                    crate::i18n::Locale::ZhHans,
                );
                apply(&view, 1.0);
                capture(
                    &window,
                    &folder,
                    &format!("focus-{theme}-{name}"),
                    theme == "light",
                );
            }
            for (name, symbol, price) in [
                ("btc", "BTC", 68000.12),
                ("eth", "ETH", 3420.56),
                ("sol", "SOL", 152.3478),
                ("bnb", "BNB", 602.45),
                ("pepe", "PEPE", 0.000004110),
                ("long-price", "PEPE", 0.000000000001),
                ("integer-price", "BTC", 106800.0),
            ] {
                let symbols = vec![format!("binance:spot:{symbol}/USDT")];
                let events = crate::focus_ticker_gui_smoke_events(&[
                    crypto_hud_market::MarketSubscription {
                        symbol: symbols[0].clone(),
                        needs_candles: true,
                    },
                ]);
                let crypto_hud_market::MarketEvent::Snapshot(snapshot) = &events[0] else {
                    panic!("expected an offline quote fixture");
                };
                let mut cache = crypto_hud_runtime::QuoteCache::new();
                cache.insert(
                    symbols[0].clone(),
                    crypto_hud_runtime::QuoteState::new(
                        price,
                        -4.86,
                        snapshot
                            .chart_closes_24h
                            .iter()
                            .map(|value| value / snapshot.price * price)
                            .collect(),
                        crypto_hud_core::MarketDataSource::Binance,
                        now,
                    ),
                );
                let mut view = build_view(&symbols, &cache, false, crate::i18n::Locale::ZhHans);
                view.updated_text = "已更新 2秒".into();
                apply(&view, 1.0);
                capture(
                    &window,
                    &folder,
                    &format!("focus-{theme}-{name}"),
                    theme == "light",
                );
                ui.window().dispatch_event(WindowEvent::PointerMoved {
                    position: LogicalPosition::new(220.0, 35.0),
                });
                assert!(boolean(&ui, "inspected-price-details"));
                capture(
                    &window,
                    &folder,
                    &format!("focus-{theme}-{name}-price-hover"),
                    theme == "light",
                );
                ui.window().dispatch_event(WindowEvent::PointerExited);
            }
        }
    }
    ui.hide().unwrap();
}
