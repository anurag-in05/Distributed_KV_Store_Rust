use rust_kv_store_distributed::server;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::time::{Duration, sleep};

struct Client {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
}

impl Client {
    async fn connect(addr: &str) -> Self {
        let stream = TcpStream::connect(addr).await.unwrap();
        let (read_half, writer) = stream.into_split();
        Client {
            reader: BufReader::new(read_half),
            writer,
        }
    }

    async fn send(&mut self, line: &str) -> String {
        self.writer
            .write_all(format!("{line}\n").as_bytes())
            .await
            .unwrap();
        let mut response = String::new();
        self.reader.read_line(&mut response).await.unwrap();
        response.trim_end().to_string()
    }
}

async fn start_server(addr: &'static str) {
    tokio::spawn(async move {
        server::run(addr).await.unwrap();
    });
    sleep(Duration::from_millis(50)).await;
}

#[tokio::test]
async fn responds_over_tcp() {
    start_server("127.0.0.1:17878").await;
    let mut client = Client::connect("127.0.0.1:17878").await;

    assert_eq!(client.send("PING").await, "PONG");
    assert_eq!(client.send("SET foo bar").await, "OK");
    assert_eq!(client.send("GET foo").await, "bar");
    assert_eq!(client.send("EXISTS foo").await, "1");
    assert_eq!(client.send("DEL foo").await, "1");
    assert_eq!(client.send("GET foo").await, "(nil)");
}

#[tokio::test]
async fn connections_do_not_share_state() {
    start_server("127.0.0.1:17879").await;
    let mut a = Client::connect("127.0.0.1:17879").await;
    let mut b = Client::connect("127.0.0.1:17879").await;

    assert_eq!(a.send("SET only_a 1").await, "OK");
    assert_eq!(b.send("GET only_a").await, "(nil)");
}
