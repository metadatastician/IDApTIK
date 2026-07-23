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
//! Two session machines share those seams:
//!
//! - [`session`] — the *batch-scripted* seat (v1, the ADR-0006 §4 loopback
//!   gate): both seats know their script up front, exchange all commands, then
//!   run the sim to completion.
//! - [`lockstep`] + [`live`] — the *live* seat (the interactive-client slice):
//!   delay-based lockstep with real-time pacing over `at`, per-tick
//!   `net:commit` watermarks, and mid-run pause/resync — a lost peer can
//!   rejoin and be handed the survivor's `RuntimeSnapshot` (`net:resync`).
//!   [`lockstep`] is the sans-IO state machine (unit-proven deterministic);
//!   [`live`] is the paced async driver; [`interactive`] is the terminal
//!   frontend reusing `idaptik-tui`'s render/keymap/input pipeline.

pub mod envelope;
pub mod error;
pub mod interactive;
pub mod live;
pub mod lockstep;
pub mod phoenix;
pub mod seat;
pub mod session;
pub mod transport;
pub mod ws;
