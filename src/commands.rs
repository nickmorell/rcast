use crate::types::{HotkeySettings, Page, PodcastPreferences, Settings};

// Every action the UI can request. The Orchestrator is the sole consumer.
#[derive(Debug)]
#[allow(dead_code)]
pub enum AppCommand {
    // -- Navigation ------------------------------------------------------------
    NavigateTo(Page),

    // -- Podcasts --------------------------------------------------------------
    AddPodcast {
        feed_url: String,
    },
    RemovePodcast(i32),
    SyncPodcast(i32),
    SyncAll,
    UpdatePodcastPreferences {
        podcast_id: i32,
        prefs: PodcastPreferences,
    },

    // -- Episodes --------------------------------------------------------------
    DownloadEpisode(i32),
    DeleteDownload(i32),
    TogglePlayed(i32),
    CompleteEpisode(i32),
    SetEpisodeSpeedPreset {
        episode_id: i32,
        speed: Option<f32>,
    },

    // -- Playback --------------------------------------------------------------
    PlayEpisode(i32),
    PlayAll(Vec<i32>),
    PlayNextInQueue,
    PausePlayback,
    ResumePlayback,
    TogglePlayback,
    JumpForward,
    JumpBackward,

    // -- Queue -----------------------------------------------------------------
    AddToQueue(i32),
    RemoveFromQueue(i32),
    ClearQueue,

    // -- Bookmarks -------------------------------------------------------------
    LoadBookmarks {
        podcast_id: i32,
        episode_id: i32,
    },
    AddBookmark {
        podcast_id: i32,
        episode_id: Option<i32>,
        position_seconds: Option<f64>,
        note_text: String,
    },
    UpdateBookmark {
        id: i32,
        note_text: String,
    },
    DeleteBookmark(i32),

    // -- OPML ------------------------------------------------------------------
    ImportOpml {
        path: std::path::PathBuf,
    },
    ExportOpml {
        path: std::path::PathBuf,
    },

    // -- Settings -------------------------------------------------------------
    SaveSettings(Settings),
    ApplyHotkeys(HotkeySettings),
}
