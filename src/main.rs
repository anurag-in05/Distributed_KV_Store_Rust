use rust_kv_store_distributed::kvstore::KvStore;
use rust_kv_store_distributed::repl;

fn main() {
    let mut store = KvStore::new();
    repl::run(&mut store);
}
