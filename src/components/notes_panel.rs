use egui::{Color32, RichText, Ui};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

use crate::commands::AppCommand;
use crate::db::models::Bookmark;

// Colour palette

const BG: Color32 = Color32::from_rgb(22, 22, 26);
const SURFACE: Color32 = Color32::from_rgb(32, 32, 38);
const SURFACE_HOVER: Color32 = Color32::from_rgb(42, 42, 50);
const MUTED: Color32 = Color32::from_rgb(130, 130, 140);
const TIMESTAMP_BG: Color32 = Color32::from_rgb(45, 65, 110);
const TIMESTAMP_FG: Color32 = Color32::from_rgb(140, 180, 255);
const PODCAST_NOTE_BG: Color32 = Color32::from_rgb(40, 38, 60);
const DELETE_RED: Color32 = Color32::from_rgb(200, 70, 70);
const CONFIRM_BG: Color32 = Color32::from_rgb(55, 30, 30);

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

impl Default for NotesPanel {
    fn default() -> Self {
        Self {
            episode_id: None,
            podcast_id: None,
            episode_title: String::new(),
            input_text: String::new(),
            edit_id: None,
            edit_text: String::new(),
            delete_confirm_id: None,
            visible: false,
            seek_request: None,
        }
    }
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

    pub fn render(
        &mut self,
        ctx: &egui::Context,
        episode_bookmarks: &[Bookmark],
        podcast_bookmarks: &[Bookmark],
        now_playing_episode_id: Option<i32>,
        current_position: f64,
        cmd_tx: &UnboundedSender<AppCommand>,
    ) {
        if !self.visible {
            return;
        }

        let panel_episode_id = match self.episode_id {
            Some(id) => id,
            None => return,
        };

        let is_live = now_playing_episode_id == Some(panel_episode_id);

        egui::SidePanel::right("notes_panel")
            .resizable(false)
            .exact_width(310.0)
            .frame(egui::Frame {
                fill: BG,
                inner_margin: egui::Margin::symmetric(0, 0),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;

                // Header
                egui::Frame::new()
                    .fill(SURFACE)
                    .inner_margin(egui::Margin::symmetric(14, 12))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Notes").strong().size(13.0).color(MUTED));
                                ui.add_space(2.0);
                                let title = truncate(&self.episode_title, 38);
                                ui.label(RichText::new(title).strong().size(14.0));
                            });

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                if ui
                                    .button(RichText::new("✕").size(14.0).color(MUTED))
                                    .clicked()
                                {
                                    self.visible = false;
                                }
                            });
                        });

                        ui.add_space(6.0);

                        // Status pill — live stamp vs. general note
                        let (dot, label, dot_color) = if is_live {
                            (
                                "●",
                                "Stamping to current time",
                                Color32::from_rgb(80, 200, 100),
                            )
                        } else {
                            ("○", "Not playing — notes saved without timestamp", MUTED)
                        };

                        ui.horizontal(|ui| {
                            ui.label(RichText::new(dot).size(10.0).color(dot_color));
                            ui.add_space(4.0);
                            ui.label(RichText::new(label).size(11.0).color(MUTED));
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
                            .fill(SURFACE)
                            .inner_margin(egui::Margin::symmetric(14, 10))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                let input = egui::TextEdit::multiline(&mut self.input_text)
                                    .hint_text("Write a note… (Shift+Enter for newline)")
                                    .desired_rows(2)
                                    .desired_width(f32::INFINITY)
                                    .frame(false);

                                let response = ui.add(input);

                                // Enter while focused (without Shift) submits.
                                // lost_focus() does not fire on Enter in multiline TextEdit.
                                let enter_pressed = response.has_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                    && !ui.input(|i| i.modifiers.shift);

                                ui.add_space(6.0);

                                let text = self.input_text.trim().to_string();
                                let add_clicked = ui
                                    .add_enabled(
                                        !text.is_empty(),
                                        egui::Button::new(
                                            egui::RichText::new("Add Note").size(12.0),
                                        )
                                        .fill(Color32::from_rgb(50, 90, 160))
                                        .min_size(egui::vec2(80.0, 26.0)),
                                    )
                                    .clicked();

                                if (enter_pressed || add_clicked) && !text.is_empty() {
                                    let position = if is_live {
                                        Some(current_position)
                                    } else {
                                        None
                                    };
                                    let _ = cmd_tx.send(AppCommand::AddBookmark {
                                        podcast_id: self.podcast_id.unwrap_or(0),
                                        episode_id: self.episode_id,
                                        position_seconds: position,
                                        note_text: text,
                                    });
                                    self.input_text.clear();
                                }
                            });

                        ui.add_space(8.0);

                        // Podcast-level notes
                        if !podcast_bookmarks.is_empty() {
                            section_label(ui, "PODCAST");
                            for b in podcast_bookmarks {
                                self.render_note(ui, b, false, cmd_tx);
                            }
                            ui.add_space(8.0);
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
                            section_label(ui, "TIMED");
                            for b in &timed {
                                self.render_note(ui, b, true, cmd_tx);
                            }
                            ui.add_space(8.0);
                        }

                        // General episode notes
                        if !untimed.is_empty() {
                            section_label(ui, "GENERAL");
                            for b in &untimed {
                                self.render_note(ui, b, false, cmd_tx);
                            }
                        }

                        // Empty state
                        if podcast_bookmarks.is_empty() && episode_bookmarks.is_empty() {
                            ui.add_space(32.0);
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    RichText::new(egui_phosphor::regular::NOTE_PENCIL)
                                        .size(40.0)
                                        .color(Color32::from_rgb(60, 60, 70)),
                                );
                                ui.add_space(8.0);
                                ui.label(RichText::new("No notes yet").size(13.0).color(MUTED));
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new("Start writing above")
                                        .size(11.0)
                                        .color(Color32::from_rgb(80, 80, 90)),
                                );
                            });
                        }

                        ui.add_space(20.0);
                    });
            });
    }

    fn render_note(
        &mut self,
        ui: &mut Ui,
        bookmark: &Bookmark,
        show_timestamp: bool,
        cmd_tx: &UnboundedSender<AppCommand>,
    ) {
        let id = bookmark.id;
        let is_editing = self.edit_id == Some(id);
        let is_confirming_delete = self.delete_confirm_id == Some(id);
        let is_podcast_note = bookmark.episode_id.is_none();

        let base_fill = if is_podcast_note {
            PODCAST_NOTE_BG
        } else {
            SURFACE
        };
        let hover_fill = if is_podcast_note {
            Color32::from_rgb(52, 50, 75)
        } else {
            SURFACE_HOVER
        };

        let note_rect = egui::Frame::new()
            .fill(base_fill)
            .inner_margin(egui::Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                if is_confirming_delete {
                    // Inline delete confirmation
                    egui::Frame::new()
                        .fill(CONFIRM_BG)
                        .corner_radius(6)
                        .inner_margin(egui::Margin::symmetric(8, 6))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.label(
                                RichText::new("Delete this note?")
                                    .size(12.0)
                                    .color(Color32::from_rgb(230, 130, 130)),
                            );
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("Delete")
                                                .size(12.0)
                                                .color(Color32::WHITE),
                                        )
                                        .fill(DELETE_RED),
                                    )
                                    .clicked()
                                {
                                    let _ = cmd_tx.send(AppCommand::DeleteBookmark(id));
                                    self.delete_confirm_id = None;
                                }
                                ui.add_space(8.0);
                                if ui.button(RichText::new("Keep").size(12.0)).clicked() {
                                    self.delete_confirm_id = None;
                                }
                            });
                        });
                } else if is_editing {
                    // Inline edit mode
                    if self.edit_id == Some(id) && self.edit_text.is_empty() {
                        self.edit_text = bookmark.note_text.clone();
                    }

                    // Show locked timestamp if present (not seekable in edit mode)
                    if show_timestamp {
                        if let Some(pos) = bookmark.position_seconds {
                            let _ = timestamp_badge(ui, pos);
                            ui.add_space(6.0);
                        }
                    }

                    let edit_input = egui::TextEdit::multiline(&mut self.edit_text)
                        .desired_rows(2)
                        .desired_width(f32::INFINITY)
                        .frame(true);

                    let resp = ui.add(edit_input);

                    // Escape cancels, Enter saves
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.edit_id = None;
                        self.edit_text.clear();
                    }

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new("Save").size(12.0))
                                    .fill(Color32::from_rgb(50, 90, 160)),
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
                        ui.add_space(8.0);
                        if ui.button(RichText::new("Cancel").size(12.0)).clicked() {
                            self.edit_id = None;
                            self.edit_text.clear();
                        }
                    });
                } else {
                    // Normal read mode
                    // Timestamp badge — clicking seeks to that position
                    if show_timestamp {
                        if let Some(pos) = bookmark.position_seconds {
                            let resp = timestamp_badge(ui, pos);
                            if resp.clicked() {
                                println!("Seeking to {} seconds", pos);
                                self.seek_request = Some(Duration::from_secs_f64(pos));
                            }
                            resp.on_hover_text("Click to seek");
                            ui.add_space(4.0);
                        }
                    }

                    ui.horizontal(|ui| {
                        let currently_hovered = ui.ui_contains_pointer();

                        ui.label(
                            RichText::new(&bookmark.note_text)
                                .size(13.0)
                                .color(Color32::from_rgb(210, 210, 215)),
                        );

                        let icon_color = if currently_hovered {
                            MUTED
                        } else {
                            Color32::from_rgb(50, 50, 58)
                        };
                        let delete_color = if currently_hovered {
                            DELETE_RED
                        } else {
                            Color32::from_rgb(50, 50, 58)
                        };

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

                            ui.add_space(4.0);

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

        // Thin separator between notes
        ui.painter().line_segment(
            [
                note_rect.response.rect.left_bottom(),
                note_rect.response.rect.right_bottom(),
            ],
            egui::Stroke::new(1.0, Color32::from_rgb(38, 38, 45)),
        );
    }
}

// Helpers

fn section_label(ui: &mut Ui, text: &str) {
    egui::Frame::new()
        .inner_margin(egui::Margin {
            left: 14,
            right: 14,
            top: 6,
            bottom: 4,
        })
        .show(ui, |ui| {
            ui.label(
                RichText::new(text)
                    .size(10.0)
                    .color(Color32::from_rgb(90, 90, 100))
                    .strong(),
            );
        });
}

fn timestamp_badge(ui: &mut Ui, position_seconds: f64) -> egui::Response {
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
                .color(TIMESTAMP_FG),
        )
        .fill(TIMESTAMP_BG)
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
