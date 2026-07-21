//! The reusable, TTY-free surface of `idaptik-tui`.
//!
//! The interactive frontend stays in the binary; this library exposes the
//! pieces other crates consume — the script/replay file format ([`script`]),
//! the CLI → [`idaptik_core::RunConfig`] mapping ([`config`]), and the headless
//! runner ([`headless`]) whose `event_log` + `debrief` + `final_snapshot` blob
//! is the determinism artifact the whole estate compares against.
//!
//! `crates/idaptik-net` (ADR-0006) reuses all three so a networked seat runs
//! *exactly* the reference pipeline: same script expansion, same sim
//! construction, same output shape — byte-identical when lockstep holds.

pub mod config;
pub mod headless;
pub mod script;
