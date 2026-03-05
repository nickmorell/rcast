use egui::{Popup, Ui};
use tokio::sync::mpsc::UnboundedSender;

use crate::commands::AppCommand;
use crate::db::models::Episode;
use crate::state::AppState;
use crate::types::{Page, SortOrder};

/// Local UI state for the podcast detail page.
/// Search, sort, and the expanded-description flag are pure rendering concerns.
#[derive(Default)]
pub struct PodcastDetailPage {
    search_query: String,
    sort_order: SortOrder,
    description_expanded: bool,
}

impl Default for SortOrder {
    fn default() -> Self {
        SortOrder::PublishDateDesc
    }
}

impl PodcastDetailPage {
    pub fn render(
        &mut self,
        ui: &mut Ui,
        state: &mut AppState,
        cmd_tx: &UnboundedSender<AppCommand>,
    ) {
        // ── Loading state ─────────────────────────────────────────────────────
        let Some(podcast) = state.detail_podcast.clone() else {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.spinner();
                ui.label("Loading...");
            });
            return;
        };

        ui.vertical(|ui| {
            ui.add_space(10.0);

            // ── Header ────────────────────────────────────────────────────────
            ui.horizontal(|ui| {
                if ui
                    .button(format!("{} Back", egui_phosphor::regular::ARROW_LEFT))
                    .clicked()
                {
                    let _ = cmd_tx.send(AppCommand::NavigateTo(Page::Home));
                }

                ui.add_space(20.0);

                let texture = state
                    .image_cache
                    .get_or_load(&podcast.image_url, ui.ctx())
                    .unwrap_or_else(|| state.image_cache.get_default_texture(ui.ctx()));

                ui.add(egui::Image::new(&texture).max_size(egui::vec2(200.0, 200.0)));

                ui.add_space(20.0);

                ui.vertical(|ui| {
                    ui.set_max_width(600.0);
                    ui.heading(&podcast.title);
                    ui.add_space(10.0);

                    let should_truncate = podcast.description.len() > 250;

                    if should_truncate && !self.description_expanded {
                        // Guard against a panic if description is < 250 chars
                        // despite the flag (e.g. multibyte boundary).
                        let end = podcast
                            .description
                            .char_indices()
                            .nth(250)
                            .map(|(i, _)| i)
                            .unwrap_or(podcast.description.len());
                        let truncated = format!("{}...", &podcast.description[..end]);
                        ui.label(truncated);

                        if ui
                            .button(format!("{} Expand", egui_phosphor::regular::CARET_DOWN))
                            .clicked()
                        {
                            self.description_expanded = true;
                        }
                    } else {
                        ui.label(&podcast.description);
                        if should_truncate
                            && ui
                                .button(format!("{} Collapse", egui_phosphor::regular::CARET_UP))
                                .clicked()
                        {
                            self.description_expanded = false;
                        }
                    }

                    if ui.button("Play All").clicked() {
                        let filtered = self.filtered_episodes(&state.detail_episodes);
                        if !filtered.is_empty() {
                            let ids: Vec<i32> = filtered.iter().map(|e| e.id).collect();
                            let _ = cmd_tx.send(AppCommand::PlayAll(ids));
                        }
                    }
                });
            });

            ui.add_space(20.0);
            ui.separator();
            ui.add_space(10.0);

            // ── Episode filter bar ────────────────────────────────────────────
            ui.horizontal(|ui| {
                ui.label("Search:");
                ui.text_edit_singleline(&mut self.search_query);

                ui.add_space(20.0);

                ui.label("Sort:");
                egui::ComboBox::from_id_salt("episode_sort_order")
                    .selected_text(match self.sort_order {
                        SortOrder::AToZ => "A to Z",
                        SortOrder::ZToA => "Z to A",
                        SortOrder::PublishDateAsc => "Oldest First",
                        SortOrder::PublishDateDesc => "Newest First",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.sort_order, SortOrder::AToZ, "A to Z");
                        ui.selectable_value(&mut self.sort_order, SortOrder::ZToA, "Z to A");
                        ui.selectable_value(
                            &mut self.sort_order,
                            SortOrder::PublishDateAsc,
                            "Oldest First",
                        );
                        ui.selectable_value(
                            &mut self.sort_order,
                            SortOrder::PublishDateDesc,
                            "Newest First",
                        );
                    });
            });

            ui.add_space(10.0);

            // ── Episode list ──────────────────────────────────────────────────
            let episodes = self.filtered_episodes(&state.detail_episodes);

            let current_episode_id = state.now_playing.as_ref().map(|np| np.episode_id);

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.push_id("episodes_table", |ui| {
                    for episode in &episodes {
                        let ep_id = episode.id;
                        let is_current = current_episode_id == Some(ep_id);

                        ui.horizontal(|ui| {
                            ui.set_width(ui.available_width());

                            // Derive text color once — played episodes are muted unless
                            // currently playing (which should always feel active).
                            let text_color = if episode.is_played && !is_current {
                                egui::Color32::from_rgb(120, 120, 125)
                            } else {
                                ui.visuals().text_color()
                            };

                            // ── Play/pause icon ───────────────────────────────
                            let (rect, response) = ui
                                .allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());

                            let icon_text = if response.hovered() {
                                if is_current {
                                    egui_phosphor::regular::PAUSE
                                } else {
                                    egui_phosphor::regular::PLAY
                                }
                            } else if episode.is_played {
                                egui_phosphor::regular::RECORD
                            } else {
                                egui_phosphor::regular::CIRCLE
                            };

                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                icon_text,
                                egui::FontId::proportional(20.0),
                                text_color,
                            );

                            if response.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }

                            if response.clicked() {
                                if is_current {
                                    let _ = cmd_tx.send(AppCommand::PausePlayback);
                                } else {
                                    let _ = cmd_tx.send(AppCommand::PlayEpisode(ep_id));
                                }
                            }

                            ui.add_space(10.0);

                            // ── Title (truncated) ─────────────────────────────
                            let title = if episode.title.len() > 50 {
                                let end = episode
                                    .title
                                    .char_indices()
                                    .nth(47)
                                    .map(|(i, _)| i)
                                    .unwrap_or(episode.title.len());
                                format!("{}...", &episode.title[..end])
                            } else {
                                episode.title.clone()
                            };
                            ui.label(egui::RichText::new(title).color(text_color));

                            // ── Right-side actions ────────────────────────────
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let menu_id = egui::Id::new(format!("ctx_menu_{}", ep_id));
                                    let menu_btn = ui.button(
                                        egui::RichText::new(egui_phosphor::regular::DOTS_THREE)
                                            .size(16.0),
                                    );

                                    if menu_btn.clicked() {
                                        Popup::toggle_id(ui.ctx(), menu_id);
                                    }

                                    egui::popup_below_widget(
                                        ui,
                                        menu_id,
                                        &menu_btn,
                                        egui::PopupCloseBehavior::CloseOnClickOutside,
                                        |ui: &mut egui::Ui| {
                                            ui.set_min_width(150.0);

                                            if ui
                                                .button(if episode.is_played {
                                                    "Mark Unplayed"
                                                } else {
                                                    "Mark Played"
                                                })
                                                .clicked()
                                            {
                                                let _ =
                                                    cmd_tx.send(AppCommand::TogglePlayed(ep_id));
                                                Popup::close_id(ui.ctx(), menu_id);
                                            }

                                            if ui.button("Add to Queue").clicked() {
                                                let _ = cmd_tx.send(AppCommand::AddToQueue(ep_id));
                                                Popup::close_id(ui.ctx(), menu_id);
                                            }

                                            if ui.button("Download").clicked() {
                                                let _ =
                                                    cmd_tx.send(AppCommand::DownloadEpisode(ep_id));
                                                Popup::close_id(ui.ctx(), menu_id);
                                            }
                                        },
                                    );

                                    ui.add_space(10.0);
                                    ui.label(
                                        egui::RichText::new(episode.format_publish_date())
                                            .color(text_color),
                                    );
                                },
                            );
                        });

                        ui.separator();
                    }
                });
            });
        });
    }

    /// Filter and sort episodes locally each frame — no DB calls, no commands.
    fn filtered_episodes<'a>(&self, episodes: &'a [Episode]) -> Vec<&'a Episode> {
        let query = self.search_query.to_lowercase();

        let mut filtered: Vec<&Episode> = episodes
            .iter()
            .filter(|e| {
                query.is_empty()
                    || e.title.to_lowercase().contains(&query)
                    || e.format_publish_date().to_lowercase().contains(&query)
            })
            .collect();

        match self.sort_order {
            SortOrder::AToZ => filtered.sort_by(|a, b| a.title.cmp(&b.title)),
            SortOrder::ZToA => filtered.sort_by(|a, b| b.title.cmp(&a.title)),
            SortOrder::PublishDateDesc => {
                filtered.sort_by_key(|e| std::cmp::Reverse(e.publish_date))
            }
            SortOrder::PublishDateAsc => filtered.sort_by_key(|e| e.publish_date),
        }

        filtered
    }
}
