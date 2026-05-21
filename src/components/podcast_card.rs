use egui::{Color32, Response, Ui};

use crate::db::models::Podcast;
use crate::design::spacing::*;
use crate::design::tokens::ThemeTokens;
use crate::design::typography::*;
use crate::image_cache::ImageCache;

pub struct PodcastCard<'a> {
    podcast: &'a Podcast,
    image_cache: &'a ImageCache,
    is_playing: bool,
    is_syncing: bool,
}

impl<'a> PodcastCard<'a> {
    pub fn new(
        podcast: &'a Podcast,
        image_cache: &'a ImageCache,
        is_playing: bool,
        is_syncing: bool,
    ) -> Self {
        Self { podcast, image_cache, is_playing, is_syncing }
    }

    pub fn show(self, ui: &mut Ui, t: &ThemeTokens) -> Response {
        const CARD_SIZE: f32 = 200.0;
        const TOTAL_HEIGHT: f32 = 270.0;
        const MAX_EXPAND: f32 = 4.0;
        const LERP_SPEED: f32 = 0.12;

        ui.vertical(|ui| {
            ui.set_width(CARD_SIZE);
            ui.set_height(TOTAL_HEIGHT);

            let (layout_rect, response) =
                ui.allocate_exact_size(egui::vec2(CARD_SIZE, TOTAL_HEIGHT), egui::Sense::click());

            let anim_id = egui::Id::new("podcast_card_hover").with(self.podcast.id);
            let hovered = response.hovered();

            let hover_t = ui.ctx().data_mut(|d| {
                let val: &mut f32 = d.get_temp_mut_or_default(anim_id);
                if hovered {
                    *val = (*val + LERP_SPEED).min(1.0);
                } else {
                    *val = (*val - LERP_SPEED).max(0.0);
                }
                *val
            });

            if hover_t > 0.0 && hover_t < 1.0 {
                ui.ctx().request_repaint();
            }

            let ease = 1.0 - (1.0 - hover_t).powi(3);
            let expand = ease * MAX_EXPAND;
            let paint_rect = layout_rect.expand(expand);

            // Playing glow
            if self.is_playing {
                ui.painter().rect(
                    paint_rect.expand(6.0),
                    RADIUS_LG,
                    Color32::TRANSPARENT,
                    egui::Stroke::new(
                        4.0,
                        Color32::from_rgba_premultiplied(
                            t.in_progress.r(),
                            t.in_progress.g(),
                            t.in_progress.b(),
                            60,
                        ),
                    ),
                    egui::epaint::StrokeKind::Outside,
                );
                ui.painter().rect(
                    paint_rect,
                    RADIUS_MD,
                    Color32::TRANSPARENT,
                    egui::Stroke::new(2.0, t.in_progress),
                    egui::epaint::StrokeKind::Outside,
                );
            }

            // Card background with hover animation
            let bg = lerp_color(t.card_bg, t.hover_bg, ease);
            let border = lerp_color(t.border, t.text_meta, ease);

            ui.painter().rect_filled(paint_rect, RADIUS_MD, bg);
            ui.painter().rect_stroke(
                paint_rect,
                RADIUS_MD,
                egui::Stroke::new(1.0, border),
                egui::epaint::StrokeKind::Outside,
            );

            // Podcast image
            let image_rect = egui::Rect::from_min_size(
                paint_rect.min + egui::vec2(8.0, 8.0),
                egui::vec2(
                    CARD_SIZE - 16.0 + expand * 2.0,
                    CARD_SIZE - 16.0 + expand * 2.0,
                ),
            );

            let texture = self
                .image_cache
                .get_or_load(&self.podcast.image_url, ui.ctx())
                .unwrap_or_else(|| self.image_cache.get_default_texture(ui.ctx()));

            ui.put(
                image_rect,
                egui::Image::new(&texture)
                    .fit_to_exact_size(image_rect.size())
                    .corner_radius(RADIUS_SM),
            );

            // Now playing badge
            if self.is_playing {
                let badge_w = 82.0;
                let badge_h = 20.0;
                let badge_rect = egui::Rect::from_min_size(
                    egui::pos2(image_rect.min.x + 6.0, image_rect.max.y - badge_h - 6.0),
                    egui::vec2(badge_w, badge_h),
                );

                ui.painter().rect_filled(
                    badge_rect,
                    badge_h / 2.0,
                    Color32::from_rgba_premultiplied(0, 0, 0, 200),
                );
                ui.painter().text(
                    badge_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{}  NOW PLAYING", egui_phosphor::regular::SPEAKER_HIGH),
                    egui::FontId::proportional(10.0),
                    t.accent,
                );
            }

            // Sync spinner overlay
            if self.is_syncing {
                let spinner_rect = egui::Rect::from_center_size(
                    image_rect.right_top() + egui::vec2(-16.0, 16.0),
                    egui::vec2(24.0, 24.0),
                );
                ui.painter().rect_filled(
                    spinner_rect.expand(4.0),
                    RADIUS_SM,
                    Color32::from_rgba_premultiplied(0, 0, 0, 180),
                );
                ui.put(spinner_rect, egui::Spinner::new().size(16.0));
                ui.ctx().request_repaint();
            }

            // Info area
            let info_rect = egui::Rect::from_min_size(
                paint_rect.min + egui::vec2(8.0, CARD_SIZE + expand * 2.0),
                egui::vec2(CARD_SIZE - 16.0 + expand * 2.0, 60.0),
            );

            ui.scope_builder(egui::UiBuilder::new().max_rect(info_rect), |ui| {
                ui.vertical(|ui| {
                    ui.add_space(SPACE_1);
                    ui.label(text_podcast_card_name(truncate(&self.podcast.title, 30), t));

                    let sync_text = if self.is_syncing {
                        "Syncing...".to_string()
                    } else if self.podcast.last_synced_at == 0 {
                        "Never synced".to_string()
                    } else {
                        format_last_synced(self.podcast.last_synced_at)
                    };

                    ui.label(text_meta(
                        format!(
                            "{} ep{}  ·  {}",
                            self.podcast.episode_count,
                            if self.podcast.episode_count == 1 { "" } else { "s" },
                            sync_text,
                        ),
                        t,
                    ));
                });
            });

            if response.hovered() {
                response.on_hover_cursor(egui::CursorIcon::PointingHand)
            } else {
                response
            }
        })
        .inner
    }
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_premultiplied(
        lerp_u8(a.r(), b.r(), t),
        lerp_u8(a.g(), b.g(), t),
        lerp_u8(a.b(), b.b(), t),
        lerp_u8(a.a(), b.a(), t),
    )
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let end = s
            .char_indices()
            .nth(max - 3)
            .map(|(i, _)| i)
            .unwrap_or(s.len());
        format!("{}...", &s[..end])
    }
}

fn format_last_synced(timestamp: i64) -> String {
    let now = chrono::Utc::now().timestamp();
    let diff = now - timestamp;
    if diff < 60 {
        "Synced just now".to_string()
    } else if diff < 3600 {
        format!("Synced {}m ago", diff / 60)
    } else if diff < 86400 {
        format!("Synced {}h ago", diff / 3600)
    } else {
        format!("Synced {}d ago", diff / 86400)
    }
}
