use std::path::PathBuf;
use std::sync::Arc;

use egui::Ui;
use tokio::sync::{mpsc::UnboundedSender, oneshot};

use crate::commands::AppCommand;
use crate::ports::{FilePicker, FolderPicker};
use crate::state::AppState;
use crate::types::{HomeDensity, Page, Settings, TrimSilenceMode};

pub struct SettingsPage {
    working: Settings,
    previous_page: Page,
    folder_picker: Option<Arc<dyn FolderPicker>>,
    file_picker: Option<Arc<dyn FilePicker>>,
    pending_folder_selection: Option<oneshot::Receiver<Option<PathBuf>>>,
    pending_import_path: Option<oneshot::Receiver<Option<PathBuf>>>,
    pending_export_path: Option<oneshot::Receiver<Option<PathBuf>>>,
    /// True while at least one slider has been dragged but not yet committed.
    slider_dirty: bool,
}

impl Default for SettingsPage {
    fn default() -> Self {
        Self {
            working: Settings::default(),
            previous_page: Page::Home,
            folder_picker: None,
            file_picker: None,
            pending_folder_selection: None,
            pending_import_path: None,
            pending_export_path: None,
            slider_dirty: false,
        }
    }
}

impl SettingsPage {
    pub fn set_folder_picker(&mut self, picker: Arc<dyn FolderPicker>) {
        self.folder_picker = Some(picker);
    }

    pub fn set_file_picker(&mut self, picker: Arc<dyn FilePicker>) {
        self.file_picker = Some(picker);
    }

    pub fn load(&mut self, settings: Settings) {
        self.working = settings;
    }

    fn poll_folder_selection(&mut self) {
        if let Some(rx) = self.pending_folder_selection.as_mut() {
            match rx.try_recv() {
                Ok(Some(path)) => {
                    self.working.download_directory = path.to_string_lossy().to_string();
                    self.pending_folder_selection = None;
                }
                Ok(None) => {
                    self.pending_folder_selection = None;
                }
                Err(oneshot::error::TryRecvError::Empty) => {}
                Err(oneshot::error::TryRecvError::Closed) => {
                    self.pending_folder_selection = None;
                }
            }
        }
    }

    // Returns the import path if the dialog just resolved, `None` otherwise.
    fn poll_import_path(&mut self) -> Option<PathBuf> {
        if let Some(rx) = self.pending_import_path.as_mut() {
            match rx.try_recv() {
                Ok(result) => {
                    self.pending_import_path = None;
                    return result;
                }
                Err(oneshot::error::TryRecvError::Closed) => {
                    self.pending_import_path = None;
                }
                Err(oneshot::error::TryRecvError::Empty) => {}
            }
        }
        None
    }

    fn poll_export_path(&mut self) -> Option<PathBuf> {
        if let Some(rx) = self.pending_export_path.as_mut() {
            match rx.try_recv() {
                Ok(result) => {
                    self.pending_export_path = None;
                    return result;
                }
                Err(oneshot::error::TryRecvError::Closed) => {
                    self.pending_export_path = None;
                }
                Err(oneshot::error::TryRecvError::Empty) => {}
            }
        }
        None
    }

