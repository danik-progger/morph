use crate::{
    core::{
        msg::ServerMessage,
        storage::{Client, Storage},
    },
    cli::ui,
};
use futures_util::{stream::SplitSink, SinkExt};
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;
use warp::ws::{Message, WebSocket};

/// A manager for clients that uses a generic storage backend.
pub struct ClientManager {
    storage: Arc<dyn Storage>,
}

impl ClientManager {
    /// Creates a new `ClientManager` with the given storage backend.
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self { storage }
    }

    /// Registers a new client, returning their unique ID.
    pub fn add_client(&self, mut sender: SplitSink<WebSocket, Message>) -> Uuid {
        let client_id = Uuid::new_v4();
        let (tx, mut rx) = mpsc::unbounded_channel::<ServerMessage>();

        // This task forwards messages from the manager to the client's WebSocket connection.
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                crate::log::middleware::log_outgoing(&message);
                let msg_str = serde_json::to_string(&message).unwrap_or_else(|e| {
                    eprintln!("Failed to serialize message: {}", e);
                    // Create a temporary error message if serialization fails
                    "{\"type\":\"Error\",\"message\":\"Internal server error: could not serialize message.\"}".to_string()
                });

                if sender.send(Message::text(msg_str)).await.is_err() {
                    // The client has disconnected. The read-half of the
                    // socket will detect this and trigger the cleanup.
                    break;
                }
            }
        });

        let new_client = Client {
            id: client_id,
            topic: None,
            sender: tx,
        };

        self.storage.add_client(new_client);
        client_id
    }

    /// Unregisters a client.
    pub fn remove_client(&self, client_id: &Uuid) {
        self.storage.remove_client(client_id);
        println!("Client {} disconnected.", client_id);
    }

    /// Subscribes a client to a specific topic.
    pub fn subscribe_client_to_topic(&self, client_id: &Uuid, topic: String) {
        self.storage.subscribe_client_to_topic(client_id, topic);
    }

    /// Sends a message to all clients in a specific topic, with an optional exclusion.
    pub async fn broadcast_to_topic(
        &self,
        topic_name: &str,
        message: ServerMessage,
        exclude_id: Option<Uuid>,
    ) {
        let clients = self.storage.get_clients_in_topic(topic_name);
        for client in clients {
            if exclude_id != Some(client.id) {
                self.send_message_to_client(&client.id, message.clone())
                    .await;
            }
        }
    }

    /// Sends a message to all connected clients.
    pub async fn broadcast_global(&self, message: ServerMessage) {
        let clients = self.storage.get_all_clients();
        for client in clients {
            self.send_message_to_client(&client.id, message.clone())
                .await;
        }
    }

    /// Sends a private message to a single client.
    pub async fn send_private_message(&self, client_id: Uuid, message: ServerMessage) {
        self.send_message_to_client(&client_id, message).await;
    }

    /// Helper to send a message to a client.
    async fn send_message_to_client(&self, client_id: &Uuid, message: ServerMessage) {
        if let Some(client) = self.storage.get_client(client_id) {
            if client.sender.send(message).is_err() {
                // The receiver is dropped, meaning the client is disconnected.
                // The cleanup is handled by the `remove_client` call in the ws::handler.
            }
        }
    }

    // // Getter methods for server-side CLI
    // pub fn get_client(&self, client_id: &Uuid) -> Option<Client> {
    //     self.storage.get_client(client_id)
    // }

    pub fn get_clients_by_topic(&self, topic: &str) -> Vec<Client> {
        self.storage.get_clients_in_topic(topic)
    }

    pub fn get_all_clients(&self) -> Vec<Client> {
        self.storage.get_all_clients()
    }

    pub fn get_all_topics(&self) -> Vec<String> {
        self.storage.get_all_topics()
    }

    /// Handles a message acknowledgment from a client.
    pub async fn handle_message_acknowledgment(&self, client_id: Uuid, msg_id: Uuid) {
        crate::log::middleware::log_ack(&client_id, &msg_id);
        ui::print_system_message(&format!(
            "Message {} acknowledged by client {}.",
            msg_id, client_id
        ));
        // Optionally, you could send a ServerMessage::MessageAcknowledged back to the client
        // or to other interested parties here.
    }
}

