use std::sync::{Arc, RwLock};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

use crate::kvstore::KvStore;
use crate::repl::dispatch;

pub async fn run(addr: &str) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    println!("listening on {addr}");

    let store = Arc::new(RwLock::new(KvStore::new()));

    loop {
        let (socket, peer) = listener.accept().await?;
        println!("connection from {peer}");
        let store = Arc::clone(&store);
        tokio::spawn(handle_connection(socket, store));
    }
}

async fn handle_connection(socket: TcpStream, store: Arc<RwLock<KvStore>>) {
    let (reader, mut writer) = socket.into_split();
    let mut lines = BufReader::new(reader).lines();

    loop {
        let line = match lines.next_line().await {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(_) => break,
        };

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Every command takes the write lock, even GET/EXISTS: lazy TTL
        // purging means a read can still mutate the map. Read/write
        // splitting is exactly the "concurrent read scaling" question
        // DESIGN.md defers until there's a baseline to measure against.
        let response = {
            let mut guard = store.write().unwrap();
            match dispatch(&mut guard, line) {
                Some(response) => response,
                None => break,
            }
        };

        if writer
            .write_all(format!("{response}\n").as_bytes())
            .await
            .is_err()
        {
            break;
        }
    }
}
