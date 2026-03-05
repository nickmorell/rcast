use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Podcast {
    pub id: i32,
    pub url: String,
    pub title: String,
    pub description: String,
    pub image_url: String,
    pub episode_count: i32,
    pub last_synced_at: i64, // 0 means never synced
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: i32,
    pub podcast_id: i32,
    pub title: String,
    pub description: String,
    pub url: String,
    pub audio_type: String,
    pub publish_date: i64,
    pub is_played: bool,
    pub duration: i64,
    pub position_seconds: f64,
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
