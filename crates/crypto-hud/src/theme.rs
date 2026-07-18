use crypto_hud_shell_state::ThemePreference;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedTheme {
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy)]
pub struct ThemePalette {
    pub widget_card_background: slint::Color,
    pub widget_header_background: slint::Color,
    pub widget_border_color: slint::Color,
    pub widget_primary_text_color: slint::Color,
    pub widget_source_text_color: slint::Color,
    pub widget_updated_text_color: slint::Color,
    pub settings_background: slint::Color,
    pub settings_heading_color: slint::Color,
    pub settings_label_color: slint::Color,
    pub settings_muted_color: slint::Color,
    pub settings_status_color: slint::Color,
    pub settings_header_background: slint::Color,
    pub settings_nav_background: slint::Color,
    pub settings_content_background: slint::Color,
    pub settings_surface_background: slint::Color,
    pub settings_selected_background: slint::Color,
    pub settings_border_color: slint::Color,
    pub settings_divider_color: slint::Color,
    pub settings_accent_color: slint::Color,
    pub settings_accent_text_color: slint::Color,
    pub settings_track_color: slint::Color,
    pub settings_knob_color: slint::Color,
    pub settings_window_edge_color: slint::Color,
    pub settings_sidebar_top_border_color: slint::Color,
    pub settings_nav_selected_background: slint::Color,
    pub settings_nav_accent_color: slint::Color,
    pub settings_dark_mode: bool,
}

pub fn palette_for(preference: ThemePreference) -> ThemePalette {
    match resolve_theme(preference) {
        ResolvedTheme::Light => light_palette(),
        ResolvedTheme::Dark => dark_palette(),
    }
}

pub fn resolve_theme(preference: ThemePreference) -> ResolvedTheme {
    match preference {
        ThemePreference::Light => ResolvedTheme::Light,
        ThemePreference::Dark => ResolvedTheme::Dark,
        ThemePreference::System => system_theme(),
    }
}

pub fn resolve_taskbar_theme() -> ResolvedTheme {
    taskbar_system_theme()
}

fn dark_palette() -> ThemePalette {
    ThemePalette {
        widget_card_background: rgba(0xf6, 0x0f, 0x17, 0x26),
        widget_header_background: rgba(0xee, 0x0d, 0x1a, 0x2d),
        widget_border_color: rgba(0xa0, 0x0e, 0xa5, 0xe9),
        widget_primary_text_color: rgb(0xf8, 0xfa, 0xfc),
        widget_source_text_color: rgb(0x67, 0xd8, 0xff),
        widget_updated_text_color: rgb(0x9d, 0xb2, 0xce),
        settings_background: rgb(0x12, 0x14, 0x13),
        settings_heading_color: rgb(0xf1, 0xf4, 0xf2),
        settings_label_color: rgb(0xe7, 0xec, 0xea),
        settings_muted_color: rgb(0xb2, 0xbb, 0xb7),
        settings_status_color: rgb(0x62, 0xd6, 0xcb),
        settings_header_background: rgb(0x12, 0x14, 0x13),
        settings_nav_background: rgb(0x12, 0x14, 0x13),
        settings_content_background: rgb(0x12, 0x14, 0x13),
        settings_surface_background: rgb(0x19, 0x1d, 0x1b),
        settings_selected_background: rgb(0x17, 0x28, 0x24),
        settings_border_color: rgb(0x31, 0x38, 0x36),
        settings_divider_color: rgb(0x28, 0x2f, 0x2d),
        settings_accent_color: rgb(0x13, 0xa5, 0x9a),
        settings_accent_text_color: rgb(0xf8, 0xfa, 0xfc),
        settings_track_color: rgb(0x37, 0x43, 0x40),
        settings_knob_color: rgb(0xf0, 0xf7, 0xf4),
        settings_window_edge_color: rgb(0x42, 0x48, 0x45),
        settings_sidebar_top_border_color: rgb(0x2a, 0x30, 0x2e),
        settings_nav_selected_background: rgb(0x22, 0x2c, 0x29),
        settings_nav_accent_color: rgb(0x19, 0xbe, 0xb1),
        settings_dark_mode: true,
    }
}

