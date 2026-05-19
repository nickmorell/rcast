use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::audio_cache::AudioCache;
use crate::audio_player::AudioPlayer;
use crate::commands::AppCommand;
use crate::components::toast::ToastMessage;
use crate::db::Database;
use crate::db::models::{DownloadStatus, Episode, Podcast};
use crate::download_manager::DownloadManager;
use crate::events::AppEvent;
use crate::types::{Page, Settings};

pub struct Orchestrator {
    cmd_rx: UnboundedReceiver<AppCommand>,
    event_tx: UnboundedSender<AppEvent>,
    db: Database,
    audio_player: AudioPlayer,
    audio_cache: AudioCache,
    download_manager: DownloadManager,
    current_detail_podcast_id: Option<i32>,
    last_saved_position: f64,
    settings: Settings,
    current_skip_outro_seconds: i32,
    // Sleep timer
    sleep_timer_target: Option<std::time::Instant>,
    // Listen-time tracking
    session_start: Option<std::time::Instant>,
    session_flushed_secs: u64,
}

impl Orchestrator {
    pub fn new(
        cmd_rx: UnboundedReceiver<AppCommand>,
        event_tx: UnboundedSender<AppEvent>,
        db: Database,
        audio_player: AudioPlayer,
        download_manager: DownloadManager,
    ) -> Self {
        Self {
            cmd_rx,
            event_tx,
            db,
            audio_player,
            audio_cache: AudioCache::new(2),
            download_manager,
            current_detail_podcast_id: None,
            last_saved_position: 0.0,
            settings: Settings::default(),
            current_skip_outro_seconds: 0,
            sleep_timer_target: None,
            session_start: None,
            session_flushed_secs: 0,
        }
    }

    pub async fn run(mut self) {
        let settings = self.db.get_settings().await.unwrap_or_default();
        self.audio_player.set_trim_silence_mode(settings.trim_silence_mode);
        self.audio_player.set_speed(settings.default_speed);
        self.settings = settings.clone();

        let _ = self.event_tx.send(AppEvent::SettingsLoaded(settings.clone()));
        self.load_all_podcasts().await;

        {
            let tx = self.event_tx.clone();
            let db = self.db.clone();
            let interval_secs = (settings.sync_interval_minutes * 60) as u64;
            tokio::spawn(async move {
                let mut ticker =
                    tokio::time::interval(std::time::Duration::from_secs(interval_secs));
                ticker.tick().await;
                loop {
                    ticker.tick().await;
                    background_sync(db.clone(), tx.clone()).await;
                }
            });
        }

        {
            let tx = self.event_tx.clone();
            let db = self.db.clone();
            tokio::spawn(async move {
                background_sync(db, tx).await;
            });
        }

        let mut save_ticker = tokio::time::interval(std::time::Duration::from_secs(10));
        save_ticker.tick().await;

        loop {
            tokio::select! {
                Some(cmd) = self.cmd_rx.recv() => {
                    self.handle(cmd).await;
                }
                _ = save_ticker.tick() => {
                    self.auto_save_position().await;
                    self.check_outro_skip().await;
                    self.check_sleep_timer().await;
                }
            }
        }
    }

    async fn auto_save_position(&mut self) {
        use crate::audio_player::PlaybackState;

        let episode_id = match self.audio_player.get_current_episode_id() {
            Some(id) => id,
            None => return,
        };

        if self.audio_player.get_state() != PlaybackState::Playing {
            return;
        }

        let position = self.audio_player.get_position().as_secs_f64();

        if (position - self.last_saved_position).abs() < 5.0 {
            return;
        }

        if self
            .db
            .update_episode_position(episode_id, position)
            .await
            .is_ok()
        {
            self.last_saved_position = position;
        }

        if let Some(start) = &self.session_start {
            let total = start.elapsed().as_secs();
            let delta = total - self.session_flushed_secs;
            if delta > 0 {
                self.db.increment_listen_seconds(episode_id, delta).await.ok();
                self.session_flushed_secs = total;
            }
        }
    }

    async fn check_outro_skip(&mut self) {
        use crate::audio_player::PlaybackState;

        if self.current_skip_outro_seconds <= 0 {
            return;
        }
        if self.audio_player.get_state() != PlaybackState::Playing {
            return;
        }

        let duration = self.audio_player.get_duration().as_secs_f64();
        let position = self.audio_player.get_position().as_secs_f64();

        if duration > 0.0 && position >= duration - self.current_skip_outro_seconds as f64 {
            self.current_skip_outro_seconds = 0;
            self.handle(AppCommand::PlayNextInQueue).await;
        }
    }

    async fn check_sleep_timer(&mut self) {
        let target = match self.sleep_timer_target {
            Some(t) => t,
            None => return,
        };

        if std::time::Instant::now() < target {
            return;
        }

        self.sleep_timer_target = None;
        self.flush_listen_session().await;
        self.audio_player.pause();
        let _ = self.event_tx.send(AppEvent::PlaybackStopped);
        let _ = self.event_tx.send(AppEvent::SleepTimerUpdated(None));
    }

    async fn flush_listen_session(&mut self) {
        if let Some(start) = self.session_start.take() {
            let total = start.elapsed().as_secs();
            let delta = total - self.session_flushed_secs;
            self.session_flushed_secs = 0;
            if delta > 0 && let Some(episode_id) = self.audio_player.get_current_episode_id() {
                self.db.increment_listen_seconds(episode_id, delta).await.ok();
            }
        }
    }

