use crate::{
    database::Database,
    types::{Episode, Podcast},
};
use chrono::Utc;
use rss::Channel;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

pub struct RssSync {
    pub(crate) database: Database,
    pub(crate) is_syncing: Arc<AtomicBool>,
}

impl RssSync {
    pub fn new(database: Database) -> Self {
        Self {
            database,
            is_syncing: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_syncing(&self) -> bool {
        self.is_syncing.load(Ordering::Relaxed)
    }

    pub fn sync_all_podcasts(&self) {
        if self.is_syncing.load(Ordering::Relaxed) {
            return; // Already syncing
        }

        self.is_syncing.store(true, Ordering::Relaxed);

        let podcasts = match self.database.get_podcasts() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Failed to get podcasts: {}", e);
                self.is_syncing.store(false, Ordering::Relaxed);
                return;
            }
        };

        for podcast in podcasts {
            if let Err(e) = self.sync_podcast(&podcast) {
                eprintln!("Failed to sync podcast {}: {}", podcast.title, e);
            }
        }

        self.is_syncing.store(false, Ordering::Relaxed);
    }

    pub fn sync_podcast(&self, podcast: &Podcast) -> Result<(), String> {
        let content = reqwest::blocking::get(&podcast.url)
            .map_err(|e| format!("Network error: {}", e))?
            .bytes()
            .map_err(|e| format!("Failed to read response: {}", e))?;

        let channel = Channel::read_from(&content[..])
            .map_err(|e| format!("Failed to parse RSS: {}", e))?;

        let existing_episodes = self
            .database
            .get_episodes_by_podcast_id(podcast.id.unwrap())
            .unwrap_or_default();

        let existing_urls: Vec<String> = existing_episodes.iter().map(|e| e.url.clone()).collect();

        let now = Utc::now().timestamp();

        for item in channel.items() {
            let episode_url = match item.enclosure() {
                Some(enc) => enc.url().to_string(),
                None => continue,
            };

            // Skip if already exists
            if existing_urls.contains(&episode_url) {
                continue;
            }

            let publish_date = item
                .pub_date()
                .and_then(|d| chrono::DateTime::parse_from_rfc2822(d).ok())
                .map(|d| d.timestamp())
                .unwrap_or(now);

            // Try to extract duration from iTunes extension
            let duration = item
                .itunes_ext()
                .and_then(|ext| ext.duration())
                .and_then(|d| {
                    // Parse duration format (HH:MM:SS or MM:SS or seconds)
                    let parts: Vec<&str> = d.split(':').collect();
                    match parts.len() {
                        3 => {
                            // HH:MM:SS
                            let hours: i64 = parts[0].parse().ok()?;
                            let minutes: i64 = parts[1].parse().ok()?;
                            let seconds: i64 = parts[2].parse().ok()?;
                            Some(hours * 3600 + minutes * 60 + seconds)
                        }
                        2 => {
                            // MM:SS
                            let minutes: i64 = parts[0].parse().ok()?;
                            let seconds: i64 = parts[1].parse().ok()?;
                            Some(minutes * 60 + seconds)
                        }
                        1 => {
                            // Seconds only
                            parts[0].parse().ok()
                        }
                        _ => None,
                    }
                })
                .unwrap_or(0);

            let episode = Episode {
                id: None,
                podcast_id: podcast.id.unwrap(),
                title: item.title().unwrap_or("Untitled").to_string(),
                description: item.description().unwrap_or("").to_string(),
                url: episode_url.clone(),
                audio_type: item
                    .enclosure()
                    .map(|e| e.mime_type().to_string())
                    .unwrap_or_else(|| "audio/mpeg".to_string()),
                publish_date,
                is_played: false,
                duration,
                created_at: now,
                updated_at: now,
            };

            self.database
                .add_episode(&episode)
                .map_err(|e| format!("Failed to add episode: {}", e))?;
        }

        Ok(())
    }

    pub fn fetch_and_add_podcast(&self, url: &str) -> Result<Podcast, String> {
        let content = reqwest::blocking::get(url)
            .map_err(|e| format!("Network error: {}", e))?
            .bytes()
            .map_err(|e| format!("Failed to read response: {}", e))?;

        let channel = Channel::read_from(&content[..])
            .map_err(|e| format!("Failed to parse RSS: {}", e))?;

        let now = Utc::now().timestamp();

        let podcast = Podcast {
            id: None,
            url: url.to_string(),
            title: channel.title().to_string(),
            description: channel.description().to_string(),
            image_url: channel
                .image()
                .map(|i| i.url().to_string())
                .unwrap_or_else(|| String::new()),
            created_at: now,
            updated_at: now,
        };

        let podcast_id = self
            .database
            .add_podcast(&podcast)
            .map_err(|e| format!("Failed to add podcast: {}", e))?;

        let mut saved_podcast = podcast.clone();
        saved_podcast.id = Some(podcast_id);

        // Add episodes
        for item in channel.items() {
            let episode_url = match item.enclosure() {
                Some(enc) => enc.url().to_string(),
                None => continue,
            };
            
            let publish_date = item
                .pub_date()
                .and_then(|d| chrono::DateTime::parse_from_rfc2822(d).ok())
                .map(|d| d.timestamp())
                .unwrap_or(now);

            // Try to extract duration from iTunes extension
            let duration = item
                .itunes_ext()
                .and_then(|ext| ext.duration())
                .and_then(|d| {
                    // Parse duration format (HH:MM:SS or MM:SS or seconds)
                    let parts: Vec<&str> = d.split(':').collect();
                    match parts.len() {
                        3 => {
                            // HH:MM:SS
                            let hours: i64 = parts[0].parse().ok()?;
                            let minutes: i64 = parts[1].parse().ok()?;
                            let seconds: i64 = parts[2].parse().ok()?;
                            Some(hours * 3600 + minutes * 60 + seconds)
                        }
                        2 => {
                            // MM:SS
                            let minutes: i64 = parts[0].parse().ok()?;
                            let seconds: i64 = parts[1].parse().ok()?;
                            Some(minutes * 60 + seconds)
                        }
                        1 => {
                            // Seconds only
                            parts[0].parse().ok()
                        }
                        _ => None,
                    }
                })
                .unwrap_or(0);

            let episode = Episode {
                id: None,
                podcast_id,
                title: item.title().unwrap_or("Untitled").to_string(),
                description: item.description().unwrap_or("").to_string(),
                url: episode_url.clone(),
                audio_type: item
                    .enclosure()
                    .map(|e| e.mime_type().to_string())
                    .unwrap_or_else(|| "audio/mpeg".to_string()),
                publish_date,
                is_played: false,
                duration,
                created_at: now,
                updated_at: now,
            };

            println!("Adding episode '{}' with URL: {}", episode.title, episode_url);

            self.database
                .add_episode(&episode)
                .map_err(|e| format!("Failed to add episode: {}", e))?;
        }

        Ok(saved_podcast)
    }

    pub fn start_background_sync(&self, interval_minutes: i32) {
        let is_syncing = self.is_syncing.clone();
        let database = self.database.clone();

        std::thread::spawn(move || {
            let sync = RssSync {
                database,
                is_syncing,
            };

            loop {
                std::thread::sleep(Duration::from_secs((interval_minutes * 60) as u64));
                sync.sync_all_podcasts();
            }
        });
    }
}

