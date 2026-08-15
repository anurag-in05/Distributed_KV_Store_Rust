use std::time::{Duration, Instant};

use rust_kv_store_distributed::kvstore::KvStore;

const OPERATIONS: u64 = 1_000_000;

fn main() {
    bench_set();
    bench_get();
    bench_mixed();
}

fn bench_set() {
    let mut store = KvStore::new();
    let start = Instant::now();
    for i in 0..OPERATIONS {
        store.set(&i.to_string(), "value", None);
    }
    report("SET", OPERATIONS, start.elapsed());
}

fn bench_get() {
    let mut store = KvStore::new();
    for i in 0..OPERATIONS {
        store.set(&i.to_string(), "value", None);
    }

    let start = Instant::now();
    for i in 0..OPERATIONS {
        store.get(&i.to_string());
    }
    report("GET", OPERATIONS, start.elapsed());
}

/// A rotating window of 1000 keys, mixing SET/GET/DEL, so the map size
/// stays bounded and deletions actually hit existing keys.
fn bench_mixed() {
    let mut store = KvStore::new();
    let start = Instant::now();
    for i in 0..OPERATIONS {
        let key = (i % 1000).to_string();
        match i % 4 {
            0 => {
                store.del(&key);
            }
            1 => {
                store.set(&key, "value", None);
            }
            _ => {
                store.get(&key);
            }
        }
    }
    report("MIXED (set/get/del)", OPERATIONS, start.elapsed());
}

fn report(name: &str, operations: u64, elapsed: Duration) {
    let secs = elapsed.as_secs_f64();
    let ops_per_sec = operations as f64 / secs;
    println!("{name}: {operations} ops in {secs:.3}s ({ops_per_sec:.0} ops/sec)");
}
