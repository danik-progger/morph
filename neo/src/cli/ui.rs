use crate::core::msg::ServerMessage;
use std::io::{self, Write};
use uuid::Uuid;

/// Handles rendering messages and prompts to the console.
pub fn print_prompt() {
    print!("\n> ");
    io::stdout().flush().unwrap();
}

pub fn print_server_message(msg: &ServerMessage) -> Option<Uuid> {
    let mut msg_id_to_ack: Option<Uuid> = None;
    match msg {
        ServerMessage::Global { id, content } => {
            println!("\n[GLOBAL] (id: {})\n", id);
            println!("{}", content);
            msg_id_to_ack = Some(*id);
        }
        ServerMessage::Topic {
            id,
            topic,
            sender,
            content,
        } => {
            println!("\n[TOPIC:{}] (from: {}, id: {})\n", topic, sender, id);
            println!("{}", content);
            msg_id_to_ack = Some(*id);
        }
        ServerMessage::Private { id, content } => {
            println!("\n[PRIVATE] (id: {})\n", id);
            println!("{}", content);
            msg_id_to_ack = Some(*id);
        }
        ServerMessage::Error { message } => {
            eprintln!("\n[SERVER ERROR] {}\n", message);
        }
        ServerMessage::MessageDelivered { msg_id } => {
            println!("\n[SYSTEM] Message {} delivered\n", msg_id);
        }
        ServerMessage::MessageAcknowledged { msg_id, client_id } => {
            println!("\n[SYSTEM] Message {} acknowledged by client {}]\n", msg_id, client_id);
        }
    }
    print_prompt();
    msg_id_to_ack
}

pub fn print_system_message(msg: &str) {
    println!("\n[SYSTEM] {}\n", msg);
    print_prompt();
}

pub fn print_error(msg: &str) {
    eprintln!("\n[ERROR] {}\n", msg);
    print_prompt();
}
