use crate::core::msg::ServerMessage;
use std::io::{self, Write};

/// Handles rendering messages and prompts to the console.
pub fn print_prompt() {
    print!("> ");
    io::stdout().flush().unwrap();
}

pub fn print_server_message(msg: &ServerMessage) {
    match msg {
        ServerMessage::Global { id, content } => {
            println!("\n[GLOBAL] (id: {})", id);
            println!("{}", content);
        }
        ServerMessage::Topic {
            id,
            topic,
            sender,
            content,
        } => {
            println!("\n[TOPIC:{}] (from: {}, id: {})", topic, sender, id);
            println!("{}", content);
        }
        ServerMessage::Private { id, content } => {
            println!("\n[PRIVATE] (id: {})", id);
            println!("{}", content);
        }
        ServerMessage::Error { message } => {
            eprintln!("\n[SERVER ERROR] {}", message);
        }
        ServerMessage::MessageDelivered { msg_id } => {
            println!("\n[SYSTEM] Message {} delivered.", msg_id);
        }
    }
    print_prompt();
}

pub fn print_system_message(msg: &str) {
    println!("\n[SYSTEM] {}", msg);
    print_prompt();
}

pub fn print_error(msg: &str) {
    eprintln!("\n[ERROR] {}", msg);
    print_prompt();
}
