use crate::core::msg::ServerMessage;
use std::io::{self, Write};
use uuid::Uuid;

/// Handles rendering messages and prompts to the console.
pub fn print_prompt() {
    print!("> ");
    io::stdout().flush().unwrap();
}

pub fn print_server_message(msg: &ServerMessage) -> Option<Uuid> {
    let mut msg_id_to_ack: Option<Uuid> = None;
    match msg {
        ServerMessage::Global { id, content } => {
            println!("\n[GLOBAL] (id: {})", id);
            println!("{}", content);
            msg_id_to_ack = Some(*id);
        }
        ServerMessage::Topic {
            id,
            topic,
            sender,
            content,
        } => {
            println!("\n[TOPIC:{}] (from: {}, id: {})", topic, sender, id);
            println!("{}", content);
            msg_id_to_ack = Some(*id);
        }
        ServerMessage::Private { id, content } => {
            println!("\n[PRIVATE] (id: {})", id);
            println!("{}", content);
            msg_id_to_ack = Some(*id);
        }
        ServerMessage::Error { message } => {
            eprintln!("\n[SERVER ERROR] {}", message);
        }
        ServerMessage::MessageDelivered { msg_id } => {
            println!("\n[SYSTEM] Message {} delivered.", msg_id);
        }
        ServerMessage::MessageAcknowledged { msg_id, client_id } => {
            println!("\n[SYSTEM] Message {} acknowledged by client {}.", msg_id, client_id);
        }
    }
    print_prompt();
    msg_id_to_ack
}

pub fn print_system_message(msg: &str) {
    println!("\n[SYSTEM] {}", msg);
    print_prompt();
}

pub fn print_error(msg: &str) {
    eprintln!("\n[ERROR] {}", msg);
    print_prompt();
}
