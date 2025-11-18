// NOTE: These tests are combined into a single test function to ensure
// the server stays alive for all of them.

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use morpheus::core::{client_manager::ClientManager, msg::ServerMessage, storage::InMemoryStorage};
use neo::core::{client::Client, msg::ClientMessage as NeoClientMessage};
use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};
use tokio::io::{AsyncRead, BufReader};
use tokio::net::TcpListener;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{protocol::Message, Error as WsError},
    MaybeTlsStream, WebSocketStream,
};
use url::Url;
use uuid::Uuid;
use warp::Filter;

// A mock reader that never returns any data, keeping the client's run loop pending.
struct PendingReader;

impl AsyncRead for PendingReader {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Pending
    }
}

// --- Test Harness for Morpheus Server ---
struct TestHarness {
    port: u16,
    client_manager: Arc<ClientManager>,
}

static HARNESS: tokio::sync::OnceCell<TestHarness> = tokio::sync::OnceCell::const_new();

async fn setup_server() -> &'static TestHarness {
    HARNESS
        .get_or_init(|| async {
            let port = find_free_port().await;
            let addr = format!("127.0.0.1:{}", port);
            let addr_clone = addr.clone();

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
                    .run(addr_clone.parse::<std::net::SocketAddr>().unwrap())
                    .await;
            });

            for _ in 0..10 {
                if tokio::net::TcpStream::connect(&addr).await.is_ok() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }

            TestHarness {
                port,
                client_manager,
            }
        })
        .await
}

async fn find_free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

// --- Listener Client ---
struct ListenerClient {
    ws: WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
}

impl ListenerClient {
    async fn new(port: u16, topic: &str) -> Result<Self> {
        let url = format!("ws://127.0.0.1:{}/ws", port);
        let (mut ws, _) = connect_async(&url).await?;

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

#[tokio::test]
async fn all_integration_tests() -> Result<()> {
    let harness = setup_server().await;

    println!("--- Running test_run_client_connects_and_subscribes ---");
    test_run_client_connects_and_subscribes(harness).await?;
    println!("--- Finished test_run_client_connects_and_subscribes ---");

    println!("--- Running test_client_sends_topic_message ---");
    test_client_sends_topic_message(harness).await?;
    println!("--- Finished test_client_sends_topic_message ---");

    Ok(())
}

async fn test_run_client_connects_and_subscribes(harness: &TestHarness) -> Result<()> {
    let topic = format!("test-topic-{}", Uuid::new_v4());
    let url = format!("ws://127.0.0.1:{}", harness.port)
        .parse::<Url>()?
        .join("ws")?;
    let topic_clone = topic.clone();

    let client_task = tokio::spawn(async move {
        let mut client = Client::new(url, topic_clone).await.unwrap();
        let mut mock_stdin = BufReader::new(PendingReader);
        client.run(&mut mock_stdin).await.unwrap();
    });

    for _ in 0..20 {
        let clients = harness.client_manager.get_clients_by_topic(&topic);
        if !clients.is_empty() {
            assert_eq!(clients.len(), 1);
            client_task.abort();
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    client_task.abort();
    panic!("Client did not connect and subscribe to the topic in time.");
}

async fn test_client_sends_topic_message(harness: &TestHarness) -> Result<()> {
    let topic = format!("test-topic-{}", Uuid::new_v4());
    let url = format!("ws://127.0.0.1:{}", harness.port)
        .parse::<Url>()?
        .join("ws")?;

    // 1. Create a listener client
    let mut listener = ListenerClient::new(harness.port, &topic).await?;

    // 2. Create the Neo client instance
    let mut neo_client = Client::new(url, topic.to_string())
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    // 3. Manually send the Connect message for the neo_client
    neo_client
        .connection
        .send(NeoClientMessage::Connect {
            topic: topic.to_string(),
        })
        .await?;

    // 4. Wait for both clients to be subscribed
    for _ in 0..20 {
        if harness.client_manager.get_clients_by_topic(&topic).len() == 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(harness.client_manager.get_clients_by_topic(&topic).len(), 2);

    // 5. Call `handle_user_input` on neo client to send a message
    let message_content = "a message from neo";
    neo_client
        .handle_user_input(message_content)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    // 6. Check listener receives the message
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
