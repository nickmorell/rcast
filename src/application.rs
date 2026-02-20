use crate::components::media_controls::MediaControlsAction;
use crate::{
    audio_downloader::AudioDownloader,
    audio_player::{AudioPlayer, PlaybackState},
    components::{AddPodcastModal, MediaControls},
    database::Database,
    image_cache::ImageCache,
    pages::{HomePage, PodcastDetailPage, SettingsPage, podcast_detail::EpisodeAction},
    rss_sync::RssSync,
    types::{Episode, Page, Settings},
};

pub struct RCast {
    database: Database,
    image_cache: ImageCache,
    audio_downloader: AudioDownloader,
    audio_player: AudioPlayer,
    rss_sync: RssSync,

    // Pages
    current_page: Page,
    home_page: HomePage,
    podcast_detail_page: PodcastDetailPage,
    settings_page: SettingsPage,

    // Modals
    add_podcast_modal: AddPodcastModal,

    // State
    settings: Settings,
    current_episode: Option<Episode>,
    current_podcast_title: Option<String>,
    volume: f32,
    show_queue: bool,
    show_speed_menu: bool,
    last_finished_episode_id: Option<i32>, // Track to prevent duplicate autoplay triggers
}

impl Default for RCast {
    fn default() -> Self {
        let database = Database::default();
        let settings = database.get_settings().unwrap_or_default();
        let audio_player = AudioPlayer::new();
        let rss_sync = RssSync::new(database.clone());

        // Start background sync
        let sync_interval = settings.sync_interval_minutes;
        rss_sync.start_background_sync(sync_interval);

        let sync_clone = RssSync::new(database.clone());
        std::thread::spawn(move || {
            sync_clone.sync_all_podcasts();
        });

        let volume = settings.default_volume;
        audio_player.set_volume(volume);

        let home_page = HomePage::new(&database);
        let settings_page = SettingsPage::new(&database);

        Self {
            image_cache: ImageCache::new(),
            audio_downloader: AudioDownloader::new(),
            audio_player,
            rss_sync,
            current_page: Page::Home,
            home_page,
            podcast_detail_page: PodcastDetailPage::new(),
            settings_page,
            add_podcast_modal: AddPodcastModal::new(),
            settings,
            current_episode: None,
            current_podcast_title: None,
            volume,
            show_queue: false,
            show_speed_menu: false,
            last_finished_episode_id: None,
            database,
        }
    }
}

impl RCast {
    pub fn new() -> Self {
        Self::default()
    }

    fn handle_episode_action(&mut self, action: EpisodeAction) {
        match action {
            EpisodeAction::Play(episode_id) => {
                println!("Playing episode ID {}", episode_id);
                if let Ok(podcasts) = self.database.get_podcasts() {
                    let mut found = false;
                    for podcast in podcasts {
                        if let Ok(episodes) = self
                            .database
                            .get_episodes_by_podcast_id(podcast.id.unwrap())
                        {
                            if let Some(episode) =
                                episodes.iter().find(|e| e.id == Some(episode_id))
                            {
                                println!(
                                    "Playing episode ID {} from podcast '{}' (podcast_id: {:?})",
                                    episode_id, podcast.title, podcast.id
                                );
                                self.current_episode = Some(episode.clone());
                                self.current_podcast_title = Some(podcast.title.clone());

                                println!("Playing episode URL: {}", episode.url);
                                match self
                                    .audio_downloader
                                    .get_or_download(&episode.url, episode_id)
                                {
                                    Ok(path) => {
                                        if let Err(e) = self.audio_player.play(
                                            path.to_str().unwrap(),
                                            episode_id,
                                            episode.duration,
                                        ) {
                                            eprintln!("Failed to play audio: {}", e);
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to download/play audio: {}", e);
                                    }
                                }

                                found = true;
                                break;
                            }
                        }
                        if found {
                            break;
                        }
                    }
                    if !found {
                        eprintln!("Episode ID {} not found in any podcast", episode_id);
                    }
                }
            }
            EpisodeAction::Pause => {
                self.audio_player.pause();
            }
            EpisodeAction::TogglePlayed(episode_id) => {
                if let Ok(podcasts) = self.database.get_podcasts() {
                    for podcast in podcasts {
                        if let Ok(episodes) = self
                            .database
                            .get_episodes_by_podcast_id(podcast.id.unwrap())
                        {
                            if let Some(episode) =
                                episodes.iter().find(|e| e.id == Some(episode_id))
                            {
                                self.database
                                    .update_episode_played(episode_id, !episode.is_played)
                                    .ok();
                                // Reload current page
                                if let Page::PodcastDetail(podcast_id) = self.current_page {
                                    self.podcast_detail_page.load(podcast_id, &self.database);
                                }
                                break;
                            }
                        }
                    }
                }
            }
            EpisodeAction::AddToQueue(episode_id) => {
                self.database.add_to_queue(episode_id).ok();
            }
        }
    }
}

impl eframe::App for RCast {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Request repaint for smooth playback progress
        ctx.request_repaint();

