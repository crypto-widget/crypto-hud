use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, Instant},
};

use slint::{
    platform::{
        software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
        Platform, PointerEventButton, WindowAdapter, WindowEvent,
    },
    ComponentHandle, LogicalPosition, Model, Rgb8Pixel,
};
use slint_interpreter::{Compiler, ComponentInstance, Value};

struct CompassPlatform {
    adapter: Rc<RefCell<Option<Rc<MinimalSoftwareWindow>>>>,
    elapsed: Rc<Cell<Duration>>,
}

impl Platform for CompassPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        self.adapter.replace(Some(window.clone()));
        Ok(window)
    }

    fn duration_since_start(&self) -> Duration {
        self.elapsed.get()
    }
}

fn number(ui: &ComponentInstance, property: &str) -> f64 {
    let Value::Number(value) = ui.get_property(property).unwrap() else {
        panic!("{property} must be numeric");
    };
    value
}

fn boolean(ui: &ComponentInstance, property: &str) -> bool {
    let Value::Bool(value) = ui.get_property(property).unwrap() else {
        panic!("{property} must be boolean");
    };
    value
}

fn advance(elapsed: &Cell<Duration>, milliseconds: u64) {
    elapsed.set(elapsed.get() + Duration::from_millis(milliseconds));
    slint::platform::update_timers_and_animations();
}

fn pointer_click(ui: &ComponentInstance, x: f64, y: f64) {
    let position = LogicalPosition::new(x as f32, y as f32);
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
}

