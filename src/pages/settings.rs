use crate::{
    database::Database,
    ports::FolderPicker,
    types::{Page, Settings},
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::oneshot;

pub struct SettingsPage {
    settings: Settings,
    previous_page: Page,
    folder_picker: Arc<dyn FolderPicker>,
    pending_folder_selection: Option<oneshot::Receiver<Option<PathBuf>>>,
}

impl SettingsPage {
    pub fn new(database: &Database, folder_picker: Arc<dyn FolderPicker>) -> Self {
        let settings = database.get_settings().unwrap_or_default();

        Self {
            settings,
            previous_page: Page::Home,
            folder_picker,
            pending_folder_selection: None,
        }
    }

    pub fn set_previous_page(&mut self, page: Page) {
        self.previous_page = page;
    }

    fn poll_folder_selection(&mut self) {
        if let Some(rx) = self.pending_folder_selection.as_mut() {
            match rx.try_recv() {
                Ok(Some(path)) => {
                    self.settings.download_directory = path.to_string_lossy().to_string();
                    self.pending_folder_selection = None;
                }
                Ok(None) => {
                    // User cancelled
                    self.pending_folder_selection = None;
                }
                Err(oneshot::error::TryRecvError::Empty) => {
                    // Still pending—wait for next frame
                }
                Err(oneshot::error::TryRecvError::Closed) => {
                    // Task panicked or dropped
                    self.pending_folder_selection = None;
                }
            }
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, database: &Database) -> (Option<Page>, bool) {
        let mut next_page = None;
        let mut save_changes = false;

        self.poll_folder_selection();

        ui.vertical(|ui| {
            ui.add_space(20.0);
            ui.heading("Settings");
            ui.add_space(20.0);

            ui.horizontal(|ui| {
                ui.label("Default Volume:");
                ui.add_space(10.0);
                ui.add(
                    egui::Slider::new(&mut self.settings.default_volume, 0.0..=100.0)
                        .text("%")
                        .fixed_decimals(0),
                );
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Skip Backward (seconds):");
                ui.add_space(10.0);
                ui.add(egui::Slider::new(
                    &mut self.settings.skip_backward_seconds,
                    5..=60,
                ));
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Skip Forward (seconds):");
                ui.add_space(10.0);
                ui.add(egui::Slider::new(
                    &mut self.settings.skip_forward_seconds,
                    5..=60,
                ));
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Sync Interval (minutes):");
                ui.add_space(10.0);
                ui.add(egui::Slider::new(
                    &mut self.settings.sync_interval_minutes,
                    5..=120,
                ));
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Auto Play Next:");
                ui.add_space(10.0);
                ui.checkbox(&mut self.settings.auto_play_next, "");
            });

            ui.add_space(40.0);

            ui.horizontal(|ui| {
                ui.label("Download Directory:");
                ui.add_space(10.0);

                // [CHANGED] Disable editing while async dialog in flight to prevent race conditions
                let is_selecting = self.pending_folder_selection.is_some();
                ui.add_enabled(
                    !is_selecting,
                    egui::TextEdit::singleline(&mut self.settings.download_directory),
                );

                ui.add_space(5.0);

                let button_text = if is_selecting {
                    "Selecting..."
                } else {
                    "Browse"
                };

                // [CHANGED] Spawn async dialog on click with duplicate-prevention guard
                if ui
                    .add_enabled(!is_selecting, egui::Button::new(button_text))
                    .clicked()
                {
                    // [ADDED] Determine starting path for better UX
                    let start_path = if self.settings.download_directory.is_empty() {
                        None
                    } else {
                        PathBuf::from(&self.settings.download_directory)
                            .parent()
                            .map(|p| p.to_path_buf())
                    };

                    // [ADDED] Initiate async folder selection via port abstraction
                    let rx = self.folder_picker.select_folder(start_path);
                    self.pending_folder_selection = Some(rx);
                }
            });

            ui.horizontal(|ui| {
                if ui.button("Save Changes").clicked() {
                    database.save_settings(&self.settings).ok();
                    save_changes = true;
                    next_page = Some(self.previous_page.clone());
                }

                ui.add_space(10.0);

                if ui.button("Cancel").clicked() {
                    next_page = Some(self.previous_page.clone());
                }
            });
        });

        (next_page, save_changes)
    }

    pub fn get_settings(&self) -> &Settings {
        &self.settings
    }
}