    async fn handle(&mut self, cmd: AppCommand) {
        match cmd {
            // Navigation
            AppCommand::NavigateTo(page) => {
                let _ = self.event_tx.send(AppEvent::NavigatedTo(page.clone()));

                match &page {
                    Page::Home => {
                        self.current_detail_podcast_id = None;
                        self.load_all_podcasts().await;
                    }
                    Page::PodcastDetail(id) => {
                        self.current_detail_podcast_id = Some(*id);
                        self.load_podcast_detail(*id).await;
                    }
                    Page::Settings => match self.db.get_settings().await {
                        Ok(s) => {
                            let _ = self.event_tx.send(AppEvent::SettingsLoaded(s));
                            match self.db.get_listening_stats().await {
                                Ok(stats) => {
                                    let _ = self
                                        .event_tx
                                        .send(AppEvent::ListeningStatsLoaded(stats));
                                }
                                Err(e) => {
                                    let _ = self.event_tx.send(AppEvent::Error(format!(
                                        "Failed to load stats: {e}"
                                    )));
                                }
                            }
                        }
                        Err(e) => {
                            let _ = self
                                .event_tx
                                .send(AppEvent::Error(format!("Failed to load settings: {e}")));
                        }
                    },
                }
            }

            // Podcasts
            AppCommand::AddPodcast { feed_url } => {
                let tx = self.event_tx.clone();
                let db = self.db.clone();
                tokio::spawn(async move {
                    add_podcast(feed_url, db, tx).await;
                });
            }
            AppCommand::RemovePodcast(id) => {
                // Delete downloaded episode files before removing the DB record.
                if let Ok(downloaded) = self.db.get_downloaded_episodes(id).await {
                    for ep in downloaded {
                        if let Some(path) = ep.downloaded_path {
                            let _ = self.download_manager.delete_file(&path);
                        }
                    }
                }
                match self.db.delete_podcast(id).await {
                    Ok(_) => {
                        let _ = self.event_tx.send(AppEvent::PodcastRemoved(id));
                        let _ = self
                            .event_tx
                            .send(AppEvent::Toast(ToastMessage::success("Podcast removed")));
                    }
                    Err(e) => {
                        let _ = self
                            .event_tx
                            .send(AppEvent::Error(format!("Failed to remove podcast: {e}")));
                    }
                }
            }
            AppCommand::SyncPodcast(id) => {
                let tx = self.event_tx.clone();
                let db = self.db.clone();
                let settings = self.settings.clone();
                tokio::spawn(async move {
                    sync_podcast(id, db, tx, &settings).await;
                });
            }
            AppCommand::SyncAll => {
                let tx = self.event_tx.clone();
                let db = self.db.clone();
                let settings = self.settings.clone();
                tokio::spawn(async move {
                    background_sync_with_settings(db, tx, settings).await;
                });
            }
            AppCommand::UpdatePodcastPreferences { podcast_id, prefs } => {
                let should_enforce = prefs.keep_episodes_count.is_some()
                    || self.settings.global_keep_episodes_count > 0;
                match self
                    .db
                    .update_podcast_preferences(podcast_id, prefs.clone())
                    .await
                {
                    Ok(_) => {
                        let _ = self.event_tx.send(AppEvent::PodcastPreferencesUpdated {
                            podcast_id,
                            prefs,
                        });
                        if should_enforce {
                            let db = self.db.clone();
                            let dm = self.download_manager.clone();
                            let settings = self.settings.clone();
                            tokio::spawn(async move {
                                enforce_retention_policy(podcast_id, &db, &dm, &settings).await;
                            });
                        }
                    }
                    Err(e) => {
                        let _ = self.event_tx.send(AppEvent::Error(format!(
                            "Failed to update podcast settings: {e}"
                        )));
                    }
                }
            }

            // Episodes
            AppCommand::DownloadEpisode(episode_id) => {
                let episode = match self.db.get_episode(episode_id).await {
                    Ok(Some(e)) => e,
                    _ => {
                        let _ = self
                            .event_tx
                            .send(AppEvent::Error("Episode not found".into()));
                        return;
                    }
                };

                let podcast = match self.db.get_podcast(episode.podcast_id).await {
                    Ok(Some(p)) => p,
                    _ => {
                        let _ = self
                            .event_tx
                            .send(AppEvent::Error("Podcast not found".into()));
                        return;
                    }
                };

                // Signal downloading state immediately.
                let _ = self.event_tx.send(AppEvent::DownloadStatusChanged {
                    episode_id,
                    status: DownloadStatus::Downloading,
                    path: None,
                });
                let _ = self
                    .db
                    .update_episode_download_status(episode_id, DownloadStatus::Downloading, None)
                    .await;

                let download_manager = self.download_manager.clone();
                let db = self.db.clone();
                let tx = self.event_tx.clone();
                let settings = self.settings.clone();
                let podcast_id = podcast.id;

                tokio::task::spawn_blocking(move || {
                    let folders = vec![podcast.title.clone()];
                    let file_name = episode.title.clone();
                    let ep_title = episode.title.clone();

                    // Skip if already downloaded.
                    if let Some(existing) = download_manager.find_file(folders.clone(), &file_name)
                    {
                        let path_str = existing.to_string_lossy().to_string();
                        let rt = tokio::runtime::Handle::current();
                        rt.block_on(async {
                            let _ = db
                                .update_episode_download_status(
                                    episode_id,
                                    DownloadStatus::Downloaded,
                                    Some(path_str.clone()),
                                )
                                .await;
                        });
                        let _ = tx.send(AppEvent::DownloadStatusChanged {
                            episode_id,
                            status: DownloadStatus::Downloaded,
                            path: Some(path_str),
                        });
                        return;
                    }

                    match download_manager.download(episode.url, folders, file_name) {
                        Ok(path) => {
                            let path_str = path.to_string_lossy().to_string();
                            let rt = tokio::runtime::Handle::current();
                            rt.block_on(async {
                                let _ = db
                                    .update_episode_download_status(
                                        episode_id,
                                        DownloadStatus::Downloaded,
                                        Some(path_str.clone()),
                                    )
                                    .await;
                                enforce_retention_policy(podcast_id, &db, &download_manager, &settings).await;
                            });
                            if settings.notify_download_complete {
                                send_notification("Download complete", &ep_title);
                            }
                            let _ = tx.send(AppEvent::DownloadStatusChanged {
                                episode_id,
                                status: DownloadStatus::Downloaded,
                                path: Some(path_str),
                            });
                            let _ = tx.send(AppEvent::Toast(ToastMessage::success("Download complete")));
                        }
                        Err(e) => {
                            let rt = tokio::runtime::Handle::current();
                            rt.block_on(async {
                                let _ = db
                                    .update_episode_download_status(
                                        episode_id,
                                        DownloadStatus::Failed,
                                        None,
                                    )
                                    .await;
                            });
                            let _ = tx.send(AppEvent::DownloadStatusChanged {
                                episode_id,
                                status: DownloadStatus::Failed,
                                path: None,
                            });
                            let _ = tx.send(AppEvent::Error(format!("Download failed: {e}")));
                        }
                    }
                });
            }

            AppCommand::DeleteDownload(episode_id) => {
                if let Ok(Some(ep)) = self.db.get_episode(episode_id).await {
                    if let Some(path) = ep.downloaded_path {
                        self.download_manager.delete_file(&path).ok();
                    }
                    self.db
                        .update_episode_download_status(episode_id, DownloadStatus::NotDownloaded, None)
                        .await
                        .ok();
                    let _ = self.event_tx.send(AppEvent::DownloadStatusChanged {
                        episode_id,
                        status: DownloadStatus::NotDownloaded,
                        path: None,
                    });
                }
            }

            AppCommand::TogglePlayed(episode_id) => {
                if let Ok(Some(ep)) = self.db.get_episode(episode_id).await {
                    self.db
                        .update_episode_played(episode_id, !ep.is_played)
                        .await
                        .ok();

                    if let Some(podcast_id) = self.current_detail_podcast_id {
                        match self.db.get_episodes(podcast_id).await {
                            Ok(episodes) => {
                                let _ = self.event_tx.send(AppEvent::EpisodesUpdated {
                                    podcast_id,
                                    episodes,
                                });
                            }
                            Err(e) => {
                                let _ = self.event_tx.send(AppEvent::Error(format!(
                                    "Failed to reload episodes: {e}"
                                )));
                            }
                        }
                    }
                }
            }

            AppCommand::SetEpisodeSpeedPreset { episode_id, speed } => {
                self.db
                    .update_episode_speed_preset(episode_id, speed)
                    .await
                    .ok();
            }
            AppCommand::CompleteEpisode(episode_id) => {
                self.db.complete_episode(episode_id).await.ok();
                if let Some(podcast_id) = self.current_detail_podcast_id
                    && let Ok(eps) = self.db.get_episodes(podcast_id).await
                {
                    let _ = self.event_tx.send(AppEvent::EpisodesUpdated {
                        podcast_id,
                        episodes: eps,
                    });
                }
            }

            // Playback
            AppCommand::PlayEpisode(episode_id) => {
                self.play_episode(episode_id).await;
            }
            AppCommand::PlayAll(episode_ids) => {
                if episode_ids.is_empty() {
                    return;
                }
                self.play_episode(episode_ids[0]).await;
                for &id in &episode_ids[1..] {
                    self.db.add_to_queue(id).await.ok();
                }
                self.refresh_queue_display().await;
            }
            AppCommand::PausePlayback => {
                self.flush_listen_session().await;
                if let Some(episode_id) = self.audio_player.get_current_episode_id() {
                    let position = self.audio_player.get_position().as_secs_f64();
                    self.db
                        .update_episode_position(episode_id, position)
                        .await
                        .ok();
                    self.last_saved_position = position;
                }
                self.audio_player.pause();
            }
            AppCommand::ResumePlayback => {
                self.session_start = Some(std::time::Instant::now());
                self.session_flushed_secs = 0;
                self.audio_player.resume();
            }
            AppCommand::TogglePlayback => {
                use crate::audio_player::PlaybackState;
                match self.audio_player.get_state() {
                    PlaybackState::Playing => {
                        self.flush_listen_session().await;
                        if let Some(episode_id) = self.audio_player.get_current_episode_id() {
                            let position = self.audio_player.get_position().as_secs_f64();
                            self.db.update_episode_position(episode_id, position).await.ok();
                            self.last_saved_position = position;
                        }
                        self.audio_player.pause();
                    }
                    PlaybackState::Paused => {
                        self.session_start = Some(std::time::Instant::now());
                        self.session_flushed_secs = 0;
                        self.audio_player.resume();
                    }
                    PlaybackState::Stopped => {}
                }
            }
            AppCommand::JumpForward => {
                self.audio_player.skip_forward(self.settings.skip_forward_seconds);
            }
            AppCommand::JumpBackward => {
                self.audio_player.skip_backward(self.settings.skip_backward_seconds);
            }
            AppCommand::PlayNextInQueue => {
                if let Some(episode_id) = self.audio_player.get_current_episode_id() {
                    self.db.complete_episode(episode_id).await.ok();
                    self.last_saved_position = 0.0;
                    // Refresh the detail list so the completed episode shows as played.
                    if let Some(podcast_id) = self.current_detail_podcast_id
                        && let Ok(eps) = self.db.get_episodes(podcast_id).await
                    {
                        let _ = self.event_tx.send(AppEvent::EpisodesUpdated {
                            podcast_id,
                            episodes: eps,
                        });
                    }
                }
                self.current_skip_outro_seconds = 0;
                match self.db.get_queue().await {
                    Ok(queue) => {
                        if let Some(first) = queue.into_iter().next() {
                            let episode_id = first.episode_id;
                            self.db.remove_from_queue(first.id).await.ok();
                            self.play_episode(episode_id).await;
                            self.refresh_queue_display().await;
                        } else {
                            self.audio_player.stop();
                            let _ = self.event_tx.send(AppEvent::PlaybackStopped);
                        }
                    }
                    Err(e) => {
                        let _ = self
                            .event_tx
                            .send(AppEvent::Error(format!("Queue error: {e}")));
                    }
                }
            }

            // Queue
            AppCommand::AddToQueue(id) => {
                self.db.add_to_queue(id).await.ok();
                self.refresh_queue_display().await;
            }
            AppCommand::RemoveFromQueue(queue_id) => {
                self.db.remove_from_queue(queue_id).await.ok();
                self.refresh_queue_display().await;
            }
            AppCommand::ClearQueue => {
                self.db.clear_queue().await.ok();
                self.refresh_queue_display().await;
            }

            // Bookmarks
            AppCommand::LoadBookmarks {
                podcast_id,
                episode_id,
            } => {
                let db = self.db.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    let episode_bookmarks = db
                        .get_bookmarks_for_episode(episode_id)
                        .await
                        .unwrap_or_default();
                    let podcast_bookmarks = db
                        .get_podcast_bookmarks(podcast_id)
                        .await
                        .unwrap_or_default();
                    let _ = tx.send(AppEvent::BookmarksLoaded {
                        episode_bookmarks,
                        podcast_bookmarks,
                    });
                });
            }
            AppCommand::AddBookmark {
                podcast_id,
                episode_id,
                position_seconds,
                note_text,
            } => {
                let db = self.db.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    let bookmark = crate::db::models::Bookmark {
                        id: 0,
                        podcast_id,
                        episode_id,
                        position_seconds,
                        note_text,
                        created_at: 0,
                        updated_at: 0,
                    };
                    match db.insert_bookmark(bookmark).await {
                        Ok(saved) => {
                            let _ = tx.send(AppEvent::BookmarkAdded(saved));
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::Error(format!("Failed to save note: {e}")));
                        }
                    }
                });
            }
            AppCommand::UpdateBookmark { id, note_text } => {
                let db = self.db.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    match db.update_bookmark(id, note_text.clone()).await {
                        Ok(_) => {
                            let _ =
                                tx.send(AppEvent::BookmarkUpdated(crate::db::models::Bookmark {
                                    id,
                                    podcast_id: 0,
                                    episode_id: None,
                                    position_seconds: None,
                                    note_text,
                                    created_at: 0,
                                    updated_at: 0,
                                }));
                        }
                        Err(e) => {
                            let _ =
                                tx.send(AppEvent::Error(format!("Failed to update note: {e}")));
                        }
                    }
                });
            }
            AppCommand::DeleteBookmark(id) => {
                let db = self.db.clone();
                let tx = self.event_tx.clone();
                tokio::spawn(async move {
                    match db.delete_bookmark(id).await {
                        Ok(_) => {
                            let _ = tx.send(AppEvent::BookmarkDeleted(id));
                        }
                        Err(e) => {
                            let _ =
                                tx.send(AppEvent::Error(format!("Failed to delete note: {e}")));
                        }
                    }
                });
            }

            // OPML
            AppCommand::ImportOpml { path } => {
                let tx = self.event_tx.clone();
                let db = self.db.clone();
                tokio::spawn(async move {
                    import_opml(path, db, tx).await;
                });
            }
            AppCommand::ExportOpml { path } => {
                let tx = self.event_tx.clone();
                let db = self.db.clone();
                tokio::spawn(async move {
                    export_opml(path, db, tx).await;
                });
            }

            // Settings
            AppCommand::SaveSettings(settings) => {
                match self.db.save_settings(settings.clone()).await {
                    Ok(_) => {
                        self.audio_player.set_trim_silence_mode(settings.trim_silence_mode);
                        self.settings = settings;
                        let _ = self.event_tx.send(AppEvent::SettingsSaved);
                    }
                    Err(e) => {
                        let _ = self
                            .event_tx
                            .send(AppEvent::Error(format!("Failed to save settings: {e}")));
                    }
                }
            }

            AppCommand::ApplyHotkeys(_) => {
                // Handled in the UI layer (application.rs); orchestrator ignores this.
            }

            // Sleep Timer
            AppCommand::SetSleepTimer(minutes) => {
                match minutes {
                    Some(n) => {
                        let target = std::time::Instant::now()
                            + std::time::Duration::from_secs(n * 60);
                        self.sleep_timer_target = Some(target);
                        let _ = self.event_tx.send(AppEvent::SleepTimerUpdated(Some(target)));
                    }
                    None => {
                        self.sleep_timer_target = None;
                        let _ = self.event_tx.send(AppEvent::SleepTimerUpdated(None));
                    }
                }
            }

            // Statistics
            AppCommand::LoadListeningStats => {
                match self.db.get_listening_stats().await {
                    Ok(stats) => {
                        let _ = self.event_tx.send(AppEvent::ListeningStatsLoaded(stats));
                    }
                    Err(e) => {
                        let _ = self
                            .event_tx
                            .send(AppEvent::Error(format!("Failed to load stats: {e}")));
                    }
                }
            }
        }
    }

    // Private helpers

    async fn load_all_podcasts(&self) {
        match self.db.get_all_podcasts().await {
            Ok(podcasts) => {
                let _ = self.event_tx.send(AppEvent::PodcastsLoaded(podcasts));
            }
            Err(e) => {
                let _ = self
                    .event_tx
                    .send(AppEvent::Error(format!("Failed to load podcasts: {e}")));
            }
        }
    }

    async fn load_podcast_detail(&self, id: i32) {
        let podcast = match self.db.get_podcast(id).await {
            Ok(Some(p)) => p,
            Ok(None) => {
                let _ = self
                    .event_tx
                    .send(AppEvent::Error(format!("Podcast {id} not found")));
                return;
            }
            Err(e) => {
                let _ = self
                    .event_tx
                    .send(AppEvent::Error(format!("Failed to load podcast: {e}")));
                return;
            }
        };

        match self.db.get_episodes(id).await {
            Ok(episodes) => {
                let _ = self
                    .event_tx
                    .send(AppEvent::PodcastDetailLoaded { podcast, episodes });
            }
            Err(e) => {
                let _ = self
                    .event_tx
                    .send(AppEvent::Error(format!("Failed to load episodes: {e}")));
            }
        }
    }

    async fn play_episode(&mut self, episode_id: i32) {
        let episode = match self.db.get_episode(episode_id).await {
            Ok(Some(e)) => e,
            _ => {
                let _ = self
                    .event_tx
                    .send(AppEvent::Error("Episode not found".into()));
                return;
            }
        };

        let podcast = match self.db.get_podcast(episode.podcast_id).await {
            Ok(Some(p)) => p,
            _ => {
                let _ = self
                    .event_tx
                    .send(AppEvent::Error("Podcast not found".into()));
                return;
            }
        };

        let podcast_id = episode.podcast_id;
        let resume_position = episode.position_seconds;
        let episode_for_event = episode.clone();
        let audio_player = self.audio_player.clone();
        let tx = self.event_tx.clone();

        // Fetch chapters concurrently if this episode has a chapters URL.
        if let Some(chapters_url) = episode.chapters_url.clone() {
            let chapter_tx = tx.clone();
            tokio::spawn(async move {
                if let Ok(chapters) = crate::chapters::fetch_chapters(&chapters_url).await {
                    let _ = chapter_tx.send(AppEvent::ChaptersLoaded(chapters));
                }
            });
        }

        // Resolve playback speed: episode → podcast → global default.
        let speed = episode
            .speed_preset
            .or(podcast.speed_preset)
            .unwrap_or(self.settings.default_speed);
        audio_player.set_speed(speed);

        // Track skip-outro duration for this episode's podcast.
        self.current_skip_outro_seconds = podcast.skip_outro_seconds;

        // Flush any listen time from the previous episode before starting a new one.
        self.flush_listen_session().await;
        self.session_start = Some(std::time::Instant::now());
        self.session_flushed_secs = 0;

        self.last_saved_position = resume_position;
        let should_resume = resume_position > 5.0;

        // Apply intro skip on first play only.
        let intro_skip = if resume_position < 1.0 {
            podcast.skip_intro_seconds
        } else {
            0
        };

        // Tier 1: use tracked downloaded_path from DB.
        if episode.download_status == DownloadStatus::Downloaded
            && let Some(path) = episode.downloaded_path.clone()
            && std::path::Path::new(&path).exists()
        {
            let ep = episode_for_event.clone();
            tokio::task::spawn_blocking(move || {
                match audio_player.play_from_file(&path, episode_id) {
                    Ok(_) => {
                        let seek_to = if intro_skip > 0 && !should_resume {
                            std::time::Duration::from_secs(intro_skip as u64)
                        } else if should_resume {
                            std::time::Duration::from_secs_f64(resume_position)
                        } else {
                            std::time::Duration::ZERO
                        };
                        if seek_to > std::time::Duration::ZERO {
                            audio_player.seek(seek_to);
                            if should_resume {
                                let mins = (resume_position as u64) / 60;
                                let secs = (resume_position as u64) % 60;
                                let _ = tx.send(AppEvent::Toast(ToastMessage::info(
                                    &format!("Resuming from {:02}:{:02}", mins, secs),
                                )));
                            }
                        }
                        let _ = tx.send(AppEvent::PlaybackStarted {
                            episode_id,
                            podcast_id,
                            episode: ep,
                        });
                    }
                    Err(e) => {
                        let _ =
                            tx.send(AppEvent::Error(format!("Playback failed: {e}")));
                    }
                }
            });
            return;
        }

        // Tier 2: legacy file scan (backwards compat).
        let downloaded_path = self
            .download_manager
            .find_file(vec![podcast.title.clone()], &episode.title);

        if let Some(path) = downloaded_path {
            let path_str = path.to_string_lossy().to_string();
            let ep = episode_for_event.clone();
            tokio::task::spawn_blocking(move || {
                match audio_player.play_from_file(&path_str, episode_id) {
                    Ok(_) => {
                        let seek_to = if intro_skip > 0 && !should_resume {
                            std::time::Duration::from_secs(intro_skip as u64)
                        } else if should_resume {
                            std::time::Duration::from_secs_f64(resume_position)
                        } else {
                            std::time::Duration::ZERO
                        };
                        if seek_to > std::time::Duration::ZERO {
                            audio_player.seek(seek_to);
                            if should_resume {
                                let mins = (resume_position as u64) / 60;
                                let secs = (resume_position as u64) % 60;
                                let _ = tx.send(AppEvent::Toast(ToastMessage::info(&format!(
                                    "Resuming from {:02}:{:02}",
                                    mins, secs
                                ))));
                            }
                        }
                        let _ = tx.send(AppEvent::PlaybackStarted {
                            episode_id,
                            podcast_id,
                            episode: ep,
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::Error(format!("Playback failed: {e}")));
                    }
                }
            });
            return;
        }

        // Tier 3: in-memory audio cache.
        if let Some(bytes_data) = self.audio_cache.get(episode_id) {
            let ep = episode_for_event.clone();
            tokio::task::spawn_blocking(move || {
                match audio_player.play_from_memory(bytes_data, episode_id) {
                    Ok(_) => {
                        let seek_to = if intro_skip > 0 && !should_resume {
                            std::time::Duration::from_secs(intro_skip as u64)
                        } else if should_resume {
                            std::time::Duration::from_secs_f64(resume_position)
                        } else {
                            std::time::Duration::ZERO
                        };
                        if seek_to > std::time::Duration::ZERO {
                            audio_player.seek(seek_to);
                            if should_resume {
                                let mins = (resume_position as u64) / 60;
                                let secs = (resume_position as u64) % 60;
                                let _ = tx.send(AppEvent::Toast(ToastMessage::info(&format!(
                                    "Resuming from {:02}:{:02}",
                                    mins, secs
                                ))));
                            }
                        }
                        let _ = tx.send(AppEvent::PlaybackStarted {
                            episode_id,
                            podcast_id,
                            episode: ep,
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::Error(format!("Playback failed: {e}")));
                    }
                }
            });
            return;
        }

        // Tier 4: fetch from network.
        let url = episode.url.clone();
        let _ = tx.send(AppEvent::Toast(ToastMessage::info("Buffering...")));

        let fetch_tx = tx.clone();
        let ep = episode_for_event.clone();
        match tokio::task::spawn_blocking(move || {
            reqwest::blocking::get(&url)
                .and_then(|r| r.bytes())
                .map_err(|e| e.to_string())
        })
        .await
        {
            Ok(Ok(raw)) => {
                let bytes_data = raw;
                self.audio_cache.insert(episode_id, bytes_data.clone());
                let audio_player2 = audio_player.clone();
                tokio::task::spawn_blocking(move || {
                    match audio_player2.play_from_memory(bytes_data, episode_id) {
                        Ok(_) => {
                            let seek_to = if intro_skip > 0 && !should_resume {
                                std::time::Duration::from_secs(intro_skip as u64)
                            } else if should_resume {
                                std::time::Duration::from_secs_f64(resume_position)
                            } else {
                                std::time::Duration::ZERO
                            };
                            if seek_to > std::time::Duration::ZERO {
                                audio_player2.seek(seek_to);
                                if should_resume {
                                    let mins = (resume_position as u64) / 60;
                                    let secs = (resume_position as u64) % 60;
                                    let _ = fetch_tx.send(AppEvent::Toast(ToastMessage::info(
                                        &format!("Resuming from {:02}:{:02}", mins, secs),
                                    )));
                                }
                            }
                            let _ = fetch_tx.send(AppEvent::PlaybackStarted {
                                episode_id,
                                podcast_id,
                                episode: ep,
                            });
                        }
                        Err(e) => {
                            let _ =
                                fetch_tx.send(AppEvent::Error(format!("Playback failed: {e}")));
                        }
                    }
                });
            }
            Ok(Err(e)) => {
                let _ = fetch_tx.send(AppEvent::Error(format!("Fetch failed: {e}")));
            }
            Err(e) => {
                let _ = fetch_tx.send(AppEvent::Error(format!("Task failed: {e}")));
            }
        }
    }

    async fn refresh_queue_display(&self) {
        match self.db.get_queue_with_details().await {
            Ok(items) => {
                let _ = self.event_tx.send(AppEvent::QueueUpdated(items));
            }
            Err(e) => {
                let _ = self
                    .event_tx
                    .send(AppEvent::Error(format!("Failed to load queue: {e}")));
            }
        }
    }

}

// -- Standalone async helpers ---------------------------------------------------

async fn enforce_retention_policy(
    podcast_id: i32,
    db: &Database,
    dm: &DownloadManager,
    settings: &Settings,
) {
    let podcast = match db.get_podcast(podcast_id).await {
        Ok(Some(p)) => p,
        _ => return,
    };

    let effective_keep = podcast
        .keep_episodes_count
        .unwrap_or(settings.global_keep_episodes_count);

    if effective_keep <= 0 {
        return; // 0 = keep all
    }

    let downloaded = match db.get_downloaded_episodes(podcast_id).await {
        Ok(eps) => eps,
        Err(_) => return,
    };

    for ep in downloaded.into_iter().skip(effective_keep as usize) {
        if let Some(path) = &ep.downloaded_path {
            dm.delete_file(path).ok();
        }
        db.update_episode_download_status(ep.id, DownloadStatus::NotDownloaded, None)
            .await
            .ok();
    }
}

fn send_notification(title: &str, body: &str) {
    let title = title.to_owned();
    let body = body.to_owned();
    std::thread::spawn(move || {
        notify_rust::Notification::new()
            .summary(&title)
            .body(&body)
            .show()
            .ok();
    });
}

// -- Standalone async task functions --------------------------------------------

async fn add_podcast(feed_url: String, db: Database, tx: UnboundedSender<AppEvent>) {
    let _ = tx.send(AppEvent::Toast(ToastMessage::info("Fetching feed...")));

    match fetch_feed(&feed_url).await {
        Ok((title, description, image_url, episodes)) => {
            let now = chrono::Utc::now().timestamp();
            let podcast = Podcast {
                id: 0,
                url: feed_url,
                title,
                description,
                image_url,
                episode_count: 0,
                last_synced_at: now,
                created_at: now,
                updated_at: now,
                speed_preset: None,
                auto_download: None,
                keep_episodes_count: None,
                skip_intro_seconds: 0,
                skip_outro_seconds: 0,
            };

            match db.insert_podcast(podcast).await {
                Ok(saved) => {
                    let episodes_with_id: Vec<Episode> = episodes
                        .into_iter()
                        .map(|mut e| {
                            e.podcast_id = saved.id;
                            e
                        })
                        .collect();
                    let _ = db.insert_episodes(episodes_with_id).await;
                    let _ = tx.send(AppEvent::PodcastAdded(saved));
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::Error(format!("Failed to save podcast: {e}")));
                }
            }
        }
        Err(e) => {
            let _ = tx.send(AppEvent::Toast(ToastMessage::error(&format!(
                "Could not fetch feed: {e}"
            ))));
        }
    }
}

