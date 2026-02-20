use crate::{
    audio_player::AudioPlayer,
    database::Database,
    image_cache::ImageCache,
    types::{Episode, Settings},
};
use egui::Color32;
use egui_alignments::center_horizontal;
use std::time::Duration;

pub struct MediaControls;

impl MediaControls {
    pub fn render(
        ui: &mut egui::Ui,
        audio_player: &AudioPlayer,
        database: &Database,
        image_cache: &ImageCache,
        _settings: &Settings,
        current_episode: Option<&Episode>,
        current_podcast_title: Option<&str>,
        current_podcast_image: Option<&str>,
        volume: &mut f32,
        show_queue: &mut bool,
        show_speed_menu: &mut bool,
    ) -> MediaControlsAction {
        let mut action = MediaControlsAction::None;

        if ui.input(|i| i.pointer.any_click()) {
            let pointer_pos = ui.input(|i| i.pointer.interact_pos());
            if let Some(pos) = pointer_pos {
                if *show_queue || *show_speed_menu {
                    ui.data_mut(|d| d.insert_temp(egui::Id::new("last_click_pos"), pos));
                }
            }
        }

        let total_width = ui.available_width();
        let left_width = total_width * 0.20;
        let middle_width = total_width * 0.60;
        let right_width = total_width * 0.20;

        ui.spacing_mut().item_spacing.x = 0.0;

        ui.horizontal(|ui| {
            // LEFT SECTION
            ui.allocate_ui_with_layout(
                egui::vec2(left_width, ui.available_height()),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_width(left_width);
                    ui.add_space(10.0);

                    if let Some(podcast_title) = current_podcast_title {
                        if let Some(image_url) = current_podcast_image {
                            let texture = image_cache
                                .get_or_load(image_url, ui.ctx())
                                .unwrap_or_else(|| image_cache.get_default_texture(ui.ctx()));
                            ui.add(egui::Image::new(&texture).max_size(egui::vec2(60.0, 60.0)));
                            ui.add_space(10.0);
                        }

                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(podcast_title)
                                    .size(12.0)
                                    .color(Color32::from_rgb(200, 200, 200)),
                            );

                            if let Some(episode) = current_episode {
                                let title = if episode.title.len() > 30 {
                                    format!("{}...", &episode.title[..27])
                                } else {
                                    episode.title.clone()
                                };
                                ui.label(egui::RichText::new(title).size(14.0).strong());
                            }
                        });
                    }
                },
            );

            // MIDDLE SECTION
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
                                    egui::RichText::new(egui_phosphor::regular::SKIP_BACK).size(28.0),
                                ),
                            );
                            if skip_back_btn.clicked() {
                                action = MediaControlsAction::SkipBackward;
                            }
                            if skip_back_btn.hovered() && has_audio {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }

                            ui.add_space(15.0);

                            let play_pause_icon = match audio_player.get_state() {
                                crate::audio_player::PlaybackState::Playing => {
                                    egui_phosphor::regular::PAUSE
                                }
                                _ => egui_phosphor::regular::PLAY,
                            };

                            let play_pause_btn = ui.add_enabled(
                                has_audio,
                                egui::Button::new(egui::RichText::new(play_pause_icon).size(28.0)),
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

                        ui.add_space(15.0);
                        ui.horizontal(|ui| {
                            let position = audio_player.get_position();
                            let duration = audio_player.get_duration();

                            ui.label(format_duration(position));
                            ui.add_space(10.0);

                            let mut pos_secs = position.as_secs_f32();
                            let dur_secs = duration.as_secs_f32();

                            ui.scope(|ui| {
                                ui.spacing_mut().slider_width = ui.available_width() * 0.90;
                                let slider = egui::Slider::new(&mut pos_secs, 0.0..=dur_secs)
                                    .show_value(false)
                                    .update_while_editing(true)
                                    .trailing_fill(true);

                                let slider_response = ui.add_sized([ui.available_width(), 20.0], slider);

                                if has_audio && slider_response.changed() {
                                    action = MediaControlsAction::Seek(Duration::from_secs_f32(pos_secs));
                                }
                            });


                            ui.add_space(10.0);

                            ui.label(format_duration(duration));
                        });
                    });
                });

            ui.add_space(15.0);

            // RIGHT SECTION
            ui.allocate_ui_with_layout(
                egui::vec2(right_width, ui.available_height()),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    ui.set_width(right_width);
                    ui.add_space(10.0);

                    let speed = audio_player.get_speed();
                    let speed_btn = ui.button(format!("{}x", speed));
                    if speed_btn.clicked() {
                        *show_speed_menu = !*show_speed_menu;
                    }

                    if *show_speed_menu {
                        let area_response = egui::Area::new(egui::Id::new("speed_menu"))
                            .fixed_pos(speed_btn.rect.left_bottom())
                            .show(ui.ctx(), |ui| {
                                egui::Frame::popup(ui.style()).show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        for &spd in &[2.0, 1.5, 1.25, 1.0, 0.75] {
                                            if ui.selectable_label(speed == spd, format!("{}x", spd)).clicked() {
                                                action = MediaControlsAction::SetSpeed(spd);
                                                *show_speed_menu = false;
                                            }
                                        }
                                    });
                                });
                            });

                        if ui.input(|i| i.pointer.any_click()) {
                            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                                if !area_response.response.rect.contains(pos) && !speed_btn.rect.contains(pos) {
                                    *show_speed_menu = false;
                                }
                            }
                        }
                    }

                    ui.add_space(5.0);

                    let queue_btn = ui.button(egui::RichText::new(egui_phosphor::regular::QUEUE).size(20.0));
                    if queue_btn.clicked() {
                        *show_queue = !*show_queue;
                    }

                    if *show_queue {
                        let queue_items = database.get_queue().unwrap_or_default();

                        let area_response = egui::Area::new(egui::Id::new("queue_menu"))
                            .fixed_pos(queue_btn.rect.left_bottom())
                            .show(ui.ctx(), |ui| {
                                egui::Frame::popup(ui.style()).show(ui, |ui| {
                                    ui.set_width(350.0);
                                    ui.set_max_height(400.0);

                                    if queue_items.is_empty() {
                                        ui.label("Queue is empty");
                                    } else {
                                        egui::ScrollArea::vertical().show(ui, |ui| {
                                            for item in &queue_items {
                                                let episode_info = database.get_podcasts().ok().and_then(|podcasts| {
                                                    for podcast in podcasts {
                                                        if let Ok(episodes) = database.get_episodes_by_podcast_id(podcast.id.unwrap()) {
                                                            if let Some(episode) = episodes.iter().find(|e| e.id == Some(item.episode_id)) {
                                                                return Some((episode.title.clone(), podcast.title.clone()));
                                                            }
                                                        }
                                                    }
                                                    None
                                                });

                                                ui.horizontal(|ui| {
                                                    ui.vertical(|ui| {
                                                        if let Some((episode_title, podcast_title)) = episode_info {
                                                            ui.label(egui::RichText::new(&podcast_title).small().color(Color32::from_rgb(180, 180, 180)));

                                                            let title = if episode_title.len() > 35 {
                                                                format!("{}...", &episode_title[..32])
                                                            } else {
                                                                episode_title
                                                            };
                                                            ui.label(title);
                                                        } else {
                                                            ui.label(format!("Episode {}", item.episode_id));
                                                        }
                                                    });

                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        if ui.button(egui::RichText::new(egui_phosphor::regular::X).size(16.0)).clicked() {
                                                            action = MediaControlsAction::RemoveFromQueue(item.id.unwrap());
                                                        }
                                                    });
                                                });
                                                ui.separator();
                                            }
                                        });
                                    }
                                });
                            });

                        if ui.input(|i| i.pointer.any_click()) {
                            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                                if !area_response.response.rect.contains(pos) && !queue_btn.rect.contains(pos) {
                                    *show_queue = false;
                                }
                            }
                        }
                    }

                    ui.add_space(5.0);

                    let volume_slider = egui::Slider::new(volume, 0.0..=100.0)
                        .show_value(false)
                        .trailing_fill(true);

                    if ui.add(volume_slider).changed() {
                        action = MediaControlsAction::VolumeChanged(*volume);
                    }

                    ui.add_space(5.0);
                    ui.label(egui::RichText::new(egui_phosphor::regular::SPEAKER_HIGH).size(20.0));
                });
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
    RemoveFromQueue(i32),
}