#[cfg(test)]
impl ClientManager {
    pub fn add_test_client(&self) -> (Uuid, mpsc::UnboundedReceiver<ServerMessage>) {
        let client_id = Uuid::new_v4();
        let (tx, rx) = mpsc::unbounded_channel();
        let client = Client {
            id: client_id,
            topic: None,
            sender: tx,
        };
        self.storage.add_client(client);
        (client_id, rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::storage::InMemoryStorage;
    use tokio::sync::mpsc::UnboundedReceiver;

    // Helper to create a mock client and return its ID and receiver
    fn setup_mock_client(manager: &ClientManager) -> (Uuid, UnboundedReceiver<ServerMessage>) {
        manager.add_test_client()
    }

    fn create_manager() -> ClientManager {
        let storage = Arc::new(InMemoryStorage::new());
        ClientManager::new(storage)
    }

    #[tokio::test]
    async fn test_remove_client() {
        let manager = create_manager();
        let (client_id, _rx) = setup_mock_client(&manager);
        let topic = "general".to_string();
        manager.subscribe_client_to_topic(&client_id, topic.clone());

        assert_eq!(manager.get_all_clients().len(), 1);
        assert_eq!(manager.get_clients_by_topic(&topic).len(), 1);

        manager.remove_client(&client_id);

        assert!(manager.get_all_clients().is_empty());
        assert!(manager.get_clients_by_topic(&topic).is_empty());
    }

    #[tokio::test]
    async fn test_send_private_message() {
        let manager = create_manager();
        let (client_id, mut rx) = setup_mock_client(&manager);

        let msg = ServerMessage::Private {
            id: Uuid::new_v4(),
            content: "Hello".to_string(),
        };

        manager.send_private_message(client_id, msg.clone()).await;

        let received = rx.recv().await.unwrap();
        assert_eq!(
            serde_json::to_string(&received).unwrap(),
            serde_json::to_string(&msg).unwrap()
        );
    }

    #[tokio::test]
    async fn test_broadcast_global() {
        let manager = create_manager();
        let (_client1_id, mut rx1) = setup_mock_client(&manager);
        let (_client2_id, mut rx2) = setup_mock_client(&manager);

        let msg = ServerMessage::Global {
            id: Uuid::new_v4(),
            content: "Global message".to_string(),
        };

        manager.broadcast_global(msg.clone()).await;

        let received1 = rx1.recv().await.unwrap();
        let received2 = rx2.recv().await.unwrap();

        assert_eq!(
            serde_json::to_string(&received1).unwrap(),
            serde_json::to_string(&msg).unwrap()
        );
        assert_eq!(
            serde_json::to_string(&received2).unwrap(),
            serde_json::to_string(&msg).unwrap()
        );
    }

    #[tokio::test]
    async fn test_broadcast_to_topic() {
        let manager = create_manager();
        let topic1 = "topic1".to_string();
        let topic2 = "topic2".to_string();

        let (client1_id, mut rx1) = setup_mock_client(&manager);
        manager.subscribe_client_to_topic(&client1_id, topic1.clone());

        let (client2_id, mut rx2) = setup_mock_client(&manager);
        manager.subscribe_client_to_topic(&client2_id, topic1.clone());

        let (client3_id, mut rx3) = setup_mock_client(&manager);
        manager.subscribe_client_to_topic(&client3_id, topic2.clone());

        let msg = ServerMessage::Topic {
            id: Uuid::new_v4(),
            topic: topic1.clone(),
            sender: "Morpheus".to_string(),
            content: "A message for topic1".to_string(),
        };

        manager.broadcast_to_topic(&topic1, msg.clone(), None).await;

        assert!(rx1.recv().await.is_some());
        assert!(rx2.recv().await.is_some());
        assert!(rx3.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_broadcast_to_topic_with_exclusion() {
        let manager = create_manager();
        let topic1 = "topic1".to_string();

        let (client1_id, mut rx1) = setup_mock_client(&manager);
        manager.subscribe_client_to_topic(&client1_id, topic1.clone());

        let (client2_id, mut rx2) = setup_mock_client(&manager);
        manager.subscribe_client_to_topic(&client2_id, topic1.clone());

        let msg = ServerMessage::Topic {
            id: Uuid::new_v4(),
            topic: topic1.clone(),
            sender: client1_id.to_string(),
            content: "A message from client1".to_string(),
        };

        manager
            .broadcast_to_topic(&topic1, msg.clone(), Some(client1_id))
            .await;

        assert!(rx1.try_recv().is_err());
        assert!(rx2.recv().await.is_some());
    }
}
