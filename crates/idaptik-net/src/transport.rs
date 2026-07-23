//! The `SessionTransport` seam (ADR-0006 §2).
//!
//! "A reliable, ordered, bidirectional text/binary message pipe with an async
//! connect, send, receive, and close — nothing Phoenix-specific, nothing
//! burble-specific." Connection is constructor-shaped (each transport has its
//! own address vocabulary), so the trait carries only the pipe itself; the
//! Phoenix Channels client is generic over it, which is what makes "swap burble
//! for a plain socket" a constructor choice.

use crate::error::NetError;
use std::future::Future;

/// A reliable, ordered, bidirectional text message pipe.
///
/// v1 carries text frames only — Phoenix's JSON serializer is text — and adds a
/// binary lane when a transport needs one (burble's data channel), not before.
pub trait SessionTransport: Send {
    /// Send one text frame.
    fn send_text(&mut self, text: String) -> impl Future<Output = Result<(), NetError>> + Send;

    /// Receive the next text frame. `Ok(None)` means the peer closed cleanly;
    /// transport-level pings/pongs are handled below this seam.
    fn recv_text(&mut self) -> impl Future<Output = Result<Option<String>, NetError>> + Send;

    /// Close the pipe. Idempotent-best-effort: closing an already-dead pipe is
    /// not an error worth surfacing.
    fn close(&mut self) -> impl Future<Output = Result<(), NetError>> + Send;
}
