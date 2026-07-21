//! `idaptik-net`: the client-side session-transport seam (ADR-0006 §2).
//!
//! The relay (`server/`) is transport-agnostic by construction; everything
//! transport-shaped lives here, client-side:
//!
//! - [`transport::SessionTransport`] — a reliable, ordered, bidirectional text
//!   message pipe. Nothing Phoenix-specific, nothing burble-specific.
//! - [`ws::PlainWebSocketTransport`] — `tokio-tungstenite` over TCP. Per
//!   ADR-0006 this is "the test transport and the fallback transport, forever";
//!   a `BurbleTransport` slots in beside it when burble ships an embeddable
//!   client (ADR-0006 §5 unblock condition — unmet as of 2026-07-21).
//! - [`phoenix::PhoenixClient`] — the Phoenix Channels wire protocol (V2 JSON
//!   serializer: join/push/reply/heartbeat), implemented once, over any
//!   `SessionTransport`.
//! - [`envelope`] — the relay envelope around the typed `Command` JSON: `seq`
//!   (relay de-duplication, ADR-0005) and `at` (lockstep tick scheduling —
//!   authored and consumed by clients, invisible to the relay, which strips
//!   only `seq`).
//! - [`seat`] — splits one headless script across the two seats and rebuilds
//!   the merged per-tick input identically on both sides.
//! - [`session`] — the seat state machine: join → hello → stream/collect →
//!   deterministic run → digest cross-check, with the ADR-0006 §3 loss states
//!   (`WaitingForPeer` → `Live` → `PeerLost` → clean `Ended`).
//!
//! Scope note (v1): the session client is *batch-scripted* — both seats know
//! their own script up front, exchange all commands through the relay, then run
//! the sim to completion. That is exactly the loopback gate ADR-0006 §4
//! specifies. Real-time pacing (input-delay scheduling) and mid-run
//! pause/resync on `PeerLost` attach to the same seams in the interactive
//! client slice; the wire shapes here already carry what they need (`at`,
//! `seq`, `net:` control messages).

pub mod envelope;
pub mod error;
pub mod phoenix;
pub mod seat;
pub mod session;
pub mod transport;
pub mod ws;