fn light_palette() -> ThemePalette {
    ThemePalette {
        widget_card_background: rgba(0xfa, 0xf8, 0xfb, 0xff),
        widget_header_background: rgba(0xed, 0xea, 0xf7, 0xff),
        widget_border_color: rgba(0x80, 0x0e, 0xa5, 0xe9),
        widget_primary_text_color: rgb(0x07, 0x14, 0x26),
        widget_source_text_color: rgb(0x03, 0x69, 0xa1),
        widget_updated_text_color: rgb(0x64, 0x74, 0x8b),
        settings_background: rgb(0xfc, 0xfc, 0xfa),
        settings_heading_color: rgb(0x15, 0x18, 0x1b),
        settings_label_color: rgb(0x22, 0x27, 0x2c),
        settings_muted_color: rgb(0x70, 0x74, 0x7a),
        settings_status_color: rgb(0x08, 0x7f, 0x78),
        settings_header_background: rgb(0xfc, 0xfc, 0xfa),
        settings_nav_background: rgb(0xfc, 0xfc, 0xfa),
        settings_content_background: rgb(0xfc, 0xfc, 0xfa),
        settings_surface_background: rgb(0xff, 0xff, 0xff),
        settings_selected_background: rgb(0xf0, 0xf7, 0xf5),
        settings_border_color: rgb(0xe7, 0xe7, 0xe8),
        settings_divider_color: rgb(0xe8, 0xe8, 0xe8),
        settings_accent_color: rgb(0x0a, 0x7f, 0x78),
        settings_accent_text_color: rgb(0xff, 0xff, 0xff),
        settings_track_color: rgb(0xc9, 0xd7, 0xd0),
        settings_knob_color: rgb(0xff, 0xff, 0xff),
        settings_window_edge_color: rgb(0xa9, 0xa9, 0xaa),
        settings_sidebar_top_border_color: rgb(0xee, 0xee, 0xed),
        settings_nav_selected_background: rgb(0xeb, 0xf1, 0xef),
        settings_nav_accent_color: rgb(0x0a, 0x71, 0x6e),
        settings_dark_mode: false,
    }
}

const fn rgb(red: u8, green: u8, blue: u8) -> slint::Color {
    slint::Color::from_rgb_u8(red, green, blue)
}

const fn rgba(alpha: u8, red: u8, green: u8, blue: u8) -> slint::Color {
    slint::Color::from_argb_u8(alpha, red, green, blue)
}

#[cfg(windows)]
fn system_theme() -> ResolvedTheme {
    registry_theme("AppsUseLightTheme")
}

#[cfg(windows)]
fn taskbar_system_theme() -> ResolvedTheme {
    registry_theme("SystemUsesLightTheme")
}

#[cfg(windows)]
fn registry_theme(value_name: &str) -> ResolvedTheme {
    use std::ffi::c_void;
    use windows_sys::Win32::{
        Foundation::ERROR_SUCCESS,
        System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD},
    };

    let subkey = wide_null("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
    let value = wide_null(value_name);
    let mut data = 1_u32;
    let mut data_size = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&mut data as *mut u32).cast::<c_void>(),
            &mut data_size,
        )
    };

    if status == ERROR_SUCCESS && data != 0 {
        ResolvedTheme::Light
    } else {
        ResolvedTheme::Dark
    }
}

#[cfg(not(windows))]
fn system_theme() -> ResolvedTheme {
    ResolvedTheme::Dark
}

#[cfg(not(windows))]
fn taskbar_system_theme() -> ResolvedTheme {
    ResolvedTheme::Dark
}

#[cfg(windows)]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_theme_preference_overrides_system() {
        assert_eq!(resolve_theme(ThemePreference::Light), ResolvedTheme::Light);
        assert_eq!(resolve_theme(ThemePreference::Dark), ResolvedTheme::Dark);
    }

    #[test]
    fn taskbar_theme_uses_the_windows_shell_preference() {
        assert!(include_str!("theme.rs").contains("registry_theme(\"SystemUsesLightTheme\")"));
    }

    #[test]
    fn settings_palettes_keep_text_readable() {
        for palette in [
            palette_for(ThemePreference::Dark),
            palette_for(ThemePreference::Light),
        ] {
            assert!(
                contrast_ratio(
                    palette.settings_heading_color,
                    palette.settings_content_background
                ) >= 7.0
            );
            assert!(
                contrast_ratio(
                    palette.settings_label_color,
                    palette.settings_content_background
                ) >= 4.5
            );
            assert!(
                contrast_ratio(
                    palette.settings_muted_color,
                    palette.settings_content_background
                ) >= 3.0
            );
            assert!(
                contrast_ratio(
                    palette.settings_label_color,
                    palette.settings_surface_background
                ) >= 4.5
            );
            assert!(
                contrast_ratio(
                    palette.settings_status_color,
                    palette.settings_content_background
                ) >= 3.0
            );
        }
    }

    fn contrast_ratio(foreground: slint::Color, background: slint::Color) -> f32 {
        let foreground = relative_luminance(foreground);
        let background = relative_luminance(background);
        let lighter = foreground.max(background);
        let darker = foreground.min(background);
        (lighter + 0.05) / (darker + 0.05)
    }

    fn relative_luminance(color: slint::Color) -> f32 {
        let argb = color.to_argb_u8();
        let red = linear_channel(argb.red as f32 / 255.0);
        let green = linear_channel(argb.green as f32 / 255.0);
        let blue = linear_channel(argb.blue as f32 / 255.0);
        0.2126 * red + 0.7152 * green + 0.0722 * blue
    }

    fn linear_channel(value: f32) -> f32 {
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }
}
