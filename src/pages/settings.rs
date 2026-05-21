use std::path::PathBuf;
use std::sync::Arc;

use egui::{Color32, Stroke, Ui};
use tokio::sync::{mpsc::UnboundedSender, oneshot};

use crate::commands::AppCommand;
use crate::design::components::*;
use crate::design::typography::*;
use crate::design::spacing::*;
use crate::ports::{FilePicker, FolderPicker};
use crate::state::AppState;
use crate::types::{HomeDensity, Page, Settings, ThemeMode, TrimSilenceMode};

pub struct SettingsPage {
    working: Settings,
    previous_page: Page,
    folder_picker: Option<Arc<dyn FolderPicker>>,
    file_picker: Option<Arc<dyn FilePicker>>,
    pending_folder_selection: Option<oneshot::Receiver<Option<PathBuf>>>,
    pending_import_path: Option<oneshot::Receiver<Option<PathBuf>>>,
    pending_export_path: Option<oneshot::Receiver<Option<PathBuf>>>,
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

        let t = state.theme.clone();
        let mut should_save = false;
        let mut any_slider_dragged = false;

        egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            ui.add_space(SPACE_5);

            if btn_back(ui, &t).clicked() {
                let _ = cmd_tx.send(AppCommand::NavigateTo(self.previous_page.clone()));
            }

            ui.add_space(SPACE_3);
            ui.label(text_page_title("Settings", &t));

            // ── Appearance ──────────────────────────────────────────────────
            section_header(ui, "Appearance", &t);

            ui.horizontal(|ui| {
                ui.label(text_label("Theme", &t));
                ui.add_space(CONTROL_GAP);
                if btn_segment(ui, "Dark", self.working.theme == ThemeMode::Dark, &t).clicked() {
                    self.working.theme = ThemeMode::Dark;
                    should_save = true;
                }
                ui.add_space(SPACE_1);
                if btn_segment(ui, "Light", self.working.theme == ThemeMode::Light, &t).clicked() {
                    self.working.theme = ThemeMode::Light;
                    should_save = true;
                }
            });

            // ── Playback ────────────────────────────────────────────────────
            section_header(ui, "Playback", &t);

            ui.horizontal(|ui| {
                ui.label(text_label("Default Volume:", &t));
                ui.add_space(CONTROL_GAP);
                let r = ui.add(
                    egui::Slider::new(&mut self.working.default_volume, 0.0..=100.0)
                        .text("%")
                        .fixed_decimals(0),
                );
                if r.changed() { self.slider_dirty = true; }
                if r.dragged() { any_slider_dragged = true; }
            });

            ui.add_space(CONTROL_GAP);

            ui.horizontal(|ui| {
                ui.label(text_label("Skip Backward (s):", &t));
                ui.add_space(CONTROL_GAP);
                let r = ui.add(egui::Slider::new(
                    &mut self.working.skip_backward_seconds,
                    5..=60,
                ));
                if r.changed() { self.slider_dirty = true; }
                if r.dragged() { any_slider_dragged = true; }
            });

            ui.add_space(CONTROL_GAP);

            ui.horizontal(|ui| {
                ui.label(text_label("Skip Forward (s):", &t));
                ui.add_space(CONTROL_GAP);
                let r = ui.add(egui::Slider::new(
                    &mut self.working.skip_forward_seconds,
                    5..=60,
                ));
                if r.changed() { self.slider_dirty = true; }
                if r.dragged() { any_slider_dragged = true; }
            });

            ui.add_space(CONTROL_GAP);

            ui.horizontal(|ui| {
                ui.label(text_label("Sync Interval (min):", &t));
                ui.add_space(CONTROL_GAP);
                let r = ui.add(egui::Slider::new(
                    &mut self.working.sync_interval_minutes,
                    5..=120,
                ));
                if r.changed() { self.slider_dirty = true; }
                if r.dragged() { any_slider_dragged = true; }
            });

            ui.add_space(CONTROL_GAP);

            ui.horizontal(|ui| {
                ui.label(text_label("Auto Play Next:", &t));
                ui.add_space(CONTROL_GAP);
                should_save |= ui.checkbox(&mut self.working.auto_play_next, "").changed();
            });

            ui.add_space(CONTROL_GAP);