        // Check if audio has finished and autoplay next is enabled
        if self.audio_player.is_finished() && self.settings.auto_play_next {
            // Only trigger autoplay once per finished episode
            let current_ep_id = self.audio_player.get_current_episode_id();
            if current_ep_id != self.last_finished_episode_id {
                self.last_finished_episode_id = current_ep_id;

                // Get the next item in the queue
                if let Ok(queue_items) = self.database.get_queue() {
                    println!("Queue has {} items", queue_items.len());
                    if let Some(first_item) = queue_items.first() {
                        let episode_id = first_item.episode_id;
                        let queue_id = first_item.id.unwrap();
                        println!("Auto-playing episode ID {} from queue", episode_id);

                        // Remove from queue
                        self.database.remove_from_queue(queue_id).ok();

                        // Play the episode
                        self.handle_episode_action(EpisodeAction::Play(episode_id));
                    } else {
                        println!("Queue is empty, stopping playback");
                        // No more items in the queue, reset to stopped state
                        self.audio_player.stop();
                        self.current_episode = None;
                        self.current_podcast_title = None;
                    }
                }
            }
        } else if !self.audio_player.is_finished() {
            // Reset the flag when audio is playing
            if self.audio_player.get_state() == PlaybackState::Playing {
                if self.last_finished_episode_id.is_some() {
                    self.last_finished_episode_id = None;
                }
            }
        }

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Add Podcast").clicked() {
                        self.add_podcast_modal.open();
                        ui.close();
                    }

                    if ui.button("Settings").clicked() {
                        self.settings_page
                            .set_previous_page(self.current_page.clone());
                        self.current_page = Page::Settings;
                        ui.close();
                    }

                    ui.separator();

                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("About", |ui| {
                    ui.label("RCast - Podcast Player");
                    ui.label("Version 0.1.0");
                });

                if self.rss_sync.is_syncing() {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spinner();
                        ui.label("Syncing...");
                    });
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match &self.current_page {
                Page::Home => {
                    if let Some(next_page) = self.home_page.render(
                        ui,
                        &self.database,
                        &self.image_cache,
                        self.audio_player.get_current_episode_id(),
                    ) {
                        self.current_page = next_page.clone();

                        if let Page::PodcastDetail(podcast_id) = next_page {
                            self.podcast_detail_page.load(podcast_id, &self.database);
                        }
                    }
                }
                Page::PodcastDetail(_) => {
                    let (next_page, episode_action) = self.podcast_detail_page.render(
                        ui,
                        &self.database,
                        &self.image_cache,
                        self.audio_player.get_current_episode_id(),
                    );

                    if let Some(page) = next_page {
                        self.current_page = page;
                    }

                    if let Some(action) = episode_action {
                        self.handle_episode_action(action);
                    }
                }
                Page::Settings => {
                    let (next_page, save_changes) = self.settings_page.render(ui, &self.database);

                    if save_changes {
                        // Reload settings
                        self.settings = self.database.get_settings().unwrap_or_default();
                        self.audio_player.set_volume(self.settings.default_volume);
                        self.volume = self.settings.default_volume;
                    }

                    if let Some(page) = next_page {
                        self.current_page = page;
                    }
                }
            }
        });

        egui::TopBottomPanel::bottom("media_controls")
            .min_height(80.0)
            .show(ctx, |ui| {
                ui.add_space(5.0);

                let current_podcast_image = if let Some(episode) = &self.current_episode {
                    self.database.get_podcasts().ok().and_then(|podcasts| {
                        podcasts
                            .iter()
                            .find(|p| p.id.unwrap() == episode.podcast_id)
                            .map(|p| p.image_url.clone())
                    })
                } else {
                    None
                };

                let action = MediaControls::render(
                    ui,
                    &self.audio_player,
                    &self.database,
                    &self.image_cache,
                    &self.settings,
                    self.current_episode.as_ref(),
                    self.current_podcast_title.as_deref(),
                    current_podcast_image.as_deref(),
                    &mut self.volume,
                    &mut self.show_queue,
                    &mut self.show_speed_menu,
                );

                match action {
                    MediaControlsAction::PlayPause => match self.audio_player.get_state() {
                        PlaybackState::Playing => self.audio_player.pause(),
                        PlaybackState::Paused => self.audio_player.resume(),
                        _ => {}
                    },
                    MediaControlsAction::SkipBackward => {
                        self.audio_player
                            .skip_backward(self.settings.skip_backward_seconds);
                    }
                    MediaControlsAction::SkipForward => {
                        self.audio_player
                            .skip_forward(self.settings.skip_forward_seconds);
                    }
                    MediaControlsAction::Seek(pos) => {
                        self.audio_player.seek(pos);
                    }
                    MediaControlsAction::VolumeChanged(vol) => {
                        self.audio_player.set_volume(vol);
                    }
                    MediaControlsAction::SetSpeed(speed) => {
                        self.audio_player.set_speed(speed);
                    }
                    MediaControlsAction::RemoveFromQueue(queue_id) => {
                        self.database.remove_from_queue(queue_id).ok();
                    }
                    _ => {}
                }

                ui.add_space(5.0);
            });

        if let Some(url) = self.add_podcast_modal.render(ctx) {
            let rss_sync = self.rss_sync.clone();
            std::thread::spawn(move || {
                if let Ok(podcast) = rss_sync.fetch_and_add_podcast(&url) {
                    println!("Added podcast: {}", podcast.title);
                }
            });

            std::thread::sleep(std::time::Duration::from_millis(500));
            self.home_page.refresh(&self.database);
        }
    }
}

impl Clone for RssSync {
    fn clone(&self) -> Self {
        Self {
            database: self.database.clone(),
            is_syncing: self.is_syncing.clone(),
        }
    }
}
