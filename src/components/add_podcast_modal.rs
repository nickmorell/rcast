use crate::design::components::*;
use crate::design::spacing::*;
use crate::design::tokens::ThemeTokens;
use crate::design::typography::*;

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

    pub fn render(&mut self, ctx: &egui::Context, t: &ThemeTokens) -> Option<String> {
        let mut result = None;

        if !self.show {
            return None;
        }

        egui::Window::new("Add Podcast")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .frame(
                egui::Frame::new()
                    .fill(t.card_bg)
                    .stroke(egui::Stroke::new(1.0, t.border))
                    .corner_radius(rounding_lg())
                    .inner_margin(egui::Margin::same(CARD_PADDING as i8)),
            )
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.set_width(400.0);

                    ui.label(text_page_title("Add Podcast", t));
                    ui.add_space(SPACE_1);
                    ui.label(text_hint(
                        if cfg!(target_os = "macos") {
                            "Paste a feed URL · Tip: ⌘V"
                        } else {
                            "Paste a feed URL · Tip: Ctrl+V"
                        },
                        t,
                    ));

                    ui.add_space(SPACE_3);

                    if let Some(err) = &self.error_message {
                        ui.label(
                            egui::RichText::new(err.as_str())
                                .size(FONT_SM)
                                .color(t.error),
                        );
                        ui.add_space(SPACE_1);
                    }

                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.url_input)
                            .hint_text("https://example.com/feed.rss")
                            .desired_width(f32::INFINITY),
                    );

                    if self.url_input.is_empty() && !response.has_focus() {
                        response.request_focus();
                    }

                    let is_valid = self.validate_url();

                    ui.add_space(SPACE_2);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let add_clicked = btn_primary_enabled(ui, "Add", is_valid, t).clicked();

                        if add_clicked
                            || (response.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                && is_valid)
                        {
                            result = Some(self.url_input.clone());
                            self.close();
                        }

                        ui.add_space(SPACE_2);

                        if btn_secondary(ui, "Cancel", t).clicked() {
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
