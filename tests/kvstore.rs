use rust_kv_store_distributed::kvstore::KvStore;
use std::thread::sleep;
use std::time::Duration;

#[test]
fn full_lifecycle() {
    let mut store = KvStore::new();

    assert_eq!(store.get("k"), None);
    assert!(!store.exists("k"));

    store.set("k", "v1", None);
    assert_eq!(store.get("k"), Some(&"v1".to_string()));
    assert!(store.exists("k"));

    store.set("k", "v2", None);
    assert_eq!(store.get("k"), Some(&"v2".to_string()));

    assert!(store.del("k"));
    assert!(!store.del("k"));
    assert_eq!(store.get("k"), None);
}

#[test]
fn ttl_expiry_is_lazy_and_purges_on_access() {
    let mut store = KvStore::new();
    store.set("k", "v", Some(Duration::from_millis(20)));

    assert!(store.exists("k"));
    sleep(Duration::from_millis(40));

    assert!(!store.exists("k"));
    assert_eq!(store.get("k"), None);
}
