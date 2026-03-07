use std::path::PathBuf;
use std::sync::Arc;

use egui::Ui;
use tokio::sync::{mpsc::UnboundedSender, oneshot};

use crate::commands::AppCommand;
use crate::ports::{FilePicker, FolderPicker};
use crate::state::AppState;
use crate::types::{HomeDensity, Page, Settings};

pub struct SettingsPage {
    working: Settings,
    previous_page: Page,
    folder_picker: Option<Arc<dyn FolderPicker>>,
    file_picker: Option<Arc<dyn FilePicker>>,
    pending_folder_selection: Option<oneshot::Receiver<Option<PathBuf>>>,
    pending_import_path: Option<oneshot::Receiver<Option<PathBuf>>>,
    pending_export_path: Option<oneshot::Receiver<Option<PathBuf>>>,
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

        ui.vertical(|ui| {
            ui.add_space(20.0);
            ui.heading("Settings");
            ui.add_space(20.0);

            ui.horizontal(|ui| {
                ui.label("Default Volume:");
                ui.add_space(10.0);
                ui.add(
                    egui::Slider::new(&mut self.working.default_volume, 0.0..=100.0)
                        .text("%")
                        .fixed_decimals(0),
                );
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Skip Backward (seconds):");
                ui.add_space(10.0);
                ui.add(egui::Slider::new(
                    &mut self.working.skip_backward_seconds,
                    5..=60,
                ));
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Skip Forward (seconds):");
                ui.add_space(10.0);
                ui.add(egui::Slider::new(
                    &mut self.working.skip_forward_seconds,
                    5..=60,
                ));
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Sync Interval (minutes):");
                ui.add_space(10.0);
                ui.add(egui::Slider::new(
                    &mut self.working.sync_interval_minutes,
                    5..=120,
                ));
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Auto Play Next:");
                ui.add_space(10.0);
                ui.checkbox(&mut self.working.auto_play_next, "");
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Home View:");
                ui.add_space(10.0);
                ui.selectable_value(
                    &mut self.working.home_density,
                    HomeDensity::Grid,
                    "Grid",
                );
                ui.add_space(4.0);
                ui.selectable_value(
                    &mut self.working.home_density,
                    HomeDensity::List,
                    "List",
                );
            });

            ui.add_space(40.0);

            ui.horizontal(|ui| {
                ui.label("Download Directory:");
                ui.add_space(10.0);

                let is_selecting = self.pending_folder_selection.is_some();
                ui.add_enabled(
                    !is_selecting,
                    egui::TextEdit::singleline(&mut self.working.download_directory),
                );

                ui.add_space(5.0);

                let button_text = if is_selecting { "Selecting..." } else { "Browse" };

                if ui
                    .add_enabled(!is_selecting, egui::Button::new(button_text))
                    .clicked()
                {
                    if let Some(picker) = &self.folder_picker {
                        let start_path = if self.working.download_directory.is_empty() {
                            None
                        } else {
                            PathBuf::from(&self.working.download_directory)
                                .parent()
                                .map(|p| p.to_path_buf())
                        };
                        self.pending_folder_selection = Some(picker.select_folder(start_path));
                    }
                }
            });

            ui.add_space(20.0);
            ui.separator();
            ui.add_space(16.0);

            // OPML import / export
            ui.label(
                egui::RichText::new("Subscriptions")
                    .strong()
                    .size(14.0),
            );
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
                {
                    if let Some(picker) = &self.file_picker {
                        self.pending_import_path =
                            Some(picker.pick_file("Import OPML Subscriptions"));
                    }
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
                {
                    if let Some(picker) = &self.file_picker {
                        self.pending_export_path =
                            Some(picker.save_file("Export OPML Subscriptions", "rcast-subscriptions.opml"));
                    }
                }
            });

            ui.add_space(20.0);

            ui.horizontal(|ui| {
                if ui.button("Save Changes").clicked() {
                    state.settings = self.working.clone();
                    let _ = cmd_tx.send(AppCommand::SaveSettings(self.working.clone()));
                    let _ = cmd_tx.send(AppCommand::NavigateTo(self.previous_page.clone()));
                }

                ui.add_space(10.0);

                if ui.button("Cancel").clicked() {
                    self.working = state.settings.clone();
                    let _ = cmd_tx.send(AppCommand::NavigateTo(self.previous_page.clone()));
                }
            });
        });
    }
}
