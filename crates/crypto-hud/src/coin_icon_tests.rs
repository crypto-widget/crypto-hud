use std::{cell::RefCell, collections::BTreeMap, path::PathBuf, rc::Rc};

use slint::{
    platform::{
        software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
        Platform, WindowAdapter, WindowEvent,
    },
    ComponentHandle, Image, ModelRc, Rgb8Pixel, VecModel,
};
use slint_interpreter::{Compiler, ComponentInstance, Struct, Value};

struct IconTestPlatform(Rc<RefCell<Option<Rc<MinimalSoftwareWindow>>>>);

impl Platform for IconTestPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        self.0.replace(Some(window.clone()));
        Ok(window)
    }
}

fn icon_fixtures() -> [Image; 2] {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [
        // A real, non-square PNG also catches accidental stretching to the box.
        Image::load_from_path(
            &root.join("plugins/com.cryptohud.market-compass/ui/champagne-coin-atlas.png"),
        )
        .unwrap(),
        Image::load_from_path(&root.join("ui/icons/feather/message-circle.svg")).unwrap(),
    ]
}

fn model(values: Vec<Value>) -> Value {
    Value::Model(ModelRc::new(VecModel::from(values)))
}

fn refresh_quotes(ui: &ComponentInstance, icon: &Image, count: usize, compass: bool) {
    let rows = (0..count)
        .map(|_| {
            Value::Struct(Struct::from_iter([
                ("symbol".into(), Value::String("AVAX/USDT".into())),
                ("price".into(), Value::String("42.50".into())),
                ("change".into(), Value::String("+2.35%".into())),
                ("positive".into(), Value::Bool(true)),
            ]))
        })
        .collect();
    ui.set_property("quote-rows", model(rows)).unwrap();
    ui.set_property(
        "quote-icons",
        model(vec![Value::Image(icon.clone()); count]),
    )
    .unwrap();
    if compass {
        ui.set_property(
            "quote-assets",
            model(vec![Value::String("AVAX".into()); count]),
        )
        .unwrap();
        ui.set_property("quote-icon-ready", model(vec![Value::Bool(true); count]))
            .unwrap();
    }
}

fn instrument_icons(source: String, item: &str) -> String {
    let marker = format!("{item} := Image {{");
    assert!(source.contains(&marker), "missing icon probe anchor {item}");
    source
        .replacen(
            "inherits Window {",
            "inherits Window {\n    in property <int> icon-probe-generation;\n    callback icon-measured(int, image, int, int);",
            1,
        )
        .replacen(
            &marker,
            &format!(
                "{marker}\n    property <int> probe-generation: root.icon-probe-generation;\n    changed probe-generation => {{ root.icon-measured(index, self.source, Math.round(self.width / 1phx), Math.round(self.height / 1phx)); }}"
            ),
            1,
        )
}

#[derive(Clone)]
struct IconMeasurement {
    image: Image,
    width: u32,
    height: u32,
}

fn render_and_measure(
    ui: &ComponentInstance,
    window: &MinimalSoftwareWindow,
    measurements: &RefCell<BTreeMap<usize, IconMeasurement>>,
    generation: &mut i32,
) -> BTreeMap<usize, IconMeasurement> {
    let size = window.size();
    let mut pixels = vec![Rgb8Pixel::default(); (size.width * size.height) as usize];
    ui.window().request_redraw();
    assert!(window.draw_if_needed(|renderer| {
        renderer.render(&mut pixels, size.width as usize);
    }));
    measurements.borrow_mut().clear();
    *generation += 1;
    ui.set_property(
        "icon-probe-generation",
        Value::Number(f64::from(*generation)),
    )
    .unwrap();
    slint::platform::update_timers_and_animations();
    measurements.borrow().clone()
}

#[test]
fn builtin_coin_icon_callbacks_are_installed_and_preserve_svg() {
    let adapter = Rc::new(RefCell::new(None));
    slint::platform::set_platform(Box::new(IconTestPlatform(adapter))).unwrap();
    let [png, svg] = icon_fixtures();
    for plugin in crate::plugin::builtin_plugins() {
        let widget = crate::widget_host::WidgetUi::from_plugin(&plugin).unwrap();
        let crate::widget_host::WidgetUi::BuiltinPriceCard(ui) = widget else {
            panic!("expected builtin widget");
        };
        let filtered = ui.invoke_resample_icon(png.clone(), 18, 18);
        assert_eq!((filtered.size().width, filtered.size().height), (18, 9));
        assert_eq!(filtered, ui.invoke_resample_icon(png.clone(), 18, 18));
        assert_eq!(svg, ui.invoke_resample_icon(svg.clone(), 9, 9));
    }
}