async fn sync_podcast(
    podcast_id: i32,
    db: Database,
    tx: UnboundedSender<AppEvent>,
    settings: &Settings,
) {
    let podcast = match db.get_podcast(podcast_id).await {
        Ok(Some(p)) => p,
        _ => return,
    };

    let _ = tx.send(AppEvent::SyncStarted(podcast_id));

    match fetch_feed(&podcast.url).await {
        Ok((_, _, _, episodes)) => {
            let now = chrono::Utc::now().timestamp();
            let episodes_with_id: Vec<Episode> = episodes
                .into_iter()
                .map(|mut e| {
                    e.podcast_id = podcast_id;
                    e.created_at = now;
                    e.updated_at = now;
                    e
                })
                .collect();

            let new_ids = db.insert_episodes(episodes_with_id).await.unwrap_or_default();

            if !new_ids.is_empty() && settings.notify_new_episodes {
                let count = new_ids.len();
                let title = podcast.title.clone();
                send_notification(
                    &title,
                    &format!("{} new episode{}", count, if count == 1 { "" } else { "s" }),
                );
            }

            // Auto-download new episodes if configured.
            let effective_auto = podcast.auto_download.unwrap_or(settings.auto_download_new_episodes);
            if effective_auto && !new_ids.is_empty() {
                for episode_id in new_ids {
                    let _ = tx.send(AppEvent::Toast(ToastMessage::info("Auto-downloading new episode...")));
                    // We fire off a DownloadEpisode-equivalent inline rather than going through the
                    // command channel (which we don't have access to here).  The download path is
                    // the same — fetch the episode and call the download manager.
                    let db2 = db.clone();
                    let tx2 = tx.clone();
                    let settings2 = settings.clone();
                    tokio::spawn(async move {
                        if let Ok(Some(ep)) = db2.get_episode(episode_id).await
                            && let Ok(Some(pod)) = db2.get_podcast(ep.podcast_id).await
                        {
                            let dm2 = crate::download_manager::DownloadManager::new(db2.clone());
                            let folders = vec![pod.title.clone()];
                            let file_name = ep.title.clone();
                            let ep_title = ep.title.clone();
                            let podcast_id2 = pod.id;
                            let _ = db2.update_episode_download_status(episode_id, DownloadStatus::Downloading, None).await;
                            let _ = tx2.send(AppEvent::DownloadStatusChanged { episode_id, status: DownloadStatus::Downloading, path: None });
                            tokio::task::spawn_blocking(move || {
                                match dm2.download(ep.url, folders, file_name) {
                                    Ok(path) => {
                                        let path_str = path.to_string_lossy().to_string();
                                        let rt = tokio::runtime::Handle::current();
                                        rt.block_on(async {
                                            let _ = db2.update_episode_download_status(episode_id, DownloadStatus::Downloaded, Some(path_str.clone())).await;
                                            enforce_retention_policy(podcast_id2, &db2, &dm2, &settings2).await;
                                        });
                                        if settings2.notify_download_complete {
                                            send_notification("Download complete", &ep_title);
                                        }
                                        let _ = tx2.send(AppEvent::DownloadStatusChanged { episode_id, status: DownloadStatus::Downloaded, path: Some(path_str) });
                                    }
                                    Err(_) => {
                                        let rt = tokio::runtime::Handle::current();
                                        rt.block_on(async {
                                            let _ = db2.update_episode_download_status(episode_id, DownloadStatus::Failed, None).await;
                                        });
                                        let _ = tx2.send(AppEvent::DownloadStatusChanged { episode_id, status: DownloadStatus::Failed, path: None });
                                    }
                                }
                            }).await.ok();
                        }
                    });
                }
            }

            let _ = db.update_podcast_synced_at(podcast_id).await;

            match db.get_episodes(podcast_id).await {
                Ok(episodes) => {
                    let _ = tx.send(AppEvent::EpisodesUpdated {
                        podcast_id,
                        episodes,
                    });
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::Error(format!("Sync DB error: {e}")));
                }
            }
        }
        Err(e) => {
            let _ = tx.send(AppEvent::Toast(ToastMessage::error(&format!(
                "Sync failed for '{}': {e}",
                podcast.title
            ))));
        }
    }

    let _ = tx.send(AppEvent::SyncCompleted(podcast_id));
}

