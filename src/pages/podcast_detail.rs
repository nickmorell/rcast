use crate::{
    database::Database,
    image_cache::ImageCache,
    types::{Episode, Page, Podcast, SortOrder},
};
use egui::Popup;

pub struct PodcastDetailPage {
    podcast: Option<Podcast>,
    episodes: Vec<Episode>,
    filtered_episodes: Vec<Episode>,
    description_expanded: bool,
    search_query: String,
    sort_order: SortOrder,
}

impl PodcastDetailPage {
    pub fn new() -> Self {
        Self {
            podcast: None,
            episodes: Vec::new(),
            filtered_episodes: Vec::new(),
            description_expanded: false,
            search_query: String::new(),
            sort_order: SortOrder::PublishDateDesc,
        }
    }

    pub fn load(&mut self, podcast_id: i32, database: &Database) {
        if let Ok(podcasts) = database.get_podcasts() {
            self.podcast = podcasts.into_iter().find(|p| p.id == Some(podcast_id));
        }

        self.episodes = database
            .get_episodes_by_podcast_id(podcast_id)
            .unwrap_or_default();

        self.apply_filters();
    }

    fn apply_filters(&mut self) {
        let query = self.search_query.to_lowercase();

        self.filtered_episodes = self
            .episodes
            .iter()
            .filter(|e| {
                if query.is_empty() {
                    true
                } else {
                    e.title.to_lowercase().contains(&query)
                        || e.format_publish_date().to_lowercase().contains(&query)
                }
            })
            .cloned()
            .collect();

        match self.sort_order {
            SortOrder::AToZ => {
                self.filtered_episodes.sort_by(|a, b| a.title.cmp(&b.title));
            }
            SortOrder::ZToA => {
                self.filtered_episodes.sort_by(|a, b| b.title.cmp(&a.title));
            }
            SortOrder::PublishDateDesc => {
                self.filtered_episodes
                    .sort_by_key(|e| std::cmp::Reverse(e.publish_date));
            }
            SortOrder::PublishDateAsc => {
                self.filtered_episodes.sort_by_key(|e| e.publish_date);
            }
        }
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        _database: &Database,
        image_cache: &ImageCache,
        current_episode_id: Option<i32>,
    ) -> (Option<Page>, Option<EpisodeAction>) {
        let mut next_page = None;
        let mut episode_action = None;

        if let Some(podcast) = self.podcast.clone() {
            ui.vertical(|ui| {
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    if ui
                        .button(format!("{} Back", egui_phosphor::regular::ARROW_LEFT))
                        .clicked()
                    {
                        next_page = Some(Page::Home);
                    }

                    ui.add_space(20.0);

                    let texture = image_cache
                        .get_or_load(&podcast.image_url, ui.ctx())
                        .unwrap_or_else(|| image_cache.get_default_texture(ui.ctx()));

                    ui.add(egui::Image::new(&texture).max_size(egui::vec2(200.0, 200.0)));

                    ui.add_space(20.0);

                    ui.vertical(|ui| {
                        ui.set_max_width(600.0);

                        ui.heading(&podcast.title);
                        ui.add_space(10.0);

                        let should_truncate = podcast.description.len() > 250;

                        if should_truncate && !self.description_expanded {
                            let truncated = format!("{}...", &podcast.description[..250]);
                            ui.label(truncated);

                            if ui
                                .button(format!("{} Expand", egui_phosphor::regular::CARET_DOWN))
                                .clicked()
                            {
                                self.description_expanded = true;
                            }
                        } else {
                            ui.label(&podcast.description);

                            if should_truncate {
                                if ui
                                    .button(format!(
                                        "{} Collapse",
                                        egui_phosphor::regular::CARET_UP
                                    ))
                                    .clicked()
                                {
                                    self.description_expanded = false;
                                }
                            }
                        }
                    });
                });

                ui.add_space(20.0);
                ui.separator();
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label("Search:");
                    let response = ui.text_edit_singleline(&mut self.search_query);
                    if response.changed() {
                        self.apply_filters();
                    }

                    ui.add_space(20.0);

                    ui.label("Sort:");
                    egui::ComboBox::from_id_salt("episode_sort_order")
                        .selected_text(match self.sort_order {
                            SortOrder::AToZ => "A to Z",
                            SortOrder::ZToA => "Z to A",
                            SortOrder::PublishDateAsc => "Oldest First",
                            SortOrder::PublishDateDesc => "Newest First",
                        })
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_value(&mut self.sort_order, SortOrder::AToZ, "A to Z")
                                .clicked()
                            {
                                self.apply_filters();
                            }
                            if ui
                                .selectable_value(&mut self.sort_order, SortOrder::ZToA, "Z to A")
                                .clicked()
                            {
                                self.apply_filters();
                            }
                            if ui
                                .selectable_value(
                                    &mut self.sort_order,
                                    SortOrder::PublishDateAsc,
                                    "Oldest First",
                                )
                                .clicked()
                            {
                                self.apply_filters();
                            }
                            if ui
                                .selectable_value(
                                    &mut self.sort_order,
                                    SortOrder::PublishDateDesc,
                                    "Newest First",
                                )
                                .clicked()
                            {
                                self.apply_filters();
                            }
                        });
                });

                ui.add_space(10.0);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.push_id("episodes_table", |ui| {
                        for episode in &self.filtered_episodes {
                            let ep_id = episode.id.unwrap();
                            let is_current = current_episode_id == Some(ep_id);

                            ui.horizontal(|ui| {
                                ui.set_width(ui.available_width());

                                let (rect, response) = ui.allocate_exact_size(
                                    egui::vec2(24.0, 24.0),
                                    egui::Sense::click(),
                                );

                                let icon_text = if response.hovered() {
                                    if is_current {
                                        egui_phosphor::regular::PAUSE
                                    } else {
                                        egui_phosphor::regular::PLAY
                                    }
                                } else {
                                    if episode.is_played {
                                        egui_phosphor::regular::RECORD
                                    } else {
                                        egui_phosphor::regular::CIRCLE
                                    }
                                };

                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    icon_text,
                                    egui::FontId::proportional(20.0),
                                    ui.visuals().text_color(),
                                );

                                if response.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }

                                if response.clicked() {
                                    if is_current {
                                        episode_action = Some(EpisodeAction::Pause);
                                    } else {
                                        episode_action = Some(EpisodeAction::Play(ep_id));
                                    }
                                }

                                ui.add_space(10.0);

                                let title = if episode.title.len() > 50 {
                                    format!("{}...", &episode.title[..47])
                                } else {
                                    episode.title.clone()
                                };
                                ui.label(title);

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let menu_id = egui::Id::new(format!("ctx_menu_{}", ep_id));
                                        let menu_btn = ui.button(
                                            egui::RichText::new(egui_phosphor::regular::DOTS_THREE)
                                                .size(16.0),
                                        );
                                        if menu_btn.clicked() {
                                            Popup::toggle_id(ui.ctx(), menu_id);
                                        }

                                        egui::popup_below_widget(
                                            ui,
                                            menu_id,
                                            &menu_btn,
                                            egui::PopupCloseBehavior::CloseOnClickOutside,
                                            |ui: &mut egui::Ui| {
                                                ui.set_min_width(150.0);
                                                if ui
                                                    .button(if episode.is_played {
                                                        "Mark Unplayed"
                                                    } else {
                                                        "Mark Played"
                                                    })
                                                    .clicked()
                                                {
                                                    episode_action =
                                                        Some(EpisodeAction::TogglePlayed(ep_id));
                                                    Popup::close_id(ui.ctx(), menu_id);
                                                }

                                                if ui.button("Add to Queue").clicked() {
                                                    println!("Adding episode {} to queue", ep_id);
                                                    episode_action =
                                                        Some(EpisodeAction::AddToQueue(ep_id));
                                                    Popup::close_id(ui.ctx(), menu_id);
                                                }
                                            },
                                        );

                                        ui.add_space(10.0);

                                        ui.label(episode.format_publish_date());
                                    },
                                );
                            });

                            ui.separator();
                        }
                    });
                });
            });
        }

        (next_page, episode_action)
    }
}

#[derive(Debug, Clone)]
pub enum EpisodeAction {
    Play(i32),
    Pause,
    TogglePlayed(i32),
    AddToQueue(i32),
}
