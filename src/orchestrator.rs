use std::collections::HashSet;

use bytes;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::audio_cache::AudioCache;
use crate::audio_player::AudioPlayer;
use crate::commands::AppCommand;
use crate::components::toast::ToastMessage;
use crate::db::Database;
use crate::db::models::{Episode, Podcast};
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
    /// Tracks which podcast detail page is open so TogglePlayed can refresh it.
    current_detail_podcast_id: Option<i32>,
    /// Prevents duplicate autoplay triggers for the same finished episode.
    last_finished_episode_id: Option<i32>,
    /// Last position (seconds) written to the DB — avoids thrashing on every tick.
    last_saved_position: f64,
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
            audio_cache: AudioCache::new(10),
            download_manager,
            current_detail_podcast_id: None,
            last_finished_episode_id: None,
            last_saved_position: 0.0,
        }
    }

    pub async fn run(mut self) {
        // Load settings and kick off a background sync on startup.
        let settings = self.db.get_settings().await.unwrap_or_default();
        let _ = self
            .event_tx
            .send(AppEvent::SettingsLoaded(settings.clone()));

        // Initial podcast load (home page).
        self.load_all_podcasts().await;

        // Periodic background sync task.
        {
            let tx = self.event_tx.clone();
            let db = self.db.clone();
            let interval_secs = (settings.sync_interval_minutes * 60) as u64;
            tokio::spawn(async move {
                let mut ticker =
                    tokio::time::interval(std::time::Duration::from_secs(interval_secs));
                ticker.tick().await; // skip the immediate tick
                loop {
                    ticker.tick().await;
                    background_sync(db.clone(), tx.clone()).await;
                }
            });
        }

        // Initial one-off sync.
        {
            let tx = self.event_tx.clone();
            let db = self.db.clone();
            tokio::spawn(async move {
                background_sync(db, tx).await;
            });
        }

        let mut save_ticker = tokio::time::interval(std::time::Duration::from_secs(10));
        save_ticker.tick().await; // skip the immediate first tick

        loop {
            tokio::select! {
                Some(cmd) = self.cmd_rx.recv() => {
                    self.handle(cmd).await;
                }
                _ = save_ticker.tick() => {
                    self.auto_save_position().await;
                }
            }
        }
    }

    /// Writes the current playback position to the DB if audio is playing and
    /// the position has moved more than 5 seconds since the last save.
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
    }

    async fn handle(&mut self, cmd: AppCommand) {
        match cmd {
            // ── Navigation ────────────────────────────────────────────────────
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
                        }
                        Err(e) => {
                            let _ = self
                                .event_tx
                                .send(AppEvent::Error(format!("Failed to load settings: {e}")));
                        }
                    },
                }
            }

            // ── Podcasts ──────────────────────────────────────────────────────
            AppCommand::AddPodcast { feed_url } => {
                let tx = self.event_tx.clone();
                let db = self.db.clone();
                tokio::spawn(async move {
                    add_podcast(feed_url, db, tx).await;
                });
            }
            AppCommand::RemovePodcast(id) => match self.db.delete_podcast(id).await {
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
            },
            AppCommand::SyncPodcast(id) => {
                let tx = self.event_tx.clone();
                let db = self.db.clone();
                tokio::spawn(async move {
                    sync_podcast(id, db, tx).await;
                });
            }
            AppCommand::SyncAll => {
                let tx = self.event_tx.clone();
                let db = self.db.clone();
                tokio::spawn(async move {
                    background_sync(db, tx).await;
                });
            }

            // ── Playback ──────────────────────────────────────────────────────
            AppCommand::PlayEpisode(episode_id) => {
                self.play_episode(episode_id).await;
            }
            AppCommand::PlayAll(episode_ids) => {
                if episode_ids.is_empty() {
                    return;
                }
                // Play first, queue the rest.
                self.play_episode(episode_ids[0]).await;
                for &id in &episode_ids[1..] {
                    self.db.add_to_queue(id).await.ok();
                }
                self.refresh_queue_display().await;
            }
            AppCommand::PausePlayback => {
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
                self.audio_player.resume();
            }
            AppCommand::PlayNextInQueue => {
                // Mark the just-finished episode as complete and reset its position.
                if let Some(episode_id) = self.audio_player.get_current_episode_id() {
                    self.db.complete_episode(episode_id).await.ok();
                    self.last_saved_position = 0.0;
                }
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

            // ── Queue ─────────────────────────────────────────────────────────
            AppCommand::AddToQueue(id) => {
                self.db.add_to_queue(id).await.ok();
                self.refresh_queue_display().await;
            }
            AppCommand::RemoveFromQueue(queue_id) => {
                self.db.remove_from_queue(queue_id).await.ok();
                self.refresh_queue_display().await;
            }

            // ── Episodes ──────────────────────────────────────────────────────
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

                let download_manager = self.download_manager.clone();
                let tx = self.event_tx.clone();

                tokio::task::spawn_blocking(move || {
                    let folders = vec![podcast.title.clone()];
                    let file_name = episode.title.clone();

                    if !download_manager.file_exists(folders.clone(), &file_name) {
                        if let Err(e) = download_manager.download(episode.url, folders, file_name) {
                            let _ = tx.send(AppEvent::Error(format!("Download failed: {e}")));
                        } else {
                            let _ = tx
                                .send(AppEvent::Toast(ToastMessage::success("Download complete")));
                        }
                    }
                });
            }
            AppCommand::TogglePlayed(episode_id) => {
                match self.db.get_episode(episode_id).await {
                    Ok(Some(ep)) => {
                        self.db
                            .update_episode_played(episode_id, !ep.is_played)
                            .await
                            .ok();

                        // Refresh episodes if detail page is open for this podcast.
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
                    _ => {}
                }
            }

            // ── Settings ─────────────────────────────────────────────────────
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
            AppCommand::SaveSettings(settings) => match self.db.save_settings(settings).await {
                Ok(_) => {
                    let _ = self.event_tx.send(AppEvent::SettingsSaved);
                    let _ = self
                        .event_tx
                        .send(AppEvent::Toast(ToastMessage::success("Settings saved")));
                }
                Err(e) => {
                    let _ = self
                        .event_tx
                        .send(AppEvent::Error(format!("Failed to save settings: {e}")));
                }
            },
        }
    }

    // ── Private helpers ───────────────────────────────────────────────────────

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
        let audio_player = self.audio_player.clone();
        let tx = self.event_tx.clone();

        // Reset the saved-position tracker for this new episode.
        self.last_saved_position = resume_position;

        // Helper: if there's a saved position > 5s, seek and notify the user.
        let should_resume = resume_position > 5.0;

        // ── Tier 1: user-downloaded file ──────────────────────────────────────
        let downloaded_path = self
            .download_manager
            .find_file(vec![podcast.title.clone()], &episode.title);

        if let Some(path) = downloaded_path {
            let path_str = path.to_string_lossy().to_string();
            tokio::task::spawn_blocking(move || {
                match audio_player.play_from_file(&path_str, episode_id) {
                    Ok(_) => {
                        if should_resume {
                            audio_player.seek(std::time::Duration::from_secs_f64(resume_position));
                            let mins = (resume_position as u64) / 60;
                            let secs = (resume_position as u64) % 60;
                            let _ = tx.send(AppEvent::Toast(ToastMessage::info(&format!(
                                "Resuming from {:02}:{:02}",
                                mins, secs
                            ))));
                        }
                        let _ = tx.send(AppEvent::PlaybackStarted {
                            episode_id,
                            podcast_id,
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::Error(format!("Playback failed: {e}")));
                    }
                }
            });
            return;
        }

        // ── Tier 2: in-memory audio cache ─────────────────────────────────────
        if let Some(bytes) = self.audio_cache.get(episode_id) {
            tokio::task::spawn_blocking(move || {
                match audio_player.play_from_memory(bytes, episode_id) {
                    Ok(_) => {
                        if should_resume {
                            audio_player.seek(std::time::Duration::from_secs_f64(resume_position));
                            let mins = (resume_position as u64) / 60;
                            let secs = (resume_position as u64) % 60;
                            let _ = tx.send(AppEvent::Toast(ToastMessage::info(&format!(
                                "Resuming from {:02}:{:02}",
                                mins, secs
                            ))));
                        }
                        let _ = tx.send(AppEvent::PlaybackStarted {
                            episode_id,
                            podcast_id,
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::Error(format!("Playback failed: {e}")));
                    }
                }
            });
            return;
        }

        // ── Tier 3: fetch from network, cache, then play ──────────────────────
        let url = episode.url.clone();
        let _ = tx.send(AppEvent::Toast(ToastMessage::info("Buffering...")));

        let fetch_tx = tx.clone();
        match tokio::task::spawn_blocking(move || {
            reqwest::blocking::get(&url)
                .and_then(|r| r.bytes())
                .map_err(|e| e.to_string())
        })
        .await
        {
            Ok(Ok(raw)) => {
                let bytes = bytes::Bytes::from(raw);
                self.audio_cache.insert(episode_id, bytes.clone());
                tokio::task::spawn_blocking(move || {
                    match audio_player.play_from_memory(bytes, episode_id) {
                        Ok(_) => {
                            if should_resume {
                                audio_player
                                    .seek(std::time::Duration::from_secs_f64(resume_position));
                                let mins = (resume_position as u64) / 60;
                                let secs = (resume_position as u64) % 60;
                                let _ = fetch_tx.send(AppEvent::Toast(ToastMessage::info(
                                    &format!("Resuming from {:02}:{:02}", mins, secs),
                                )));
                            }
                            let _ = fetch_tx.send(AppEvent::PlaybackStarted {
                                episode_id,
                                podcast_id,
                            });
                        }
                        Err(e) => {
                            let _ = fetch_tx.send(AppEvent::Error(format!("Playback failed: {e}")));
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

// ── Standalone async task functions ───────────────────────────────────────────

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

async fn sync_podcast(podcast_id: i32, db: Database, tx: UnboundedSender<AppEvent>) {
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

            let _ = db.insert_episodes(episodes_with_id).await;
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
    let podcasts = match db.get_all_podcasts().await {
        Ok(p) => p,
        Err(_) => return,
    };
    for podcast in podcasts {
        let db = db.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            sync_podcast(podcast.id, db, tx).await;
        });
    }
}

/// Parses an RSS feed URL and returns (title, description, image_url, episodes).
async fn fetch_feed(url: &str) -> anyhow::Result<(String, String, String, Vec<Episode>)> {
    let body = reqwest::get(url).await?.text().await?;
    let channel = rss::Channel::read_from(body.as_bytes())?;

    let title = channel.title().to_string();
    let description = channel.description().to_string();
    let image_url = channel
        .image()
        .map(|i| i.url().to_string())
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
            })
        })
        .collect();

    Ok((title, description, image_url, episodes))
}

// ── OPML import ───────────────────────────────────────────────────────────────

/// Reads an OPML file, extracts every `xmlUrl` attribute, and subscribes to
/// each feed that isn't already in the database.
///
/// Reports `added` / `skipped` (already subscribed) / `failed` (fetch error)
/// counts via `AppEvent::OpmlImported`.
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

    // Load existing feed URLs so we can skip duplicates without hitting the network.
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
                let podcast = crate::db::models::Podcast {
                    id: 0,
                    url: url.clone(),
                    title,
                    description,
                    image_url,
                    episode_count: 0,
                    last_synced_at: now,
                    created_at: now,
                    updated_at: now,
                };

                match db2.insert_podcast(podcast).await {
                    Ok(saved) => {
                        let eps: Vec<crate::db::models::Episode> = episodes
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

/// Extracts all `xmlUrl` attribute values from an OPML document.
/// Permissive — does not require `type="rss"`, just looks for `xmlUrl`.
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
                        if key == "xmlurl" {
                            if let Ok(val) = attr.decode_and_unescape_value(reader.decoder()) {
                                let url: String = val.trim().to_string();
                                if !url.is_empty() {
                                    xml_url = Some(url);
                                }
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

// ── OPML export ───────────────────────────────────────────────────────────────

/// Reads all podcasts from the DB and writes a valid OPML 2.0 file to `path`.
/// Uses `xmlUrl` only — no `htmlUrl` since we store RSS feeds, not website URLs.
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

/// Generates a well-formed OPML 2.0 document from a list of podcasts.
fn build_opml(podcasts: &[crate::db::models::Podcast]) -> String {
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
        // Escape XML special characters in title and URL.
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

/// Escapes the five XML special characters for use in attribute values.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\'', "&apos;")
}
