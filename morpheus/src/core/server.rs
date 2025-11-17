use crate::{
    cli::{commands, ui},
    core::{client_manager::ClientManager, msg::ServerMessage},
};
use std::sync::Arc;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use uuid::Uuid;

/// The main server structure that handles CLI commands.
pub struct Server {
    client_manager: Arc<ClientManager>,
}

impl Server {
    pub fn new(client_manager: Arc<ClientManager>) -> Self {
        Self { client_manager }
    }

    /// Runs the main CLI loop for the server.
    pub async fn run_cli(&self) {
        ui::print_prompt();
        let mut stdin = BufReader::new(io::stdin());
        let mut line = String::new();

        loop {
            line.clear();
            if stdin.read_line(&mut line).await.is_err() {
                ui::print_error("Could not read from stdin.");
                break;
            }

            match commands::parse_command(line.trim()) {
                commands::Command::Help => {
                    let help_text = r#"Morpheus Server Commands:
/list all                - List all connected clients
/list topics             - List all active topics
/list <topic>            - List clients in a specific topic
/global <msg>            - Send a message to all clients
/topic <topic> <msg>     - Send a message to a topic
/private <client_id> <msg> - Send a private message
/exit                    - Shutdown the server"#;
                    ui::print_system_message(help_text);
                }
                commands::Command::List(scope) => self.handle_list_command(scope),
                commands::Command::Global(content) => self.handle_global_command(content).await,
                commands::Command::Topic { topic, content } => {
                    self.handle_topic_command(topic, content).await
                }
                commands::Command::Private { client_id, content } => {
                    self.handle_private_command(client_id, content).await
                }
                commands::Command::Exit => {
                    ui::print_system_message("Shutting down...");
                    std::process::exit(0);
                }
                commands::Command::Unknown(err) if !err.is_empty() => ui::print_error(&err),
                _ => ui::print_prompt(),
            }
        }
    }

    fn handle_list_command(&self, scope: commands::ListScope) {
        match scope {
            commands::ListScope::All => {
                println!("\nAll connected clients:");
                for client in self.client_manager.get_all_clients() {
                    println!(
                        "- {} (Topic: {})",
                        client.id,
                        client.topic.as_deref().unwrap_or("None")
                    );
                }
            }
            commands::ListScope::Topics => {
                println!("\nActive topics:");
                for topic in self.client_manager.get_all_topics() {
                    println!("- {}", topic);
                }
            }
            commands::ListScope::Topic(topic) => {
                println!("\nClients in topic '{}':", topic);
                for client in self.client_manager.get_clients_by_topic(&topic) {
                    println!("- {}", client.id);
                }
            }
        }
        ui::print_prompt();
    }

    async fn handle_global_command(&self, content: String) {
        let msg = ServerMessage::Global {
            id: Uuid::new_v4(),
            content: content.clone(),
        };
        self.client_manager.broadcast_global(msg).await;
        ui::print_confirmation(&format!("Global message sent: {}", content));
    }

    async fn handle_topic_command(&self, topic: String, content: String) {
        let msg = ServerMessage::Topic {
            id: Uuid::new_v4(),
            topic: topic.clone(),
            sender: "Morpheus".to_string(),
            content: content.clone(),
        };
        self.client_manager
            .broadcast_to_topic(&topic, msg, None)
            .await;
        ui::print_confirmation(&format!("Message sent to topic '{}': {}", topic, content));
    }

    async fn handle_private_command(&self, client_id: Uuid, content: String) {
        let msg_id = Uuid::new_v4();
        let msg = ServerMessage::Private {
            id: msg_id,
            content: content.clone(),
        };
        self.client_manager
            .send_private_message(client_id, msg)
            .await;
        ui::print_confirmation(&format!(
            "Private message (id: {}) sent to {}: {}",
            msg_id, client_id, content
        ));
    }
}
