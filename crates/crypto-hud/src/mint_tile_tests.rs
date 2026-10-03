use std::{
    cell::Cell,
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};

use slint::{
    platform::{
        software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
        Platform, PointerEventButton, WindowAdapter, WindowEvent,
    },
    ComponentHandle, LogicalPosition, VecModel,
};
use slint_interpreter::{Compiler, Struct, Value};

struct OffscreenPlatform;

impl Platform for OffscreenPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer))
    }
}

#[test]
fn mint_tile_fits_prices_and_supports_mouse_and_keyboard_locking() {
    slint::platform::set_platform(Box::new(OffscreenPlatform)).unwrap();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("plugins/com.cryptohud.mint-tile/ui/main.slint");
    // Expose measurements only in the test compilation, keeping the plugin contract unchanged.
    let source = std::fs::read_to_string(&path)
        .unwrap()
        .replace(
            "property <length> price-font-size:",
            "out property <length> price-font-size:",
        )
        .replace(
            "    price_measure := Text {",
            r#"    out property <length> fitted-price-width: fitted_measure.preferred-width;
    out property <string> inspected-chart-status: chart_status.text;
    out property <bool> inspected-chart-status-visible: chart_status.visible;
    out property <bool> inspected-lock-focused: lock_focus.has-focus;
    fitted_measure := Text {
        text: root.measured-price;
        font-size: root.price-font-size;
        font-weight: 900;
        visible: false;
    }
    price_measure := Text {"#,
        );
    let result = spin_on::spin_on(Compiler::default().build_from_source(source, path));
    let definition = result.component("MintTile").unwrap_or_else(|| {
        panic!(
            "mint tile should compile: {:?}",
            result.diagnostics().collect::<Vec<_>>()
        )
    });
    let ui = definition.create().unwrap();
    ui.set_property("content-opacity", Value::Number(100.0))
        .unwrap();
    ui.set_property("layout-lock-text", Value::String("Lock position".into()))
        .unwrap();
    ui.show().unwrap();

    let mut prices = vec![
        crypto_hud_core::format_price(0.00001234),
        crypto_hud_core::format_price(0.000000000001),
        crypto_hud_core::format_price(106800.12),
    ];
    prices.extend(
        crate::i18n::Locale::ALL
            .map(|locale| crate::i18n::text(locale).runtime_connecting.to_string()),
    );
    for scale in [0.3, 1.0, 2.0] {
        ui.set_property("widget-scale", Value::Number(scale))
            .unwrap();
        ui.set_property("widget-width", Value::Number(360.0 * scale))
            .unwrap();
        ui.set_property("widget-height", Value::Number(440.0 * scale))
            .unwrap();
        for price in &prices {
            let row = Value::Struct(Struct::from_iter([
                ("symbol".to_string(), Value::String("SHIB/USDT".into())),
                ("price".to_string(), Value::String(price.as_str().into())),
                ("change".to_string(), Value::String("+1.23%".into())),
                ("positive".to_string(), Value::Bool(true)),
            ]));
            ui.set_property(
                "quote-rows",
                Value::Model(Rc::new(VecModel::from(vec![row])).into()),
            )
            .unwrap();
            let Value::Number(width) = ui.get_property("fitted-price-width").unwrap() else {
                panic!("fitted price width must be numeric");
            };
            assert!(
                width > 0.0 && width <= 276.0 * scale,
                "{price:?} at {scale} must fit the price area, saw {width}"
            );
        }
    }
    ui.set_property("widget-scale", Value::Number(1.0)).unwrap();
    ui.set_property("widget-width", Value::Number(360.0))
        .unwrap();
    ui.set_property("widget-height", Value::Number(440.0))
        .unwrap();

    let widget = crate::widget_host::WidgetUi::DynamicSlint(crate::widget_host::DynamicWidgetUi {
        instance: ui.clone_strong(),
    });
    let now = Instant::now() + Duration::from_secs(7200);
    let symbol = "binance:spot:BTC/USDT".to_string();
    let mut cache = crypto_hud_runtime::QuoteCache::new();
    for failed in [true, false] {
        cache.insert(
            symbol.clone(),
            crypto_hud_runtime::QuoteState::new_with_chart_status(
                106800.12,
                1.23,
                vec![106000.0, 106800.12],
                vec![],
                crypto_hud_core::MarketDataSource::Binance,
                now - Duration::from_secs(2),
                Some(if failed {
                    now - Duration::from_secs(3600)
                } else {
                    now
                }),
                failed.then(|| "candle request timed out".to_string()),
            ),
        );
        let view = crypto_hud_runtime::build_widget_runtime_view(
            crypto_hud_runtime::WidgetRuntimeViewParams {
                widget_id: "mint-tile-test",
                symbols: std::slice::from_ref(&symbol),
                quote_cache: &cache,
                source_prefix: "live feed",
                provider_labels: Default::default(),
                labels: Default::default(),
                has_market_error: false,
                now,
                display_options: Default::default(),
            },
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
        assert_eq!(
            ui.get_property("inspected-chart-status-visible").unwrap(),
            Value::Bool(failed)
        );
        if failed {
            assert_eq!(
                ui.get_property("inspected-chart-status").unwrap(),
                Value::String("live feed · Binance · source issue".into())
            );
        }
        assert_eq!(
            ui.get_property("updated-text").unwrap(),
            Value::String("Updated 2s".into())
        );
    }
    // The optional accessibility label uses existing Rust translations in every locale.
    for locale in crate::i18n::Locale::ALL {
        let label = crate::i18n::text(locale).always_on_top;
        widget.set_layout_lock_text(label.into());
        assert_eq!(
            ui.get_property("layout-lock-text").unwrap(),
            Value::String(label.into())
        );
    }
    ui.set_property("layout-locked", Value::Bool(false))
        .unwrap();
    let toggles = Rc::new(Cell::new(0));
    let weak = ui.as_weak();
    let callback_toggles = toggles.clone();
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
    // Tab reaches the lock before any mouse interaction; each activation toggles once.
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
    assert_eq!(
        toggles.get(),
        2,
        "keyboard activation must work exactly once per press"
    );
    ui.window().dispatch_event(WindowEvent::KeyPressRepeated {
        text: slint::platform::Key::Space.into(),
    });
    assert_eq!(toggles.get(), 2, "holding Space must not repeatedly toggle");
    assert_eq!(
        ui.get_property("layout-locked").unwrap(),
        Value::Bool(false)
    );
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
            position: LogicalPosition::new(120.0, 230.0),
            button: PointerEventButton::Left,
        });
        ui.window().dispatch_event(WindowEvent::PointerMoved {
            position: LogicalPosition::new(160.0, 250.0),
        });
        ui.window().dispatch_event(WindowEvent::PointerReleased {
            position: LogicalPosition::new(160.0, 250.0),
            button: PointerEventButton::Left,
        });
    };
    click(312.0, 55.0);
    assert_eq!(ui.get_property("layout-locked").unwrap(), Value::Bool(true));
    drag();
    assert_eq!(drags.get(), 0, "locked cards must reject dragging");
    click(312.0, 55.0);
    assert_eq!(
        ui.get_property("layout-locked").unwrap(),
        Value::Bool(false)
    );
    drag();
    assert!(drags.get() > 0, "unlocked cards must send drag callbacks");
    assert_eq!(toggles.get(), 4, "dragging must never toggle the lock");
    ui.hide().unwrap();
}
