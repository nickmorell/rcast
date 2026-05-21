use egui::{Color32, RichText, Ui};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

use crate::commands::AppCommand;
use crate::db::models::Bookmark;
use crate::design::spacing::*;
use crate::design::tokens::ThemeTokens;
use crate::design::typography::*;

#[derive(Default)]
pub struct NotesPanel {
    pub episode_id: Option<i32>,
    pub podcast_id: Option<i32>,
    pub episode_title: String,

    pub input_text: String,

    edit_id: Option<i32>,
    edit_text: String,

    delete_confirm_id: Option<i32>,

    pub visible: bool,
    pub seek_request: Option<Duration>,
}

impl NotesPanel {
    pub fn open(&mut self, episode_id: i32, podcast_id: i32, title: String) -> bool {
        let changed = self.episode_id != Some(episode_id);
        self.episode_id = Some(episode_id);
        self.podcast_id = Some(podcast_id);
        self.episode_title = title;
        self.visible = true;
        if changed {
            self.input_text.clear();
            self.edit_id = None;
            self.delete_confirm_id = None;
        }
        changed
    }

    pub fn close(&mut self) {
        self.visible = false;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        episode_bookmarks: &[Bookmark],
        podcast_bookmarks: &[Bookmark],
        now_playing_episode_id: Option<i32>,
        current_position: f64,
        cmd_tx: &UnboundedSender<AppCommand>,
        t: &ThemeTokens,
    ) {
        if !self.visible {
            return;
        }

        let panel_episode_id = match self.episode_id {
            Some(id) => id,
            None => return,
        };

        let is_live = now_playing_episode_id == Some(panel_episode_id);

        egui::Panel::right("notes_panel")
            .resizable(false)
            .exact_size(310.0)
            .frame(egui::Frame {
                fill: t.page_bg,
                inner_margin: egui::Margin::symmetric(0, 0),
                ..Default::default()
            })
            .show_inside(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;

                // Header
                egui::Frame::new()
                    .fill(t.card_bg)
                    .inner_margin(egui::Margin::symmetric(14, 12))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(text_label("Notes", t));
                                ui.add_space(SPACE_1 / 2.0);
                                let title = truncate(&self.episode_title, 38);
                                ui.label(text_body(title, t));
                            });

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                if ui
                                    .button(
                                        RichText::new(egui_phosphor::regular::X)
                                            .size(14.0)
                                            .color(t.text_meta),
                                    )
                                    .clicked()
                                {
                                    self.visible = false;
                                }
                            });
                        });

                        ui.add_space(SPACE_1 + 2.0);

                        // Status pill — live stamp vs. general note
                        let (dot, label, dot_color) = if is_live {
                            ("●", "Stamping to current time", t.success)
                        } else {
                            ("○", "Not playing — notes saved without timestamp", t.text_meta)
                        };

                        ui.horizontal(|ui| {
                            ui.label(RichText::new(dot).size(10.0).color(dot_color));
                            ui.add_space(ICON_GAP);
                            ui.label(text_hint(label, t));
                        });
                    });

                ui.add_space(1.0);

                egui::ScrollArea::vertical()
                    .id_salt("notes_scroll")
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.spacing_mut().item_spacing.y = 0.0;

                        // New note input
                        egui::Frame::new()
                            .fill(t.card_bg)
                            .inner_margin(egui::Margin::symmetric(14, 10))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                let input = egui::TextEdit::multiline(&mut self.input_text)
                                    .hint_text("Write a note… (Shift+Enter for newline)")
                                    .desired_rows(2)
                                    .desired_width(f32::INFINITY)
                                    .frame(egui::Frame::NONE);

                                let response = ui.add(input);

                                let enter_pressed = response.has_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                    && !ui.input(|i| i.modifiers.shift);

                                ui.add_space(SPACE_1 + 2.0);

                                let text = self.input_text.trim().to_string();
                                let add_clicked = ui
                                    .add_enabled(
                                        !text.is_empty(),
                                        egui::Button::new(
                                            egui::RichText::new("Add Note").size(12.0),
                                        )
                                        .fill(t.accent)
                                        .min_size(egui::vec2(80.0, 26.0)),
                                    )
                                    .clicked();

                                if (enter_pressed || add_clicked) && !text.is_empty() {
                                    let position = if is_live { Some(current_position) } else { None };
                                    let _ = cmd_tx.send(AppCommand::AddBookmark {
                                        podcast_id: self.podcast_id.unwrap_or(0),
                                        episode_id: self.episode_id,
                                        position_seconds: position,
                                        note_text: text,
                                    });
                                    self.input_text.clear();
                                }
                            });

                        ui.add_space(SPACE_2);

                        // Podcast-level notes
                        if !podcast_bookmarks.is_empty() {
                            section_label(ui, "PODCAST", t);
                            for b in podcast_bookmarks {
                                self.render_note(ui, b, false, cmd_tx, t);
                            }
                            ui.add_space(SPACE_2);
                        }

                        // Timed episode notes
                        let timed: Vec<&Bookmark> = episode_bookmarks
                            .iter()
                            .filter(|b| b.position_seconds.is_some())
                            .collect();
                        let untimed: Vec<&Bookmark> = episode_bookmarks
                            .iter()
                            .filter(|b| b.position_seconds.is_none())
                            .collect();

                        if !timed.is_empty() {
                            section_label(ui, "TIMED", t);
                            for b in &timed {
                                self.render_note(ui, b, true, cmd_tx, t);
                            }
                            ui.add_space(SPACE_2);
                        }

                        // General episode notes
                        if !untimed.is_empty() {
                            section_label(ui, "GENERAL", t);
                            for b in &untimed {
                                self.render_note(ui, b, false, cmd_tx, t);
                            }
                        }

                        // Empty state
                        if podcast_bookmarks.is_empty() && episode_bookmarks.is_empty() {
                            ui.add_space(SPACE_5);
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    RichText::new(egui_phosphor::regular::NOTE_PENCIL)
                                        .size(40.0)
                                        .color(t.text_disabled),
                                );
                                ui.add_space(SPACE_2);
                                ui.label(text_body("No notes yet", t));
                                ui.add_space(SPACE_1);
                                ui.label(text_hint("Start writing above", t));
                            });
                        }

                        ui.add_space(SPACE_4 + SPACE_1);
                    });
            });
    }

    fn render_note(
        &mut self,
        ui: &mut Ui,
        bookmark: &Bookmark,
        show_timestamp: bool,
        cmd_tx: &UnboundedSender<AppCommand>,
        t: &ThemeTokens,
    ) {
        let id = bookmark.id;
        let is_editing = self.edit_id == Some(id);
        let is_confirming_delete = self.delete_confirm_id == Some(id);
        let is_podcast_note = bookmark.episode_id.is_none();

        let base_fill = if is_podcast_note { t.input_bg } else { t.card_bg };
        let hover_fill = t.hover_bg;

        let is_hovered = ui.rect_contains_pointer(ui.cursor().expand(200.0));
        let frame_fill = if is_hovered { hover_fill } else { base_fill };

        let note_rect = egui::Frame::new()
            .fill(frame_fill)
            .inner_margin(egui::Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                if is_confirming_delete {
                    let confirm_bg = Color32::from_rgba_premultiplied(
                        t.error.r() / 4,
                        t.error.g() / 4,
                        t.error.b() / 4,
                        180,
                    );
                    egui::Frame::new()
                        .fill(confirm_bg)
                        .corner_radius(6)
                        .inner_margin(egui::Margin::symmetric(8, 6))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.label(
                                RichText::new("Delete this note?")
                                    .size(12.0)
                                    .color(t.error),
                            );
                            ui.add_space(SPACE_1 + 2.0);
                            ui.horizontal(|ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("Delete")
                                                .size(12.0)
                                                .color(Color32::WHITE),
                                        )
                                        .fill(t.error),
                                    )
                                    .clicked()
                                {
                                    let _ = cmd_tx.send(AppCommand::DeleteBookmark(id));
                                    self.delete_confirm_id = None;
                                }
                                ui.add_space(SPACE_2);
                                if ui.button(RichText::new("Keep").size(12.0)).clicked() {
                                    self.delete_confirm_id = None;
                                }
                            });
                        });
                } else if is_editing {
                    if self.edit_id == Some(id) && self.edit_text.is_empty() {
                        self.edit_text = bookmark.note_text.clone();
                    }

                    if show_timestamp && let Some(pos) = bookmark.position_seconds {
                        let _ = timestamp_badge(ui, pos, t);
                        ui.add_space(SPACE_1 + 2.0);
                    }

                    let edit_input = egui::TextEdit::multiline(&mut self.edit_text)
                        .desired_rows(2)
                        .desired_width(f32::INFINITY);

                    let resp = ui.add(edit_input);

                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.edit_id = None;
                        self.edit_text.clear();
                    }

                    ui.add_space(SPACE_1 + 2.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new("Save").size(12.0))
                                    .fill(t.accent),
                            )
                            .clicked()
                        {
                            let text = self.edit_text.trim().to_string();
                            if !text.is_empty() {
                                let _ = cmd_tx.send(AppCommand::UpdateBookmark {
                                    id,
                                    note_text: text,
                                });
                            }
                            self.edit_id = None;
                            self.edit_text.clear();
                        }
                        ui.add_space(SPACE_2);
                        if ui.button(RichText::new("Cancel").size(12.0)).clicked() {
                            self.edit_id = None;
                            self.edit_text.clear();
                        }
                    });
                } else {
                    // Normal read mode
                    if show_timestamp && let Some(pos) = bookmark.position_seconds {
                        let resp = timestamp_badge(ui, pos, t);
                        if resp.clicked() {
                            self.seek_request = Some(Duration::from_secs_f64(pos));
                        }
                        resp.on_hover_text("Click to seek");
                        ui.add_space(ICON_GAP);
                    }

                    ui.horizontal(|ui| {
                        let currently_hovered = ui.ui_contains_pointer();

                        ui.label(text_body(&bookmark.note_text, t));

                        let icon_color = if currently_hovered { t.text_secondary } else { t.text_disabled };
                        let delete_color = if currently_hovered { t.error } else { t.text_disabled };

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .button(
                                    RichText::new(egui_phosphor::regular::TRASH)
                                        .size(14.0)
                                        .color(delete_color),
                                )
                                .on_hover_text("Delete note")
                                .clicked()
                            {
                                self.edit_id = None;
                                self.edit_text.clear();
                                self.delete_confirm_id = Some(id);
                            }

                            ui.add_space(ICON_GAP);

                            if ui
                                .button(
                                    RichText::new(egui_phosphor::regular::PENCIL_SIMPLE)
                                        .size(14.0)
                                        .color(icon_color),
                                )
                                .on_hover_text("Edit note")
                                .clicked()
                            {
                                self.delete_confirm_id = None;
                                if self.edit_id == Some(id) {
                                    self.edit_id = None;
                                    self.edit_text.clear();
                                } else {
                                    self.edit_id = Some(id);
                                    self.edit_text = bookmark.note_text.clone();
                                }
                            }
                        });
                    });
                }
            });

        ui.painter().line_segment(
            [
                note_rect.response.rect.left_bottom(),
                note_rect.response.rect.right_bottom(),
            ],
            egui::Stroke::new(1.0, t.divider),
        );
    }
}

fn section_label(ui: &mut Ui, text: &str, t: &ThemeTokens) {
    egui::Frame::new()
        .inner_margin(egui::Margin { left: 14, right: 14, top: 6, bottom: 4 })
        .show(ui, |ui| {
            ui.label(
                RichText::new(text)
                    .size(FONT_XS)
                    .color(t.text_disabled)
                    .strong(),
            );
        });
}

fn timestamp_badge(ui: &mut Ui, position_seconds: f64, t: &ThemeTokens) -> egui::Response {
    let secs = position_seconds as u64;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    let label = if h > 0 {
        format!("{h:02}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    };

    ui.add(
        egui::Button::new(
            egui::RichText::new(label)
                .font(egui::FontId::monospace(11.0))
                .color(t.accent),
        )
        .fill(t.accent_tint)
        .corner_radius(4.0)
        .min_size(egui::vec2(52.0, 18.0)),
    )
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let end = s
            .char_indices()
            .nth(max_chars - 1)
            .map(|(i, _)| i)
            .unwrap_or(s.len());
        format!("{}…", &s[..end])
    }
}
