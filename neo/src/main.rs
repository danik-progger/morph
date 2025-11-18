use neo::core::client::Client;
use clap::Parser;
use url::Url;
use tokio::io::{self, BufReader};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Server address to connect to (e.g., ws://127.0.0.1:8080)
    #[arg(short, long)]
    address: String,

    /// Topic to subscribe to
    #[arg(short, long)]
    topic: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    match Url::parse(&args.address) {
        Ok(base_url) => {
            // The server listens on the /ws path
            match base_url.join("ws") {
                Ok(ws_url) => {
                    println!("Connecting to {} on topic '{}'...", ws_url, args.topic);
                    if let Err(e) = run_client(ws_url, args.topic).await {
                        eprintln!("Client error: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to construct WebSocket URL: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Invalid server address: {}", e);
        }
    }
}

async fn run_client(url: Url, topic: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut client = Client::new(url, topic).await?;
    let mut stdin = BufReader::new(io::stdin());
    client.run(&mut stdin).await
}
