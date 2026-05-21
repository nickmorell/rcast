use crate::{
    audio_player::AudioPlayer,
    chapters::{current_chapter, Chapter},
    db::models::Episode,
    design::{spacing::*, tokens::ThemeTokens, typography::*},
    image_cache::ImageCache,
    types::QueueDisplayItem,
};
use egui_alignments::center_horizontal;
use std::time::Duration;

/// Read-only now-playing context passed to the media bar each frame.
pub struct NowPlayingContext<'a> {
    pub episode: Option<&'a Episode>,
    pub podcast_title: Option<&'a str>,
    pub podcast_image: Option<&'a str>,
    pub chapters: &'a [Chapter],
    pub queue_items: &'a [QueueDisplayItem],
    pub image_cache: &'a ImageCache,
    pub sleep_timer_ends_at: Option<std::time::Instant>,
    pub notes_open: bool,
}

/// Mutable popup-visibility and transient UI state for the media bar.
pub struct MediaControlsState {
    pub show_queue: bool,
    pub show_speed_menu: bool,
    pub show_chapters: bool,
    pub show_sleep_timer: bool,
    pub volume: f32,
}

impl Default for MediaControlsState {
    fn default() -> Self {
        Self {
            show_queue: false,
            show_speed_menu: false,
            show_chapters: false,
            show_sleep_timer: false,
            volume: 100.0,
        }
    }
}

pub struct MediaControls;

