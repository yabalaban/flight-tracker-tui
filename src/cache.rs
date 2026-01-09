use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct Cache<T: Clone> {
    data: Arc<RwLock<HashMap<String, CacheEntry<T>>>>,
    ttl: Duration,
}

struct CacheEntry<T> {
    value: T,
    inserted_at: Instant,
}

impl<T: Clone> Cache<T> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    pub fn get(&self, key: &str) -> Option<T> {
        let data = self.data.read().ok()?;
        let entry = data.get(key)?;

        if entry.inserted_at.elapsed() < self.ttl {
            Some(entry.value.clone())
        } else {
            None
        }
    }

    pub fn set(&self, key: String, value: T) {
        if let Ok(mut data) = self.data.write() {
            data.insert(
                key,
                CacheEntry {
                    value,
                    inserted_at: Instant::now(),
                },
            );
        }
    }

    /// Remove all expired entries from the cache.
    #[allow(dead_code)]
    pub fn clear_expired(&self) {
        if let Ok(mut data) = self.data.write() {
            data.retain(|_, entry| entry.inserted_at.elapsed() < self.ttl);
        }
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.data.read().map(|d| d.len()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_cache_set_and_get() {
        let cache: Cache<String> = Cache::new(Duration::from_secs(60));

        cache.set("key1".to_string(), "value1".to_string());

        assert_eq!(cache.get("key1"), Some("value1".to_string()));
        assert_eq!(cache.get("nonexistent"), None);
    }

    #[test]
    fn test_cache_expiration() {
        let cache: Cache<String> = Cache::new(Duration::from_millis(50));

        cache.set("key1".to_string(), "value1".to_string());
        assert_eq!(cache.get("key1"), Some("value1".to_string()));

        // Wait for TTL to expire
        thread::sleep(Duration::from_millis(60));

        assert_eq!(cache.get("key1"), None);
    }

    #[test]
    fn test_cache_overwrite() {
        let cache: Cache<String> = Cache::new(Duration::from_secs(60));

        cache.set("key1".to_string(), "value1".to_string());
        cache.set("key1".to_string(), "value2".to_string());

        assert_eq!(cache.get("key1"), Some("value2".to_string()));
    }

    #[test]
    fn test_cache_clear_expired() {
        let cache: Cache<String> = Cache::new(Duration::from_millis(50));

        cache.set("key1".to_string(), "value1".to_string());
        cache.set("key2".to_string(), "value2".to_string());

        assert_eq!(cache.len(), 2);

        thread::sleep(Duration::from_millis(60));
        cache.clear_expired();

        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_with_option_type() {
        let cache: Cache<Option<i32>> = Cache::new(Duration::from_secs(60));

        cache.set("some".to_string(), Some(42));
        cache.set("none".to_string(), None);

        assert_eq!(cache.get("some"), Some(Some(42)));
        assert_eq!(cache.get("none"), Some(None));
    }

    #[test]
    fn test_cache_clone() {
        let cache1: Cache<String> = Cache::new(Duration::from_secs(60));
        cache1.set("key1".to_string(), "value1".to_string());

        let cache2 = cache1.clone();

        // Both caches share the same underlying data
        assert_eq!(cache2.get("key1"), Some("value1".to_string()));

        cache2.set("key2".to_string(), "value2".to_string());
        assert_eq!(cache1.get("key2"), Some("value2".to_string()));
    }
}
