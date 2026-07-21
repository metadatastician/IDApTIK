//! A minimal Phoenix Channels client (V2 JSON serializer) over any
//! [`SessionTransport`].
//!
//! The V2 wire frame is a five-element JSON array:
//! `[join_ref, ref, topic, event, payload]` — refs are strings (or null), the
//! payload is arbitrary JSON. Replies come back as `phx_reply` events carrying
//! `{"status": "ok" | "error", "response": ...}` with the pushing frame's ref;
//! everything else on the topic is a broadcast. Liveness is a `heartbeat` push
//! on the reserved `"phoenix"` topic — the server drops sockets that go silent
//! (60 s default), so a client waiting out a long grace period must keep
//! heartbeating.
//!
//! One channel per client: the session slice joins exactly one
//! `session:<id>` topic (ADR-0006 §3), so multiplexing is complexity with no
//! consumer. Replies are awaited in-line (pushes are sequential in the seat
//! state machine); broadcasts that arrive while awaiting a reply are buffered,
//! never dropped.

use crate::error::NetError;
use crate::transport::SessionTransport;
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::time::Duration;
use tokio::time::{Instant, timeout};

/// How long a push may wait for its `phx_reply` before the client gives up.
const REPLY_TIMEOUT: Duration = Duration::from_secs(10);

/// Outbound silence threshold after which a heartbeat is sent. Phoenix's
/// default socket timeout is 60 s; half of half leaves comfortable margin.
const HEARTBEAT_AFTER: Duration = Duration::from_secs(15);

/// A non-reply message pushed to us on the joined topic.
#[derive(Debug, Clone)]
pub struct Broadcast {
    pub event: String,
    pub payload: Value,
}

/// A single-channel Phoenix client over a [`SessionTransport`].
pub struct PhoenixClient<T: SessionTransport> {
    transport: T,
    topic: Option<String>,
    join_ref: Option<String>,
    next_ref: u64,
    pending: VecDeque<Broadcast>,
    outstanding_heartbeat: Option<String>,
    last_send: Instant,
}

