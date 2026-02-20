use crate::{database::Database, image_cache::ImageCache, types::Podcast};
use egui::{Color32, Response, Ui};

pub struct PodcastCard<'a> {
    podcast: &'a Podcast,
    database: &'a Database,
    image_cache: &'a ImageCache,
    is_playing: bool,
}

impl<'a> PodcastCard<'a> {
    pub fn new(
        podcast: &'a Podcast,
        database: &'a Database,
        image_cache: &'a ImageCache,
        is_playing: bool,
    ) -> Self {
        Self {
            podcast,
            database,
            image_cache,
            is_playing,
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

            let info_rect = egui::Rect::from_min_size(
                rect.min + egui::vec2(8.0, CARD_SIZE),
                egui::vec2(CARD_SIZE - 16.0, 60.0),
            );

            ui.scope_builder(egui::UiBuilder::new().max_rect(info_rect), |ui| {
                ui.vertical(|ui| {
                    ui.add_space(4.0);

                    let title = if self.podcast.title.len() > 30 {
                        format!("{}...", &self.podcast.title[..27])
                    } else {
                        self.podcast.title.clone()
                    };

                    ui.label(egui::RichText::new(title).strong());

                    let episode_count = self
                        .database
                        .get_episode_count_by_podcast(self.podcast.id.unwrap())
                        .unwrap_or(0);

                    ui.label(
                        egui::RichText::new(format!("{} episodes", episode_count))
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
