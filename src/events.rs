use crate::components::toast::ToastMessage;
use crate::db::models::{DownloadStatus, Episode, Podcast};
use crate::types::{Page, PodcastPreferences, QueueDisplayItem, Settings};

#[derive(Debug)]
pub enum AppEvent {
    // Navigation
    NavigatedTo(Page),

    // Data
    PodcastsLoaded(Vec<Podcast>),
    PodcastAdded(Podcast),
    PodcastRemoved(i32),
    PodcastDetailLoaded {
        podcast: Podcast,
        episodes: Vec<Episode>,
    },
    EpisodesUpdated {
        podcast_id: i32,
        episodes: Vec<Episode>,
    },
    SyncStarted(i32),
    SyncCompleted(i32),
    PodcastPreferencesUpdated {
        podcast_id: i32,
        prefs: PodcastPreferences,
    },

    // Downloads
    DownloadStatusChanged {
        episode_id: i32,
        status: DownloadStatus,
        path: Option<String>,
    },

    // Queue
    QueueUpdated(Vec<QueueDisplayItem>),

    // Playback
    PlaybackStarted {
        episode_id: i32,
        podcast_id: i32,
        episode: Episode,
    },
    PlaybackStopped,

    // Settings
    SettingsLoaded(Settings),
    SettingsSaved,

    // Bookmarks
    BookmarksLoaded {
        episode_bookmarks: Vec<crate::db::models::Bookmark>,
        podcast_bookmarks: Vec<crate::db::models::Bookmark>,
    },
    BookmarkAdded(crate::db::models::Bookmark),
    BookmarkUpdated(crate::db::models::Bookmark),
    BookmarkDeleted(i32),

    // OPML
    OpmlImported {
        added: usize,
        skipped: usize,
        failed: usize,
    },
    OpmlExported {
        path: String,
    },

    // Cross-cutting
    Toast(ToastMessage),
    Error(String),
}
