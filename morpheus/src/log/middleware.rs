use crate::core::msg::{ClientMessage, ServerMessage};
use chrono::Local;
use tracing::info;

pub fn init_file_logger() {
    let log_dir = "logs";
    if !std::path::Path::new(log_dir).exists() {
        std::fs::create_dir(log_dir).expect("Failed to create log directory");
    }
    let now = Local::now();
    let log_filename = format!("morpheus-{}.log", now.format("%Y-%m-%d-%H-%M-%S"));
    let log_path = std::path::Path::new(log_dir).join(log_filename);
    let log_file = std::fs::File::create(log_path).expect("Failed to create log file");

    tracing_subscriber::fmt()
        .with_writer(log_file)
        .with_ansi(false) // ANSI codes are not useful in a file
        .init();
}

pub fn log_incoming(client_id: &uuid::Uuid, msg: &ClientMessage) {
    let msg_json = serde_json::to_string(msg).unwrap_or_else(|_| "Failed to serialize".to_string());
    info!
        (target: "morpheus::log",
        "INCOMING from [{}]:\n{}",
        client_id,
        msg_json
    );
}

pub fn log_outgoing(msg: &ServerMessage) {
    let msg_json = serde_json::to_string(msg).unwrap_or_else(|_| "Failed to serialize".to_string());
    info!(target: "morpheus::log", "OUTGOING:\n{}", msg_json);
}

pub fn log_ack(client_id: &uuid::Uuid, msg_id: &uuid::Uuid) {
    info!
        (target: "morpheus::log",
        "ACK from [{}]: msg_id={}",
        client_id,
        msg_id
    );
}
