//! IDApTIK's transport/error adapter around the canonical Burble Phoenix
//! Channels client.
//!
//! The game state machines keep depending on [`SessionTransport`] and
//! [`NetError`], while Phoenix V2 framing, reply correlation, broadcast
//! buffering, topic filtering, and heartbeats are supplied by
//! `burble-client`. This keeps the game independent of the concrete socket
//! without maintaining a second protocol implementation.

use crate::error::NetError;
use crate::transport::SessionTransport;
use serde_json::Value;
use std::time::Duration;

pub use burble_client::Broadcast;

struct TransportAdapter<T>(T);

impl<T: SessionTransport> burble_client::TextTransport for TransportAdapter<T> {
    async fn send_text(&mut self, text: String) -> Result<(), burble_client::Error> {
        self.0
            .send_text(text)
            .await
            .map_err(|error| burble_client::Error::Protocol(error.to_string()))
    }

    async fn recv_text(&mut self) -> Result<Option<String>, burble_client::Error> {
        self.0
            .recv_text()
            .await
            .map_err(|error| burble_client::Error::Protocol(error.to_string()))
    }

    async fn close(&mut self) -> Result<(), burble_client::Error> {
        self.0
            .close()
            .await
            .map_err(|error| burble_client::Error::Protocol(error.to_string()))
    }
}

/// A single-channel Phoenix client backed by Burble's embeddable client.
pub struct PhoenixClient<T: SessionTransport> {
    inner: burble_client::PhoenixClient<TransportAdapter<T>>,
}

impl<T: SessionTransport> PhoenixClient<T> {
    pub fn new(transport: T) -> Self {
        Self {
            inner: burble_client::PhoenixClient::new(TransportAdapter(transport)),
        }
    }

    pub async fn join(&mut self, topic: &str, params: Value) -> Result<Value, NetError> {
        Ok(self.inner.join(topic, params).await?)
    }

    pub async fn push(&mut self, event: &str, payload: Value) -> Result<Value, NetError> {
        Ok(self.inner.push(event, payload).await?)
    }

    pub async fn next_broadcast(&mut self, wait: Duration) -> Result<Option<Broadcast>, NetError> {
        Ok(self.inner.next_broadcast(wait).await?)
    }

    pub async fn leave(&mut self) {
        self.inner.leave().await;
    }
}
