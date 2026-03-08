use egui::{Color32, Response, Ui};

use crate::db::models::Podcast;
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
        Self {
            podcast,
            image_cache,
            is_playing,
            is_syncing,
        }
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        const CARD_SIZE: f32 = 200.0;
        const TOTAL_HEIGHT: f32 = 270.0;
        // Maximum extra pixels added at full hover (1% of card width each side ≈ 2px)
        const MAX_EXPAND: f32 = 4.0;
        // Lerp speed per frame - reaches 1.0 in ~8 frames (≈130ms at 60fps)
        const LERP_SPEED: f32 = 0.12;

        ui.vertical(|ui| {
            ui.set_width(CARD_SIZE);
            ui.set_height(TOTAL_HEIGHT);

            // Allocate the layout rect at normal size so nothing shifts.
            let (layout_rect, response) =
                ui.allocate_exact_size(egui::vec2(CARD_SIZE, TOTAL_HEIGHT), egui::Sense::click());

            // Per-card hover animation state
            let anim_id = egui::Id::new("podcast_card_hover").with(self.podcast.id);
            let hovered = response.hovered();

            let hover_t = ui.ctx().data_mut(|d| {
                let t: &mut f32 = d.get_temp_mut_or_default(anim_id);
                if hovered {
                    *t = (*t + LERP_SPEED).min(1.0);
                } else {
                    *t = (*t - LERP_SPEED).max(0.0);
                }
                *t
            });

            if hover_t > 0.0 && hover_t < 1.0 {
                ui.ctx().request_repaint();
            }

            let ease = 1.0 - (1.0 - hover_t).powi(3);
            let expand = ease * MAX_EXPAND;

            let paint_rect = layout_rect.expand(expand);

            // Playing state glow
            if self.is_playing {
                // Soft outer glow ring
                ui.painter().rect(
                    paint_rect.expand(6.0),
                    12.0,
                    Color32::TRANSPARENT,
                    egui::Stroke::new(4.0, Color32::from_rgba_premultiplied(70, 130, 220, 60)),
                    egui::epaint::StrokeKind::Outside,
                );
                // Crisp inner border
                ui.painter().rect(
                    paint_rect,
                    10.0,
                    Color32::TRANSPARENT,
                    egui::Stroke::new(2.0, Color32::from_rgb(100, 160, 255)),
                    egui::epaint::StrokeKind::Outside,
                );
            }

            // Card background
            let bg_rest = Color32::from_rgb(28, 28, 30);
            let bg_hover = Color32::from_rgb(52, 52, 56);
            let bg = lerp_color(bg_rest, bg_hover, ease);

            ui.painter().rect_filled(paint_rect, 10.0, bg);
            ui.painter().rect_stroke(
                paint_rect,
                10.0,
                egui::Stroke::new(
                    1.0,
                    lerp_color(
                        Color32::from_rgb(48, 48, 52),
                        Color32::from_rgb(72, 72, 80),
                        ease,
                    ),
                ),
                egui::epaint::StrokeKind::Outside,
            );

            // Podcast Image
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
                    .corner_radius(6.0),
            );

            // Now playing badge
            if self.is_playing {
                // Small pill at the bottom-left of the image
                let badge_w = 82.0;
                let badge_h = 20.0;
                let badge_rect = egui::Rect::from_min_size(
                    egui::pos2(image_rect.min.x + 6.0, image_rect.max.y - badge_h - 6.0),
                    egui::vec2(badge_w, badge_h),
                );

                ui.painter().rect_filled(
                    badge_rect,
                    badge_h / 2.0,
                    Color32::from_rgba_premultiplied(20, 50, 110, 230),
                );
                ui.painter().text(
                    badge_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{}  NOW PLAYING", egui_phosphor::regular::SPEAKER_HIGH),
                    egui::FontId::proportional(10.0),
                    Color32::from_rgb(140, 180, 255),
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
                    6.0,
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
                    ui.add_space(4.0);

                    let title = truncate(&self.podcast.title, 30);
                    ui.label(egui::RichText::new(title).strong());

                    let sync_text = if self.is_syncing {
                        "Syncing...".to_string()
                    } else if self.podcast.last_synced_at == 0 {
                        "Never synced".to_string()
                    } else {
                        format_last_synced(self.podcast.last_synced_at)
                    };

                    ui.label(
                        egui::RichText::new(format!(
                            "{} ep{}  ·  {}",
                            self.podcast.episode_count,
                            if self.podcast.episode_count == 1 {
                                ""
                            } else {
                                "s"
                            },
                            sync_text
                        ))
                        .small()
                        .color(Color32::from_rgb(150, 150, 150)),
                    );
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

// Helpers

// Linear interpolation between two `Color32` values.
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
