//! Headless runner: expand a script, simulate to the end, print one JSON blob
//! (`event_log` + `debrief` + `final_snapshot`) to stdout. No TTY required.

use crate::config;
use crate::script::{ScriptFile, expand};
use idaptik_core::Debrief;
use idaptik_core::scenario::event::Event;
use idaptik_core::scenario::{GhostLobbySim, RuntimeSnapshot, ghost_lobby};
use serde::Serialize;
use std::path::Path;

/// The headless stdout payload.
#[derive(Serialize)]
struct HeadlessOutput {
    format: &'static str,
    event_log: Vec<Event>,
    debrief: Option<Debrief>,
    final_snapshot: RuntimeSnapshot,
}

/// Build a sim from a script's config and seed.
pub fn build(script: &ScriptFile) -> Result<GhostLobbySim, String> {
    let diff = config::parse_difficulty(&script.difficulty)?;
    let cfg = config::run_config(diff, script.reduced_motion);
    GhostLobbySim::new(ghost_lobby(), cfg, script.seed)
        .map_err(|e| format!("invalid scenario: {e:?}"))
}

/// Simulate a script to its end (or `max_ticks`), returning the sim and the full
/// event log (including the startup events).
pub fn simulate(script: &ScriptFile) -> Result<(GhostLobbySim, Vec<Event>), String> {
    let mut sim = build(script)?;
    let mut log = sim.drain_events();
    for input in expand(script) {
        if sim.is_ended() {
            break;
        }
        log.extend(sim.tick(&input));
    }
    Ok((sim, log))
}

/// Load, run and print a headless script.
pub fn run(path: &Path) -> Result<(), String> {
    let script = load(path)?;
    let (sim, log) = simulate(&script)?;
    let out = HeadlessOutput {
        format: "idaptik-ghost-lobby-headless-v1",
        debrief: sim.debrief().cloned(),
        final_snapshot: sim.snapshot(),
        event_log: log,
    };
    let json = serde_json::to_string_pretty(&out).map_err(|e| format!("serialize: {e}"))?;
    println!("{json}");
    Ok(())
}

/// Read and parse a script file.
pub fn load(path: &Path) -> Result<ScriptFile, String> {
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("parse script {}: {e}", path.display()))
}
