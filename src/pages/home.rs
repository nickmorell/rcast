use egui::Ui;
use tokio::sync::mpsc::UnboundedSender;

use crate::commands::AppCommand;
use crate::components::podcast_card::PodcastCard;
use crate::db::models::Podcast;
use crate::state::AppState;
use crate::types::{HomeDensity, Page, Settings, SortOrder};

pub struct HomePage {
    search_query: String,
    sort_order: SortOrder,
    pub show_queue: bool,
    pub show_speed_menu: bool,
}

impl Default for HomePage {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            sort_order: SortOrder::AToZ,
            show_queue: false,
            show_speed_menu: false,
        }
    }
}

impl HomePage {
    pub fn render(
        &mut self,
        ui: &mut Ui,
        state: &mut AppState,
        cmd_tx: &UnboundedSender<AppCommand>,
    ) {
        if state.podcasts.is_empty() {
            let available = ui.available_size();
            ui.allocate_ui_with_layout(
                available,
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    ui.add_space(available.y * 0.28);
                    ui.label(
                        egui::RichText::new(egui_phosphor::regular::HEADPHONES)
                            .size(72.0)
                            .color(egui::Color32::from_rgb(80, 80, 85)),
                    );
                    ui.add_space(16.0);
                    ui.label(egui::RichText::new("No podcasts yet").size(22.0).strong());
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("Add a podcast to get started.")
                            .size(14.0)
                            .color(egui::Color32::from_rgb(150, 150, 155)),
                    );
                    ui.add_space(20.0);
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(format!(
                                    "{}  Add Podcast",
                                    egui_phosphor::regular::PLUS_CIRCLE
                                ))
                                .size(15.0),
                            )
                            .min_size(egui::vec2(140.0, 36.0)),
                        )
                        .clicked()
                    {
                        state.open_add_podcast_requested = true;
                    }
                },
            );
            return;
        }

        ui.vertical(|ui| {
            ui.add_space(10.0);

            // Filter / sort / density bar
            ui.horizontal(|ui| {
                ui.label("Search:");
                ui.text_edit_singleline(&mut self.search_query);

                ui.add_space(20.0);

                ui.label("Sort:");
                egui::ComboBox::from_id_salt("sort_order")
                    .selected_text(match self.sort_order {
                        SortOrder::AToZ => "A to Z",
                        SortOrder::ZToA => "Z to A",
                        SortOrder::PublishDateAsc => "Publish Date (Oldest)",
                        SortOrder::PublishDateDesc => "Publish Date (Newest)",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.sort_order, SortOrder::AToZ, "A to Z");
                        ui.selectable_value(&mut self.sort_order, SortOrder::ZToA, "Z to A");
                        ui.selectable_value(
                            &mut self.sort_order,
                            SortOrder::PublishDateAsc,
                            "Publish Date (Oldest)",
                        );
                        ui.selectable_value(
                            &mut self.sort_order,
                            SortOrder::PublishDateDesc,
                            "Publish Date (Newest)",
                        );
                    });

                // Density toggle
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let is_list = state.settings.home_density == HomeDensity::List;

                    let list_color = if is_list {
                        egui::Color32::from_rgb(140, 180, 255)
                    } else {
                        egui::Color32::from_rgb(120, 120, 130)
                    };
                    let grid_color = if !is_list {
                        egui::Color32::from_rgb(140, 180, 255)
                    } else {
                        egui::Color32::from_rgb(120, 120, 130)
                    };

                    if ui
                        .button(
                            egui::RichText::new(egui_phosphor::regular::LIST)
                                .size(18.0)
                                .color(list_color),
                        )
                        .on_hover_text("List view")
                        .clicked()
                        && !is_list
                    {
                        state.settings.home_density = HomeDensity::List;
                        let _ = cmd_tx.send(AppCommand::SaveSettings(state.settings.clone()));
                    }

                    if ui
                        .button(
                            egui::RichText::new(egui_phosphor::regular::GRID_FOUR)
                                .size(18.0)
                                .color(grid_color),
                        )
                        .on_hover_text("Grid view")
                        .clicked()
                        && is_list
                    {
                        state.settings.home_density = HomeDensity::Grid;
                        let _ = cmd_tx.send(AppCommand::SaveSettings(state.settings.clone()));
                    }
                });
            });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            // Filter + sort
            let query = self.search_query.to_lowercase();
            let mut filtered: Vec<&Podcast> = state
                .podcasts
                .iter()
                .filter(|p| {
                    query.is_empty()
                        || p.title.to_lowercase().contains(&query)
                        || p.description.to_lowercase().contains(&query)
                })
                .collect();

            match self.sort_order {
                SortOrder::AToZ => filtered.sort_by(|a, b| a.title.cmp(&b.title)),
                SortOrder::ZToA => filtered.sort_by(|a, b| b.title.cmp(&a.title)),
                SortOrder::PublishDateAsc => filtered.sort_by_key(|p| p.updated_at),
                SortOrder::PublishDateDesc => {
                    filtered.sort_by_key(|p| std::cmp::Reverse(p.updated_at))
                }
            }

            let podcast_ids: Vec<i32> = filtered.iter().map(|p| p.id).collect();
            let density = state.settings.home_density;

            egui::ScrollArea::vertical().show(ui, |ui| match density {
                HomeDensity::Grid => {
                    ui.add_space(5.0);
                    ui.horizontal_wrapped(|ui| {
                        for id in &podcast_ids {
                            let Some(podcast) = state.podcasts.iter().find(|p| p.id == *id) else {
                                continue;
                            };
                            let is_playing = state
                                .now_playing
                                .as_ref()
                                .map(|np| np.podcast_id == podcast.id)
                                .unwrap_or(false);
                            let is_syncing = state.syncing_podcast_ids.contains(&podcast.id);
                            ui.add_space(10.0);
                            if PodcastCard::new(podcast, &state.image_cache, is_playing, is_syncing)
                                .show(ui)
                                .clicked()
                            {
                                let _ = cmd_tx
                                    .send(AppCommand::NavigateTo(Page::PodcastDetail(podcast.id)));
                            }
                        }
                    });
                }
                HomeDensity::List => {
                    ui.set_width(ui.available_width());
                    for id in &podcast_ids {
                        let Some(podcast) = state.podcasts.iter().find(|p| p.id == *id) else {
                            continue;
                        };
                        let is_playing = state
                            .now_playing
                            .as_ref()
                            .map(|np| np.podcast_id == podcast.id)
                            .unwrap_or(false);
                        let is_syncing = state.syncing_podcast_ids.contains(&podcast.id);

                        if render_podcast_row(
                            ui,
                            podcast,
                            is_playing,
                            is_syncing,
                            &state.image_cache,
                        ) {
                            let _ = cmd_tx
                                .send(AppCommand::NavigateTo(Page::PodcastDetail(podcast.id)));
                        }

                        ui.separator();
                    }
                }
            });
        });
    }
}

