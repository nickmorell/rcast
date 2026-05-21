#![allow(dead_code)]
use egui::{Button, Color32, Response, Stroke, Ui, Vec2};
use super::tokens::ThemeTokens;
use super::typography::*;
use super::spacing::*;

// ─── Buttons ────────────────────────────────────────────────────────────────

pub fn btn_primary(ui: &mut Ui, label: &str, t: &ThemeTokens) -> Response {
    ui.add(
        Button::new(text_button(label).color(Color32::WHITE))
            .fill(t.accent)
            .corner_radius(rounding_sm()),
    )
}

pub fn btn_primary_enabled(ui: &mut Ui, label: &str, enabled: bool, t: &ThemeTokens) -> Response {
    ui.add_enabled(
        enabled,
        Button::new(text_button(label).color(Color32::WHITE))
            .fill(t.accent)
            .corner_radius(rounding_sm()),
    )
}

pub fn btn_secondary(ui: &mut Ui, label: &str, t: &ThemeTokens) -> Response {
    ui.add(
        Button::new(text_button(label).color(t.text_primary))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0, t.border))
            .corner_radius(rounding_sm()),
    )
}

pub fn btn_back(ui: &mut Ui, t: &ThemeTokens) -> Response {
    btn_ghost(ui, &format!("{} Back", egui_phosphor::regular::ARROW_LEFT), t)
}

pub fn btn_ghost(ui: &mut Ui, label: &str, t: &ThemeTokens) -> Response {
    ui.add(
        Button::new(text_button(label).color(t.accent))
            .fill(Color32::TRANSPARENT)
            .frame(false),
    )
}

pub fn btn_destructive(ui: &mut Ui, label: &str, t: &ThemeTokens) -> Response {
    ui.add(
        Button::new(text_button(label).color(t.error))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0, t.error))
            .corner_radius(rounding_sm()),
    )
}

pub fn btn_segment(ui: &mut Ui, label: &str, active: bool, t: &ThemeTokens) -> Response {
    let (fg, bg, stroke) = if active {
        (t.accent, t.accent_tint, Stroke::new(1.0, t.accent))
    } else {
        (t.text_secondary, Color32::TRANSPARENT, Stroke::new(1.0, t.border))
    };
    ui.add(
        Button::new(text_button(label).color(fg))
            .fill(bg)
            .stroke(stroke)
            .corner_radius(rounding_sm()),
    )
}

// ─── Section header ──────────────────────────────────────────────────────────

pub fn section_header(ui: &mut Ui, label: &str, t: &ThemeTokens) {
    ui.add_space(SECTION_GAP);
    ui.label(
        egui::RichText::new(label.to_uppercase())
            .font(egui::FontId::new(
                FONT_SM,
                egui::FontFamily::Name("Medium".into()),
            ))
            .color(t.accent),
    );
    ui.add_space(SPACE_2);
}

// ─── Divider ─────────────────────────────────────────────────────────────────

pub fn divider(ui: &mut Ui, t: &ThemeTokens) {
    ui.add_space(SPACE_2);
    let rect = ui.available_rect_before_wrap();
    let y = rect.top();
    ui.painter().hline(rect.x_range(), y, Stroke::new(1.0, t.divider));
    ui.add_space(SPACE_2);
}

// ─── Episode state dot ───────────────────────────────────────────────────────

pub fn episode_dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 5.0, color);
}

// ─── Placeholder artwork ─────────────────────────────────────────────────────

pub fn artwork_placeholder(ui: &mut Ui, size: f32, t: &ThemeTokens) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    ui.painter().rect_filled(rect, rounding_md(), t.card_bg);
    ui.painter().rect_stroke(
        rect,
        rounding_md(),
        Stroke::new(1.0, t.border),
        egui::epaint::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        egui_phosphor::regular::MICROPHONE,
        egui::FontId::new(size * 0.35, egui::FontFamily::Name("phosphor".into())),
        t.text_disabled,
    );
}
