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

        ui.vertical(|ui| {
            ui.set_width(CARD_SIZE);
            ui.set_height(TOTAL_HEIGHT);

            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(CARD_SIZE, TOTAL_HEIGHT), egui::Sense::click());

            if self.is_playing {
                ui.painter().rect(
                    rect.expand(4.0),
                    12.0,
                    Color32::TRANSPARENT,
                    egui::Stroke::new(3.0, Color32::from_rgb(70, 130, 220)),
                    egui::epaint::StrokeKind::Outside,
                );
            }

            let bg_color = if response.hovered() {
                Color32::from_rgb(48, 48, 50)
            } else {
                Color32::from_rgb(28, 28, 30)
            };

            ui.painter().rect_filled(rect, 8.0, bg_color);
            ui.painter().rect_stroke(
                rect,
                8.0,
                egui::Stroke::new(1.0, Color32::from_rgb(48, 48, 50)),
                egui::epaint::StrokeKind::Outside,
            );

            let image_rect = egui::Rect::from_min_size(
                rect.min + egui::vec2(8.0, 8.0),
                egui::vec2(CARD_SIZE - 16.0, CARD_SIZE - 16.0),
            );

            let texture = self
                .image_cache
                .get_or_load(&self.podcast.image_url, ui.ctx())
                .unwrap_or_else(|| self.image_cache.get_default_texture(ui.ctx()));

            ui.put(
                image_rect,
                egui::Image::new(&texture).fit_to_exact_size(image_rect.size()),
            );

            // ── Sync spinner overlay (top-right corner of image) ──────────────
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
                // Keep repainting while syncing so the spinner animates.
                ui.ctx().request_repaint();
            }

            let info_rect = egui::Rect::from_min_size(
                rect.min + egui::vec2(8.0, CARD_SIZE),
                egui::vec2(CARD_SIZE - 16.0, 60.0),
            );

            ui.scope_builder(egui::UiBuilder::new().max_rect(info_rect), |ui| {
                ui.vertical(|ui| {
                    ui.add_space(4.0);

                    let title = if self.podcast.title.len() > 30 {
                        let end = self
                            .podcast
                            .title
                            .char_indices()
                            .nth(27)
                            .map(|(i, _)| i)
                            .unwrap_or(self.podcast.title.len());
                        format!("{}...", &self.podcast.title[..end])
                    } else {
                        self.podcast.title.clone()
                    };

                    ui.label(egui::RichText::new(title).strong());

                    // Episode count + last synced time on the same row.
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

/// Returns a human-readable "synced X ago" string from a unix timestamp.
fn format_last_synced(timestamp: i64) -> String {
    let now = chrono::Utc::now().timestamp();
    let diff = now - timestamp;

    if diff < 60 {
        "Synced just now".to_string()
    } else if diff < 3600 {
        let mins = diff / 60;
        format!("Synced {}m ago", mins)
    } else if diff < 86400 {
        let hours = diff / 3600;
        format!("Synced {}h ago", hours)
    } else {
        let days = diff / 86400;
        format!("Synced {}d ago", days)
    }
}
