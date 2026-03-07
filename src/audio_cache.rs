use bytes::Bytes;
use std::collections::{HashMap, VecDeque};

pub struct AudioCache {
    map: HashMap<i32, Bytes>,
    // Front = most recently used, back = least recently used.
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

    pub fn get(&mut self, episode_id: i32) -> Option<Bytes> {
        if self.map.contains_key(&episode_id) {
            self.promote(episode_id);
            self.map.get(&episode_id).cloned()
        } else {
            None
        }
    }

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

    fn promote(&mut self, episode_id: i32) {
        self.order.retain(|&id| id != episode_id);
        self.order.push_front(episode_id);
    }
}
