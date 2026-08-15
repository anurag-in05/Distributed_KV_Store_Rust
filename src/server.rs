use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

use crate::kvstore::KvStore;
use crate::repl::dispatch;

pub async fn run(addr: &str) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    println!("listening on {addr}");

    loop {
        let (socket, peer) = listener.accept().await?;
        println!("connection from {peer}");
        tokio::spawn(handle_connection(socket));
    }
}

async fn handle_connection(socket: TcpStream) {
    let mut store = KvStore::new();
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

        let response = match dispatch(&mut store, line) {
            Some(response) => response,
            None => break,
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
