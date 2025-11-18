use anyhow::Result;
use futures_util::SinkExt;
use morpheus::core::msg::ClientMessage;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::OnceCell;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use warp::Filter;

static PORT: OnceCell<u16> = OnceCell::const_new();

async fn find_free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

async fn start_server() -> u16 {
    let port = find_free_port().await;
    let addr = format!("127.0.0.1:{}", port);
    let addr_clone = addr.clone();

    tokio::spawn(async move {
        let storage = std::sync::Arc::new(morpheus::core::storage::InMemoryStorage::new());
        let client_manager =
            std::sync::Arc::new(morpheus::core::client_manager::ClientManager::new(storage));
        let _server = morpheus::core::server::Server::new(client_manager.clone());

        let ws_route = warp::path("ws")
            .and(warp::ws())
            .and(warp::any().map(move || client_manager.clone()))
            .map(|ws: warp::ws::Ws, manager| {
                ws.on_upgrade(move |socket| {
                    morpheus::ws::handler::client_connected(socket, manager)
                })
            });

        let warp_server =
            warp::serve(ws_route).run(addr_clone.parse::<std::net::SocketAddr>().unwrap());
        warp_server.await;
    });

    // Poll the server to see when it's up.
    for _ in 0..10 {
        if tokio::net::TcpStream::connect(&addr).await.is_ok() {
            return port;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    panic!("Server did not start in time");
}

async fn get_server_port() -> u16 {
    *PORT.get_or_init(start_server).await
}

#[tokio::test]
async fn all_smoke_tests() -> Result<()> {
    let port = get_server_port().await;

    println!("--- Running test_server_starts ---");
    test_server_starts(port).await?;
    println!("--- Finished test_server_starts ---");

    println!("--- Running test_single_client_connects ---");
    test_single_client_connects(port).await?;
    println!("--- Finished test_single_client_connects ---");

    println!("--- Running test_10_clients_connect ---");
    test_10_clients_connect(port).await?;
    println!("--- Finished test_10_clients_connect ---");

    Ok(())
}

async fn test_server_starts(port: u16) -> Result<()> {
    let addr = format!("127.0.0.1:{}", port);
    assert!(tokio::net::TcpStream::connect(&addr).await.is_ok());
    Ok(())
}

async fn test_single_client_connects(port: u16) -> Result<()> {
    let url = format!("ws://127.0.0.1:{}/ws", port);

    let (mut ws_stream, _) = connect_async(&url).await?;
    println!("Client connected to {}", &url);

    let topic = "general".to_string();
    let connect_msg = ClientMessage::Connect {
        topic: topic.clone(),
    };
    let connect_msg_str = serde_json::to_string(&connect_msg)?;
    ws_stream.send(Message::Text(connect_msg_str)).await?;
    println!("Sent connect message");

    ws_stream.close(None).await?;
    println!("Client disconnected");

    Ok(())
}

async fn test_10_clients_connect(port: u16) -> Result<()> {
    let url = format!("ws://127.0.0.1:{}/ws", port);

    let mut join_handles = Vec::new();

    for i in 0..10 {
        let url = url.clone();
        let handle = tokio::spawn(async move {
            let (mut ws_stream, _) = connect_async(&url).await.expect("Failed to connect client");
            println!("Client {} connected", i);

            let topic = format!("topic-{}", i);
            let connect_msg = ClientMessage::Connect { topic };
            let connect_msg_str =
                serde_json::to_string(&connect_msg).expect("Failed to serialize message");
            ws_stream
                .send(Message::Text(connect_msg_str))
                .await
                .expect("Failed to send message");

            // Keep connection open for a bit
            tokio::time::sleep(Duration::from_millis(50)).await;

            ws_stream
                .close(None)
                .await
                .expect("Failed to close connection");
            println!("Client {} disconnected", i);
        });
        join_handles.push(handle);
    }

    for handle in join_handles {
        handle.await?;
    }

    Ok(())
}
