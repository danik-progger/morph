use crate::{
    cli::{commands, ui},
    core::msg::ClientMessage,
    ws::conn::Connection,
};
use tokio::io::{AsyncBufRead, AsyncBufReadExt};
use url::Url;

/// The main client structure.
pub struct Client {
    topic: String,
    pub connection: Connection,
}

impl Client {
    /// Creates a new client and connects to the server.
    pub async fn new(
        url: Url,
        topic: String,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let connection = Connection::connect(url).await?;
        Ok(Self { topic, connection })
    }

    /// Runs the main client loop.
    pub async fn run<R: AsyncBufRead + Unpin>(
        &mut self,
        input_reader: &mut R,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Send the initial connection message
        self.connection
            .send(ClientMessage::Connect {
                topic: self.topic.clone(),
            })
            .await?;

        ui::print_system_message(&format!(
            "Connected to topic '{}'. Type /help for commands.",
            self.topic
        ));

        let mut input_buf = String::new();

        loop {
            tokio::select! {
                // Handle incoming messages from the server
                Some(Ok(msg)) = self.connection.recv() => {
                    if let Some(msg_id) = ui::print_server_message(&msg) {
                        // Send acknowledgment back to the server
                        self.connection.send(ClientMessage::MessageReceived { msg_id }).await?;
                    }
                },
                // Handle user input from the command line
                result = input_reader.read_line(&mut input_buf) => {
                    match result {
                        Ok(0) => break, // EOF
                        Ok(_) => {
                            let content = input_buf.trim();
                            if !content.is_empty() {
                                self.handle_user_input(content).await?;
                            }
                            input_buf.clear();
                            ui::print_prompt();
                        }
                        Err(e) => {
                            ui::print_error(&format!("Error reading from stdin: {}", e));
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Handles user input from the command line.
    pub async fn handle_user_input(
        &mut self,
        input: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match commands::parse_command(input) {
            commands::Command::Message(content) => {
                let message = ClientMessage::Message {
                    topic: self.topic.clone(),
                    content,
                };
                self.connection.send(message).await?;
            }
            commands::Command::Reply { msg_id, content } => {
                let message = ClientMessage::ReplyToMorpheus {
                    original_msg_id: msg_id,
                    content,
                };
                self.connection.send(message).await?;
            }
            commands::Command::Help => {
                let help_text = "Commands:\n/h, /help                  - Show this help message\n/m, /msg <text>            - Send a message to the current topic\n/r, /reply <msg_id> <text> - Reply to a message";
                ui::print_system_message(help_text);
            }
            commands::Command::Unknown(error_msg) => {
                ui::print_error(&error_msg);
            }
        }
        Ok(())
    }
}