async fn background_sync(db: Database, tx: UnboundedSender<AppEvent>) {
    let settings = db.get_settings().await.unwrap_or_default();
    background_sync_with_settings(db, tx, settings).await;
}

async fn background_sync_with_settings(
    db: Database,
    tx: UnboundedSender<AppEvent>,
    settings: Settings,
) {
    let podcasts = match db.get_all_podcasts().await {
        Ok(p) => p,
        Err(_) => return,
    };
    for podcast in podcasts {
        let db = db.clone();
        let tx = tx.clone();
        let settings = settings.clone();
        tokio::spawn(async move {
            sync_podcast(podcast.id, db, tx, &settings).await;
        });
    }
}

// Parses an RSS feed URL and returns (title, description, image_url, episodes).
async fn fetch_feed(url: &str) -> anyhow::Result<(String, String, String, Vec<Episode>)> {
    let body = reqwest::get(url).await?.text().await?;
    let channel = rss::Channel::read_from(body.as_bytes())?;

    let title = channel.title().to_string();
    let description = channel.description().to_string();
    let image_url = channel
        .image()
        .map(|i| i.url().to_string())
        .or_else(|| {
            channel
                .itunes_ext()
                .and_then(|e| e.image())
                .map(|u| u.to_string())
        })
        .unwrap_or_default();

    let now = chrono::Utc::now().timestamp();

    let episodes = channel
        .items()
        .iter()
        .filter_map(|item| {
            let enclosure = item.enclosure()?;

            let duration = item
                .itunes_ext()
                .and_then(|ext| ext.duration())
                .and_then(|d| {
                    let parts: Vec<&str> = d.split(':').collect();
                    match parts.len() {
                        3 => {
                            let h: i64 = parts[0].parse().ok()?;
                            let m: i64 = parts[1].parse().ok()?;
                            let s: i64 = parts[2].parse().ok()?;
                            Some(h * 3600 + m * 60 + s)
                        }
                        2 => {
                            let m: i64 = parts[0].parse().ok()?;
                            let s: i64 = parts[1].parse().ok()?;
                            Some(m * 60 + s)
                        }
                        1 => parts[0].parse().ok(),
                        _ => None,
                    }
                })
                .unwrap_or(0);

            let chapters_url = item
                .extensions()
                .get("podcast")
                .and_then(|ns| ns.get("chapters"))
                .and_then(|v| v.first())
                .and_then(|ext| ext.attrs().get("url"))
                .cloned();

            Some(Episode {
                id: 0,
                podcast_id: 0,
                title: item.title().unwrap_or("Untitled").to_string(),
                description: item.description().unwrap_or("").to_string(),
                url: enclosure.url().to_string(),
                audio_type: enclosure.mime_type().to_string(),
                publish_date: item
                    .pub_date()
                    .and_then(|d| chrono::DateTime::parse_from_rfc2822(d).ok())
                    .map(|d| d.timestamp())
                    .unwrap_or(0),
                is_played: false,
                duration,
                position_seconds: 0.0,
                created_at: now,
                updated_at: now,
                download_status: DownloadStatus::NotDownloaded,
                downloaded_path: None,
                speed_preset: None,
                chapters_url,
                total_listen_seconds: 0,
            })
        })
        .collect();

    Ok((title, description, image_url, episodes))
}

