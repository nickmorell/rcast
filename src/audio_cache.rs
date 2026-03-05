use bytes::Bytes;
use std::collections::{HashMap, VecDeque};

/// In-memory LRU cache for audio episode bytes.
///
/// Stores up to `max_entries` episodes. When the limit is exceeded the
/// least-recently-used episode is evicted. The cache is intentionally
/// in-memory only — it is wiped when the process exits.
///
/// `Bytes` is internally reference-counted so cloning a value to hand to
/// the audio player is a pointer increment, not a copy of the audio data.
pub struct AudioCache {
    map: HashMap<i32, Bytes>,
    /// Front = most recently used, back = least recently used.
    order: VecDeque<i32>,
    max_entries: usize,
}

impl AudioCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            max_entries,
        }
    }

    /// Returns a cheap clone of the cached bytes if present, and promotes
    /// the entry to most-recently-used.
    pub fn get(&mut self, episode_id: i32) -> Option<Bytes> {
        if self.map.contains_key(&episode_id) {
            self.promote(episode_id);
            self.map.get(&episode_id).cloned()
        } else {
            None
        }
    }

    /// Inserts an entry. If the cache is full, the least-recently-used
    /// episode is evicted first.
    pub fn insert(&mut self, episode_id: i32, bytes: Bytes) {
        if self.map.contains_key(&episode_id) {
            // Already present — just promote and update.
            self.promote(episode_id);
            self.map.insert(episode_id, bytes);
            return;
        }

        if self.map.len() >= self.max_entries {
            if let Some(lru_id) = self.order.pop_back() {
                self.map.remove(&lru_id);
            }
        }

        self.map.insert(episode_id, bytes);
        self.order.push_front(episode_id);
    }

    // ── Private ───────────────────────────────────────────────────────────────

    /// Move `episode_id` to the front of the recency queue.
    fn promote(&mut self, episode_id: i32) {
        self.order.retain(|&id| id != episode_id);
        self.order.push_front(episode_id);
    }
}
