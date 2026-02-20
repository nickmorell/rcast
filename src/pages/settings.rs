use crate::{
    database::Database,
    types::{Page, Settings},
};

pub struct SettingsPage {
    settings: Settings,
    previous_page: Page,
}

impl SettingsPage {
    pub fn new(database: &Database) -> Self {
        let settings = database.get_settings().unwrap_or_default();

        Self {
            settings,
            previous_page: Page::Home,
        }
    }

    pub fn set_previous_page(&mut self, page: Page) {
        self.previous_page = page;
    }

    pub fn render(&mut self, ui: &mut egui::Ui, database: &Database) -> (Option<Page>, bool) {
        let mut next_page = None;
        let mut save_changes = false;

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
