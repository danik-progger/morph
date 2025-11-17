use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Messages sent from the client to the server.
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Initial message to connect and subscribe to a topic.
    Connect { topic: String },
    /// A message sent to a topic.
    Message { topic: String, content: String },
    /// A private reply to a message from Morpheus.
    ReplyToMorpheus {
        /// The ID of the message being replied to.
        original_msg_id: Uuid,
        content: String,
    },
    /// Acknowledgment that a message was received by the client.
    MessageReceived { msg_id: Uuid },
}

/// Messages sent from the server to the client.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// A global message from Morpheus to all clients.
    Global { id: Uuid, content: String },
    /// A message sent to a specific topic.
    Topic {
        id: Uuid,
        topic: String,
        /// The sender of the message.
        sender: String,
        content: String,
    },
    /// A private message from Morpheus.
    Private { id: Uuid, content: String },
    /// Confirmation that a private message was delivered.
    MessageDelivered { msg_id: Uuid },
    /// Acknowledgment that a message was received by a client.
    MessageAcknowledged { msg_id: Uuid, client_id: Uuid },
    /// An error message from the server.
    Error { message: String },
}
