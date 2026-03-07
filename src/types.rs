use serde::{Deserialize, Serialize};

// Navigation

#[derive(Debug, Clone, PartialEq)]
pub enum Page {
    Home,
    PodcastDetail(i32),
    Settings,
}

// Sort order

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortOrder {
    AToZ,
    ZToA,
    PublishDateAsc,
    PublishDateDesc,
}

// Home density

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum HomeDensity {
    Grid,
    List,
}

// Settings

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub default_volume: f32,
    pub skip_backward_seconds: i32,
    pub skip_forward_seconds: i32,
    pub sync_interval_minutes: i32,
    pub auto_play_next: bool,
    pub download_directory: String,
    pub home_density: HomeDensity,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_volume: 50.0,
            skip_backward_seconds: 15,
            skip_forward_seconds: 15,
            sync_interval_minutes: 30,
            auto_play_next: true,
            download_directory: dirs::data_local_dir()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            home_density: HomeDensity::Grid,
        }
    }
}

// Queue

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: i32,
    pub episode_id: i32,
    pub position: i32,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct QueueDisplayItem {
    pub queue_id: i32,
    pub episode_id: i32,
    pub episode_title: String,
    pub podcast_title: String,
}
