use crate::core::msg::{ClientMessage, ServerMessage};
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Error as WsError, Message},
    MaybeTlsStream, WebSocketStream,
};
use url::Url;

type WsSink = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
type WsStream = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

/// Represents a WebSocket connection to the server.
pub struct Connection {
    write: WsSink,
    read: WsStream,
}

impl Connection {
    /// Attempts to connect to the specified URL.
    pub async fn connect(url: Url) -> Result<Self, WsError> {
        let (ws_stream, _) = connect_async(url).await?;
        let (write, read) = ws_stream.split();
        Ok(Self { write, read })
    }

    /// Sends a `ClientMessage` to the server.
    pub async fn send(&mut self, msg: ClientMessage) -> Result<(), WsError> {
        let json_msg = serde_json::to_string(&msg).unwrap();
        self.write.send(Message::Text(json_msg)).await
    }

    /// Receives a `ServerMessage` from the server.
    /// Returns `None` if the connection is closed.
    pub async fn recv(&mut self) -> Option<Result<ServerMessage, serde_json::Error>> {
        loop {
            match self.read.next().await {
                Some(Ok(Message::Text(text))) => return Some(serde_json::from_str(&text)),
                Some(Ok(Message::Close(_))) => return None,
                Some(Err(_)) => return None,
                Some(Ok(_)) => continue, // Ignore other message types
                None => return None,     // Stream is closed
            }
        }
    }
}
