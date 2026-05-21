use egui::{Color32, Stroke, Visuals};
use super::tokens::ThemeTokens;

pub fn build_visuals(t: &ThemeTokens) -> Visuals {
    let mut v = Visuals::dark();

    v.dark_mode = t.is_dark;
    v.panel_fill = t.page_bg;
    v.window_fill = t.card_bg;
    v.extreme_bg_color = t.input_bg;
    v.faint_bg_color = t.hover_bg;

    // Text / selection colors that egui falls back to
    v.hyperlink_color = t.accent;
    v.selection.bg_fill = t.accent;
    v.selection.stroke = Stroke::new(1.0, t.accent);

    // Non-interactive (labels, separators)
    v.widgets.noninteractive.bg_fill = t.card_bg;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, t.border);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, t.text_primary);

    // Idle interactive (buttons, sliders, text inputs)
    v.widgets.inactive.bg_fill = t.border;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, t.border);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, t.text_primary);

    // Hovered
    v.widgets.hovered.bg_fill = t.hover_bg;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, t.text_secondary);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, t.text_primary);

    // Active / pressed
    v.widgets.active.bg_fill = t.accent;
    v.widgets.active.bg_stroke = Stroke::new(1.0, t.accent);
    v.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);

    // Open (ComboBox dropdown button while menu is visible)
    v.widgets.open.bg_fill = t.input_bg;
    v.widgets.open.bg_stroke = Stroke::new(1.0, t.accent);
    v.widgets.open.fg_stroke = Stroke::new(1.0, t.text_primary);

    v
}