// OPML import
async fn import_opml(path: std::path::PathBuf, db: Database, tx: UnboundedSender<AppEvent>) {
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            let _ = tx.send(AppEvent::Error(format!("Could not read OPML file: {e}")));
            return;
        }
    };

    let feed_urls = parse_opml_feed_urls(&raw);

    if feed_urls.is_empty() {
        let _ = tx.send(AppEvent::Error(
            "No podcast feeds found in OPML file. Make sure it contains xmlUrl attributes.".into(),
        ));
        return;
    }

    let existing: std::collections::HashSet<String> = match db.get_all_podcasts().await {
        Ok(podcasts) => podcasts.into_iter().map(|p| p.url).collect(),
        Err(_) => std::collections::HashSet::new(),
    };

    let _ = tx.send(AppEvent::Toast(ToastMessage::info(&format!(
        "Importing {} podcast{}...",
        feed_urls.len(),
        if feed_urls.len() == 1 { "" } else { "s" }
    ))));

    let mut added = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;

    for url in feed_urls {
        if existing.contains(&url) {
            skipped += 1;
            continue;
        }

        let db2 = db.clone();
        let tx2 = tx.clone();
        match fetch_feed(&url).await {
            Ok((title, description, image_url, episodes)) => {
                let now = chrono::Utc::now().timestamp();
                let podcast = Podcast {
                    id: 0,
                    url: url.clone(),
                    title,
                    description,
                    image_url,
                    episode_count: 0,
                    last_synced_at: now,
                    created_at: now,
                    updated_at: now,
                    speed_preset: None,
                    auto_download: None,
                    keep_episodes_count: None,
                    skip_intro_seconds: 0,
                    skip_outro_seconds: 0,
                };

                match db2.insert_podcast(podcast).await {
                    Ok(saved) => {
                        let eps: Vec<Episode> = episodes
                            .into_iter()
                            .map(|mut e| {
                                e.podcast_id = saved.id;
                                e
                            })
                            .collect();
                        let _ = db2.insert_episodes(eps).await;
                        let _ = tx2.send(AppEvent::PodcastAdded(saved));
                        added += 1;
                    }
                    Err(_) => {
                        failed += 1;
                    }
                }
            }
            Err(_) => {
                failed += 1;
            }
        }
    }

    let _ = tx.send(AppEvent::OpmlImported {
        added,
        skipped,
        failed,
    });
}

