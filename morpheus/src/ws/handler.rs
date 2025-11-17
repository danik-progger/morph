use crate::core::{
    client_manager::ClientManager,
    msg::{ClientMessage, ServerMessage},
};
use futures_util::StreamExt;
use std::sync::Arc;
use uuid::Uuid;
use warp::ws::{Message, WebSocket};

pub async fn client_connected(ws: WebSocket, client_manager: Arc<ClientManager>) {
    let (ws_sender, mut ws_receiver) = ws.split();

    // Use an unbounded channel to handle messages from the client manager
    let client_id = client_manager.add_client(ws_sender);
    println!("Client {} connected.", client_id);

    // This loop handles messages received from the client
    while let Some(result) = ws_receiver.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("Error receiving message from client {}: {}", client_id, e);
                break;
            }
        };
        handle_message(&client_id, msg, &client_manager).await;
    }

    // Client disconnected
    client_manager.remove_client(&client_id);
}

async fn handle_message(client_id: &Uuid, msg: Message, client_manager: &Arc<ClientManager>) {
    if let Ok(text) = msg.to_str() {
        match serde_json::from_str::<ClientMessage>(text) {
            Ok(client_message) => match client_message {
                ClientMessage::Connect { topic } => {
                    println!("Client {} subscribing to topic '{}'", client_id, topic);
                    client_manager.subscribe_client_to_topic(client_id, topic);
                }
                ClientMessage::Message { topic, content } => {
                    println!("Client {} sent message to topic '{}'", client_id, topic);
                    let message = ServerMessage::Topic {
                        id: Uuid::new_v4(),
                        topic: topic.clone(),
                        sender: client_id.to_string(),
                        content,
                    };
                    // Broadcast to topic, excluding the sender
                    client_manager
                        .broadcast_to_topic(&topic, message, Some(*client_id))
                        .await;
                }
                ClientMessage::ReplyToMorpheus {
                    original_msg_id,
                    content,
                } => {
                    // For now, just print replies to the server console
                    println!(
                        "\n[REPLY to {} from {}]: {}",
                        original_msg_id, client_id, content
                    );
                }
            },
            Err(e) => {
                eprintln!(
                    "Error deserializing message from client {}: {}",
                    client_id, e
                );
                let error_msg = ServerMessage::Error {
                    message: "Invalid message format".to_string(),
                };
                client_manager
                    .send_private_message(*client_id, error_msg)
                    .await;
            }
        }
    }
}
