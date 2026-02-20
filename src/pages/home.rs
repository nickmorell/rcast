use crate::{
    components::PodcastCard,
    database::Database,
    image_cache::ImageCache,
    types::{Page, Podcast, SortOrder},
};
use egui_alignments::center_vertical;

pub struct HomePage {
    podcasts: Vec<Podcast>,
    filtered_podcasts: Vec<Podcast>,
    search_query: String,
    sort_order: SortOrder,
}

impl HomePage {
    pub fn new(database: &Database) -> Self {
        let podcasts = database.get_podcasts().unwrap_or_default();

        Self {
            filtered_podcasts: podcasts.clone(),
            podcasts,
            search_query: String::new(),
            sort_order: SortOrder::AToZ,
        }
    }

    pub fn refresh(&mut self, database: &Database) {
        self.podcasts = database.get_podcasts().unwrap_or_default();
        self.apply_filters();
    }

    fn apply_filters(&mut self) {
        let query = self.search_query.to_lowercase();

        self.filtered_podcasts = self
            .podcasts
            .iter()
            .filter(|p| {
                if query.is_empty() {
                    true
                } else {
                    p.title.to_lowercase().contains(&query)
                        || p.description.to_lowercase().contains(&query)
                }
            })
            .cloned()
            .collect();

        match self.sort_order {
            SortOrder::AToZ => {
                self.filtered_podcasts.sort_by(|a, b| a.title.cmp(&b.title));
            }
            SortOrder::ZToA => {
                self.filtered_podcasts.sort_by(|a, b| b.title.cmp(&a.title));
            }
            SortOrder::PublishDateAsc => {
                self.filtered_podcasts.sort_by_key(|p| p.updated_at);
            }
            SortOrder::PublishDateDesc => {
                self.filtered_podcasts
                    .sort_by_key(|p| std::cmp::Reverse(p.updated_at));
            }
        }
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        database: &Database,
        image_cache: &ImageCache,
        current_episode_id: Option<i32>,
    ) -> Option<Page> {
        let mut next_page = None;

        if self.podcasts.is_empty() {
            center_vertical(ui, |ui| {
                ui.label("Add your first podcast by clicking File > Add Podcast in the menu bar.");
            });

            return next_page;
        }

        ui.vertical(|ui| {
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Search:");
                let response = ui.text_edit_singleline(&mut self.search_query);
                if response.changed() {
                    self.apply_filters();
                }

                ui.add_space(20.0);

                ui.label("Sort:");
                egui::ComboBox::from_id_salt("sort_order")
                    .selected_text(match self.sort_order {
                        SortOrder::AToZ => "A to Z",
                        SortOrder::ZToA => "Z to A",
                        SortOrder::PublishDateAsc => "Publish Date (Oldest)",
                        SortOrder::PublishDateDesc => "Publish Date (Newest)",
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
                                "Publish Date (Oldest)",
                            )
                            .clicked()
                        {
                            self.apply_filters();
                        }
                        if ui
                            .selectable_value(
                                &mut self.sort_order,
                                SortOrder::PublishDateDesc,
                                "Publish Date (Newest)",
                            )
                            .clicked()
                        {
                            self.apply_filters();
                        }
                    });
            });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(20.0); // Extra padding for the glow effect

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(5.0);
                ui.horizontal_wrapped(|ui| {
                    for podcast in &self.filtered_podcasts {
                        let podcast_id = podcast.id.unwrap();
                        let is_playing = current_episode_id.is_some();

                        if PodcastCard::new(podcast, database, image_cache, is_playing)
                            .show(ui)
                            .clicked()
                        {
                            next_page = Some(Page::PodcastDetail(podcast_id));
                        }
                    }
                });
            });
        });

        next_page
    }
}