#[test]
fn coin_icon_sources_follow_widget_scale_dpi_and_reuse_quote_refreshes() {
    let adapter = Rc::new(RefCell::new(None));
    slint::platform::set_platform(Box::new(IconTestPlatform(adapter.clone()))).unwrap();
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = icon_fixtures();
    for (path, component, item, compact, compass, natural_width, natural_height) in [
        (
            "ui/price-card-window.slint",
            "PriceCardWindow",
            "quote_icon",
            false,
            false,
            286.0,
            194.0,
        ),
        (
            "ui/price-card-window.slint",
            "PriceCardWindow",
            "quote_icon",
            true,
            false,
            236.0,
            112.0,
        ),
        (
            "plugins/com.cryptohud.market-compass/ui/main.slint",
            "MarketCompass",
            "fallback_icon",
            false,
            true,
            480.0,
            480.0,
        ),
    ] {
        let path = manifest_dir.join(path);
        let source = instrument_icons(std::fs::read_to_string(&path).unwrap(), item);
        let compiled = spin_on::spin_on(Compiler::default().build_from_source(source, path));
        let definition = compiled.component(component).unwrap_or_else(|| {
            panic!(
                "{component}: {:?}",
                compiled.diagnostics().collect::<Vec<_>>()
            )
        });
        let ui = definition.create().unwrap();
        crate::image_resampling::install_image_resampling(&ui).unwrap();
        if !compass {
            ui.set_property("compact-mode", Value::Bool(compact))
                .unwrap();
        } else {
            // Keep automatic rotation independent from wall-clock test duration.
            ui.set_property("config-switch-interval-seconds", Value::Number(3600.0))
                .unwrap();
        }
        ui.set_property("content-opacity", Value::Number(100.0))
            .unwrap();
        let measured = Rc::new(RefCell::new(BTreeMap::new()));
        let observed = measured.clone();
        ui.set_callback("icon-measured", move |args| {
            let [Value::Number(index), Value::Image(image), Value::Number(width), Value::Number(height)] = args else {
                panic!("invalid icon measurement: {args:?}");
            };
            observed.borrow_mut().insert(*index as usize, IconMeasurement {
                image: image.clone(), width: *width as u32, height: *height as u32,
            });
            Value::Void
        }).unwrap();
        ui.show().unwrap();
        let window = adapter.borrow().as_ref().unwrap().clone();
        let mut generation = 0;
        for count in if compass { vec![1, 8] } else { vec![1] } {
            for (fixture_index, icon) in fixtures.iter().enumerate() {
                for (scale, dpi) in [(0.5_f32, 1.0_f32), (0.3, 1.25), (0.85, 2.0)] {
                    ui.window()
                        .dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor: dpi });
                    ui.set_property("widget-scale", Value::Number(f64::from(scale)))
                        .unwrap();
                    let width = (natural_width * scale).round();
                    let height = (natural_height * scale).round();
                    ui.set_property("widget-width", Value::Number(f64::from(width)))
                        .unwrap();
                    ui.set_property("widget-height", Value::Number(f64::from(height)))
                        .unwrap();
                    ui.window().set_size(slint::LogicalSize::new(width, height));
                    refresh_quotes(&ui, icon, count, compass);
                    let first = render_and_measure(&ui, &window, &measured, &mut generation);
                    assert_eq!(first.len(), count, "{component}: all icon sources measured");
                    for (&index, value) in &first {
                        let logical_box = if compass {
                            if index == 0 {
                                38.0
                            } else {
                                34.0
                            }
                        } else {
                            18.0
                        };
                        let physical_box = (logical_box * scale * dpi).round() as u32;
                        assert_eq!((value.width, value.height), (physical_box, physical_box));
                        if fixture_index == 0 {
                            assert_eq!(value.image.size().width, physical_box);
                            assert_eq!(value.image.size().height, physical_box.div_ceil(2));
                            assert_ne!(value.image, *icon, "PNG must be prefiltered");
                        } else {
                            assert_eq!(value.image, *icon, "SVG remains a vector source");
                        }
                    }
                    refresh_quotes(&ui, icon, count, compass);
                    let refreshed = render_and_measure(&ui, &window, &measured, &mut generation);
                    for (&index, value) in &first {
                        assert_eq!(
                            value.image, refreshed[&index].image,
                            "quote refresh should reuse filtered source"
                        );
                    }
                    if compass && count == 8 {
                        ui.set_property("rotation-step", Value::Number(1.0))
                            .unwrap();
                        let rotated = render_and_measure(&ui, &window, &measured, &mut generation);
                        assert_eq!(
                            first[&0].image, rotated[&1].image,
                            "selected icon filter should be shared"
                        );
                        assert_eq!(
                            first[&1].image, rotated[&0].image,
                            "unselected icon filter should be shared"
                        );
                        ui.set_property("rotation-step", Value::Number(0.0))
                            .unwrap();
                    }
                }
            }
        }
        ui.hide().unwrap();
    }
}
