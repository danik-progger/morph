// NOTE: These tests are combined into a single test function to ensure
// the server stays alive for all of them.

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use morpheus::core::{
    client_manager::ClientManager,
    msg::{ClientMessage, ServerMessage},
    storage::InMemoryStorage,
};
use std::{sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::OnceCell;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{protocol::Message, Error as WsError},
    MaybeTlsStream, WebSocketStream,
};
use uuid::Uuid;
use warp::Filter;

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

struct TestClient {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl TestClient {
    async fn new(port: u16, topic: &str) -> Result<Self> {
        let url = format!("ws://127.0.0.1:{}/ws", port);
        let (mut ws, _) = connect_async(&url).await?;

        let connect_msg = ClientMessage::Connect {
            topic: topic.to_string(),
        };
        let connect_msg_str = serde_json::to_string(&connect_msg)?;
        ws.send(Message::Text(connect_msg_str)).await?;

        Ok(Self { ws })
    }

    async fn send_message(&mut self, topic: &str, content: &str) -> Result<()> {
        let msg = ClientMessage::Message {
            topic: topic.to_string(),
            content: content.to_string(),
        };
        let msg_str = serde_json::to_string(&msg)?;
        self.ws.send(Message::Text(msg_str)).await?;
        Ok(())
    }

    async fn recv(&mut self) -> Result<Option<ServerMessage>, WsError> {
        let msg = self.ws.next().await;
        match msg {
            Some(Ok(Message::Text(text))) => Ok(Some(serde_json::from_str(&text).unwrap())),
            Some(Err(e)) => Err(e),
            _ => Ok(None),
        }
    }

    async fn close(mut self) -> Result<()> {
        self.ws.close(None).await?;
        Ok(())
    }
}

#[tokio::test]
async fn all_integration_tests() -> Result<()> {
    let harness = setup_server().await;

    println!("--- Running test_topic_messaging ---");
    test_topic_messaging(harness).await?;
    println!("--- Finished test_topic_messaging ---");

    println!("--- Running test_private_message ---");
    test_private_message(harness).await?;
    println!("--- Finished test_private_message ---");

    println!("--- Running test_global_message ---");
    test_global_message(harness).await?;
    println!("--- Finished test_global_message ---");

    Ok(())
}

async fn test_topic_messaging(harness: &TestHarness) -> Result<()> {
    let topic = &format!("cats-{}", Uuid::new_v4());

    let mut client1 = TestClient::new(harness.port, topic).await?;
    let mut client2 = TestClient::new(harness.port, topic).await?;

    for _ in 0..10 {
        if harness.client_manager.get_clients_by_topic(topic).len() == 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(harness.client_manager.get_clients_by_topic(topic).len(), 2);

    let msg_content = "meow";
    client1.send_message(topic, msg_content).await?;

    let received_msg = client2.recv().await?.expect("Did not receive message");
    if let ServerMessage::Topic { content, .. } = received_msg {
        assert_eq!(content, msg_content);
    } else {
        panic!("Incorrect message type received");
    }

    let result = tokio::time::timeout(Duration::from_millis(100), client1.recv()).await;
    assert!(
        result.is_err(),
        "Client 1 should not receive its own message"
    );

    client1.close().await?;
    client2.close().await?;
    Ok(())
}

async fn test_global_message(harness: &TestHarness) -> Result<()> {
    let topic1 = &format!("topic1-{}", Uuid::new_v4());
    let topic2 = &format!("topic2-{}", Uuid::new_v4());
    let mut client1 = TestClient::new(harness.port, topic1).await?;
    let mut client2 = TestClient::new(harness.port, topic2).await?;

    for _ in 0..10 {
        if harness.client_manager.get_clients_by_topic(topic1).len() == 1
            && harness.client_manager.get_clients_by_topic(topic2).len() == 1
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let global_content = "This is a message for everyone";
    let msg = ServerMessage::Global {
        id: Uuid::new_v4(),
        content: global_content.to_string(),
    };
    harness.client_manager.broadcast_global(msg).await;

    let received1 = client1
        .recv()
        .await?
        .expect("Client 1 did not receive message");
    if let ServerMessage::Global { content, .. } = received1 {
        assert_eq!(content, global_content);
    } else {
        panic!("Client 1 received incorrect message type");
    }

    let received2 = client2
        .recv()
        .await?
        .expect("Client 2 did not receive message");
    if let ServerMessage::Global { content, .. } = received2 {
        assert_eq!(content, global_content);
    } else {
        panic!("Client 2 received incorrect message type");
    }

    client1.close().await?;
    client2.close().await?;
    Ok(())
}

async fn test_private_message(harness: &TestHarness) -> Result<()> {
    let topic = &format!("general-{}", Uuid::new_v4());
    let mut client1 = TestClient::new(harness.port, topic).await?;

    let mut client1_id = None;
    for _ in 0..10 {
        let clients = harness.client_manager.get_clients_by_topic(topic);
        if !clients.is_empty() {
            assert_eq!(clients.len(), 1);
            client1_id = Some(clients[0].id);
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let client1_id = client1_id.expect("Client 1 not found in time");

    let mut client2 = TestClient::new(harness.port, topic).await?;

    for _ in 0..10 {
        if harness.client_manager.get_clients_by_topic(topic).len() == 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let private_content = "This is a secret";
    let msg = ServerMessage::Private {
        id: Uuid::new_v4(),
        content: private_content.to_string(),
    };
    harness
        .client_manager
        .send_private_message(client1_id, msg)
        .await;

    let received_msg = client1
        .recv()
        .await?
        .expect("Client 1 did not receive message");
    if let ServerMessage::Private { content, .. } = received_msg {
        assert_eq!(content, private_content);
    } else {
        panic!("Client 1 received incorrect message type");
    }

    let result = tokio::time::timeout(Duration::from_millis(100), client2.recv()).await;
    assert!(
        result.is_err(),
        "Client 2 should not receive the private message"
    );

    client1.close().await?;
    client2.close().await?;
    Ok(())
}
