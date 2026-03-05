use egui::Ui;
use tokio::sync::mpsc::UnboundedSender;

use crate::commands::AppCommand;
use crate::components::podcast_card::PodcastCard;
use crate::db::models::Podcast;
use crate::state::AppState;
use crate::types::{Page, SortOrder};

/// Local UI state for the home page.
/// Search query and sort order are purely rendering concerns — they never
/// go to the orchestrator. `show_queue` and `show_speed_menu` live here
/// because they're passed through to MediaControls in application.rs.
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

            // ── Filter / sort bar ─────────────────────────────────────────────
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
            });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(20.0);

            // ── Filter + sort (local, no commands, rebuilt every frame) ────────
            // Collect matching IDs first to avoid borrow-checker conflicts with
            // `state.image_cache` below.
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

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(5.0);
                ui.horizontal_wrapped(|ui| {
                    for id in podcast_ids {
                        let Some(podcast) = state.podcasts.iter().find(|p| p.id == id) else {
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
            });
        });
    }
}
