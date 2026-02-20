use egui::Color32;

pub struct AddPodcastModal {
    pub show: bool,
    pub url_input: String,
    pub error_message: Option<String>,
}

impl AddPodcastModal {
    pub fn new() -> Self {
        Self {
            show: false,
            url_input: String::new(),
            error_message: None,
        }
    }

    pub fn open(&mut self) {
        self.show = true;
        self.url_input.clear();
        self.error_message = None;
    }

    pub fn close(&mut self) {
        self.show = false;
        self.url_input.clear();
        self.error_message = None;
    }

    pub fn render(&mut self, ctx: &egui::Context) -> Option<String> {
        let mut result = None;

        if !self.show {
            return None;
        }

        egui::Window::new("Add Podcast")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.set_width(400.0);

                    if let Some(err) = &self.error_message {
                        ui.colored_label(Color32::from_rgb(220, 80, 80), err);
                        ui.add_space(5.0);
                    }

                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.url_input)
                            .hint_text("https://example.com/feed.rss")
                            .desired_width(380.0),
                    );

                    if self.url_input.is_empty() && !response.has_focus() {
                        response.request_focus();
                    }

                    let is_valid = self.validate_url();

                    ui.add_space(20.0);

                    ui.horizontal(|ui| {
                        let add_button = ui.add_enabled(is_valid, egui::Button::new("Add"));

                        if add_button.clicked()
                            || (response.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                && is_valid)
                        {
                            result = Some(self.url_input.clone());
                            self.close();
                        }

                        if ui.button("Cancel").clicked() {
                            self.close();
                        }
                    });
                });
            });

        result
    }

    fn validate_url(&mut self) -> bool {
        if self.url_input.is_empty() {
            self.error_message = None;
            return false;
        }

        match url::Url::parse(&self.url_input) {
            Ok(_) => {
                self.error_message = None;
                true
            }
            Err(_) => {
                self.error_message = Some("Invalid URL".to_string());
                false
            }
        }
    }
}
