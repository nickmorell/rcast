use crate::components::toast::ToastQueue;
use crate::db::models::{Episode, Podcast};
use crate::image_cache::ImageCache;
use crate::types::{QueueDisplayItem, Settings};
use std::collections::HashSet;

/// Which episode is currently loaded into the audio player.
/// Stored on AppState so both the home page cards and the detail page rows
/// can derive their `is_playing` flag with a cheap field comparison.
pub struct NowPlaying {
    pub episode_id: i32,
    pub podcast_id: i32,
}

/// The single source of truth for all UI rendering.
/// Mutated exclusively inside `RCast::handle_event`.
/// Pages receive `&AppState` or `&mut AppState` — they never mutate it directly.
pub struct AppState {
    // ── Home page ─────────────────────────────────────────────────────────────
    pub podcasts: Vec<Podcast>,

    // ── Detail page ───────────────────────────────────────────────────────────
    /// `None` while loading — detail page shows a spinner in the interim.
    pub detail_podcast: Option<Podcast>,
    pub detail_episodes: Vec<Episode>,

    // ── Playback ──────────────────────────────────────────────────────────────
    pub now_playing: Option<NowPlaying>,

    // ── Queue (displayed in MediaControls) ────────────────────────────────────
    pub queue_display: Vec<QueueDisplayItem>,

    // ── Settings ─────────────────────────────────────────────────────────────
    pub settings: Settings,

    // ── Images ────────────────────────────────────────────────────────────────
    pub image_cache: ImageCache,

    // ── Sync status ───────────────────────────────────────────────────────────
    /// IDs of podcasts currently being synced — used to show spinners on cards.
    pub syncing_podcast_ids: HashSet<i32>,

    // ── Toasts ────────────────────────────────────────────────────────────────
    pub toasts: ToastQueue,

    // ── UI flags (set by pages, consumed by application.rs) ──────────────────
    /// Set to true by any page that wants the Add Podcast modal to open.
    /// Consumed (and reset) by application.rs at the top of each frame.
    pub open_add_podcast_requested: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            podcasts: Vec::new(),
            detail_podcast: None,
            detail_episodes: Vec::new(),
            now_playing: None,
            queue_display: Vec::new(),
            settings: Settings::default(),
            image_cache: ImageCache::new(),
            syncing_podcast_ids: HashSet::new(),
            toasts: ToastQueue::default(),
            open_add_podcast_requested: false,
        }
    }
}
