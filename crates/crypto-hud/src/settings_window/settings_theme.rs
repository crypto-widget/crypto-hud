use crypto_hud_shell_state::AppSettings;

use crate::{theme, SettingsWindow};

pub(super) fn apply_theme_to_settings_window(ui: &SettingsWindow, settings: AppSettings) {
    let palette = theme::palette_for(settings.theme);
    ui.set_settings_background(palette.settings_background);
    ui.set_settings_heading_color(palette.settings_heading_color);
    ui.set_settings_label_color(palette.settings_label_color);
    ui.set_settings_muted_color(palette.settings_muted_color);
    ui.set_settings_status_color(palette.settings_status_color);
    ui.set_settings_header_background(palette.settings_header_background);
    ui.set_settings_nav_background(palette.settings_nav_background);
    ui.set_settings_content_background(palette.settings_content_background);
    ui.set_settings_surface_background(palette.settings_surface_background);
    ui.set_settings_selected_background(palette.settings_selected_background);
    ui.set_settings_border_color(palette.settings_border_color);
    ui.set_settings_divider_color(palette.settings_divider_color);
    ui.set_settings_accent_color(palette.settings_accent_color);
    ui.set_settings_accent_text_color(palette.settings_accent_text_color);
    ui.set_settings_track_color(palette.settings_track_color);
    ui.set_settings_knob_color(palette.settings_knob_color);
    ui.set_settings_window_edge_color(palette.settings_window_edge_color);
    ui.set_settings_sidebar_top_border_color(palette.settings_sidebar_top_border_color);
    ui.set_settings_nav_selected_background(palette.settings_nav_selected_background);
    ui.set_settings_nav_accent_color(palette.settings_nav_accent_color);
    ui.set_settings_dark_mode(palette.settings_dark_mode);
    ui.set_settings_preview_card_background(palette.widget_card_background);
    ui.set_settings_preview_header_background(palette.widget_header_background);
    ui.set_settings_preview_border_color(palette.widget_border_color);
    ui.set_settings_preview_primary_color(palette.widget_primary_text_color);
    ui.set_settings_preview_source_color(palette.widget_source_text_color);
    ui.set_settings_preview_updated_color(palette.widget_updated_text_color);
}
