//! `PlainWebSocketTransport` — `tokio-tungstenite` over TCP.
//!
//! Per ADR-0006 §2 this is "the test transport and the fallback transport,
//! forever". Under `wasm32`/Route A the same trait gets a browser-`WebSocket`
//! implementation instead; that lands with the gossamer wrap slice.

use crate::error::NetError;
use crate::transport::SessionTransport;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

/// A plain WebSocket pipe (ws:// or wss://).
pub struct PlainWebSocketTransport {
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl PlainWebSocketTransport {
    /// Connect to a WebSocket URL (e.g. `ws://127.0.0.1:4000/socket/websocket`).
    /// The Phoenix `vsn=2.0.0` query parameter is appended if absent, so callers
    /// pass the endpoint and the protocol layer stays in one place.
    pub async fn connect(url: &str) -> Result<Self, NetError> {
        let url = if url.contains("vsn=") {
            url.to_owned()
        } else if url.contains('?') {
            format!("{url}&vsn=2.0.0")
        } else {
            format!("{url}?vsn=2.0.0")
        };
        let (stream, _response) = connect_async(&url)
            .await
            .map_err(|e| NetError::Transport(format!("connect {url}: {e}")))?;
        Ok(Self { stream })
    }
}

impl SessionTransport for PlainWebSocketTransport {
    async fn send_text(&mut self, text: String) -> Result<(), NetError> {
        self.stream
            .send(Message::text(text))
            .await
            .map_err(|e| NetError::Transport(format!("send: {e}")))
    }

    async fn recv_text(&mut self) -> Result<Option<String>, NetError> {
        // Pings are answered by tungstenite when the stream is polled; pongs
        // and other control frames are simply not ours to surface.
        while let Some(frame) = self.stream.next().await {
            match frame.map_err(|e| NetError::Transport(format!("recv: {e}")))? {
                Message::Text(t) => return Ok(Some(t.to_string())),
                Message::Binary(_) => {
                    return Err(NetError::Protocol(
                        "unexpected binary frame on the Phoenix socket".into(),
                    ));
                }
                Message::Close(_) => return Ok(None),
                Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => continue,
            }
        }
        Ok(None)
    }

    async fn close(&mut self) -> Result<(), NetError> {
        // Best-effort: a peer that already vanished is a closed pipe, not an error.
        let _ = self.stream.close(None).await;
        Ok(())
    }
}
