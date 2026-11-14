// SPDX-License-Identifier: MIT OR Apache-2.0

use egui::{Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle, Vec2};

pub const BG: Color32 = Color32::from_rgb(11, 13, 16);
pub const PANEL: Color32 = Color32::from_rgb(18, 22, 29);
pub const PANEL_ALT: Color32 = Color32::from_rgb(23, 28, 37);
pub const LINE: Color32 = Color32::from_rgb(35, 42, 53);
pub const TEXT: Color32 = Color32::from_rgb(233, 237, 242);
pub const MUTED: Color32 = Color32::from_rgb(139, 150, 167);
pub const ACCENT: Color32 = Color32::from_rgb(255, 106, 61);
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(255, 176, 120);
pub const TEAL: Color32 = Color32::from_rgb(55, 214, 196);
pub const GREEN: Color32 = Color32::from_rgb(120, 220, 140);
pub const DANGER: Color32 = Color32::from_rgb(255, 84, 112);

pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = Color32::from_rgb(9, 11, 14);
    visuals.faint_bg_color = PANEL_ALT;
    visuals.override_text_color = Some(TEXT);
    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(255, 106, 61, 90);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT_SOFT);
    visuals.hyperlink_color = TEAL;
    visuals.widgets.noninteractive.bg_fill = PANEL;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, MUTED);
    visuals.widgets.inactive.bg_fill = PANEL_ALT;
    visuals.widgets.inactive.weak_bg_fill = PANEL_ALT;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(30, 37, 48);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(30, 37, 48);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.active.bg_fill = Color32::from_rgb(38, 46, 59);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(38, 46, 59);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.open.bg_fill = PANEL_ALT;
    visuals.window_corner_radius = CornerRadius::same(14);
    visuals.menu_corner_radius = CornerRadius::same(12);
    visuals.window_stroke = Stroke::new(1.0, LINE);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(9.0, 5.0);
    style.spacing.interact_size = Vec2::new(30.0, 26.0);
    style.spacing.scroll.bar_width = 8.0;
    style.spacing.scroll.floating = true;
    style.text_styles.insert(
        TextStyle::Body,
        FontId::new(13.5, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(13.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(11.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(12.0, FontFamily::Monospace),
    );
    ctx.set_style(style);
}
