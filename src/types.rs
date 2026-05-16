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

// Trim silence mode

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum TrimSilenceMode {
    #[default]
    Off,
    SmartSpeed,
    SkipSilence,
}

// Hotkey settings (one string per action, empty = unbound)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeySettings {
    pub play_pause: String,
    pub next: String,
    pub prev: String,
    pub skip_forward: String,
    pub skip_backward: String,
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self {
            play_pause: String::new(),
            next: String::new(),
            prev: String::new(),
            skip_forward: String::new(),
            skip_backward: String::new(),
        }
    }
}

// Per-show playback/download preferences (all optional — None means inherit global)

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PodcastPreferences {
    pub speed_preset: Option<f32>,
    pub auto_download: Option<bool>,
    pub keep_episodes_count: Option<i32>,
    pub skip_intro_seconds: i32,
    pub skip_outro_seconds: i32,
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
    // Playback defaults
    pub default_speed: f32,
    pub trim_silence_mode: TrimSilenceMode,
    // Download / retention
    pub auto_download_new_episodes: bool,
    pub global_keep_episodes_count: i32,
    // Hotkeys
    pub hotkeys: HotkeySettings,
    // Notifications
    pub notify_new_episodes: bool,
    pub notify_download_complete: bool,
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
            default_speed: 1.0,
            trim_silence_mode: TrimSilenceMode::Off,
            auto_download_new_episodes: false,
            global_keep_episodes_count: 0,
            hotkeys: HotkeySettings::default(),
            notify_new_episodes: true,
            notify_download_complete: true,
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
    #[allow(dead_code)]
    pub episode_id: i32,
    pub episode_title: String,
    pub podcast_title: String,
}
