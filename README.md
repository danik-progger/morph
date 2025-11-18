# 🌐 Morpheus Communication System

A communication system inspired by the Matrix movie, allowing Morpheus and his followers to communicate securely on various topics. 🔐

## Overview 📋

This project consists of two main applications:
- **Morpheus Server 🖥️**: A central server that manages client connections and message routing
- **Neo Client 💻**: A client application that connects to the server and participates in topic-based communication

## Features ✨

### Morpheus Server 🖥️
- 🔌 WebSocket-based server for client connections
- 📢 Topic-based messaging system
- 👥 Client management and tracking
- ⌨️ Command-line interface for server administration
- 📨 Message routing (global, topic-specific, and private messages)
- ✅ Message acknowledgment and delivery confirmation
- 📝 Logging functionality

### Neo Client 💻
- 🔌 WebSocket-based client for connecting to the server
- 📌 Topic subscription capability
- 📢 Topic-based messaging
- 💬 Reply functionality to Morpheus messages
- ✅ Message acknowledgment to confirm receipt

## Prerequisites 🛠️

- 🦀 Rust
- 📦 Cargo

## Getting Started 🚀

### Running the Morpheus Server 🖥️

1. Navigate to the morpheus directory:
   ```bash
   cd morpheus
   ```

2. Run the server:
   ```bash
   cargo run
   ```

   or with options

   ```bash
   cargo run -- --address 127.0.0.1 --port 8080
   ```

   Available options:
   - `--address <IP>`: IP address to bind to (default: 127.0.0.1) 🌐
   - `--port <PORT>`: Port to listen on (default: 8080) 🌐

3. The server will start and display a command prompt where you can issue server commands.

### Running the Neo Client 💻

1. First, ensure the Morpheus server is running. ✅

2. In a new terminal, navigate to the neo directory:
   ```bash
   cd neo
   ```

3. Run the client:
   ```bash
   cargo run -- --address ws://127.0.0.1:8080 --topic general
   ```

   Available options:
   - `--address <ADDRESS>`: Server address to connect to (e.g., ws://127.0.0.1:8080) 🌐
   - `--topic <TOPIC>`: Topic to subscribe to (e.g., "general", "resistance", etc.) 📌

## Server Commands ⌨️

Once the Morpheus server is running, you can use the following commands:

- `/help` or `/h` 🆘 - Show all commands
- `/list` or `/l` 👥 - List all connected clients
- `/list all` 👥 - List all connected clients (same as `/list`)
- `/list topics` 📚 - List all active topics
- `/list <topic>` 👥 - List clients in a specific topic
- `/global <message>` or `/g <message>` 📢 - Send a message to all clients
- `/topic <topic> <message>` or `/t <topic> <message>` 📢 - Send a message to a specific topic
- `/private <client_id> <message>` or `/p <client_id> <message>` 💬 - Send a private message to a specific client
- `/exit` or `/e` 🚪 - Shutdown the server

## Client Commands 💬

Once the Neo client is connected, you can use the following commands:

- `Type any text` 📝 - Send a message to the current topic
- `/msg <message>` or `/m <message>` 📝 - Send a message to the current topic
- `/reply <msg_id> <message>` or `/r <msg_id> <message>` 💬 - Reply to a specific message from Morpheus
- `/help` or `/h` 🆘 - Show available commands

## Security Features 🔐

- ⚠️ Clients can only respond to messages from Morpheus, not initiate direct communication
- ✅ Message acknowledgment system for delivery confirmation
- 👤 Client identification and tracking
- 🛡️ Topic-based message isolation

## Architecture 🏗️

The system uses:
- 🔌 WebSocket protocol for real-time communication
- ⚡ Asynchronous Rust with Tokio runtime for high performance
- 📦 JSON-based message serialization
- 🔢 UUIDs for unique client and message identification
- 💾 In-memory storage for client management

## Testing 🧪

Run the tests for both applications:

```bash
# For morpheus server
cd morpheus
cargo test

# For neo client
cd neo
cargo test
```

## Project Structure 📁

```
morpheus/ - Server application 🖥️
├── src/
│   ├── cli/          - Command line interface ⌨️
│   ├── core/         - Core business logic 🔧
│   ├── log/          - Logging middleware 📝
│   └── ws/           - WebSocket handling 🔌
neo/ - Client application 💻
├── src/
│   ├── cli/          - Command line interface ⌨️
│   ├── core/         - Core business logic 🔧
│   └── ws/           - WebSocket handling 🔌
```

## GitHub Actions 🤖

The project includes GitHub Actions workflows for:
- 🔄 Continuous Integration (build and test)
- ✨ Code formatting checks
- 🔍 Code linting (clippy)
- 🛡️ Security auditing
- 🌍 Cross-platform testing
