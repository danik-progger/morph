use crate::{
    cli::{commands, ui},
    core::msg::ClientMessage,
    ws::conn::Connection,
};
use tokio::io::{self as tokio_io, AsyncBufReadExt, BufReader};
use url::Url;

/// The main client structure.
pub struct Client {
    topic: String,
    connection: Connection,
}

impl Client {
    /// Creates a new client and connects to the server.
    pub async fn new(url: Url, topic: String) -> Result<Self, Box<dyn std::error::Error>> {
        let connection = Connection::connect(url).await?;
        Ok(Self { topic, connection })
    }

    /// Runs the main client loop.
    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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

        let mut stdin = BufReader::new(tokio_io::stdin());
        let mut input_buf = String::new();

        loop {
            tokio::select! {
                // Handle incoming messages from the server
                Some(Ok(msg)) = self.connection.recv() => {
                    ui::print_server_message(&msg);
                },
                // Handle user input from the command line
                result = stdin.read_line(&mut input_buf) => {
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
    async fn handle_user_input(&mut self, input: &str) -> Result<(), Box<dyn std::error::Error>> {
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
