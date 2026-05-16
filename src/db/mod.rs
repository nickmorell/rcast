pub mod models;

use std::fs::create_dir_all;
use std::sync::{Arc, Mutex};

use anyhow::anyhow;
use dirs::data_local_dir;
use rusqlite::{Connection, params};

use crate::errors::DatabaseError;
use crate::migrations::run_migrations;
use crate::types::{HomeDensity, PodcastPreferences, QueueDisplayItem, QueueItem, Settings, TrimSilenceMode};
use models::{Bookmark, DownloadStatus, Episode, Podcast};

#[derive(Clone)]
pub struct Database {
    connection: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(name: &str) -> anyhow::Result<Self> {
        let db_dir = data_local_dir()
            .ok_or_else(|| anyhow!("Cannot determine data directory"))?
            .join(name);

        create_dir_all(&db_dir)?;

        let db_path = db_dir.join("rcast.db");
        let mut connection = Connection::open(&db_path)?;
        connection.execute("PRAGMA foreign_keys = ON", [])?;
        run_migrations(&mut connection).map_err(|e| anyhow!("Migration failed: {e}"))?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    // Podcasts

    pub async fn get_all_podcasts(&self) -> anyhow::Result<Vec<Podcast>> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            let mut stmt = conn.prepare(
                "SELECT p.id, p.url, p.title, p.description, p.image_url,
                        p.last_synced_at, p.created_at, p.updated_at,
                        COUNT(e.id) as episode_count,
                        p.speed_preset, p.auto_download, p.keep_episodes_count,
                        p.skip_intro_seconds, p.skip_outro_seconds
                 FROM podcasts p
                 LEFT JOIN episodes e ON e.podcast_id = p.id
                 GROUP BY p.id
                 ORDER BY p.title",
            )?;

            let podcasts = stmt
                .query_map([], |row| {
                    Ok(Podcast {
                        id: row.get(0)?,
                        url: row.get(1)?,
                        title: row.get(2)?,
                        description: row.get(3)?,
                        image_url: row.get(4)?,
                        last_synced_at: row.get(5)?,
                        created_at: row.get(6)?,
                        updated_at: row.get(7)?,
                        episode_count: row.get(8)?,
                        speed_preset: row.get(9)?,
                        auto_download: row.get::<_, Option<i32>>(10)?.map(|v| v != 0),
                        keep_episodes_count: row.get(11)?,
                        skip_intro_seconds: row.get::<_, Option<i32>>(12)?.unwrap_or(0),
                        skip_outro_seconds: row.get::<_, Option<i32>>(13)?.unwrap_or(0),
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(podcasts)
        })
        .await?
    }

    pub async fn get_podcast(&self, id: i32) -> anyhow::Result<Option<Podcast>> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            let mut stmt = conn.prepare(
                "SELECT p.id, p.url, p.title, p.description, p.image_url,
                        p.last_synced_at, p.created_at, p.updated_at,
                        COUNT(e.id) as episode_count,
                        p.speed_preset, p.auto_download, p.keep_episodes_count,
                        p.skip_intro_seconds, p.skip_outro_seconds
                 FROM podcasts p
                 LEFT JOIN episodes e ON e.podcast_id = p.id
                 WHERE p.id = ?
                 GROUP BY p.id",
            )?;

            let mut rows = stmt.query_map([id], |row| {
                Ok(Podcast {
                    id: row.get(0)?,
                    url: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    image_url: row.get(4)?,
                    last_synced_at: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                    episode_count: row.get(8)?,
                    speed_preset: row.get(9)?,
                    auto_download: row.get::<_, Option<i32>>(10)?.map(|v| v != 0),
                    keep_episodes_count: row.get(11)?,
                    skip_intro_seconds: row.get::<_, Option<i32>>(12)?.unwrap_or(0),
                    skip_outro_seconds: row.get::<_, Option<i32>>(13)?.unwrap_or(0),
                })
            })?;

            Ok(rows.next().transpose()?)
        })
        .await?
    }

    pub async fn insert_podcast(&self, podcast: Podcast) -> anyhow::Result<Podcast> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            conn.execute(
                "INSERT INTO podcasts
                    (url, title, description, image_url, last_synced_at, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    podcast.url,
                    podcast.title,
                    podcast.description,
                    podcast.image_url,
                    podcast.last_synced_at,
                    podcast.created_at,
                    podcast.updated_at,
                ],
            )?;

            Ok(Podcast {
                id: conn.last_insert_rowid() as i32,
                episode_count: 0,
                ..podcast
            })
        })
        .await?
    }

    pub async fn update_podcast_synced_at(&self, podcast_id: i32) -> anyhow::Result<()> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            let now = chrono::Utc::now().timestamp();
            conn.execute(
                "UPDATE podcasts SET last_synced_at = ?1, updated_at = ?1 WHERE id = ?2",
                params![now, podcast_id],
            )?;
            Ok(())
        })
        .await?
    }

    pub async fn update_podcast_preferences(
        &self,
        podcast_id: i32,
        prefs: PodcastPreferences,
    ) -> anyhow::Result<()> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            let now = chrono::Utc::now().timestamp();
            conn.execute(
                "UPDATE podcasts SET
                    speed_preset = ?1,
                    auto_download = ?2,
                    keep_episodes_count = ?3,
                    skip_intro_seconds = ?4,
                    skip_outro_seconds = ?5,
                    updated_at = ?6
                 WHERE id = ?7",
                params![
                    prefs.speed_preset,
                    prefs.auto_download.map(|b| b as i32),
                    prefs.keep_episodes_count,
                    prefs.skip_intro_seconds,
                    prefs.skip_outro_seconds,
                    now,
                    podcast_id,
                ],
            )?;
            Ok(())
        })
        .await?
    }

    pub async fn delete_podcast(&self, id: i32) -> anyhow::Result<()> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            conn.execute("DELETE FROM podcasts WHERE id = ?", [id])?;
            Ok(())
        })
        .await?
    }

    // Episodes

    pub async fn get_episodes(&self, podcast_id: i32) -> anyhow::Result<Vec<Episode>> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            let mut stmt = conn.prepare(
                "SELECT id, podcast_id, title, description, url, audio_type,
                        publish_date, is_played, duration, position_seconds,
                        created_at, updated_at,
                        download_status, downloaded_path, speed_preset
                 FROM episodes
                 WHERE podcast_id = ?
                 ORDER BY publish_date DESC",
            )?;

            let episodes = stmt
                .query_map([podcast_id], |row| {
                    Ok(Episode {
                        id: row.get(0)?,
                        podcast_id: row.get(1)?,
                        title: row.get(2)?,
                        description: row.get(3)?,
                        url: row.get(4)?,
                        audio_type: row.get(5)?,
                        publish_date: row.get(6)?,
                        is_played: row.get::<_, i32>(7)? != 0,
                        duration: row.get(8)?,
                        position_seconds: row.get(9)?,
                        created_at: row.get(10)?,
                        updated_at: row.get(11)?,
                        download_status: row
                            .get::<_, Option<String>>(12)?
                            .map(|s| DownloadStatus::from_str(&s))
                            .unwrap_or_default(),
                        downloaded_path: row.get(13)?,
                        speed_preset: row.get(14)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(episodes)
        })
        .await?
    }

    pub async fn get_episode(&self, id: i32) -> anyhow::Result<Option<Episode>> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            let mut stmt = conn.prepare(
                "SELECT id, podcast_id, title, description, url, audio_type,
                        publish_date, is_played, duration, position_seconds,
                        created_at, updated_at,
                        download_status, downloaded_path, speed_preset
                 FROM episodes WHERE id = ?",
            )?;

            let mut rows = stmt.query_map([id], |row| {
                Ok(Episode {
                    id: row.get(0)?,
                    podcast_id: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    url: row.get(4)?,
                    audio_type: row.get(5)?,
                    publish_date: row.get(6)?,
                    is_played: row.get::<_, i32>(7)? != 0,
                    duration: row.get(8)?,
                    position_seconds: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                    download_status: row
                        .get::<_, Option<String>>(12)?
                        .map(|s| DownloadStatus::from_str(&s))
                        .unwrap_or_default(),
                    downloaded_path: row.get(13)?,
                    speed_preset: row.get(14)?,
                })
            })?;

            Ok(rows.next().transpose()?)
        })
        .await?
    }

    // Returns the IDs of newly inserted episodes (episodes that didn't exist before).
    pub async fn insert_episodes(&self, episodes: Vec<Episode>) -> anyhow::Result<Vec<i32>> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            let tx = conn.transaction()?;
            let mut new_ids = Vec::new();

            for ep in &episodes {
                tx.execute(
                    "INSERT OR IGNORE INTO episodes
                        (podcast_id, title, description, url, audio_type,
                         publish_date, is_played, duration, position_seconds,
                         created_at, updated_at)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,0.0,?9,?10)",
                    params![
                        ep.podcast_id,
                        ep.title,
                        ep.description,
                        ep.url,
                        ep.audio_type,
                        ep.publish_date,
                        ep.is_played as i32,
                        ep.duration,
                        ep.created_at,
                        ep.updated_at,
                    ],
                )?;
                if tx.changes() > 0 {
                    new_ids.push(tx.last_insert_rowid() as i32);
                }
            }

            tx.commit()?;
            Ok(new_ids)
        })
        .await?
    }

    pub async fn update_episode_played(
        &self,
        episode_id: i32,
        is_played: bool,
    ) -> anyhow::Result<()> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            let now = chrono::Utc::now().timestamp();
            conn.execute(
                "UPDATE episodes SET is_played = ?, updated_at = ? WHERE id = ?",
                params![is_played as i32, now, episode_id],
            )?;
            Ok(())
        })
        .await?
    }

    pub async fn update_episode_position(
        &self,
        episode_id: i32,
        position_seconds: f64,
    ) -> anyhow::Result<()> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            let now = chrono::Utc::now().timestamp();
            conn.execute(
                "UPDATE episodes SET position_seconds = ?1, updated_at = ?2 WHERE id = ?3",
                params![position_seconds, now, episode_id],
            )?;
            Ok(())
        })
        .await?
    }

    pub async fn update_episode_download_status(
        &self,
        episode_id: i32,
        status: DownloadStatus,
        path: Option<String>,
    ) -> anyhow::Result<()> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            let now = chrono::Utc::now().timestamp();
            conn.execute(
                "UPDATE episodes SET download_status = ?1, downloaded_path = ?2, updated_at = ?3 WHERE id = ?4",
                params![status.as_str(), path, now, episode_id],
            )?;
            Ok(())
        })
        .await?
    }

    pub async fn update_episode_speed_preset(
        &self,
        episode_id: i32,
        speed: Option<f32>,
    ) -> anyhow::Result<()> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            let now = chrono::Utc::now().timestamp();
            conn.execute(
                "UPDATE episodes SET speed_preset = ?1, updated_at = ?2 WHERE id = ?3",
                params![speed, now, episode_id],
            )?;
            Ok(())
        })
        .await?
    }

    pub async fn complete_episode(&self, episode_id: i32) -> anyhow::Result<()> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            let now = chrono::Utc::now().timestamp();
            conn.execute(
                "UPDATE episodes
                 SET is_played = 1, position_seconds = 0.0, updated_at = ?1
                 WHERE id = ?2",
                params![now, episode_id],
            )?;
            Ok(())
        })
        .await?
    }

    // Returns downloaded episodes for a podcast ordered newest-first (for retention pruning).
    pub async fn get_downloaded_episodes(
        &self,
        podcast_id: i32,
    ) -> anyhow::Result<Vec<Episode>> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            let mut stmt = conn.prepare(
                "SELECT id, podcast_id, title, description, url, audio_type,
                        publish_date, is_played, duration, position_seconds,
                        created_at, updated_at,
                        download_status, downloaded_path, speed_preset
                 FROM episodes
                 WHERE podcast_id = ? AND download_status = 'downloaded'
                 ORDER BY publish_date DESC",
            )?;

            let episodes = stmt
                .query_map([podcast_id], |row| {
                    Ok(Episode {
                        id: row.get(0)?,
                        podcast_id: row.get(1)?,
                        title: row.get(2)?,
                        description: row.get(3)?,
                        url: row.get(4)?,
                        audio_type: row.get(5)?,
                        publish_date: row.get(6)?,
                        is_played: row.get::<_, i32>(7)? != 0,
                        duration: row.get(8)?,
                        position_seconds: row.get(9)?,
                        created_at: row.get(10)?,
                        updated_at: row.get(11)?,
                        download_status: DownloadStatus::Downloaded,
                        downloaded_path: row.get(13)?,
                        speed_preset: row.get(14)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(episodes)
        })
        .await?
    }

    // Bookmarks

    pub async fn get_bookmarks_for_episode(
        &self,
        episode_id: i32,
    ) -> anyhow::Result<Vec<Bookmark>> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            let mut stmt = conn.prepare(
                "SELECT id, podcast_id, episode_id, position_seconds, note_text,
                        created_at, updated_at
                 FROM bookmarks
                 WHERE episode_id = ?1
                 ORDER BY COALESCE(position_seconds, 9999999), created_at",
            )?;

            let rows = stmt
                .query_map([episode_id], |row| {
                    Ok(Bookmark {
                        id: row.get(0)?,
                        podcast_id: row.get(1)?,
                        episode_id: row.get(2)?,
                        position_seconds: row.get(3)?,
                        note_text: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(rows)
        })
        .await?
    }

    pub async fn get_podcast_bookmarks(&self, podcast_id: i32) -> anyhow::Result<Vec<Bookmark>> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            let mut stmt = conn.prepare(
                "SELECT id, podcast_id, episode_id, position_seconds, note_text,
                        created_at, updated_at
                 FROM bookmarks
                 WHERE podcast_id = ?1 AND episode_id IS NULL
                 ORDER BY created_at",
            )?;

            let rows = stmt
                .query_map([podcast_id], |row| {
                    Ok(Bookmark {
                        id: row.get(0)?,
                        podcast_id: row.get(1)?,
                        episode_id: row.get(2)?,
                        position_seconds: row.get(3)?,
                        note_text: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(rows)
        })
        .await?
    }

    pub async fn insert_bookmark(&self, bookmark: Bookmark) -> anyhow::Result<Bookmark> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            let now = chrono::Utc::now().timestamp();
            conn.execute(
                "INSERT INTO bookmarks
                    (podcast_id, episode_id, position_seconds, note_text, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                params![
                    bookmark.podcast_id,
                    bookmark.episode_id,
                    bookmark.position_seconds,
                    bookmark.note_text,
                    now,
                ],
            )?;
            Ok(Bookmark {
                id: conn.last_insert_rowid() as i32,
                created_at: now,
                updated_at: now,
                ..bookmark
            })
        })
        .await?
    }

    pub async fn update_bookmark(&self, id: i32, note_text: String) -> anyhow::Result<()> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            let now = chrono::Utc::now().timestamp();
            conn.execute(
                "UPDATE bookmarks SET note_text = ?1, updated_at = ?2 WHERE id = ?3",
                params![note_text, now, id],
            )?;
            Ok(())
        })
        .await?
    }

    pub async fn delete_bookmark(&self, id: i32) -> anyhow::Result<()> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            conn.execute("DELETE FROM bookmarks WHERE id = ?1", [id])?;
            Ok(())
        })
        .await?
    }

    // Queue

    pub async fn get_queue(&self) -> anyhow::Result<Vec<QueueItem>> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            let mut stmt = conn.prepare(
                "SELECT id, episode_id, position, created_at FROM queue ORDER BY position",
            )?;

            let items = stmt
                .query_map([], |row| {
                    Ok(QueueItem {
                        id: row.get(0)?,
                        episode_id: row.get(1)?,
                        position: row.get(2)?,
                        created_at: row.get(3)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(items)
        })
        .await?
    }

    pub async fn get_queue_with_details(&self) -> anyhow::Result<Vec<QueueDisplayItem>> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            let mut stmt = conn.prepare(
                "SELECT q.id, q.episode_id, e.title, p.title
                 FROM queue q
                 JOIN episodes e ON e.id = q.episode_id
                 JOIN podcasts p ON p.id = e.podcast_id
                 ORDER BY q.position",
            )?;

            let items = stmt
                .query_map([], |row| {
                    Ok(QueueDisplayItem {
                        queue_id: row.get(0)?,
                        episode_id: row.get(1)?,
                        episode_title: row.get(2)?,
                        podcast_title: row.get(3)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(items)
        })
        .await?
    }

    pub async fn add_to_queue(&self, episode_id: i32) -> anyhow::Result<()> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            let max_position: Option<i32> = conn
                .query_row("SELECT MAX(position) FROM queue", [], |row| row.get(0))
                .unwrap_or(None);

            let position = max_position.unwrap_or(-1) + 1;
            let now = chrono::Utc::now().timestamp();

            conn.execute(
                "INSERT INTO queue (episode_id, position, created_at) VALUES (?, ?, ?)",
                params![episode_id, position, now],
            )?;
            Ok(())
        })
        .await?
    }

    pub async fn remove_from_queue(&self, queue_id: i32) -> anyhow::Result<()> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            conn.execute("DELETE FROM queue WHERE id = ?", [queue_id])?;
            Ok(())
        })
        .await?
    }

    pub async fn clear_queue(&self) -> anyhow::Result<()> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            conn.execute("DELETE FROM queue", [])?;
            Ok(())
        })
        .await?
    }

    // Settings

    pub async fn get_settings(&self) -> anyhow::Result<Settings> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            let mut settings = Settings::default();

            let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;

            for row in rows.flatten() {
                match row.0.as_str() {
                    "default_volume" => settings.default_volume = row.1.parse().unwrap_or(50.0),
                    "skip_backward_seconds" => {
                        settings.skip_backward_seconds = row.1.parse().unwrap_or(15)
                    }
                    "skip_forward_seconds" => {
                        settings.skip_forward_seconds = row.1.parse().unwrap_or(15)
                    }
                    "sync_interval_minutes" => {
                        settings.sync_interval_minutes = row.1.parse().unwrap_or(30)
                    }
                    "auto_play_next" => settings.auto_play_next = row.1 == "true",
                    "download_directory" => settings.download_directory = row.1,
                    "home_density" => {
                        settings.home_density = match row.1.as_str() {
                            "list" => HomeDensity::List,
                            _ => HomeDensity::Grid,
                        }
                    }
                    "default_speed" => settings.default_speed = row.1.parse().unwrap_or(1.0),
                    "trim_silence_mode" => {
                        settings.trim_silence_mode = match row.1.as_str() {
                            "smart_speed" => TrimSilenceMode::SmartSpeed,
                            "skip_silence" => TrimSilenceMode::SkipSilence,
                            _ => TrimSilenceMode::Off,
                        }
                    }
                    "auto_download_new_episodes" => {
                        settings.auto_download_new_episodes = row.1 == "true"
                    }
                    "global_keep_episodes_count" => {
                        settings.global_keep_episodes_count = row.1.parse().unwrap_or(0)
                    }
                    "hotkey_play_pause" => settings.hotkeys.play_pause = row.1,
                    "hotkey_next" => settings.hotkeys.next = row.1,
                    "hotkey_prev" => settings.hotkeys.prev = row.1,
                    "hotkey_skip_forward" => settings.hotkeys.skip_forward = row.1,
                    "hotkey_skip_backward" => settings.hotkeys.skip_backward = row.1,
                    "notify_new_episodes" => settings.notify_new_episodes = row.1 == "true",
                    "notify_download_complete" => {
                        settings.notify_download_complete = row.1 == "true"
                    }
                    _ => {}
                }
            }

            Ok(settings)
        })
        .await?
    }

    pub async fn save_settings(&self, settings: Settings) -> anyhow::Result<()> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;

            let trim_str = match settings.trim_silence_mode {
                TrimSilenceMode::Off => "off",
                TrimSilenceMode::SmartSpeed => "smart_speed",
                TrimSilenceMode::SkipSilence => "skip_silence",
            };

            let rows: &[(&str, String)] = &[
                ("default_volume", settings.default_volume.to_string()),
                ("skip_backward_seconds", settings.skip_backward_seconds.to_string()),
                ("skip_forward_seconds", settings.skip_forward_seconds.to_string()),
                ("sync_interval_minutes", settings.sync_interval_minutes.to_string()),
                ("auto_play_next", settings.auto_play_next.to_string()),
                ("download_directory", settings.download_directory.clone()),
                (
                    "home_density",
                    match settings.home_density {
                        HomeDensity::Grid => "grid".to_string(),
                        HomeDensity::List => "list".to_string(),
                    },
                ),
                ("default_speed", settings.default_speed.to_string()),
                ("trim_silence_mode", trim_str.to_string()),
                ("auto_download_new_episodes", settings.auto_download_new_episodes.to_string()),
                ("global_keep_episodes_count", settings.global_keep_episodes_count.to_string()),
                ("hotkey_play_pause", settings.hotkeys.play_pause.clone()),
                ("hotkey_next", settings.hotkeys.next.clone()),
                ("hotkey_prev", settings.hotkeys.prev.clone()),
                ("hotkey_skip_forward", settings.hotkeys.skip_forward.clone()),
                ("hotkey_skip_backward", settings.hotkeys.skip_backward.clone()),
                ("notify_new_episodes", settings.notify_new_episodes.to_string()),
                ("notify_download_complete", settings.notify_download_complete.to_string()),
            ];

            for (key, value) in rows {
                conn.execute(
                    "INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)",
                    params![key, value],
                )?;
            }

            Ok(())
        })
        .await?
    }
}

impl Database {
    pub fn get_download_directory_sync(&self) -> Result<String, DatabaseError> {
        let conn = self
            .connection
            .lock()
            .map_err(|_| DatabaseError::LockPoisoned)?;

        let result: Result<String, _> = conn.query_row(
            "SELECT value FROM settings WHERE key = 'download_directory'",
            [],
            |row| row.get(0),
        );

        Ok(result.unwrap_or_else(|_| {
            dirs::data_local_dir()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
        }))
    }
}
