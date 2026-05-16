use egui::Ui;
use tokio::sync::mpsc::UnboundedSender;

use crate::commands::AppCommand;
use crate::db::models::{DownloadStatus, Episode};
use crate::state::AppState;
use crate::types::{Page, PodcastPreferences, SortOrder};

pub struct PodcastDetailPage {
    search_query: String,
    sort_order: SortOrder,
    description_expanded: bool,
    // Per-show preferences panel
    prefs_open: bool,
    working_prefs: PodcastPreferences,
    prefs_loaded_for: Option<i32>,
}

impl Default for SortOrder {
    fn default() -> Self {
        SortOrder::PublishDateDesc
    }
}

impl Default for PodcastDetailPage {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            sort_order: SortOrder::PublishDateDesc,
            description_expanded: false,
            prefs_open: false,
            working_prefs: PodcastPreferences::default(),
            prefs_loaded_for: None,
        }
    }
}

impl PodcastDetailPage {
    pub fn render(
        &mut self,
        ui: &mut Ui,
        state: &mut AppState,
        cmd_tx: &UnboundedSender<AppCommand>,
        is_paused: bool,
    ) {
        // Loading state
        let Some(podcast) = state.detail_podcast.clone() else {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.spinner();
                ui.label("Loading...");
            });
            return;
        };

        // Sync working_prefs when we load a new podcast
        if self.prefs_loaded_for != Some(podcast.id) {
            self.working_prefs = PodcastPreferences {
                speed_preset: podcast.speed_preset,
                auto_download: podcast.auto_download,
                keep_episodes_count: podcast.keep_episodes_count,
                skip_intro_seconds: podcast.skip_intro_seconds,
                skip_outro_seconds: podcast.skip_outro_seconds,
            };
            self.prefs_loaded_for = Some(podcast.id);
        }

        ui.vertical(|ui| {
            ui.add_space(10.0);

            // Header
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

            // Per-show preferences panel
            let podcast_id = podcast.id;
            let prefs_header = egui::CollapsingHeader::new(
                egui::RichText::new(format!(
                    "{}  Podcast Settings",
                    egui_phosphor::regular::SLIDERS
                ))
                .size(13.0),
            )
            .id_salt("podcast_prefs_panel")
            .open(Some(self.prefs_open));

            let prefs_response = prefs_header.show(ui, |ui| {
                egui::Grid::new("podcast_prefs_grid")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        // Speed preset
                        ui.label("Speed Preset:");
                        ui.horizontal(|ui| {
                            let has_preset = self.working_prefs.speed_preset.is_some();
                            let mut speed_val =
                                self.working_prefs.speed_preset.unwrap_or(1.0);
                            if ui
                                .add_enabled(
                                    has_preset,
                                    egui::Slider::new(&mut speed_val, 0.5..=3.0)
                                        .step_by(0.25)
                                        .fixed_decimals(2)
                                        .text("x"),
                                )
                                .changed()
                            {
                                self.working_prefs.speed_preset = Some(speed_val);
                            }
                            ui.add_space(8.0);
                            if has_preset {
                                if ui
                                    .button("Use Global")
                                    .on_hover_text("Inherit speed from global settings")
                                    .clicked()
                                {
                                    self.working_prefs.speed_preset = None;
                                }
                            } else if ui
                                .button("Override")
                                .on_hover_text("Set a custom speed for this podcast")
                                .clicked()
                            {
                                self.working_prefs.speed_preset =
                                    Some(state.settings.default_speed);
                            }
                        });
                        ui.end_row();

                        // Auto-download
                        ui.label("Auto-Download:");
                        ui.horizontal(|ui| {
                            let use_global = self.working_prefs.auto_download.is_none();
                            let enabled = self.working_prefs.auto_download.unwrap_or(false);
                            if ui.selectable_label(use_global, "Use Global").clicked() {
                                self.working_prefs.auto_download = None;
                            }
                            ui.add_space(4.0);
                            if ui
                                .add_enabled(
                                    !use_global,
                                    egui::Button::new("Off").selected(!use_global && !enabled),
                                )
                                .clicked()
                            {
                                self.working_prefs.auto_download = Some(false);
                            }
                            ui.add_space(4.0);
                            if ui
                                .add_enabled(
                                    !use_global,
                                    egui::Button::new("On").selected(!use_global && enabled),
                                )
                                .clicked()
                            {
                                self.working_prefs.auto_download = Some(true);
                            }
                        });
                        ui.end_row();

                        // Keep episodes count
                        ui.label("Keep Episodes:");
                        ui.horizontal(|ui| {
                            let has_keep = self.working_prefs.keep_episodes_count.is_some();
                            let mut keep_val =
                                self.working_prefs.keep_episodes_count.unwrap_or(0);
                            if ui
                                .add_enabled(
                                    has_keep,
                                    egui::Slider::new(&mut keep_val, 0..=50).custom_formatter(
                                        |v, _| {
                                            if v == 0.0 {
                                                "All".to_string()
                                            } else {
                                                format!("{}", v as i32)
                                            }
                                        },
                                    ),
                                )
                                .changed()
                            {
                                self.working_prefs.keep_episodes_count = Some(keep_val);
                            }
                            ui.add_space(8.0);
                            if has_keep {
                                if ui.button("Use Global").clicked() {
                                    self.working_prefs.keep_episodes_count = None;
                                }
                            } else {
                                if ui.button("Override").clicked() {
                                    self.working_prefs.keep_episodes_count = Some(0);
                                }
                            }
                        });
                        ui.end_row();

                        // Skip intro
                        ui.label("Skip Intro (sec):");
                        ui.add(
                            egui::DragValue::new(&mut self.working_prefs.skip_intro_seconds)
                                .range(0..=300)
                                .speed(1.0),
                        );
                        ui.end_row();

                        // Skip outro
                        ui.label("Skip Outro (sec):");
                        ui.add(
                            egui::DragValue::new(&mut self.working_prefs.skip_outro_seconds)
                                .range(0..=300)
                                .speed(1.0),
                        );
                        ui.end_row();
                    });

                ui.add_space(8.0);
                if ui.button("Save Podcast Settings").clicked() {
                    let _ = cmd_tx.send(AppCommand::UpdatePodcastPreferences {
                        podcast_id,
                        prefs: self.working_prefs.clone(),
                    });
                    self.prefs_loaded_for = None; // force re-sync after save
                }
            });

            if prefs_response.header_response.clicked() {
                self.prefs_open = !self.prefs_open;
            }

            ui.add_space(10.0);

            // Episode filter bar
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

            // Episode list
            let episodes = self.filtered_episodes(&state.detail_episodes);

            let current_episode_id = state.now_playing.as_ref().map(|np| np.episode_id);

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.push_id("episodes_table", |ui| {
                    for episode in &episodes {
                        let ep_id = episode.id;
                        let is_current = current_episode_id == Some(ep_id);

                        let row_frame = if is_current {
                            egui::Frame::default()
                                .stroke(egui::Stroke::new(
                                    1.5,
                                    egui::Color32::from_rgb(70, 130, 220),
                                ))
                                .inner_margin(egui::Margin {
                                    left: 6,
                                    right: 2,
                                    top: 2,
                                    bottom: 2,
                                })
                        } else {
                            egui::Frame::default()
                        };

                        row_frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.set_width(ui.available_width());

                            let text_color = if episode.is_played && !is_current {
                                egui::Color32::from_rgb(120, 120, 125)
                            } else {
                                ui.visuals().text_color()
                            };

                            // Play/pause icon
                            let (rect, response) = ui
                                .allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());

                            let icon_text = if response.hovered() {
                                if is_current {
                                    // Show PLAY when paused (clicking will resume),
                                    // PAUSE when playing (clicking will pause).
                                    if is_paused {
                                        egui_phosphor::regular::PLAY
                                    } else {
                                        egui_phosphor::regular::PAUSE
                                    }
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
                                    // TogglePlayback handles both pause→resume and play→pause
                                    let _ = cmd_tx.send(AppCommand::TogglePlayback);
                                } else {
                                    let _ = cmd_tx.send(AppCommand::PlayEpisode(ep_id));
                                }
                            }

                            ui.add_space(10.0);

                            // Title (truncated)
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

                            // Right-side actions
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let menu_btn = ui.button(
                                        egui::RichText::new(egui_phosphor::regular::DOTS_THREE)
                                            .size(16.0),
                                    );

                                    egui::Popup::menu(&menu_btn).show(|ui: &mut egui::Ui| {
                                        ui.set_min_width(150.0);

                                        if ui
                                            .button(if episode.is_played {
                                                "Mark Unplayed"
                                            } else {
                                                "Mark Played"
                                            })
                                            .clicked()
                                        {
                                            let _ = cmd_tx.send(AppCommand::TogglePlayed(ep_id));
                                            ui.close();
                                        }

                                        if ui.button("Add to Queue").clicked() {
                                            let _ = cmd_tx.send(AppCommand::AddToQueue(ep_id));
                                            ui.close();
                                        }

                                        match episode.download_status {
                                            DownloadStatus::NotDownloaded
                                            | DownloadStatus::Failed => {
                                                if ui.button("Download").clicked() {
                                                    let _ = cmd_tx
                                                        .send(AppCommand::DownloadEpisode(ep_id));
                                                    ui.close();
                                                }
                                            }
                                            DownloadStatus::Downloading => {
                                                ui.add_enabled(false, egui::Button::new("Downloading…"));
                                            }
                                            DownloadStatus::Downloaded => {
                                                if ui.button("Delete Download").clicked() {
                                                    let _ = cmd_tx
                                                        .send(AppCommand::DeleteDownload(ep_id));
                                                    ui.close();
                                                }
                                            }
                                        }
                                    });

                                    ui.add_space(10.0);

                                    // Download status indicator
                                    match episode.download_status {
                                        DownloadStatus::Downloaded => {
                                            ui.label(
                                                egui::RichText::new(
                                                    egui_phosphor::regular::DOWNLOAD_SIMPLE,
                                                )
                                                .size(14.0)
                                                .color(egui::Color32::from_rgb(80, 180, 100)),
                                            )
                                            .on_hover_text("Downloaded");
                                        }
                                        DownloadStatus::Downloading => {
                                            ui.spinner();
                                        }
                                        DownloadStatus::Failed => {
                                            ui.label(
                                                egui::RichText::new(
                                                    egui_phosphor::regular::WARNING,
                                                )
                                                .size(14.0)
                                                .color(egui::Color32::from_rgb(220, 100, 80)),
                                            )
                                            .on_hover_text("Download failed");
                                        }
                                        DownloadStatus::NotDownloaded => {}
                                    }

                                    ui.add_space(6.0);
                                    ui.label(
                                        egui::RichText::new(episode.format_publish_date())
                                            .color(text_color),
                                    );

                                    ui.add_space(6.0);

                                    let notes_btn = ui
                                        .button(
                                            egui::RichText::new(
                                                egui_phosphor::regular::NOTE_PENCIL,
                                            )
                                            .size(15.0)
                                            .color(egui::Color32::from_rgb(130, 130, 140)),
                                        )
                                        .on_hover_text("Notes");
                                    if notes_btn.clicked() {
                                        state.notes_open_request = Some((
                                            ep_id,
                                            episode.podcast_id,
                                            episode.title.clone(),
                                        ));
                                    }
                                },
                            );
                        });
                        }); // row_frame

                        ui.separator();
                    }
                });
            });
        });
    }

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