fn render(window: &MinimalSoftwareWindow, capture: Option<(&Path, &str)>) {
    let size = window.size();
    let mut pixels = vec![
        Rgb8Pixel {
            r: 236,
            g: 234,
            b: 229,
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

fn runtime_view(
    symbols: &[String],
    price: Option<f64>,
    chart_ready: bool,
    failed: bool,
    locale: crate::i18n::Locale,
) -> crypto_hud_runtime::WidgetRuntimeView {
    let now = Instant::now();
    let mut cache = crypto_hud_runtime::QuoteCache::new();
    if let Some(price) = price {
        for (index, symbol) in symbols.iter().enumerate() {
            cache.insert(
                symbol.clone(),
                crypto_hud_runtime::QuoteState::new(
                    price,
                    if index % 2 == 0 { 2.35 } else { -1.26 },
                    if chart_ready {
                        if index % 2 == 0 {
                            vec![100.0, 106.0, 104.0, 112.0]
                        } else {
                            vec![112.0, 104.0, 106.0, 100.0]
                        }
                    } else {
                        Vec::new()
                    },
                    crypto_hud_core::MarketDataSource::Binance,
                    now,
                ),
            );
        }
    }
    crypto_hud_runtime::build_widget_runtime_view(crypto_hud_runtime::WidgetRuntimeViewParams {
        widget_id: "market-compass-test",
        symbols,
        quote_cache: &cache,
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
        display_options: Default::default(),
    })
}

#[derive(Debug, Clone)]
struct NodeMeasurement {
    label: String,
    label_visible: bool,
    selected: bool,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Debug, Clone)]
struct PanelMeasurement {
    price: String,
    glyph_width: f64,
    available_width: f64,
    x: f64,
    y: f64,
    font_size: f64,
    chart_ready: bool,
    change_color: Value,
    line_color: Value,
    up_color: Value,
    down_color: Value,
    endpoint_ratio: f64,
}

fn instrument(source: String) -> String {
    let root_marker = "export component MarketCompass inherits Window {";
    let node_marker = "coin_node := Rectangle {";
    let panel_marker = "active_panel := Rectangle {";
    for marker in [root_marker, node_marker, panel_marker] {
        assert!(
            source.contains(marker),
            "missing inspection anchor {marker}"
        );
    }
    source
        .replacen(root_marker, &format!(r#"{root_marker}
    in property <int> probe-generation;
    out property <int> inspected-active-index: root.active-index();
    out property <int> inspected-orbit-count: root.orbit-count;
    out property <int> inspected-artwork-level: root.artwork-level;
    out property <length> inspected-orbit-x: card.x + root.orbit-center-x;
    out property <length> inspected-orbit-y: card.y + root.orbit-center-y;
    out property <length> inspected-orbit-radius: root.orbit-radius;
    out property <bool> inspected-paused: root.pointer-within || root.keyboard-active;
    out property <bool> inspected-lock-revealed: root.lock-revealed;
    out property <bool> inspected-lock-focused: lock_focus.has-focus;
    out property <float> inspected-lock-opacity: lock_control.opacity;
    out property <length> inspected-lock-x: card.x + lock_control.x + lock_control.width / 2;
    out property <length> inspected-lock-y: card.y + lock_control.y + lock_control.height / 2;
    out property <color> inspected-gain-color: root.gain-color;
    out property <color> inspected-loss-color: root.loss-color;
    out property <bool> inspected-navigation-focused: navigation_focus.has-focus;
    public function inspected-focus-navigation() {{ navigation_focus.focus(); }}
    public function inspected-focus-lock() {{ lock_focus.focus(); }}
    callback node-measured(int, string, bool, bool, length, length, length, length);
    callback panel-measured(string, length, length, length, length, length, bool, color, color, color, color, float);
"#), 1)
        .replacen(node_marker, &format!(r#"{node_marker}
            property <int> inspected-generation: root.probe-generation;
            changed inspected-generation => {{
                if self.visible {{
                    root.node-measured(index, node_label.text, node_label.visible, coin_node.selected,
                        card.x + self.x, card.y + self.y, self.width, self.height);
                }}
            }}
"#), 1)
        .replacen(panel_marker, &format!(r#"{panel_marker}
            property <int> inspected-generation: root.probe-generation;
            inspected_glyphs := Text {{
                text: price_text.text;
                font-size: price_text.font-size;
                font-weight: price_text.font-weight;
                font-family: price_text.font-family;
                visible: false;
            }}
            changed inspected-generation => {{
                if self.visible {{
                    root.panel-measured(price_text.text, inspected_glyphs.preferred-width, price_text.width,
                        card.x + self.x + price_text.x, card.y + self.y + price_text.y,
                        price_text.font-size, active_panel.selected-chart-ready,
                        change_text.color, chart_line.stroke, active_panel.up-candle-color, active_panel.down-candle-color,
                        (chart_dot.y + chart_dot.height / 2) / chart_band.height * 1000);
                }}
            }}
"#), 1)
}

#[test]
fn market_compass_preserves_price_layout_rotation_and_controls() {
    let adapter = Rc::new(RefCell::new(None));
    let elapsed = Rc::new(Cell::new(Duration::ZERO));
    slint::platform::set_platform(Box::new(CompassPlatform {
        adapter: adapter.clone(),
        elapsed: elapsed.clone(),
    }))
    .unwrap();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("plugins/com.cryptohud.market-compass/ui/main.slint");
    let source = instrument(std::fs::read_to_string(&path).unwrap()).replacen(
        "background: transparent;",
        "background: #dedbd5;",
        1,
    );
    let compiled = spin_on::spin_on(Compiler::default().build_from_source(source, path));
    let definition = compiled.component("MarketCompass").unwrap_or_else(|| {
        panic!(
            "Market Compass should compile: {:?}",
            compiled.diagnostics().collect::<Vec<_>>()
        )
    });
    let ui = definition.create().unwrap();
    let widget = crate::widget_host::WidgetUi::DynamicSlint(crate::widget_host::DynamicWidgetUi {
        instance: ui.clone_strong(),
    });
    let nodes = Rc::new(RefCell::new(BTreeMap::new()));
    let observed_nodes = nodes.clone();
    ui.set_callback("node-measured", move |args| {
        let [Value::Number(index), Value::String(label), Value::Bool(label_visible), Value::Bool(selected), Value::Number(x), Value::Number(y), Value::Number(width), Value::Number(height)] = args else {
            panic!("invalid node measurement: {args:?}");
        };
        observed_nodes.borrow_mut().insert(*index as usize, NodeMeasurement {
            label: label.to_string(), label_visible: *label_visible, selected: *selected,
            x: *x, y: *y, width: *width, height: *height,
        });
        Value::Void
    }).unwrap();
    let panel = Rc::new(RefCell::new(None));
    let observed_panel = panel.clone();
    ui.set_callback("panel-measured", move |args| {
        let [Value::String(price), Value::Number(glyph_width), Value::Number(available_width), Value::Number(x), Value::Number(y), Value::Number(font_size), Value::Bool(chart_ready), change_color, line_color, up_color, down_color, Value::Number(endpoint_ratio)] = args else {
            panic!("invalid panel measurement: {args:?}");
        };
        observed_panel.replace(Some(PanelMeasurement {
            price: price.to_string(), glyph_width: *glyph_width, available_width: *available_width,
            x: *x, y: *y, font_size: *font_size, chart_ready: *chart_ready,
            change_color: change_color.clone(), line_color: line_color.clone(),
            up_color: up_color.clone(), down_color: down_color.clone(),
            endpoint_ratio: *endpoint_ratio,
        }));
        Value::Void
    }).unwrap();
    let generation = Cell::new(0);
    let measure = || {
        nodes.borrow_mut().clear();
        panel.replace(None);
        generation.set(generation.get() + 1);
        ui.set_property(
            "probe-generation",
            Value::Number(f64::from(generation.get())),
        )
        .unwrap();
        slint::platform::update_timers_and_animations();
    };
    let apply = |view: &crypto_hud_runtime::WidgetRuntimeView, scale, logos| {
        crate::widget_host::apply_runtime_view_to_widget(
            &widget,
            view,
            &[],
            &[],
            logos,
            Default::default(),
            scale,
        );
        let Value::Model(endpoints) = ui.get_property("quote-chart-end-y-ratios").unwrap() else {
            panic!("per-coin chart endpoints should be a model");
        };
        assert_eq!(endpoints.row_count(), view.quote_rows.len());
        for (index, endpoint) in view.quote_chart_end_y_ratios.iter().enumerate() {
            assert_eq!(
                endpoints.row_data(index),
                Some(Value::Number(f64::from(*endpoint)))
            );
        }
    };
    ui.set_property("config-switch-interval-seconds", Value::Number(60.0))
        .unwrap();
    ui.set_property("layout-lock-text", Value::String("Lock position".into()))
        .unwrap();
    ui.set_property("empty-text", Value::String("No pairs".into()))
        .unwrap();
    ui.show().unwrap();
    let window = adapter.borrow().as_ref().unwrap().clone();
    let symbols = ["BTC", "ETH", "SOL", "BNB", "XRP", "DOGE", "ADA", "PEPE"]
        .map(|asset| format!("binance:spot:{asset}/USDT"));
    let capture_dir = std::env::var_os("CRYPTO_HUD_MARKET_COMPASS_CAPTURE_DIR").map(PathBuf::from);

    for scale in [0.3, 1.0, 2.0] {
        ui.set_property("widget-scale", Value::Number(scale))
            .unwrap();
        ui.set_property("widget-width", Value::Number(480.0 * scale))
            .unwrap();
        ui.set_property("widget-height", Value::Number(480.0 * scale))
            .unwrap();
        ui.window().set_size(slint::LogicalSize::new(
            480.0 * scale as f32,
            480.0 * scale as f32,
        ));
        ui.window().dispatch_event(WindowEvent::PointerExited);
        assert!(
            !boolean(&ui, "inspected-lock-revealed"),
            "lock stays hidden at rest"
        );
        ui.window().dispatch_event(WindowEvent::PointerMoved {
            position: LogicalPosition::new(240.0 * scale as f32, 235.0 * scale as f32),
        });
        assert!(
            boolean(&ui, "inspected-paused"),
            "hover must pause rotation"
        );
        for theme in ["light", "dark"] {
            ui.set_property("theme-name", Value::String(theme.into()))
                .unwrap();
            for count in 0..=8 {
                let price = [68000.12, 0.000004110, 88888888.0][count % 3];
                let view = runtime_view(
                    &symbols[..count],
                    Some(price),
                    true,
                    false,
                    crate::i18n::Locale::ZhHans,
                );
                apply(&view, scale as f32, false);
                ui.set_property("rotation-step", Value::Number(0.0))
                    .unwrap();
                advance(&elapsed, 700);
                let name = format!("market-compass-{theme}-{count}-{}", (scale * 100.0) as i32);
                let capture = capture_dir
                    .as_deref()
                    .filter(|_| scale == 1.0 && [0, 1, 8].contains(&count));
                render(&window, capture.map(|folder| (folder, name.as_str())));
                measure();
                assert_eq!(number(&ui, "inspected-orbit-count"), count as f64);
                assert_eq!(nodes.borrow().len(), count);
                if count == 0 {
                    assert!(
                        panel.borrow().is_none(),
                        "empty state has no active price panel"
                    );
                    continue;
                }
                let observed = panel.borrow().as_ref().unwrap().clone();
                assert_eq!(observed.price, view.quote_rows[0].price);
                assert!(
                    observed.glyph_width > 0.0
                        && observed.glyph_width <= observed.available_width + 0.1,
                    "{theme}/{scale}/{count}: {} needs {}px, has {}px",
                    observed.price,
                    observed.glyph_width,
                    observed.available_width
                );
                let center_x = number(&ui, "inspected-orbit-x");
                let center_y = number(&ui, "inspected-orbit-y");
                let radius = number(&ui, "inspected-orbit-radius");
                for (index, node) in nodes.borrow().iter() {
                    assert!(
                        node.label_visible && !node.label.is_empty(),
                        "node {index} loses its identity without a logo"
                    );
                    assert_eq!(node.selected, *index == 0);
                    assert!(
                        node.x >= -0.1
                            && node.y >= -0.1
                            && node.x + node.width <= 480.0 * scale + 0.1
                            && node.y + node.height <= 480.0 * scale + 0.1,
                        "node {index} escapes its window at {scale}: {node:?}"
                    );
                    let distance = ((node.x + node.width / 2.0 - center_x).powi(2)
                        + (node.y + node.height / 2.0 - center_y).powi(2))
                    .sqrt();
                    assert!(
                        (distance - radius).abs() < 0.15,
                        "node {index} left the circular rail"
                    );
                }
                let selected = nodes.borrow()[&0].clone();
                assert!((selected.x + selected.width / 2.0 - center_x).abs() < 0.1);
                assert!((selected.y + selected.height / 2.0 - (center_y - radius)).abs() < 0.1);
                if count == 8 {
                    let next = nodes.borrow()[&1].clone();
                    assert!(
                        next.x + next.width / 2.0 > center_x
                            && next.y + next.height / 2.0 < center_y,
                        "the reference places the next coin clockwise from the top slot"
                    );
                }
            }
        }
    }

    // Cached prices keep their type size when chart data has not arrived yet.
    ui.set_property("widget-scale", Value::Number(1.0)).unwrap();
    ui.set_property("widget-width", Value::Number(480.0))
        .unwrap();
    ui.set_property("widget-height", Value::Number(480.0))
        .unwrap();
    ui.window().set_size(slint::LogicalSize::new(480.0, 480.0));
    ui.window().dispatch_event(WindowEvent::PointerMoved {
        position: LogicalPosition::new(240.0, 235.0),
    });
    let ready_view = runtime_view(
        &symbols,
        Some(88888888.0),
        true,
        false,
        crate::i18n::Locale::ZhHans,
    );
    apply(&ready_view, 1.0, true);
    render(&window, None);
    measure();
    let ready_panel = panel.borrow().as_ref().unwrap().clone();
    assert!(ready_panel.chart_ready);
    assert!(
        nodes.borrow().values().all(|node| node.label_visible),
        "missing logos must retain every node label"
    );
    let waiting_chart = runtime_view(
        &symbols,
        Some(88888888.0),
        false,
        false,
        crate::i18n::Locale::ZhHans,
    );
    apply(&waiting_chart, 1.0, false);
    render(&window, None);
    measure();
    let waiting_panel = panel.borrow().as_ref().unwrap().clone();
    assert!(!waiting_panel.chart_ready);
    assert_eq!(waiting_panel.price, ready_panel.price);
    assert_eq!(waiting_panel.font_size, ready_panel.font_size);

    for locale in [
        crate::i18n::Locale::ZhHans,
        crate::i18n::Locale::En,
        crate::i18n::Locale::Ar,
    ] {
        widget.set_rtl_layout(crate::i18n::is_rtl(locale));
        for failed in [false, true] {
            let view = runtime_view(&symbols[..1], None, false, failed, locale);
            apply(&view, 1.0, false);
            render(&window, None);
            measure();
            let observed = panel.borrow().as_ref().unwrap().clone();
            assert_eq!(observed.price, view.quote_rows[0].price);
            assert!(!observed.chart_ready);
            assert!(
                observed.glyph_width <= observed.available_width + 0.1,
                "localized loading/error label must fit: {:?}",
                observed.price
            );
        }
    }

    apply(&ready_view, 1.0, false);
    widget.set_rtl_layout(false);
    for red_up in [false, true] {
        ui.set_property("red-up-enabled", Value::Bool(red_up))
            .unwrap();
        measure();
        let observed = panel.borrow().as_ref().unwrap().clone();
        let gain = ui.get_property("inspected-gain-color").unwrap();
        let loss = ui.get_property("inspected-loss-color").unwrap();
        let (up, down) = if red_up { (loss, gain) } else { (gain, loss) };
        assert_eq!(observed.change_color, up);
        assert_eq!(observed.line_color, up);
        assert_eq!(observed.up_color, up);
        assert_eq!(observed.down_color, down);
    }

    // Desktop DPI changes are independent of the widget's logical scale.
    ui.set_property("theme-name", Value::String("light".into()))
        .unwrap();
    for (dpi, artwork_level, physical_size) in [(1.25, 125.0, 600), (1.5, 150.0, 720)] {
        ui.window()
            .dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor: dpi });
        ui.window().set_size(slint::LogicalSize::new(480.0, 480.0));
        render(&window, None);
        measure();
        assert_eq!(
            window.size(),
            slint::PhysicalSize::new(physical_size, physical_size)
        );
        assert_eq!(number(&ui, "inspected-artwork-level"), artwork_level);
        let observed = panel.borrow().as_ref().unwrap().clone();
        assert_eq!(observed.price, "88888888");
        assert!(
            observed.glyph_width > 0.0 && observed.glyph_width <= observed.available_width + 0.1,
            "eight digits must remain complete at desktop DPI {dpi}: {observed:?}"
        );
    }
    ui.window()
        .dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor: 1.0 });
    ui.window().set_size(slint::LogicalSize::new(480.0, 480.0));
    assert_eq!(number(&ui, "inspected-artwork-level"), 100.0);

    // Deterministic timer advancement exercises the actual production Timer.
    ui.window().dispatch_event(WindowEvent::PointerExited);
    ui.set_property("config-switch-interval-seconds", Value::Number(1.0))
        .unwrap();
    ui.set_property("rotation-step", Value::Number(0.0))
        .unwrap();
    advance(&elapsed, 0);
    advance(&elapsed, 700);
    render(&window, None);
    measure();
    let before = panel.borrow().as_ref().unwrap().clone();
    assert!((before.endpoint_ratio - 150.0).abs() < 0.01);
    assert_eq!(number(&ui, "inspected-active-index"), 0.0);
    advance(&elapsed, 400);
    assert_eq!(
        number(&ui, "inspected-active-index"),
        1.0,
        "timer advances to the next coin"
    );
    // Pause before the next tick while completing the current animation.
    ui.window().dispatch_event(WindowEvent::PointerMoved {
        position: LogicalPosition::new(240.0, 235.0),
    });
    advance(&elapsed, 300);
    render(&window, None);
    measure();
    for node in nodes.borrow().values() {
        let distance = ((node.x + node.width / 2.0 - number(&ui, "inspected-orbit-x")).powi(2)
            + (node.y + node.height / 2.0 - number(&ui, "inspected-orbit-y")).powi(2))
        .sqrt();
        assert!(
            (distance - number(&ui, "inspected-orbit-radius")).abs() < 0.15,
            "nodes must follow the circular rail during animation, not cut across its chord"
        );
    }
    advance(&elapsed, 400);
    render(&window, None);
    measure();
    let after = panel.borrow().as_ref().unwrap().clone();
    assert!(
        (after.endpoint_ratio - 850.0).abs() < 0.01,
        "the graph endpoint follows the selected coin, not the first quote"
    );
    assert_eq!(
        (before.x, before.y, before.available_width),
        (after.x, after.y, after.available_width),
        "center price stays fixed while nodes rotate"
    );
    let paused_index = number(&ui, "inspected-active-index");
    advance(&elapsed, 2100);
    assert_eq!(
        number(&ui, "inspected-active-index"),
        paused_index,
        "reading under the pointer pauses the carousel"
    );

    let clicked_node = nodes.borrow()[&4].clone();
    pointer_click(
        &ui,
        clicked_node.x + clicked_node.width / 2.0,
        clicked_node.y + clicked_node.height / 2.0,
    );
    assert_eq!(
        number(&ui, "inspected-active-index"),
        4.0,
        "clicking a node selects it immediately"
    );
    ui.window().dispatch_event(WindowEvent::PointerMoved {
        position: LogicalPosition::new(240.0, 235.0),
    });
    advance(&elapsed, 700);
    measure();
    let selected = nodes.borrow()[&4].clone();
    assert!((selected.x + selected.width / 2.0 - number(&ui, "inspected-orbit-x")).abs() < 0.1);
    assert!(
        (selected.y + selected.height / 2.0
            - (number(&ui, "inspected-orbit-y") - number(&ui, "inspected-orbit-radius")))
        .abs()
            < 0.1
    );
    let clicked_panel = panel.borrow().as_ref().unwrap().clone();
    assert_eq!((clicked_panel.x, clicked_panel.y), (before.x, before.y));

    let toggles = Rc::new(Cell::new(0));
    let callback_toggles = toggles.clone();
    let weak = ui.as_weak();
    ui.set_callback("toggle-layout-lock", move |_| {
        let ui = weak.upgrade().unwrap();
        ui.set_property("layout-locked", Value::Bool(!boolean(&ui, "layout-locked")))
            .unwrap();
        callback_toggles.set(callback_toggles.get() + 1);
        Value::Void
    })
    .unwrap();
    ui.window().dispatch_event(WindowEvent::PointerExited);
    assert!(!boolean(&ui, "inspected-lock-revealed"));
    ui.window().dispatch_event(WindowEvent::PointerMoved {
        position: LogicalPosition::new(240.0, 235.0),
    });
    assert!(boolean(&ui, "inspected-lock-revealed"));
    let lock_x = number(&ui, "inspected-lock-x");
    let lock_y = number(&ui, "inspected-lock-y");
    pointer_click(&ui, lock_x, lock_y);
    assert_eq!(toggles.get(), 1);
    assert!(boolean(&ui, "layout-locked"));
    ui.window().dispatch_event(WindowEvent::PointerExited);
    assert!(
        !boolean(&ui, "inspected-lock-revealed"),
        "a locked widget does not leave the overlay visible at rest"
    );

    let drags = Rc::new(Cell::new(0));
    let callback_drags = drags.clone();
    ui.set_callback("drag-move", move |_| {
        callback_drags.set(callback_drags.get() + 1);
        Value::Void
    })
    .unwrap();
    let drag = || {
        ui.window().dispatch_event(WindowEvent::PointerPressed {
            position: LogicalPosition::new(240.0, 220.0),
            button: PointerEventButton::Left,
        });
        ui.window().dispatch_event(WindowEvent::PointerMoved {
            position: LogicalPosition::new(260.0, 230.0),
        });
        ui.window().dispatch_event(WindowEvent::PointerReleased {
            position: LogicalPosition::new(260.0, 230.0),
            button: PointerEventButton::Left,
        });
    };
    drag();
    assert_eq!(drags.get(), 0, "locking must suppress dragging");
    pointer_click(&ui, lock_x, lock_y);
    assert_eq!(toggles.get(), 2);
    drag();
    assert!(drags.get() > 0, "an unlocked dial remains draggable");

    // A single coin never advances while unattended.
    apply(
        &runtime_view(
            &symbols[..1],
            Some(68000.12),
            true,
            false,
            crate::i18n::Locale::ZhHans,
        ),
        1.0,
        false,
    );
    ui.set_property("rotation-step", Value::Number(0.0))
        .unwrap();
    ui.window().dispatch_event(WindowEvent::PointerExited);
    assert!(!boolean(&ui, "inspected-paused"));
    advance(&elapsed, 5000);
    assert_eq!(number(&ui, "rotation-step"), 0.0);

    if let Some(folder) = capture_dir.as_deref() {
        // Reproducible reference captures use real runtime paths and price formatting.
        let now = Instant::now();
        let mut cache = crypto_hud_runtime::QuoteCache::new();
        let prices = [
            68000.12,
            3420.56,
            152.3478,
            602.45,
            0.5212,
            0.1264,
            0.3512,
            0.000004110,
        ];
        let closes = [
            128.0, 133.0, 130.0, 138.0, 142.0, 138.0, 145.0, 138.0, 141.0, 137.0, 139.0, 131.0,
            135.0, 127.0, 129.0, 123.0, 125.0, 122.0, 124.0, 118.0, 121.0, 110.0, 108.0, 100.0,
            103.0, 100.0, 104.0, 101.0, 109.0, 106.0, 110.0, 105.0, 102.0, 100.0,
        ];
        for (symbol, price) in symbols.iter().zip(prices) {
            cache.insert(
                symbol.clone(),
                crypto_hud_runtime::QuoteState::new(
                    price,
                    -4.86,
                    closes.iter().map(|close| close * price / 100.0).collect(),
                    crypto_hud_core::MarketDataSource::Binance,
                    now,
                ),
            );
        }
        let mut reference = crypto_hud_runtime::build_widget_runtime_view(
            crypto_hud_runtime::WidgetRuntimeViewParams {
                widget_id: "market-compass-reference",
                symbols: &symbols,
                quote_cache: &cache,
                source_prefix: "",
                provider_labels: Default::default(),
                labels: Default::default(),
                has_market_error: false,
                now,
                display_options: Default::default(),
            },
        );
        reference.updated_text = "Updated 2s".to_string();
        assert!(reference
            .quote_chart_up_candle_paths
            .iter()
            .all(String::is_empty));
        assert!(reference
            .quote_chart_down_candle_paths
            .iter()
            .all(String::is_empty));
        ui.set_property("config-switch-interval-seconds", Value::Number(60.0))
            .unwrap();
        ui.set_property("red-up-enabled", Value::Bool(false))
            .unwrap();
        widget.set_change_period_text("24h".into());
        apply(&reference, 1.0, true);
        ui.set_property("rotation-step", Value::Number(0.0))
            .unwrap();
        ui.window().dispatch_event(WindowEvent::PointerExited);
        advance(&elapsed, 700);
        for theme in ["light", "dark"] {
            ui.set_property("theme-name", Value::String(theme.into()))
                .unwrap();
            render(
                &window,
                Some((folder, &format!("market-compass-reference-{theme}"))),
            );
        }
        for theme in ["light", "dark"] {
            ui.set_property("theme-name", Value::String(theme.into()))
                .unwrap();
            for (dpi, level) in [(1.25, 125), (1.5, 150), (2.0, 200)] {
                ui.window()
                    .dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor: dpi });
                ui.window().set_size(slint::LogicalSize::new(480.0, 480.0));
                assert_eq!(number(&ui, "inspected-artwork-level"), f64::from(level));
                render(
                    &window,
                    Some((
                        folder,
                        &format!("market-compass-reference-{theme}-dpi-{level}"),
                    )),
                );
            }
        }
        ui.window()
            .dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor: 1.0 });
        ui.window().set_size(slint::LogicalSize::new(480.0, 480.0));
        ui.set_property("rotation-step", Value::Number(-1.0))
            .unwrap();
        advance(&elapsed, 700);
        for theme in ["light", "dark"] {
            ui.set_property("theme-name", Value::String(theme.into()))
                .unwrap();
            render(
                &window,
                Some((folder, &format!("market-compass-reference-pepe-{theme}"))),
            );
        }
        ui.set_property("config-switch-interval-seconds", Value::Number(1.0))
            .unwrap();
    }

    apply(&ready_view, 1.0, false);
    ui.window().dispatch_event(WindowEvent::PointerExited);
    ui.window()
        .dispatch_event(WindowEvent::WindowActiveChanged(true));
    ui.invoke("inspected-focus-navigation", &[]).unwrap();
    assert!(boolean(&ui, "inspected-navigation-focused"));
    ui.set_property("rotation-step", Value::Number(0.0))
        .unwrap();
    for (key, expected) in [
        (slint::platform::Key::LeftArrow, 7.0),
        (slint::platform::Key::RightArrow, 0.0),
        (slint::platform::Key::RightArrow, 1.0),
    ] {
        ui.window()
            .dispatch_event(WindowEvent::KeyPressed { text: key.into() });
        ui.window()
            .dispatch_event(WindowEvent::KeyReleased { text: key.into() });
        assert_eq!(number(&ui, "inspected-active-index"), expected);
    }
    advance(&elapsed, 2100);
    assert_eq!(
        number(&ui, "inspected-active-index"),
        1.0,
        "keyboard focus pauses rotation"
    );

    // Keyboard focus reveals the floating lock even without pointer hover.
    ui.window().dispatch_event(WindowEvent::PointerExited);
    ui.invoke("inspected-focus-lock", &[]).unwrap();
    assert!(boolean(&ui, "inspected-lock-focused"));
    assert!(boolean(&ui, "inspected-lock-revealed"));
    assert_eq!(number(&ui, "inspected-lock-opacity"), 1.0);
    let toggles_before_keyboard = toggles.get();
    for (index, key) in [slint::platform::Key::Space, slint::platform::Key::Return]
        .into_iter()
        .enumerate()
    {
        ui.window()
            .dispatch_event(WindowEvent::KeyPressed { text: key.into() });
        ui.window()
            .dispatch_event(WindowEvent::KeyReleased { text: key.into() });
        assert_eq!(
            toggles.get(),
            toggles_before_keyboard + index + 1,
            "each lock activation key toggles exactly once"
        );
    }
    ui.window().dispatch_event(WindowEvent::KeyPressRepeated {
        text: slint::platform::Key::Space.into(),
    });
    assert_eq!(toggles.get(), toggles_before_keyboard + 2);

    ui.hide().unwrap();
}