// List-view row

fn render_podcast_row(
    ui: &mut egui::Ui,
    podcast: &Podcast,
    is_playing: bool,
    is_syncing: bool,
    image_cache: &crate::image_cache::ImageCache,
) -> bool {
    let row_height = 56.0;
    let thumb_size = 40.0;

    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), row_height),
        egui::Sense::click(),
    );

    if response.hovered() {
        ui.painter()
            .rect_filled(rect, 0.0, egui::Color32::from_rgb(38, 38, 44));
    }

    // Blue left border when playing
    if is_playing {
        ui.painter().rect_filled(
            egui::Rect::from_min_size(rect.min, egui::vec2(3.0, row_height)),
            0.0,
            egui::Color32::from_rgb(70, 130, 220),
        );
    }

    let inner_rect = rect.shrink2(egui::vec2(12.0, 8.0));
    let thumb_rect = egui::Rect::from_min_size(
        egui::pos2(
            inner_rect.min.x + if is_playing { 8.0 } else { 0.0 },
            inner_rect.min.y,
        ),
        egui::vec2(thumb_size, thumb_size),
    );

    // Thumbnail
    let texture = image_cache
        .get_or_load(&podcast.image_url, ui.ctx())
        .unwrap_or_else(|| image_cache.get_default_texture(ui.ctx()));

    ui.put(
        thumb_rect,
        egui::Image::new(&texture)
            .fit_to_exact_size(egui::vec2(thumb_size, thumb_size))
            .corner_radius(4.0),
    );

    // Text column
    let text_x = thumb_rect.right() + 12.0;
    let text_width = inner_rect.right() - text_x - if is_syncing { 28.0 } else { 4.0 };
    let text_rect = egui::Rect::from_min_size(
        egui::pos2(text_x, inner_rect.min.y),
        egui::vec2(text_width, inner_rect.height()),
    );

    ui.scope_builder(egui::UiBuilder::new().max_rect(text_rect), |ui| {
        ui.vertical(|ui| {
            ui.add_space(4.0);
            ui.label(egui::RichText::new(&podcast.title).strong().size(14.0));

            let sync_text = if is_syncing {
                "Syncing...".to_string()
            } else if podcast.last_synced_at == 0 {
                "Never synced".to_string()
            } else {
                format_last_synced(podcast.last_synced_at)
            };

            ui.label(
                egui::RichText::new(format!(
                    "{} ep{}  ·  {}",
                    podcast.episode_count,
                    if podcast.episode_count == 1 { "" } else { "s" },
                    sync_text,
                ))
                .size(11.0)
                .color(egui::Color32::from_rgb(130, 130, 140)),
            );
        });
    });

    // Spinner at right edge when syncing
    if is_syncing {
        let spinner_rect = egui::Rect::from_center_size(
            egui::pos2(rect.right() - 20.0, rect.center().y),
            egui::vec2(16.0, 16.0),
        );
        ui.put(spinner_rect, egui::Spinner::new().size(14.0));
        ui.ctx().request_repaint();
    }

    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    response.clicked()
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
