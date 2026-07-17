//! The grounded network: one vantage-keyed graph running from the wider internet
//! down to the devices in the room the infiltrator is standing in (see
//! docs/superpowers/specs/2026-07-14-grounded-network-design.md).
pub mod access;
pub mod addressing;
pub mod dns;
pub mod effect;
pub mod graph;
pub mod reach;
pub mod route;
pub mod session;
pub mod topology;

pub use access::can_reach;
pub use addressing::segment_of;
pub use dns::resolve;
pub use effect::{Effect, apply_actuation};
pub use graph::{Actuation, Dependency, DnsRecord, GroundedGraph, Node, Segment};
pub use graph::{VantageDef, VantageKind};
pub use reach::reachable_from;
pub use route::{Hop, ping, traceroute};
pub use session::{AgentSession, SessionError};
pub use topology::{GROUNDED_SLICE_JSON, grounded_slice};
