//! One error type across the transport / protocol / session layers.

use std::fmt;

/// Anything that can go wrong between "construct a transport" and "artifacts
/// written". Variants are layered so a failure names the layer that produced it.
#[derive(Debug)]
pub enum NetError {
    /// The underlying pipe failed (connect, send, receive, close).
    Transport(String),
    /// The pipe worked but the bytes were not the protocol we expected
    /// (malformed Phoenix frame, non-object command payload, bad envelope).
    Protocol(String),
    /// The protocol worked but the session logic refused it (config mismatch
    /// between seats, relay rejected a push, wrong role).
    Session(String),
    /// A wait gave up. The label names what was being waited for.
    Timeout(&'static str),
    /// The transport closed while we still needed it.
    Closed,
}

impl fmt::Display for NetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetError::Transport(m) => write!(f, "transport: {m}"),
            NetError::Protocol(m) => write!(f, "protocol: {m}"),
            NetError::Session(m) => write!(f, "session: {m}"),
            NetError::Timeout(what) => write!(f, "timed out waiting for {what}"),
            NetError::Closed => write!(f, "transport closed"),
        }
    }
}

impl std::error::Error for NetError {}