            ui.horizontal(|ui| {
                ui.label(text_label("Home View:", &t));
                ui.add_space(CONTROL_GAP);
                if btn_segment(ui, "Grid", self.working.home_density == HomeDensity::Grid, &t)
                    .clicked()
                {
                    self.working.home_density = HomeDensity::Grid;
                    should_save = true;
                }
                ui.add_space(SPACE_1);
                if btn_segment(ui, "List", self.working.home_density == HomeDensity::List, &t)
                    .clicked()
                {
                    self.working.home_density = HomeDensity::List;
                    should_save = true;
                }
            });

            // ── Playback Defaults ────────────────────────────────────────────
            section_header(ui, "Playback Defaults", &t);

            ui.horizontal(|ui| {
                ui.label(text_label("Default Speed:", &t));
                ui.add_space(CONTROL_GAP);
                let r = ui.add(
                    egui::Slider::new(&mut self.working.default_speed, 0.5..=3.0)
                        .step_by(0.25)
                        .fixed_decimals(2)
                        .text("x"),
                );
                if r.changed() { self.slider_dirty = true; }
                if r.dragged() { any_slider_dragged = true; }
            });

            ui.add_space(CONTROL_GAP);

            ui.horizontal(|ui| {
                ui.label(text_label("Trim Silence:", &t));
                ui.add_space(CONTROL_GAP);
                if btn_segment(
                    ui,
                    "Off",
                    self.working.trim_silence_mode == TrimSilenceMode::Off,
                    &t,
                )
                .clicked()
                {
                    self.working.trim_silence_mode = TrimSilenceMode::Off;
                    should_save = true;
                }
                ui.add_space(SPACE_1);
                if btn_segment(
                    ui,
                    "Smart Speed",
                    self.working.trim_silence_mode == TrimSilenceMode::SmartSpeed,
                    &t,
                )
                .on_hover_text("Speeds up silent sections to 2×")
                .clicked()
                {
                    self.working.trim_silence_mode = TrimSilenceMode::SmartSpeed;
                    should_save = true;
                }
                ui.add_space(SPACE_1);
                if btn_segment(
                    ui,
                    "Skip Silence",
                    self.working.trim_silence_mode == TrimSilenceMode::SkipSilence,
                    &t,
                )
                .on_hover_text("Drops silent sections entirely")
                .clicked()
                {
                    self.working.trim_silence_mode = TrimSilenceMode::SkipSilence;
                    should_save = true;
                }
            });

            // ── Downloads ───────────────────────────────────────────────────
            section_header(ui, "Downloads", &t);

