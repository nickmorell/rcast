pub mod models;

use std::fs::create_dir_all;
use std::sync::{Arc, Mutex};

use anyhow::anyhow;
use dirs::data_local_dir;
use rusqlite::{Connection, params};

use crate::errors::DatabaseError;
use crate::migrations::run_migrations;
use crate::types::{QueueDisplayItem, QueueItem, Settings};
use models::{Bookmark, Episode, Podcast};

/// Thin wrapper around a rusqlite connection pool.
///
/// rusqlite is synchronous so every public async method wraps its work in
/// `tokio::task::spawn_blocking`.  The `Arc<Mutex<Connection>>` makes the
/// handle cheap to clone and safe to share across spawned tasks.
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

    // ── Podcasts ──────────────────────────────────────────────────────────────

    pub async fn get_all_podcasts(&self) -> anyhow::Result<Vec<Podcast>> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            let mut stmt = conn.prepare(
                "SELECT p.id, p.url, p.title, p.description, p.image_url,
                        p.last_synced_at, p.created_at, p.updated_at,
                        COUNT(e.id) as episode_count
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
                        COUNT(e.id) as episode_count
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

    /// Stamps `last_synced_at` with the current time after a successful sync.
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

    pub async fn delete_podcast(&self, id: i32) -> anyhow::Result<()> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            conn.execute("DELETE FROM podcasts WHERE id = ?", [id])?;
            Ok(())
        })
        .await?
    }

    // ── Episodes ──────────────────────────────────────────────────────────────

    pub async fn get_episodes(&self, podcast_id: i32) -> anyhow::Result<Vec<Episode>> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            let mut stmt = conn.prepare(
                "SELECT id, podcast_id, title, description, url, audio_type,
                        publish_date, is_played, duration, position_seconds,
                        created_at, updated_at
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
                        created_at, updated_at
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
                })
            })?;

            Ok(rows.next().transpose()?)
        })
        .await?
    }

    pub async fn insert_episodes(&self, episodes: Vec<Episode>) -> anyhow::Result<()> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
            let tx = conn.transaction()?;

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
            }

            tx.commit()?;
            Ok(())
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

    /// Saves the current playback position for an episode. Called periodically
    /// while playing and on pause. Only writes if the position has changed by
    /// more than 5 seconds to avoid unnecessary DB churn.
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

    /// Marks an episode as fully played and resets its saved position to zero.
    /// Called when an episode finishes naturally (end of track or autoplay kicks in).
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

    // ── Bookmarks ─────────────────────────────────────────────────────────────

    /// Returns all bookmarks for a specific episode, ordered by position then
    /// creation time. Includes both timed and untimed episode notes.
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

    /// Returns podcast-level notes (episode_id IS NULL) for a podcast.
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

    // ── Queue ─────────────────────────────────────────────────────────────────

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

    /// Returns queue items with denormalised episode and podcast titles for display.
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

    // ── Settings ─────────────────────────────────────────────────────────────

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

            let rows: &[(&str, String)] = &[
                ("default_volume", settings.default_volume.to_string()),
                (
                    "skip_backward_seconds",
                    settings.skip_backward_seconds.to_string(),
                ),
                (
                    "skip_forward_seconds",
                    settings.skip_forward_seconds.to_string(),
                ),
                (
                    "sync_interval_minutes",
                    settings.sync_interval_minutes.to_string(),
                ),
                ("auto_play_next", settings.auto_play_next.to_string()),
                ("download_directory", settings.download_directory.clone()),
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

    pub async fn get_download_directory(&self) -> anyhow::Result<String> {
        let conn = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| anyhow!("Lock error: {e}"))?;
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
        })
        .await?
    }
}

// ── Allow DownloadManager (sync) to call get_download_directory synchronously ─

impl Database {
    /// Synchronous variant used by DownloadManager (which is not async).
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
