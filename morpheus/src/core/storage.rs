use crate::core::msg::ServerMessage;
use async_trait::async_trait;
use dashmap::DashMap;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Represents a connected client's data stored on the server.
#[derive(Clone, Debug)]
pub struct Client {
    pub id: Uuid,
    pub topic: Option<String>,
    pub sender: mpsc::Sender<ServerMessage>,
}

/// A trait defining the contract for storing client and topic information.
/// This allows for different storage backends (e.g., in-memory, Redis).
#[async_trait]
pub trait Storage: Send + Sync {
    fn add_client(&self, client: Client);
    fn remove_client(&self, client_id: &Uuid) -> Option<Client>;
    fn get_client(&self, client_id: &Uuid) -> Option<Client>;
    fn get_all_clients(&self) -> Vec<Client>;
    fn subscribe_client_to_topic(&self, client_id: &Uuid, topic: String);
    fn get_clients_in_topic(&self, topic: &str) -> Vec<Client>;
    fn get_all_topics(&self) -> Vec<String>;
}

/// An in-memory storage implementation using DashMap for concurrent access.
pub struct InMemoryStorage {
    clients: DashMap<Uuid, Client>,
    topics: DashMap<String, Vec<Uuid>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            clients: DashMap::new(),
            topics: DashMap::new(),
        }
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Storage for InMemoryStorage {
    fn add_client(&self, client: Client) {
        self.clients.insert(client.id, client);
    }

    fn remove_client(&self, client_id: &Uuid) -> Option<Client> {
        if let Some((_, client)) = self.clients.remove(client_id) {
            if let Some(topic_name) = &client.topic {
                if let Some(mut topic_clients) = self.topics.get_mut(topic_name) {
                    topic_clients.retain(|id| id != client_id);
                }
            }
            Some(client)
        } else {
            None
        }
    }

    fn get_client(&self, client_id: &Uuid) -> Option<Client> {
        self.clients.get(client_id).map(|c| c.value().clone())
    }

    fn get_all_clients(&self) -> Vec<Client> {
        self.clients.iter().map(|c| c.value().clone()).collect()
    }

    fn subscribe_client_to_topic(&self, client_id: &Uuid, topic: String) {
        if let Some(mut client) = self.clients.get_mut(client_id) {
            // Remove from old topic if it exists
            if let Some(old_topic) = client.topic.take() {
                if let Some(mut clients) = self.topics.get_mut(&old_topic) {
                    clients.retain(|id| id != client_id);
                }
            }
            // Add to new topic
            client.topic = Some(topic.clone());
            self.topics.entry(topic).or_default().push(*client_id);
        }
    }

    fn get_clients_in_topic(&self, topic: &str) -> Vec<Client> {
        self.topics
            .get(topic)
            .map(|client_ids| {
                client_ids
                    .iter()
                    .filter_map(|id| self.clients.get(id).map(|c| c.value().clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn get_all_topics(&self) -> Vec<String> {
        self.topics
            .iter()
            .filter(|entry| !entry.value().is_empty())
            .map(|entry| entry.key().clone())
            .collect()
    }
}