impl MediaControls {
    pub fn render(
        ui: &mut egui::Ui,
        audio_player: &AudioPlayer,
        ctx: &NowPlayingContext<'_>,
        state: &mut MediaControlsState,
        t: &ThemeTokens,
    ) -> MediaControlsAction {
        let current_episode = ctx.episode;
        let current_podcast_title = ctx.podcast_title;
        let current_podcast_image = ctx.podcast_image;
        let chapters = ctx.chapters;
        let queue_items = ctx.queue_items;
        let image_cache = ctx.image_cache;
        let sleep_timer_ends_at = ctx.sleep_timer_ends_at;
        let notes_open = ctx.notes_open;
        let mut action = MediaControlsAction::None;

        let total_width = ui.available_width();
        let left_width = total_width * 0.20;
        let middle_width = total_width * 0.60;
        let right_width = total_width * 0.20;

        ui.spacing_mut().item_spacing.x = 0.0;

        ui.horizontal(|ui| {
            // LEFT: now-playing info
            ui.allocate_ui_with_layout(
                egui::vec2(left_width, ui.available_height()),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_width(left_width);
                    ui.add_space(CONTROL_GAP);

                    if let Some(podcast_title) = current_podcast_title {
                        if let Some(image_url) = current_podcast_image {
                            let texture = image_cache
                                .get_or_load(image_url, ui.ctx())
                                .unwrap_or_else(|| image_cache.get_default_texture(ui.ctx()));
                            ui.add(egui::Image::new(&texture).max_size(egui::vec2(60.0, 60.0)));
                            ui.add_space(CONTROL_GAP);
                        }

                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(podcast_title)
                                    .size(FONT_SM)
                                    .color(t.text_secondary),
                            );

                            if let Some(episode) = current_episode {
                                let title = if episode.title.len() > 30 {
                                    let end = episode
                                        .title
                                        .char_indices()
                                        .nth(27)
                                        .map(|(i, _)| i)
                                        .unwrap_or(episode.title.len());
                                    format!("{}...", &episode.title[..end])
                                } else {
                                    episode.title.clone()
                                };
                                ui.label(
                                    egui::RichText::new(title)
                                        .size(FONT_MD)
                                        .color(t.text_primary)
                                        .strong(),
                                );
                            }

                            if !chapters.is_empty() {
                                let pos_secs = audio_player.get_position().as_secs_f64();
                                if let Some(ch) = current_chapter(chapters, pos_secs) {
                                    let ch_name = if ch.title.len() > 28 {
                                        let end = ch
                                            .title
                                            .char_indices()
                                            .nth(25)
                                            .map(|(i, _)| i)
                                            .unwrap_or(ch.title.len());
                                        format!("{}...", &ch.title[..end])
                                    } else {
                                        ch.title.clone()
                                    };
                                    ui.label(
                                        egui::RichText::new(format!("◆ {}", ch_name))
                                            .size(FONT_XS)
                                            .color(t.accent),
                                    );
                                }
                            }
                        });
                    }
                },
            );

            // MIDDLE: playback controls + seek bar
            ui.allocate_ui_with_layout(
                egui::vec2(middle_width, ui.available_height()),
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(middle_width);
                        let has_audio = current_episode.is_some();

                        center_horizontal(ui, |ui| {
                            let skip_back_btn = ui.add_enabled(
                                has_audio,
                                egui::Button::new(
                                    egui::RichText::new(egui_phosphor::regular::SKIP_BACK)
                                        .size(28.0),
                                ),
                            );
                            if skip_back_btn.clicked() {
                                action = MediaControlsAction::SkipBackward;
                            }
                            if skip_back_btn.hovered() && has_audio {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }

                            ui.add_space(SPACE_4);

                            let play_pause_icon = match audio_player.get_state() {
                                crate::audio_player::PlaybackState::Playing => {
                                    egui_phosphor::regular::PAUSE
                                }
                                _ => egui_phosphor::regular::PLAY,
                            };

                            let play_pause_btn = ui.add_enabled(
                                has_audio,
                                egui::Button::new(
                                    egui::RichText::new(play_pause_icon).size(28.0),
                                ),
                            );
                            if play_pause_btn.clicked() {
                                action = MediaControlsAction::PlayPause;
                            }
                            if play_pause_btn.hovered() && has_audio {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }

                            ui.add_space(15.0);

                            let skip_fwd_btn = ui.add_enabled(
                                has_audio,
                                egui::Button::new(
                                    egui::RichText::new(egui_phosphor::regular::SKIP_FORWARD)
                                        .size(28.0),
                                ),
                            );
                            if skip_fwd_btn.clicked() {
                                action = MediaControlsAction::SkipForward;
                            }
                            if skip_fwd_btn.hovered() && has_audio {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                        });

                        ui.add_space(SPACE_4);
                        ui.horizontal(|ui| {
                            let position = audio_player.get_position();
                            let duration = audio_player.get_duration();

                            ui.label(format_duration(position));
                            ui.add_space(CONTROL_GAP);

                            let mut pos_secs = position.as_secs_f32();
                            let dur_secs = duration.as_secs_f32();

                            ui.scope(|ui| {
                                ui.spacing_mut().slider_width = ui.available_width() * 0.90;
                                let slider = egui::Slider::new(
                                    &mut pos_secs,
                                    0.0..=dur_secs.max(1.0),
                                )
                                    .show_value(false)
                                    .update_while_editing(true)
                                    .trailing_fill(true);

                                let slider_response = ui.add_enabled(has_audio, slider);
                                if has_audio && slider_response.changed() {
                                    action = MediaControlsAction::Seek(
                                        Duration::from_secs_f32(pos_secs),
                                    );
                                }
                            });

                            ui.add_space(CONTROL_GAP);
                            ui.label(format_duration(duration));
                        });
                    });
                },
            );

            ui.add_space(SPACE_4);

            // RIGHT: notes, speed, queue, volume
            ui.allocate_ui_with_layout(
                egui::vec2(right_width, ui.available_height()),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    ui.set_width(right_width);
                    ui.add_space(CONTROL_GAP);

                    let speed = audio_player.get_speed();
                    let speed_btn = ui.button(format!("{}x", speed));
                    if speed_btn.clicked() {
                        state.show_speed_menu = !state.show_speed_menu;
                    }

                    if state.show_speed_menu {
                        let area_response =
                            egui::Area::new(egui::Id::new("speed_menu"))
                                .fixed_pos(speed_btn.rect.left_bottom())
                                .show(ui.ctx(), |ui| {
                                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                                        ui.vertical(|ui| {
                                            for &spd in &[2.0f32, 1.5, 1.25, 1.0, 0.75] {
                                                if ui
                                                    .selectable_label(
                                                        speed == spd,
                                                        format!("{}x", spd),
                                                    )
                                                    .clicked()
                                                {
                                                    action = MediaControlsAction::SetSpeed(spd);
                                                    state.show_speed_menu = false;
                                                }
                                            }
                                            if current_episode.is_some() {
                                                ui.separator();
                                                if ui
                                                    .button("Set as show default")
                                                    .on_hover_text(
                                                        "Save current speed as the default for this podcast",
                                                    )
                                                    .clicked()
                                                {
                                                    action =
                                                        MediaControlsAction::SetShowDefaultSpeed(
                                                            speed,
                                                        );
                                                    state.show_speed_menu = false;
                                                }
                                            }
                                        });
                                    });
                                });

                        if ui.input(|i| i.pointer.any_click())
                            && ui.input(|i| i.pointer.interact_pos()).is_some_and(|pos| {
                                !area_response.response.rect.contains(pos)
                                    && !speed_btn.rect.contains(pos)
                            })
                        {
                            state.show_speed_menu = false;
                        }
                    }

                    ui.add_space(SPACE_1);

                    let has_chapters = !chapters.is_empty();
                    let chapters_icon = egui::RichText::new(egui_phosphor::regular::LIST_BULLETS)
                        .size(20.0)
                        .color(if has_chapters {
                            ui.visuals().text_color()
                        } else {
                            ui.visuals().text_color().gamma_multiply(0.35)
                        });
                    let chapters_btn = ui
                        .add_enabled(has_chapters, egui::Button::new(chapters_icon))
                        .on_hover_text("Chapters");
                    if chapters_btn.clicked() {
                        state.show_chapters = !state.show_chapters;
                    }

                    if state.show_chapters && has_chapters {
                        let pos_secs = audio_player.get_position().as_secs_f64();
                        let area_response =
                            egui::Area::new(egui::Id::new("chapters_menu"))
                                .fixed_pos(chapters_btn.rect.left_top() - egui::vec2(0.0, 10.0))
                                .pivot(egui::Align2::LEFT_BOTTOM)
                                .show(ui.ctx(), |ui| {
                                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                                        ui.set_width(300.0);
                                        ui.set_max_height(350.0);
                                        egui::ScrollArea::vertical().show(ui, |ui| {
                                            for ch in chapters {
                                                if !ch.toc {
                                                    continue;
                                                }
                                                let is_current = current_chapter(chapters, pos_secs)
                                                    .map(|c| std::ptr::eq(c, ch))
                                                    .unwrap_or(false);
                                                let start = std::time::Duration::from_secs_f64(ch.start_time);
                                                let time_str = format_duration(start);
                                                let row = ui.horizontal(|ui| {
                                                    ui.label(
                                                        egui::RichText::new(&time_str)
                                                            .size(FONT_XS)
                                                            .color(t.text_meta),
                                                    );
                                                    ui.add_space(ICON_GAP);
                                                    let label = egui::RichText::new(&ch.title)
                                                        .size(FONT_MD)
                                                        .strong();
                                                    let label = if is_current {
                                                        label.color(t.accent)
                                                    } else {
                                                        label
                                                    };
                                                    ui.label(label);
                                                    if is_current {
                                                        ui.with_layout(
                                                            egui::Layout::right_to_left(egui::Align::Center),
                                                            |ui| {
                                                                ui.label(
                                                                    egui::RichText::new("✓")
                                                                        .size(12.0)
                                                                        .color(t.accent),
                                                                );
                                                            },
                                                        );
                                                    }
                                                });
                                                if row.response.interact(egui::Sense::click()).clicked() {
                                                    action = MediaControlsAction::Seek(start);
                                                    state.show_chapters = false;
                                                }
                                                ui.separator();
                                            }
                                        });
                                    });
                                });

                        if ui.input(|i| i.pointer.any_click())
                            && ui.input(|i| i.pointer.interact_pos()).is_some_and(|pos| {
                                !area_response.response.rect.contains(pos)
                                    && !chapters_btn.rect.contains(pos)
                            })
                        {
                            state.show_chapters = false;
                        }
                    }

                    ui.add_space(SPACE_1);

                    let queue_btn = ui.button(
                        egui::RichText::new(egui_phosphor::regular::QUEUE).size(20.0),
                    );
                    if queue_btn.clicked() {
                        state.show_queue = !state.show_queue;
                    }

                    ui.add_space(SPACE_1);

                    // Sleep timer button
                    let timer_label = if let Some(ends_at) = sleep_timer_ends_at {
                        let remaining = ends_at
                            .checked_duration_since(std::time::Instant::now())
                            .unwrap_or_default();
                        let mins = remaining.as_secs() / 60;
                        let secs = remaining.as_secs() % 60;
                        format!("{:02}:{:02}", mins, secs)
                    } else {
                        egui_phosphor::regular::MOON.to_string()
                    };
                    let timer_icon = egui::RichText::new(&timer_label)
                        .size(if sleep_timer_ends_at.is_some() { FONT_XS } else { 20.0 })
                        .color(if sleep_timer_ends_at.is_some() {
                            t.accent
                        } else {
                            t.text_primary
                        });
                    let timer_btn = ui
                        .button(timer_icon)
                        .on_hover_text(if sleep_timer_ends_at.is_some() {
                            "Sleep timer active"
                        } else {
                            "Sleep timer"
                        });
                    if timer_btn.clicked() {
                        state.show_sleep_timer = !state.show_sleep_timer;
                    }

                    if state.show_sleep_timer {
                        let area_response =
                            egui::Area::new(egui::Id::new("sleep_timer_menu"))
                                .fixed_pos(timer_btn.rect.left_top() - egui::vec2(0.0, 10.0))
                                .pivot(egui::Align2::LEFT_BOTTOM)
                                .show(ui.ctx(), |ui| {
                                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                                        ui.vertical(|ui| {
                                            for &mins in &[5u64, 15, 30, 45, 60] {
                                                if ui
                                                    .selectable_label(
                                                        sleep_timer_ends_at.map(|t| {
                                                            let remaining = t
                                                                .checked_duration_since(
                                                                    std::time::Instant::now(),
                                                                )
                                                                .unwrap_or_default()
                                                                .as_secs();
                                                            // Highlight if within 60s of the preset
                                                            remaining.abs_diff(mins * 60) < 60
                                                        }).unwrap_or(false),
                                                        format!("{} min", mins),
                                                    )
                                                    .clicked()
                                                {
                                                    action = MediaControlsAction::SetSleepTimer(
                                                        Some(mins),
                                                    );
                                                    state.show_sleep_timer = false;
                                                }
                                            }
                                            if sleep_timer_ends_at.is_some() {
                                                ui.separator();
                                                if ui.button("Off").clicked() {
                                                    action =
                                                        MediaControlsAction::SetSleepTimer(None);
                                                    state.show_sleep_timer = false;
                                                }
                                            }
                                        });
                                    });
                                });

                        if ui.input(|i| i.pointer.any_click())
                            && ui.input(|i| i.pointer.interact_pos()).is_some_and(|pos| {
                                !area_response.response.rect.contains(pos)
                                    && !timer_btn.rect.contains(pos)
                            })
                        {
                            state.show_sleep_timer = false;
                        }
                    }

                    ui.add_space(SPACE_1);

                    let notes_icon = egui::RichText::new(egui_phosphor::regular::NOTE_PENCIL)
                        .size(20.0)
                        .color(if notes_open { t.accent } else { t.text_primary });
                    let notes_btn = ui.button(notes_icon)
                        .on_hover_text(if notes_open { "Close notes" } else { "Open notes" });
                    if notes_btn.clicked() {
                        action = MediaControlsAction::ToggleNotes;
                    }

                    if state.show_queue {
                        let area_response =
                            egui::Area::new(egui::Id::new("queue_menu"))
                                .fixed_pos(queue_btn.rect.left_bottom())
                                .show(ui.ctx(), |ui| {
                                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                                        ui.set_width(350.0);
                                        ui.set_max_height(400.0);

                                        if queue_items.is_empty() {
                                            ui.label("Queue is empty");
                                        } else {
                                            egui::ScrollArea::vertical().show(ui, |ui| {
                                                for item in queue_items {
                                                    ui.horizontal(|ui| {
                                                        ui.vertical(|ui| {
                                                            ui.label(
                                                                egui::RichText::new(
                                                                    &item.podcast_title,
                                                                )
                                                                .small()
                                                                .color(t.text_meta),
                                                            );

                                                            let title = if item.episode_title.len()
                                                                > 35
                                                            {
                                                                let end = item
                                                                    .episode_title
                                                                    .char_indices()
                                                                    .nth(32)
                                                                    .map(|(i, _)| i)
                                                                    .unwrap_or(
                                                                        item.episode_title.len(),
                                                                    );
                                                                format!(
                                                                    "{}...",
                                                                    &item.episode_title[..end]
                                                                )
                                                            } else {
                                                                item.episode_title.clone()
                                                            };
                                                            ui.label(title);
                                                        });

                                                        ui.with_layout(
                                                            egui::Layout::right_to_left(
                                                                egui::Align::Center,
                                                            ),
                                                            |ui| {
                                                                if ui
                                                                    .button(
                                                                        egui::RichText::new(
                                                                            egui_phosphor::regular::X,
                                                                        )
                                                                            .size(16.0),
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    action =
                                                                        MediaControlsAction::RemoveFromQueue(
                                                                            item.queue_id,
                                                                        );
                                                                }
                                                            },
                                                        );
                                                    });
                                                    ui.separator();
                                                }
                                            });
                                        }
                                    });
                                });

                        if ui.input(|i| i.pointer.any_click())
                            && ui.input(|i| i.pointer.interact_pos()).is_some_and(|pos| {
                                !area_response.response.rect.contains(pos)
                                    && !queue_btn.rect.contains(pos)
                            })
                        {
                            state.show_queue = false;
                        }
                    }

                    ui.add_space(SPACE_1);

                    let volume_slider = egui::Slider::new(&mut state.volume, 0.0..=100.0)
                        .show_value(false)
                        .trailing_fill(true);
                    if ui.add(volume_slider).changed() {
                        action = MediaControlsAction::VolumeChanged(state.volume);
                    }

                    ui.add_space(SPACE_1);
                    ui.label(
                        egui::RichText::new(egui_phosphor::regular::SPEAKER_HIGH).size(20.0),
                    );
                },
            );
        });

        action
    }
}

fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

#[derive(Debug, Clone)]
pub enum MediaControlsAction {
    None,
    PlayPause,
    SkipBackward,
    SkipForward,
    Seek(Duration),
    VolumeChanged(f32),
    SetSpeed(f32),
    SetShowDefaultSpeed(f32),
    RemoveFromQueue(i32),
    ToggleNotes,
    SetSleepTimer(Option<u64>),
}
