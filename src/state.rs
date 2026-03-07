use crate::components::toast::ToastQueue;
use crate::db::models::Bookmark;
use crate::db::models::{Episode, Podcast};
use crate::image_cache::ImageCache;
use crate::types::{QueueDisplayItem, Settings};
use std::collections::HashSet;

pub struct NowPlaying {
    pub episode_id: i32,
    pub podcast_id: i32,
}

pub struct AppState {
    // Home page
    pub podcasts: Vec<Podcast>,

    // Detail page
    pub detail_podcast: Option<Podcast>,
    pub detail_episodes: Vec<Episode>,

    // Playback
    pub now_playing: Option<NowPlaying>,

    // Queue
    pub queue_display: Vec<QueueDisplayItem>,

    // Settings
    pub settings: Settings,

    // Images
    pub image_cache: ImageCache,

    // Bookmarks
    pub notes_episode_bookmarks: Vec<Bookmark>,
    pub notes_podcast_bookmarks: Vec<Bookmark>,
    pub notes_open_request: Option<(i32, i32, String)>,

    // Sync status
    // IDs of podcasts currently being synced — used to show spinners on cards.
    pub syncing_podcast_ids: HashSet<i32>,

    // Toasts
    pub toasts: ToastQueue,

    pub open_add_podcast_requested: bool,
    pub seek_request: Option<std::time::Duration>,
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
            notes_episode_bookmarks: Vec::new(),
            notes_podcast_bookmarks: Vec::new(),
            notes_open_request: None,
            syncing_podcast_ids: HashSet::new(),
            toasts: ToastQueue::default(),
            open_add_podcast_requested: false,
            seek_request: None,
        }
    }
}
