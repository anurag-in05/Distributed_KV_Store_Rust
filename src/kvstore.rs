use std::collections::HashMap;
use std::time::{Duration, Instant};

struct Entry {
    value: String,
    expires_at: Option<Instant>,
}

impl Entry {
    fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(deadline) => Instant::now() >= deadline,
            None => false,
        }
    }
}

pub struct KvStore {
    data: HashMap<String, Entry>,
}

impl KvStore {
    pub fn new() -> Self {
        KvStore {
            data: HashMap::new(),
        }
    }

    /// `ttl` of `None` means the key never expires. A `SET` always replaces
    /// the whole entry, so any TTL the key had before is cleared.
    pub fn set(&mut self, key: &str, value: &str, ttl: Option<Duration>) {
        let expires_at = ttl.map(|duration| Instant::now() + duration);
        self.data.insert(
            key.to_string(),
            Entry {
                value: value.to_string(),
                expires_at,
            },
        );
    }

    pub fn get(&mut self, key: &str) -> Option<&String> {
        self.purge_if_expired(key);
        self.data.get(key).map(|entry| &entry.value)
    }

    pub fn del(&mut self, key: &str) -> bool {
        self.purge_if_expired(key);
        self.data.remove(key).is_some()
    }

    pub fn exists(&mut self, key: &str) -> bool {
        self.purge_if_expired(key);
        self.data.contains_key(key)
    }

    fn purge_if_expired(&mut self, key: &str) {
        if self.data.get(key).is_some_and(Entry::is_expired) {
            self.data.remove(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn set_then_get_returns_value() {
        let mut store = KvStore::new();
        store.set("foo", "bar", None);
        assert_eq!(store.get("foo"), Some(&"bar".to_string()));
    }

    #[test]
    fn get_missing_key_returns_none() {
        let mut store = KvStore::new();
        assert_eq!(store.get("missing"), None);
    }

    #[test]
    fn set_overwrites_existing_value() {
        let mut store = KvStore::new();
        store.set("foo", "bar", None);
        store.set("foo", "baz", None);
        assert_eq!(store.get("foo"), Some(&"baz".to_string()));
    }

    #[test]
    fn del_removes_key_and_reports_presence() {
        let mut store = KvStore::new();
        store.set("foo", "bar", None);
        assert!(store.del("foo"));
        assert!(!store.del("foo"));
        assert_eq!(store.get("foo"), None);
    }

    #[test]
    fn exists_reflects_presence() {
        let mut store = KvStore::new();
        assert!(!store.exists("foo"));
        store.set("foo", "bar", None);
        assert!(store.exists("foo"));
    }

    #[test]
    fn key_expires_after_ttl() {
        let mut store = KvStore::new();
        store.set("foo", "bar", Some(Duration::from_millis(20)));
        assert!(store.exists("foo"));
        sleep(Duration::from_millis(40));
        assert_eq!(store.get("foo"), None);
        assert!(!store.exists("foo"));
    }

    #[test]
    fn plain_set_clears_previous_ttl() {
        let mut store = KvStore::new();
        store.set("foo", "bar", Some(Duration::from_millis(20)));
        store.set("foo", "baz", None);
        sleep(Duration::from_millis(40));
        assert_eq!(store.get("foo"), Some(&"baz".to_string()));
    }
}