            ui.horizontal(|ui| {
                ui.label(text_label("Download Directory:", &t));
                ui.add_space(CONTROL_GAP);

                let is_selecting = self.pending_folder_selection.is_some();
                let dir_r = ui.add_enabled(
                    !is_selecting,
                    egui::TextEdit::singleline(&mut self.working.download_directory),
                );
                should_save |= dir_r.lost_focus();

                ui.add_space(SPACE_2);

                let button_label = if is_selecting { "Selecting…" } else { "Browse" };
                if ui
                    .add_enabled(
                        !is_selecting,
                        egui::Button::new(
                            egui::RichText::new(button_label).color(t.text_primary),
                        )
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::new(1.0, t.border)),
                    )
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

            if self.pending_folder_selection.is_none()
                && self.working.download_directory != state.settings.download_directory
            {
                should_save = true;
            }

            ui.add_space(CONTROL_GAP);

            should_save |= ui
                .checkbox(
                    &mut self.working.auto_download_new_episodes,
                    "Auto-download new episodes",
                )
                .changed();

            ui.add_space(CONTROL_GAP);

            ui.horizontal(|ui| {
                ui.label(text_label("Keep Episodes:", &t));
                ui.add_space(CONTROL_GAP);
                let r = ui
                    .add(
                        egui::Slider::new(&mut self.working.global_keep_episodes_count, 0..=50)
                            .custom_formatter(|v, _| {
                                if v == 0.0 { "All".to_string() } else { format!("{}", v as i32) }
                            }),
                    )
                    .on_hover_text(
                        "Number of downloaded episodes to keep per podcast. 0 = keep all.",
                    );
                if r.changed() { self.slider_dirty = true; }
                if r.dragged() { any_slider_dragged = true; }
            });

            // ── Subscriptions ────────────────────────────────────────────────
            section_header(ui, "Subscriptions", &t);

            ui.label(text_body(
                "Import podcasts from another app, or export your subscriptions as an OPML file.",
                &t,
            ));
            ui.add_space(SPACE_2);

            ui.horizontal(|ui| {
                let is_importing = self.pending_import_path.is_some();
                let is_exporting = self.pending_export_path.is_some();
                let busy = is_importing || is_exporting;

                if ui
                    .add_enabled(
                        !busy,
                        egui::Button::new(
                            egui::RichText::new(if is_importing {
                                "Selecting file…"
                            } else {
                                "⬆  Import OPML"
                            })
                            .color(t.text_primary),
                        )
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::new(1.0, t.border)),
                    )
                    .on_hover_text("Import podcast subscriptions from an OPML file")
                    .clicked()
                    && let Some(picker) = &self.file_picker
                {
                    self.pending_import_path =
                        Some(picker.pick_file("Import OPML Subscriptions"));
                }

                ui.add_space(CONTROL_GAP);

                if ui
                    .add_enabled(
                        !busy,
                        egui::Button::new(
                            egui::RichText::new(if is_exporting {
                                "Selecting location…"
                            } else {
                                "⬇  Export OPML"
                            })
                            .color(t.text_primary),
                        )
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::new(1.0, t.border)),
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

            // ── Notifications ────────────────────────────────────────────────
            section_header(ui, "Notifications", &t);

            should_save |= ui
                .checkbox(
                    &mut self.working.notify_new_episodes,
                    "Notify when new episodes are available",
                )
                .changed();
            ui.add_space(SPACE_1);
            should_save |= ui
                .checkbox(
                    &mut self.working.notify_download_complete,
                    "Notify when a download completes",
                )
                .changed();

            // ── Keyboard Shortcuts ───────────────────────────────────────────
            section_header(ui, "Keyboard Shortcuts", &t);

            ui.label(text_meta(
                "Format: Ctrl+Shift+P  •  Leave blank to disable",
                &t,
            ));
            ui.add_space(SPACE_2);

            egui::Grid::new("hotkeys_grid")
                .num_columns(2)
                .spacing([SPACE_4, SPACE_1])
                .show(ui, |ui| {
                    ui.label(text_label("Play / Pause:", &t));
                    should_save |= ui
                        .text_edit_singleline(&mut self.working.hotkeys.play_pause)
                        .lost_focus();
                    ui.end_row();

                    ui.label(text_label("Next:", &t));
                    should_save |= ui
                        .text_edit_singleline(&mut self.working.hotkeys.next)
                        .lost_focus();
                    ui.end_row();

                    ui.label(text_label("Skip Forward:", &t));
                    should_save |= ui
                        .text_edit_singleline(&mut self.working.hotkeys.skip_forward)
                        .lost_focus();
                    ui.end_row();

                    ui.label(text_label("Skip Backward:", &t));
                    should_save |= ui
                        .text_edit_singleline(&mut self.working.hotkeys.skip_backward)
                        .lost_focus();
                    ui.end_row();
                });

            // ── Statistics ───────────────────────────────────────────────────
            section_header(ui, "Statistics", &t);

            egui::Grid::new("stats_grid")
                .num_columns(2)
                .spacing([SPACE_4, SPACE_1])
                .show(ui, |ui| {
                    if let Some(stats) = &state.listening_stats {
                        let total_hours = stats.total_listen_seconds / 3600;
                        let total_mins = (stats.total_listen_seconds % 3600) / 60;
                        let listen_str = if total_hours > 0 {
                            format!("{} h {} m", total_hours, total_mins)
                        } else {
                            format!("{} m", total_mins)
                        };

                        ui.label(text_meta("Total listening time:", &t));
                        ui.label(text_label(listen_str, &t));
                        ui.end_row();

                        ui.label(text_meta("Episodes completed:", &t));
                        ui.label(text_label(stats.episodes_completed.to_string(), &t));
                        ui.end_row();

                        ui.label(text_meta("Shows in library:", &t));
                        ui.label(text_label(stats.total_podcasts.to_string(), &t));
                        ui.end_row();

                        ui.label(text_meta("Total episodes:", &t));
                        ui.label(text_label(stats.total_episodes.to_string(), &t));
                        ui.end_row();
                    } else {
                        ui.label(text_meta("Loading…", &t));
                        ui.end_row();
                    }
                });

            ui.add_space(SPACE_6);
        });

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
