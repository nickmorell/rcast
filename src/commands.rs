use crate::types::{Page, Settings};

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

    // -- Episodes --------------------------------------------------------------
    DownloadEpisode(i32),
    TogglePlayed(i32),

    // -- Playback --------------------------------------------------------------
    PlayEpisode(i32),
    PlayAll(Vec<i32>),
    PlayNextInQueue,
    PausePlayback,
    ResumePlayback,

    // -- Queue -----------------------------------------------------------------
    AddToQueue(i32),
    RemoveFromQueue(i32),

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
    // Import subscriptions from an OPML file at the given path.
    ImportOpml {
        path: std::path::PathBuf,
    },
    // Export all subscriptions to an OPML file at the given path.
    ExportOpml {
        path: std::path::PathBuf,
    },

    // -- Settings -------------------------------------------------------------
    SaveSettings(Settings),
}
