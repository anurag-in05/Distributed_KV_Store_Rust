mod kvstore;
mod repl;

use kvstore::KvStore;

fn main() {
    let mut store = KvStore::new();
    repl::run(&mut store);
}