    pub fn render(
        &mut self,
        ui: &mut Ui,
        state: &mut AppState,
        cmd_tx: &UnboundedSender<AppCommand>,
    ) {
        self.poll_folder_selection();

        if let Some(path) = self.poll_import_path() {
            let _ = cmd_tx.send(AppCommand::ImportOpml { path });
        }
        if let Some(path) = self.poll_export_path() {
            let _ = cmd_tx.send(AppCommand::ExportOpml { path });
        }

        // Track whether any setting changed this frame so we can auto-save.
        let mut should_save = false;
        // Track whether any slider is actively being dragged this frame.
        // We save slider changes only after the drag ends to avoid spamming DB writes.
        let mut any_slider_dragged = false;

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(20.0);

            if ui
                .button(
                    egui::RichText::new(format!(
                        "{}  Back",
                        egui_phosphor::regular::ARROW_LEFT
                    ))
                    .size(13.0),
                )
                .clicked()
            {
                let _ = cmd_tx.send(AppCommand::NavigateTo(self.previous_page.clone()));
            }

            ui.add_space(12.0);
            ui.heading("Settings");
            ui.add_space(20.0);

            ui.horizontal(|ui| {
                ui.label("Default Volume:");
                ui.add_space(10.0);
                let r = ui.add(
                    egui::Slider::new(&mut self.working.default_volume, 0.0..=100.0)
                        .text("%")
                        .fixed_decimals(0),
                );
                if r.changed() { self.slider_dirty = true; }
                if r.dragged() { any_slider_dragged = true; }
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Skip Backward (seconds):");
                ui.add_space(10.0);
                let r = ui.add(egui::Slider::new(
                    &mut self.working.skip_backward_seconds,
                    5..=60,
                ));
                if r.changed() { self.slider_dirty = true; }
                if r.dragged() { any_slider_dragged = true; }
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Skip Forward (seconds):");
                ui.add_space(10.0);
                let r = ui.add(egui::Slider::new(
                    &mut self.working.skip_forward_seconds,
                    5..=60,
                ));
                if r.changed() { self.slider_dirty = true; }
                if r.dragged() { any_slider_dragged = true; }
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Sync Interval (minutes):");
                ui.add_space(10.0);
                let r = ui.add(egui::Slider::new(
                    &mut self.working.sync_interval_minutes,
                    5..=120,
                ));
                if r.changed() { self.slider_dirty = true; }
                if r.dragged() { any_slider_dragged = true; }
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Auto Play Next:");
                ui.add_space(10.0);
                should_save |= ui.checkbox(&mut self.working.auto_play_next, "").changed();
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Home View:");
                ui.add_space(10.0);
                should_save |= ui
                    .selectable_value(&mut self.working.home_density, HomeDensity::Grid, "Grid")
                    .changed();
                ui.add_space(4.0);
                should_save |= ui
                    .selectable_value(&mut self.working.home_density, HomeDensity::List, "List")
                    .changed();
            });

            ui.add_space(40.0);

            ui.horizontal(|ui| {
                ui.label("Download Directory:");
                ui.add_space(10.0);

                let is_selecting = self.pending_folder_selection.is_some();
                let dir_r = ui.add_enabled(
                    !is_selecting,
                    egui::TextEdit::singleline(&mut self.working.download_directory),
                );
                // Save when the user commits the download directory text field.
                should_save |= dir_r.lost_focus();

                ui.add_space(5.0);

                let button_text = if is_selecting { "Selecting..." } else { "Browse" };

                if ui
                    .add_enabled(!is_selecting, egui::Button::new(button_text))
                    .clicked()
                    && let Some(picker) = &self.folder_picker
                {
                    let start_path = if self.working.download_directory.is_empty() {
                        None
                    } else {
                        PathBuf::from(&self.working.download_directory)
                            .parent()
                            .map(|p| p.to_path_buf())
                    };
                    self.pending_folder_selection = Some(picker.select_folder(start_path));
                }
            });

            // Save after folder picker resolves (poll_folder_selection already updated working).
            if self.pending_folder_selection.is_none()
                && self.working.download_directory != state.settings.download_directory
            {
                should_save = true;
            }

            ui.add_space(20.0);
            ui.separator();
            ui.add_space(16.0);

            // OPML import / export
            ui.label(egui::RichText::new("Subscriptions").strong().size(14.0));
            ui.add_space(8.0);

            ui.label(
                egui::RichText::new(
                    "Import podcasts from another app, or export your subscriptions as an OPML file.",
                )
                .small()
                .color(egui::Color32::from_rgb(150, 150, 155)),
            );

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                let is_importing = self.pending_import_path.is_some();
                let is_exporting = self.pending_export_path.is_some();
                let busy = is_importing || is_exporting;

                if ui
                    .add_enabled(
                        !busy,
                        egui::Button::new(if is_importing {
                            "Selecting file..."
                        } else {
                            "⬆  Import OPML"
                        }),
                    )
                    .on_hover_text("Import podcast subscriptions from an OPML file")
                    .clicked()
                    && let Some(picker) = &self.file_picker
                {
                    self.pending_import_path =
                        Some(picker.pick_file("Import OPML Subscriptions"));
                }

                ui.add_space(10.0);

                if ui
                    .add_enabled(
                        !busy,
                        egui::Button::new(if is_exporting {
                            "Selecting location..."
                        } else {
                            "⬇  Export OPML"
                        }),
                    )
                    .on_hover_text("Export your subscriptions as an OPML file")
                    .clicked()
                    && let Some(picker) = &self.file_picker
                {
                    self.pending_export_path = Some(
                        picker.save_file("Export OPML Subscriptions", "rcast-subscriptions.opml"),
                    );
                }
            });

            ui.add_space(20.0);
            ui.separator();
            ui.add_space(16.0);

            // --- Playback Defaults ---
            ui.label(egui::RichText::new("Playback Defaults").strong().size(14.0));
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("Default Speed:");
                ui.add_space(10.0);
                let r = ui.add(
                    egui::Slider::new(&mut self.working.default_speed, 0.5..=3.0)
                        .step_by(0.25)
                        .fixed_decimals(2)
                        .text("x"),
                );
                if r.changed() { self.slider_dirty = true; }
                if r.dragged() { any_slider_dragged = true; }
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("Trim Silence:");
                ui.add_space(10.0);
                should_save |= ui
                    .selectable_value(
                        &mut self.working.trim_silence_mode,
                        TrimSilenceMode::Off,
                        "Off",
                    )
                    .changed();
                ui.add_space(4.0);
                should_save |= ui
                    .selectable_value(
                        &mut self.working.trim_silence_mode,
                        TrimSilenceMode::SmartSpeed,
                        "Smart Speed",
                    )
                    .on_hover_text("Speeds up silent sections to 2×")
                    .changed();
                ui.add_space(4.0);
                should_save |= ui
                    .selectable_value(
                        &mut self.working.trim_silence_mode,
                        TrimSilenceMode::SkipSilence,
                        "Skip Silence",
                    )
                    .on_hover_text("Drops silent sections entirely")
                    .changed();
            });

            ui.add_space(20.0);
            ui.separator();
            ui.add_space(16.0);

            // --- Downloads ---
            ui.label(egui::RichText::new("Downloads").strong().size(14.0));
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                should_save |= ui
                    .checkbox(
                        &mut self.working.auto_download_new_episodes,
                        "Auto-download new episodes",
                    )
                    .changed();
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("Keep Episodes:");
                ui.add_space(10.0);
                let r = ui
                    .add(
                        egui::Slider::new(&mut self.working.global_keep_episodes_count, 0..=50)
                            .custom_formatter(|v, _| {
                                if v == 0.0 {
                                    "All".to_string()
                                } else {
                                    format!("{}", v as i32)
                                }
                            }),
                    )
                    .on_hover_text(
                        "Number of downloaded episodes to keep per podcast. 0 = keep all.",
                    );
                if r.changed() { self.slider_dirty = true; }
                if r.dragged() { any_slider_dragged = true; }
            });

            ui.add_space(20.0);
            ui.separator();
            ui.add_space(16.0);

            // --- Notifications ---
            ui.label(egui::RichText::new("Notifications").strong().size(14.0));
            ui.add_space(8.0);

            should_save |= ui
                .checkbox(
                    &mut self.working.notify_new_episodes,
                    "Notify when new episodes are available",
                )
                .changed();
            ui.add_space(4.0);
            should_save |= ui
                .checkbox(
                    &mut self.working.notify_download_complete,
                    "Notify when a download completes",
                )
                .changed();

            ui.add_space(20.0);
            ui.separator();
            ui.add_space(16.0);

            // --- Hotkeys ---
            ui.label(egui::RichText::new("Keyboard Shortcuts").strong().size(14.0));
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Format: Ctrl+Shift+P  •  Leave blank to disable")
                    .small()
                    .color(egui::Color32::from_rgb(150, 150, 155)),
            );
            ui.add_space(8.0);

            egui::Grid::new("hotkeys_grid")
                .num_columns(2)
                .spacing([16.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Play / Pause:");
                    should_save |= ui
                        .text_edit_singleline(&mut self.working.hotkeys.play_pause)
                        .lost_focus();
                    ui.end_row();

                    ui.label("Next:");
                    should_save |= ui
                        .text_edit_singleline(&mut self.working.hotkeys.next)
                        .lost_focus();
                    ui.end_row();

                    ui.label("Skip Forward:");
                    should_save |= ui
                        .text_edit_singleline(&mut self.working.hotkeys.skip_forward)
                        .lost_focus();
                    ui.end_row();

                    ui.label("Skip Backward:");
                    should_save |= ui
                        .text_edit_singleline(&mut self.working.hotkeys.skip_backward)
                        .lost_focus();
                    ui.end_row();
                });

            ui.add_space(20.0);
            ui.separator();
            ui.add_space(16.0);

            // --- Statistics ---
            ui.label(egui::RichText::new("Statistics").strong().size(14.0));
            ui.add_space(8.0);

            egui::Grid::new("stats_grid")
                .num_columns(2)
                .spacing([16.0, 6.0])
                .show(ui, |ui| {
                    if let Some(stats) = &state.listening_stats {
                        let total_hours = stats.total_listen_seconds / 3600;
                        let total_mins = (stats.total_listen_seconds % 3600) / 60;
                        let listen_str = if total_hours > 0 {
                            format!("{} h {} m", total_hours, total_mins)
                        } else {
                            format!("{} m", total_mins)
                        };

                        ui.label(
                            egui::RichText::new("Total listening time:")
                                .color(egui::Color32::from_rgb(150, 150, 155)),
                        );
                        ui.label(listen_str);
                        ui.end_row();

                        ui.label(
                            egui::RichText::new("Episodes completed:")
                                .color(egui::Color32::from_rgb(150, 150, 155)),
                        );
                        ui.label(stats.episodes_completed.to_string());
                        ui.end_row();

                        ui.label(
                            egui::RichText::new("Shows in library:")
                                .color(egui::Color32::from_rgb(150, 150, 155)),
                        );
                        ui.label(stats.total_podcasts.to_string());
                        ui.end_row();

                        ui.label(
                            egui::RichText::new("Total episodes:")
                                .color(egui::Color32::from_rgb(150, 150, 155)),
                        );
                        ui.label(stats.total_episodes.to_string());
                        ui.end_row();
                    } else {
                        ui.label(
                            egui::RichText::new("Loading…")
                                .color(egui::Color32::from_rgb(150, 150, 155)),
                        );
                        ui.end_row();
                    }
                });

            ui.add_space(32.0);
        });

        // Commit slider changes once the user releases the drag handle.
        if self.slider_dirty && !any_slider_dragged {
            self.slider_dirty = false;
            should_save = true;
        }

        if should_save {
            state.settings = self.working.clone();
            let _ = cmd_tx.send(AppCommand::SaveSettings(self.working.clone()));
        }
    }
}
