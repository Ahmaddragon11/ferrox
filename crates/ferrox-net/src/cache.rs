// SPDX-License-Identifier: MIT OR Apache-2.0

use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct CachedAsset {
    pub bytes: Vec<u8>,
    pub content_type: String,
    pub inserted_at: Instant,
    pub ttl: Duration,
}

impl CachedAsset {
    pub fn is_expired(&self) -> bool {
        self.inserted_at.elapsed() > self.ttl
    }
}

/// Bounded in-memory cache with LRU-like eviction and byte budget for instant repeat reads.
pub struct MemoryAssetCache {
    max_bytes: usize,
    current_bytes: RwLock<usize>,
    entries: RwLock<HashMap<String, CachedAsset>>,
    order: RwLock<VecDeque<String>>,
}

impl MemoryAssetCache {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes,
            current_bytes: RwLock::new(0),
            entries: RwLock::new(HashMap::new()),
            order: RwLock::new(VecDeque::new()),
        }
    }

    pub fn get(&self, key: &str) -> Option<CachedAsset> {
        let guard = self.entries.read();
        let entry = guard.get(key)?;
        if entry.is_expired() {
            drop(guard);
            self.remove(key);
            return None;
        }
        Some(entry.clone())
    }

    pub fn insert(
        &self,
        key: impl Into<String>,
        bytes: Vec<u8>,
        content_type: String,
        ttl: Duration,
    ) {
        let key = key.into();
        let len = bytes.len();
        if len > self.max_bytes {
            return;
        }

        self.remove(&key);

        while *self.current_bytes.read() + len > self.max_bytes {
            let oldest = self.order.write().pop_front();
            match oldest {
                Some(old_key) => self.remove(&old_key),
                None => break,
            }
        }

        *self.current_bytes.write() += len;
        self.entries.write().insert(
            key.clone(),
            CachedAsset {
                bytes,
                content_type,
                inserted_at: Instant::now(),
                ttl,
            },
        );
        self.order.write().push_back(key);
    }

    pub fn remove(&self, key: &str) {
        if let Some(removed) = self.entries.write().remove(key) {
            let mut total = self.current_bytes.write();
            *total = total.saturating_sub(removed.bytes.len());
        }
    }

    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn current_bytes(&self) -> usize {
        *self.current_bytes.read()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evicts_when_over_budget() {
        let cache = MemoryAssetCache::new(10);
        cache.insert(
            "a",
            vec![1, 2, 3, 4, 5, 6],
            "text/plain".into(),
            Duration::from_secs(60),
        );
        cache.insert(
            "b",
            vec![7, 8, 9, 10, 11, 12],
            "text/plain".into(),
            Duration::from_secs(60),
        );
        assert!(cache.get("a").is_none());
        assert!(cache.get("b").is_some());
        assert_eq!(cache.current_bytes(), 6);
    }
}
