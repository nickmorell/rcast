use crate::components::toast::ToastMessage;
use crate::db::models::{Episode, Podcast};
use crate::types::{Page, QueueDisplayItem, Settings};

/// Everything the background runtime can communicate back to the UI.
/// Processed in `RCast::handle_event` every frame.
#[derive(Debug)]
pub enum AppEvent {
    // ── Navigation ────────────────────────────────────────────────────────────
    /// Switch the active page immediately (before data is ready).
    NavigatedTo(Page),

    // ── Data ──────────────────────────────────────────────────────────────────
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
    /// Fired when a podcast sync begins — UI shows a spinner on that card.
    SyncStarted(i32),
    /// Fired when a podcast sync finishes (success or failure).
    SyncCompleted(i32),

    // ── Queue ─────────────────────────────────────────────────────────────────
    QueueUpdated(Vec<QueueDisplayItem>),

    // ── Playback ──────────────────────────────────────────────────────────────
    /// Fired after audio actually starts playing.
    PlaybackStarted {
        episode_id: i32,
        podcast_id: i32,
    },
    PlaybackStopped,

    // ── Settings ─────────────────────────────────────────────────────────────
    SettingsLoaded(Settings),
    SettingsSaved,

    // ── Bookmarks ─────────────────────────────────────────────────────────────
    BookmarksLoaded {
        episode_bookmarks: Vec<crate::db::models::Bookmark>,
        podcast_bookmarks: Vec<crate::db::models::Bookmark>,
    },
    BookmarkAdded(crate::db::models::Bookmark),
    BookmarkUpdated(crate::db::models::Bookmark),
    BookmarkDeleted(i32),

    // ── OPML ──────────────────────────────────────────────────────────────────
    /// Fired after an OPML import completes (successfully or partially).
    OpmlImported {
        added: usize,
        skipped: usize,
        failed: usize,
    },
    /// Fired after an OPML export completes.
    OpmlExported {
        path: String,
    },

    // ── Cross-cutting ─────────────────────────────────────────────────────────
    Toast(ToastMessage),
    Error(String),
}
