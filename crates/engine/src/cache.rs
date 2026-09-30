//! The C++ `CircularBuffer`: a bounded map that evicts the oldest *insertion*. Reads do not refresh an entry and re-inserting a key keeps its age, so this is FIFO, not LRU. The goldens depend on which entries survive (online rows inserted into the series cache in particular), so the semantics are kept exactly on top of `lru::LruCache` by never promoting.

use std::hash::Hash;
use std::num::NonZeroUsize;

use lru::LruCache;

pub struct FifoCache<K: Hash + Eq, V> {
    entries: LruCache<K, V>,
}

impl<K: Hash + Eq, V: Clone> FifoCache<K, V> {
    /// A zero capacity is a programming error in a constant, not a runtime condition.
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: LruCache::new(
                NonZeroUsize::new(capacity).expect("cache capacity is a nonzero constant"),
            ),
        }
    }

    pub fn get(&self, key: &K) -> Option<V> {
        self.entries.peek(key).cloned()
    }

    pub fn get_ref(&self, key: &K) -> Option<&V> {
        self.entries.peek(key)
    }

    pub fn contains(&self, key: &K) -> bool {
        self.entries.contains(key)
    }

    /// Updates in place without changing the entry's age; a new key evicts the oldest insertion when full.
    pub fn insert(&mut self, key: K, value: V) {
        if let Some(slot) = self.entries.peek_mut(&key) {
            *slot = value;
        } else {
            self.entries.push(key, value);
        }
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.entries.pop(key)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evicts_by_insertion_order_not_use() {
        let mut cache = FifoCache::new(2);
        cache.insert("a", 1);
        cache.insert("b", 2);
        assert_eq!(cache.get(&"a"), Some(1));
        cache.insert("a", 3);
        cache.insert("c", 4);
        assert!(!cache.contains(&"a"));
        assert_eq!(cache.get(&"b"), Some(2));
        assert_eq!(cache.get(&"c"), Some(4));
    }
}
