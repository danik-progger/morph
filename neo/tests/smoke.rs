// IMPORTANT: This test starts a morpheus server in the background.

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use morpheus::core::{client_manager::ClientManager, msg::ServerMessage, storage::InMemoryStorage};
use neo::core::{client::Client as NeoClient, msg::ClientMessage as NeoClientMessage};
use std::{sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::OnceCell;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{protocol::Message, Error as WsError},
    MaybeTlsStream, WebSocketStream,
};
use url::Url;
use uuid::Uuid;
use warp::Filter;

// --- Test Harness for Morpheus Server ---

struct TestHarness {
    port: u16,
    client_manager: Arc<ClientManager>,
}

static HARNESS: OnceCell<TestHarness> = OnceCell::const_new();

async fn find_free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

async fn setup_server() -> &'static TestHarness {
    HARNESS
        .get_or_init(|| async {
            let port = find_free_port().await;
            let addr = format!("127.0.0.1:{}", port);

            let storage = Arc::new(InMemoryStorage::new());
            let client_manager = Arc::new(ClientManager::new(storage));
            let server_client_manager = client_manager.clone();

            tokio::spawn(async move {
                let ws_route = warp::path("ws")
                    .and(warp::ws())
                    .and(warp::any().map(move || server_client_manager.clone()))
                    .map(|ws: warp::ws::Ws, manager| {
                        ws.on_upgrade(move |socket| {
                            morpheus::ws::handler::client_connected(socket, manager)
                        })
                    });

                warp::serve(ws_route)
                    .run(addr.parse::<std::net::SocketAddr>().unwrap())
                    .await;
            });

            tokio::time::sleep(Duration::from_millis(100)).await;

            TestHarness {
                port,
                client_manager,
            }
        })
        .await
}

// --- Listener Client ---

struct ListenerClient {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl ListenerClient {
    async fn new(port: u16, topic: &str) -> Result<Self> {
        let url = format!("ws://127.0.0.1:{}/ws", port);
        let (mut ws, _) = connect_async(&url).await?;

        // The listener sends a message that the morpheus server understands
        let connect_msg = morpheus::core::msg::ClientMessage::Connect {
            topic: topic.to_string(),
        };
        let connect_msg_str = serde_json::to_string(&connect_msg)?;
        ws.send(Message::Text(connect_msg_str)).await?;

        Ok(Self { ws })
    }

    async fn recv(&mut self) -> Result<Option<ServerMessage>, WsError> {
        let msg = self.ws.next().await;
        match msg {
            Some(Ok(Message::Text(text))) => Ok(Some(serde_json::from_str(&text).unwrap())),
            Some(Err(e)) => Err(e),
            _ => Ok(None),
        }
    }
}

// --- Tests ---

#[tokio::test]
async fn test_neo_client_connect_and_send() -> Result<()> {
    let harness = setup_server().await;
    let topic = &format!("neo-smoke-test-{}", Uuid::new_v4());
    let url = format!("ws://127.0.0.1:{}/ws", harness.port);

    // 1. Create a listener client to receive messages
    let mut listener = ListenerClient::new(harness.port, topic).await?;

    // 2. Create the Neo client instance
    let mut neo_client = NeoClient::new(Url::parse(&url)?, topic.to_string())
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    // 3. Manually send the Connect message (part of neo_client.run())
    neo_client
        .connection
        .send(NeoClientMessage::Connect {
            topic: topic.to_string(),
        })
        .await?;

    // 4. Wait for both clients to be subscribed
    for _ in 0..10 {
        if harness.client_manager.get_clients_by_topic(topic).len() == 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(
        harness.client_manager.get_clients_by_topic(topic).len(),
        2,
        "Both clients should be subscribed"
    );

    // 5. Handle user input to send a message
    let message_content = "hello from neo";
    neo_client
        .handle_user_input(message_content)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    // 6. Assert that the listener client receives the message
    let received_msg = listener
        .recv()
        .await?
        .expect("Listener did not receive message");

    if let ServerMessage::Topic { content, .. } = received_msg {
        assert_eq!(content, message_content);
    } else {
        panic!("Incorrect message type received by listener");
    }

    Ok(())
}
