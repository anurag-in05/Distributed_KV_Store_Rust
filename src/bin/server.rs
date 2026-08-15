use rust_kv_store_distributed::server;

const ADDR: &str = "127.0.0.1:7878";

#[tokio::main]
async fn main() -> std::io::Result<()> {
    server::run(ADDR).await
}
