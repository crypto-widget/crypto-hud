use std::path::PathBuf;

use slint::{platform::WindowEvent, ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer};
use slint_interpreter::{ComponentInstance, Value};

fn displayed_icon(ui: &ComponentInstance) -> Image {
    let Value::Image(image) = ui.get_property("inspected-icon-source").unwrap() else {
        panic!("inspected icon must be an image");
    };
    image
}

/// Exercise each card's actual binding with a decoded PNG, transparent raster
/// texels, and an SVG, including the host's routine model replacement.
pub(crate) fn verify_card_icon_sampling(
    ui: &ComponentInstance,
    widget: &crate::widget_host::WidgetUi,
    view: &crypto_hud_runtime::WidgetRuntimeView,
    natural_size: (f32, f32),
    icon_width: f32,
    mut capture: impl FnMut(&str),
) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let png = Image::load_from_path(
        &root.join("plugins/com.cryptohud.market-compass/ui/champagne-coin-atlas-200.png"),
    )
    .unwrap();
    assert_eq!(png.size().width, png.size().height * 2);
    let svg =
        Image::load_from_path(&root.join("plugins/com.cryptohud.mint-tile/ui/icons/unlock.svg"))
            .unwrap();
    let mut pixels = SharedPixelBuffer::<Rgba8Pixel>::new(96, 48);
    for (index, pixel) in pixels.make_mut_slice().iter_mut().enumerate() {
        *pixel = if (index + index / 96).is_multiple_of(2) {
            Rgba8Pixel::new(32, 192, 96, 255)
        } else {
            Rgba8Pixel::new(0, 0, 255, 0)
        };
    }
    let translucent = Image::from_rgba8(pixels);
    for theme in ["light", "dark"] {
        ui.set_property("theme-name", Value::String(theme.into()))
            .unwrap();
        for dpi in [1.0_f32, 1.25, 1.5] {
            ui.window()
                .dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor: dpi });
            for scale in [0.3_f32, 0.5, 0.65] {
                ui.set_property("widget-scale", Value::Number(f64::from(scale)))
                    .unwrap();
                ui.set_property(
                    "widget-width",
                    Value::Number(f64::from((natural_size.0 * scale).round())),
                )
                .unwrap();
                ui.set_property(
                    "widget-height",
                    Value::Number(f64::from((natural_size.1 * scale).round())),
                )
                .unwrap();
                ui.window().set_size(slint::LogicalSize::new(
                    natural_size.0 * scale,
                    natural_size.1 * scale,
                ));
                let apply = |icons: &[Image]| {
                    crate::widget_host::apply_runtime_view_to_widget(
                        widget,
                        view,
                        icons,
                        &[!icons.is_empty()],
                        !icons.is_empty(),
                        Default::default(),
                        scale,
                    );
                };
                let name = format!(
                    "icon-{theme}-scale-{}-dpi-{}",
                    (scale * 100.0).round(),
                    (dpi * 100.0).round()
                );
                apply(std::slice::from_ref(&png));
                capture(&name);
                let filtered = displayed_icon(ui);
                let box_size = (icon_width * scale * dpi).round() as u32;
                assert_eq!(filtered.size().width, box_size, "{name}");
                assert!(
                    filtered.size().height.abs_diff(box_size.div_ceil(2)) <= 1,
                    "{name}: preserve non-square aspect ratio"
                );
                assert_ne!(filtered, png, "{name}: PNG must be filtered before display");
                assert!(filtered
                    .to_rgba8()
                    .unwrap()
                    .as_slice()
                    .iter()
                    .any(|pixel| pixel.a > 0 && pixel.a < 255));
                apply(std::slice::from_ref(&png));
                capture(&format!("{name}-refresh"));
                assert_eq!(
                    displayed_icon(ui),
                    filtered,
                    "{name}: refresh must reuse filtered pixels"
                );
                apply(&[]);
                let hidden_size = displayed_icon(ui).size();
                assert_eq!((hidden_size.width, hidden_size.height), (0, 0));
                apply(std::slice::from_ref(&png));
                assert_eq!(
                    displayed_icon(ui),
                    filtered,
                    "{name}: toggling icons must reuse cached pixels"
                );
                apply(std::slice::from_ref(&translucent));
                let translucent_output = displayed_icon(ui).to_rgba8().unwrap();
                assert_eq!(translucent_output.width(), box_size);
                for pixel in translucent_output.as_slice() {
                    assert!(
                        pixel.a > 0 && pixel.a < 255,
                        "{name}: preserve coverage alpha"
                    );
                    assert_eq!(
                        (pixel.r, pixel.g, pixel.b),
                        (32, 192, 96),
                        "{name}: transparent blue must not create a halo"
                    );
                }
                apply(std::slice::from_ref(&svg));
                assert_eq!(
                    displayed_icon(ui),
                    svg,
                    "{name}: SVG must remain vector artwork"
                );
            }
        }
    }
    ui.window()
        .dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor: 1.0 });
    for (property, value) in [
        ("widget-scale", 1.0),
        ("widget-width", natural_size.0),
        ("widget-height", natural_size.1),
    ] {
        ui.set_property(property, Value::Number(f64::from(value)))
            .unwrap();
    }
    ui.window()
        .set_size(slint::LogicalSize::new(natural_size.0, natural_size.1));
}
