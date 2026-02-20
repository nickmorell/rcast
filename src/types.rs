use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Podcast {
    pub id: Option<i32>,
    pub url: String,
    pub title: String,
    pub description: String,
    pub image_url: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: Option<i32>,
    pub podcast_id: i32,
    pub title: String,
    pub description: String,
    pub url: String,
    pub audio_type: String,
    pub publish_date: i64,
    pub is_played: bool,
    pub duration: i64, // Duration in seconds
    pub created_at: i64,
    pub updated_at: i64,
}

impl Episode {
    pub fn format_publish_date(&self) -> String {
        let now = chrono::Utc::now().timestamp();
        let diff = now - self.publish_date;
        let days = diff / 86400;

        if days == 0 {
            "Today".to_string()
        } else if days >= 1 && days <= 6 {
            format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
        } else if days >= 7 && days <= 21 {
            let weeks = days / 7;
            format!("{} week{} ago", weeks, if weeks == 1 { "" } else { "s" })
        } else {
            chrono::DateTime::from_timestamp(self.publish_date, 0)
                .map(|dt| dt.format("%m/%d/%Y").to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: Option<i32>,
    pub episode_id: i32,
    pub position: i32,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub default_volume: f32,
    pub skip_backward_seconds: i32,
    pub skip_forward_seconds: i32,
    pub sync_interval_minutes: i32,
    pub auto_play_next: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_volume: 50.0,
            skip_backward_seconds: 15,
            skip_forward_seconds: 15,
            sync_interval_minutes: 30,
            auto_play_next: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Page {
    Home,
    PodcastDetail(i32),
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortOrder {
    AToZ,
    ZToA,
    PublishDateAsc,
    PublishDateDesc,
}
