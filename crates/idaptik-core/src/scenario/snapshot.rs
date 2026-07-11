//! JSON export surfaces, mirroring the Exchange-House prototype's three exports:
//! the declarative **definition**, a **runtime snapshot** (full state incl. RNG),
//! and the after-action **debrief** — plus the canonical **event log**.
//!
//! Every surface is `format`-tagged and round-trips through `serde_json`.

use crate::scenario::command::RunConfig;
use crate::scenario::definition::{ScenarioDefinition, ValidationReport};
use crate::scenario::event::Event;
use crate::scenario::outcome::Debrief;
use crate::scenario::rng::Mulberry32;
use crate::scenario::state::RuntimeState;
use serde::{Deserialize, Serialize};

/// The runtime-snapshot export format tag.
pub const SNAPSHOT_FORMAT: &str = "idaptik-ghost-lobby-runtime-v1";
/// The combined-export format tag.
pub const EXPORT_FORMAT: &str = "idaptik-ghost-lobby-export-v1";

/// A full, restorable snapshot of a run at a given tick (state incl. RNG).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeSnapshot {
    pub format: String,
    pub definition: ScenarioDefinition,
    pub cfg: RunConfig,
    pub seed: u32,
    pub tick: u64,
    pub rng: Mulberry32,
    pub state: RuntimeState,
    /// Whether the run was paused at snapshot time (round-trips so a paused run
    /// restores paused rather than silently resuming).
    #[serde(default)]
    pub paused: bool,
    pub validation: ValidationReport,
}

/// The definition export surface (the scenario itself carries its own tag).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DefinitionExport {
    pub format: String,
    pub definition: ScenarioDefinition,
    pub validation: ValidationReport,
}

/// The debrief export surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebriefExport {
    pub format: String,
    pub debrief: Option<Debrief>,
}

/// The combined export: definition + snapshot + optional debrief + event log.
/// This is the Exchange-House-style reflective dump of a whole run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioExport {
    pub format: String,
    pub definition: ScenarioDefinition,
    pub snapshot: RuntimeSnapshot,
    pub debrief: Option<Debrief>,
    pub event_log: Vec<Event>,
}
