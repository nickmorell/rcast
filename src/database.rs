use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::{
    errors::DatabaseError,
    migrations::run_migrations,
    types::{Episode, Podcast, QueueItem, Settings},
};

#[derive(Clone)]
pub struct Database {
    pub(crate) connection: Arc<Mutex<Connection>>,
}

impl Default for Database {
    fn default() -> Self {
        let mut connection = Connection::open("rcast.db").unwrap();
        connection.execute("PRAGMA foreign_keys = ON", []).unwrap();

        run_migrations(&mut connection).unwrap();

        Self {
            connection: Arc::new(Mutex::new(connection)),
        }
    }
}

impl Database {
    pub fn get_podcasts(&self) -> Result<Vec<Podcast>, DatabaseError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| DatabaseError::LockPoisoned)?;
        let mut stmt = connection.prepare("SELECT * FROM podcasts")?;

        let podcasts: Vec<Podcast> = stmt
            .query_map([], |row| {
                Ok(Podcast {
                    id: row.get(0)?,
                    url: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    image_url: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();

        Ok(podcasts)
    }

    pub fn get_episodes_by_podcast_id(&self, id: i32) -> Result<Vec<Episode>, DatabaseError> {
        let conn = self
            .connection
            .lock()
            .map_err(|_| DatabaseError::LockPoisoned)?;
        let mut stmt = conn.prepare(
            "SELECT id, podcast_id, title, description, url, audio_type, publish_date, is_played, duration, created_at, updated_at
             FROM episodes WHERE podcast_id = ? ORDER BY publish_date DESC"
        )?;

        let episodes: Vec<Episode> = stmt
            .query_map([id], |row| {
                Ok(Episode {
                    id: row.get(0)?,
                    podcast_id: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    url: row.get(4)?,
                    audio_type: row.get(5)?,
                    publish_date: row.get(6)?,
                    is_played: row.get::<_, i32>(7)? == 1,
                    duration: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();

        Ok(episodes)
    }

    pub fn add_podcast(&self, podcast: &Podcast) -> Result<i32, DatabaseError> {
        let conn = self
            .connection
            .lock()
            .map_err(|_| DatabaseError::LockPoisoned)?;

        conn.execute(
            "INSERT INTO podcasts (url, title, description, image_url, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
            [
                &podcast.url,
                &podcast.title,
                &podcast.description,
                &podcast.image_url,
                &podcast.created_at.to_string(),
                &podcast.updated_at.to_string(),
            ],
        )?;

        Ok(conn.last_insert_rowid() as i32)
    }

    pub fn add_episode(&self, episode: &Episode) -> Result<i32, DatabaseError> {
        let conn = self
            .connection
            .lock()
            .map_err(|_| DatabaseError::LockPoisoned)?;

        conn.execute(
            "INSERT INTO episodes (podcast_id, title, description, url, audio_type, publish_date, is_played, duration, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [
                &episode.podcast_id.to_string(),
                &episode.title,
                &episode.description,
                &episode.url,
                &episode.audio_type,
                &episode.publish_date.to_string(),
                &(if episode.is_played { 1 } else { 0 }).to_string(),
                &episode.duration.to_string(),
                &episode.created_at.to_string(),
                &episode.updated_at.to_string(),
            ],
        )?;

        Ok(conn.last_insert_rowid() as i32)
    }

    pub fn update_episode_played(
        &self,
        episode_id: i32,
        is_played: bool,
    ) -> Result<(), DatabaseError> {
        let conn = self
            .connection
            .lock()
            .map_err(|_| DatabaseError::LockPoisoned)?;

        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "UPDATE episodes SET is_played = ?, updated_at = ? WHERE id = ?",
            [if is_played { 1 } else { 0 }, now as i32, episode_id],
        )?;

        Ok(())
    }

    pub fn get_settings(&self) -> Result<Settings, DatabaseError> {
        let conn = self
            .connection
            .lock()
            .map_err(|_| DatabaseError::LockPoisoned)?;

        let mut settings = Settings::default();

        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let settings_iter = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for setting in settings_iter.flatten() {
            match setting.0.as_str() {
                "default_volume" => settings.default_volume = setting.1.parse().unwrap_or(50.0),
                "skip_backward_seconds" => {
                    settings.skip_backward_seconds = setting.1.parse().unwrap_or(15)
                }
                "skip_forward_seconds" => {
                    settings.skip_forward_seconds = setting.1.parse().unwrap_or(15)
                }
                "sync_interval_minutes" => {
                    settings.sync_interval_minutes = setting.1.parse().unwrap_or(30)
                }
                "auto_play_next" => settings.auto_play_next = setting.1 == "true",
                _ => {}
            }
        }

        Ok(settings)
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), DatabaseError> {
        let conn = self
            .connection
            .lock()
            .map_err(|_| DatabaseError::LockPoisoned)?;

        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('default_volume', ?)",
            [settings.default_volume.to_string()],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('skip_backward_seconds', ?)",
            [settings.skip_backward_seconds.to_string()],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('skip_forward_seconds', ?)",
            [settings.skip_forward_seconds.to_string()],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('sync_interval_minutes', ?)",
            [settings.sync_interval_minutes.to_string()],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('auto_play_next', ?)",
            [settings.auto_play_next.to_string()],
        )?;

        Ok(())
    }

    pub fn get_queue(&self) -> Result<Vec<QueueItem>, DatabaseError> {
        let conn = self
            .connection
            .lock()
            .map_err(|_| DatabaseError::LockPoisoned)?;

        let mut stmt = conn.prepare("SELECT * FROM queue ORDER BY position")?;
        let queue: Vec<QueueItem> = stmt
            .query_map([], |row| {
                Ok(QueueItem {
                    id: row.get(0)?,
                    episode_id: row.get(1)?,
                    position: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();

        Ok(queue)
    }

    pub fn add_to_queue(&self, episode_id: i32) -> Result<(), DatabaseError> {
        let conn = self
            .connection
            .lock()
            .map_err(|_| DatabaseError::LockPoisoned)?;

        let max_position: Option<i32> = conn
            .query_row("SELECT MAX(position) FROM queue", [], |row| row.get(0))
            .unwrap_or(None);

        let position = max_position.unwrap_or(-1) + 1;
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT INTO queue (episode_id, position, created_at) VALUES (?, ?, ?)",
            [episode_id, position, now as i32],
        )?;

        Ok(())
    }

    pub fn remove_from_queue(&self, queue_id: i32) -> Result<(), DatabaseError> {
        let conn = self
            .connection
            .lock()
            .map_err(|_| DatabaseError::LockPoisoned)?;

        conn.execute("DELETE FROM queue WHERE id = ?", [queue_id])?;

        Ok(())
    }

    pub fn get_episode_count_by_podcast(&self, podcast_id: i32) -> Result<i32, DatabaseError> {
        let conn = self
            .connection
            .lock()
            .map_err(|_| DatabaseError::LockPoisoned)?;

        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM episodes WHERE podcast_id = ?",
            [podcast_id],
            |row| row.get(0),
        )?;

        Ok(count)
    }
}