fn parse_opml_feed_urls(opml: &str) -> Vec<String> {
    use quick_xml::Reader;
    use quick_xml::events::Event;

    let mut reader = Reader::from_str(opml);
    reader.config_mut().trim_text(true);

    let mut urls = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) => {
                let name = e.name();
                let tag = std::str::from_utf8(name.as_ref())
                    .unwrap_or("")
                    .to_lowercase();

                if tag == "outline" {
                    let mut xml_url: Option<String> = None;

                    for attr in e.attributes().flatten() {
                        let key = std::str::from_utf8(attr.key.as_ref())
                            .unwrap_or("")
                            .to_lowercase();
                        if key == "xmlurl"
                            && let Ok(val) = attr.decoded_and_normalized_value(quick_xml::XmlVersion::Implicit1_0, reader.decoder())
                        {
                            let url: String = val.trim().to_string();
                            if !url.is_empty() {
                                xml_url = Some(url);
                            }
                        }
                    }

                    if let Some(url) = xml_url {
                        urls.push(url);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    urls
}

// OPML export
async fn export_opml(path: std::path::PathBuf, db: Database, tx: UnboundedSender<AppEvent>) {
    let podcasts = match db.get_all_podcasts().await {
        Ok(p) => p,
        Err(e) => {
            let _ = tx.send(AppEvent::Error(format!(
                "Could not load podcasts for export: {e}"
            )));
            return;
        }
    };

    if podcasts.is_empty() {
        let _ = tx.send(AppEvent::Error("No podcasts to export.".into()));
        return;
    }

    let opml = build_opml(&podcasts);

    match std::fs::write(&path, &opml) {
        Ok(_) => {
            let path_str = path.to_string_lossy().to_string();
            let _ = tx.send(AppEvent::OpmlExported { path: path_str });
        }
        Err(e) => {
            let _ = tx.send(AppEvent::Error(format!("Failed to write OPML file: {e}")));
        }
    }
}

fn build_opml(podcasts: &[Podcast]) -> String {
    let mut lines = vec![
        r#"<?xml version="1.0" encoding="utf-8"?>"#.to_string(),
        r#"<opml version="2.0">"#.to_string(),
        r#"  <head>"#.to_string(),
        r#"    <title>RCast Subscriptions</title>"#.to_string(),
        format!(
            "    <dateCreated>{}</dateCreated>",
            chrono::Utc::now().to_rfc2822()
        ),
        r#"  </head>"#.to_string(),
        r#"  <body>"#.to_string(),
    ];

    for podcast in podcasts {
        let title = escape_xml(&podcast.title);
        let xml_url = escape_xml(&podcast.url);
        lines.push(format!(
            r#"    <outline type="rss" text="{title}" xmlUrl="{xml_url}"/>"#
        ));
    }

    lines.push(r#"  </body>"#.to_string());
    lines.push(r#"</opml>"#.to_string());

    lines.join("\n")
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\'', "&apos;")
}