impl<T: SessionTransport> PhoenixClient<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            topic: None,
            join_ref: None,
            next_ref: 0,
            pending: VecDeque::new(),
            outstanding_heartbeat: None,
            last_send: Instant::now(),
        }
    }

    fn take_ref(&mut self) -> String {
        self.next_ref += 1;
        self.next_ref.to_string()
    }

    async fn send_frame(
        &mut self,
        join_ref: Option<&str>,
        msg_ref: &str,
        topic: &str,
        event: &str,
        payload: Value,
    ) -> Result<(), NetError> {
        let frame = json!([join_ref, msg_ref, topic, event, payload]);
        let text = serde_json::to_string(&frame)
            .map_err(|e| NetError::Protocol(format!("encode frame: {e}")))?;
        self.transport.send_text(text).await?;
        self.last_send = Instant::now();
        Ok(())
    }

    /// Read and decode one frame: `(ref, topic, event, payload)`.
    async fn read_frame(&mut self) -> Result<(Option<String>, String, String, Value), NetError> {
        let Some(text) = self.transport.recv_text().await? else {
            return Err(NetError::Closed);
        };
        let value: Value = serde_json::from_str(&text)
            .map_err(|e| NetError::Protocol(format!("decode frame: {e}")))?;
        let Value::Array(mut parts) = value else {
            return Err(NetError::Protocol(format!("frame is not an array: {text}")));
        };
        if parts.len() != 5 {
            return Err(NetError::Protocol(format!(
                "frame has {} elements, expected 5: {text}",
                parts.len()
            )));
        }
        let payload = parts.pop().expect("length checked");
        let event = as_opt_string(parts.pop().expect("length checked"))
            .ok_or_else(|| NetError::Protocol("frame event is not a string".into()))?;
        let topic = as_opt_string(parts.pop().expect("length checked"))
            .ok_or_else(|| NetError::Protocol("frame topic is not a string".into()))?;
        let msg_ref = as_opt_string(parts.pop().expect("length checked"));
        Ok((msg_ref, topic, event, payload))
    }

    /// Wait for the `phx_reply` matching `wanted`, buffering broadcasts that
    /// arrive first. Returns the reply's `response` on `"ok"` status.
    async fn await_reply(&mut self, wanted: &str) -> Result<Value, NetError> {
        let deadline = Instant::now() + REPLY_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(NetError::Timeout("phx_reply"));
            }
            let frame = timeout(remaining, self.read_frame())
                .await
                .map_err(|_| NetError::Timeout("phx_reply"))??;
            match self.classify(frame)? {
                Classified::Reply { msg_ref, response } if msg_ref == wanted => {
                    return response;
                }
                Classified::Reply { .. } | Classified::Consumed => continue,
                Classified::Broadcast(b) => self.pending.push_back(b),
            }
        }
    }

    /// Sort one frame into reply / heartbeat-ack / broadcast, and turn channel
    /// terminations (`phx_error` / `phx_close`) into errors at the seam.
    fn classify(
        &mut self,
        (msg_ref, _topic, event, payload): (Option<String>, String, String, Value),
    ) -> Result<Classified, NetError> {
        match event.as_str() {
            "phx_reply" => {
                if let (Some(r), Some(hb)) = (&msg_ref, &self.outstanding_heartbeat)
                    && r == hb
                {
                    self.outstanding_heartbeat = None;
                    return Ok(Classified::Consumed);
                }
                let msg_ref =
                    msg_ref.ok_or_else(|| NetError::Protocol("phx_reply without a ref".into()))?;
                let status = payload.get("status").and_then(Value::as_str).unwrap_or("");
                let response = payload.get("response").cloned().unwrap_or(Value::Null);
                let response = if status == "ok" {
                    Ok(response)
                } else {
                    Err(NetError::Session(format!(
                        "push rejected ({status}): {response}"
                    )))
                };
                Ok(Classified::Reply { msg_ref, response })
            }
            "phx_error" => Err(NetError::Session("channel errored (phx_error)".into())),
            "phx_close" => Err(NetError::Closed),
            _ => Ok(Classified::Broadcast(Broadcast { event, payload })),
        }
    }

    /// Join `topic`. One join per client; the join ref outlives the call.
    pub async fn join(&mut self, topic: &str, params: Value) -> Result<Value, NetError> {
        if self.topic.is_some() {
            return Err(NetError::Session("already joined a topic".into()));
        }
        let msg_ref = self.take_ref();
        self.send_frame(Some(&msg_ref.clone()), &msg_ref, topic, "phx_join", params)
            .await?;
        let response = self.await_reply(&msg_ref).await?;
        self.topic = Some(topic.to_owned());
        self.join_ref = Some(msg_ref);
        Ok(response)
    }

    /// Push `event` on the joined topic and await its reply's `response`.
    pub async fn push(&mut self, event: &str, payload: Value) -> Result<Value, NetError> {
        let topic = self
            .topic
            .clone()
            .ok_or_else(|| NetError::Session("push before join".into()))?;
        let join_ref = self.join_ref.clone();
        let msg_ref = self.take_ref();
        self.send_frame(join_ref.as_deref(), &msg_ref, &topic, event, payload)
            .await?;
        self.await_reply(&msg_ref).await
    }

    /// Next broadcast on the joined topic, waiting up to `wait`. `Ok(None)`
    /// means the wait elapsed quietly. Heartbeats are interleaved so a long
    /// wait (a `PeerLost` grace period) cannot look like a dead socket.
    pub async fn next_broadcast(&mut self, wait: Duration) -> Result<Option<Broadcast>, NetError> {
        if let Some(b) = self.pending.pop_front() {
            return Ok(Some(b));
        }
        let deadline = Instant::now() + wait;
        loop {
            if self.outstanding_heartbeat.is_none() && self.last_send.elapsed() >= HEARTBEAT_AFTER {
                let msg_ref = self.take_ref();
                self.send_frame(None, &msg_ref, "phoenix", "heartbeat", json!({}))
                    .await?;
                self.outstanding_heartbeat = Some(msg_ref);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(None);
            }
            let until_heartbeat = HEARTBEAT_AFTER.saturating_sub(self.last_send.elapsed());
            let slice = remaining.min(until_heartbeat.max(Duration::from_millis(50)));
            match timeout(slice, self.read_frame()).await {
                Err(_) => continue, // slice elapsed: heartbeat check, then keep waiting
                Ok(frame) => match self.classify(frame?)? {
                    Classified::Broadcast(b) => return Ok(Some(b)),
                    Classified::Reply { .. } | Classified::Consumed => continue,
                },
            }
        }
    }

    /// Leave the channel and close the pipe, best-effort.
    pub async fn leave(&mut self) {
        if let (Some(topic), Some(join_ref)) = (self.topic.clone(), self.join_ref.clone()) {
            let msg_ref = self.take_ref();
            let _ = self
                .send_frame(Some(&join_ref), &msg_ref, &topic, "phx_leave", json!({}))
                .await;
        }
        let _ = self.transport.close().await;
    }
}

enum Classified {
    Reply {
        msg_ref: String,
        response: Result<Value, NetError>,
    },
    Broadcast(Broadcast),
    /// Handled internally (a heartbeat acknowledgement).
    Consumed,
}

fn as_opt_string(v: Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s),
        _ => None,
    }
}
