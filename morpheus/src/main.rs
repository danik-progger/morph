use clap::Parser;
use morpheus::{
    core::{client_manager::ClientManager, server::Server, storage::InMemoryStorage},
    ws::handler::client_connected,
};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tracing::info;
use warp::Filter;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// IP address to bind to
    #[arg(short, long, default_value_t = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)))]
    address: IpAddr,

    /// Port to listen on
    #[arg(short, long, default_value_t = 8080)]
    port: u16,
}

#[tokio::main]
async fn main() {
    morpheus::log::middleware::init_file_logger();
    info!("Logger initialized, starting server...");
    let args = Args::parse();
    let addr = SocketAddr::new(args.address, args.port);

    println!("Morpheus server starting on {}", addr);

    // The storage backend is created here and wrapped in an Arc.
    let storage = Arc::new(InMemoryStorage::new());
    // The ClientManager is created with a dynamic reference to the storage.
    let client_manager = Arc::new(ClientManager::new(storage));

    let server = Server::new(client_manager.clone());

    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(with_client_manager(client_manager.clone()))
        .map(|ws: warp::ws::Ws, manager| {
            ws.on_upgrade(move |socket| client_connected(socket, manager))
        });

    // Start the warp server in a separate task.
    let warp_server = tokio::spawn(warp::serve(ws_route).run(addr));

    // Start the CLI in the main task.
    let cli_server = tokio::spawn(async move {
        server.run_cli().await;
    });

    // Wait for either the server or the CLI to finish.
    tokio::select! {
        _ = warp_server => {
            eprintln!("Warp server has concluded.");
        },
        _ = cli_server => {
            eprintln!("CLI has concluded.");
        }
    }
}

fn with_client_manager(
    client_manager: Arc<ClientManager>,
) -> impl Filter<Extract = (Arc<ClientManager>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || client_manager.clone())
}
