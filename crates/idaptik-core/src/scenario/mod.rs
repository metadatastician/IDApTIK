//! The "Envelope 001 – Ghost Lobby" scenario: a deterministic, event-sourced,
//! definition-as-data port of the canonical HTML prototype (ADR-0004).
//!
//! # Shape
//!
//! * [`ScenarioDefinition`] — declarative content (rooms, doors, cameras, props,
//!   objectives, difficulty presets, the full [`tuning`] table, scoring). Pure
//!   serde data; round-trips through JSON and self-validates via
//!   [`ScenarioDefinition::validate`] / [`ScenarioDefinition::ok`].
//! * [`ghost_lobby`] — builds the canonical definition by projecting
//!   [`constants`] (the single source of truth for every ported magic number).
//! * [`Command`] / [`TickInput`] / [`Buttons`] — the typed input the simulation
//!   consumes; [`Event`] — the typed output it emits; [`log_view`] renders the
//!   human [`LogLine`] view.
//! * [`Mulberry32`] — the exact PRNG port; [`roll_init`] the reset roll.
//!
//! The simulation itself ([`GhostLobbySim`]) and the JSON export/debrief
//! surfaces are added in the next stage; this stage lays the deterministic
//! foundation (RNG, constants, definition, commands, events).

pub mod command;
pub mod common;
pub mod constants;
pub mod definition;
pub mod event;
pub mod ghost_lobby;
pub mod ids;
pub mod mathf;
pub mod outcome;
pub mod rng;
pub mod seams;
pub mod sim;
pub mod snapshot;
pub mod state;
pub mod tuning;

pub use command::{Button, Buttons, Command, Edge, RunConfig, TickInput, fold};
pub use common::BillyMode;
pub use common::{
    Channel, ChuteMethod, CrisisReason, DenyReason, ExtractMethod, FailReason, Grade, ObjectKind,
    ObjectiveStatus, Outcome, Phase, ReportedTarget, Severity, Tone,
};
pub use definition::{
    BillyDef, CameraDef, Check, DoorDef, HideSpotDef, ObjectiveDef, ObjectiveKind, PlayerDef,
    PropSpawn, PropsDef, RoomDef, ScenarioDefinition, SpawnRanges, ValidationError,
    ValidationReport, WorldDef,
};
pub use event::{Event, LogLine, format_time, log_view};
pub use ghost_lobby::{GHOST_LOBBY_JSON, ghost_lobby};
pub use ids::{CameraId, DoorId, HideSpotId, IdIndex, ObjectiveId, RoomId};
pub use mathf::TICK_DT;
pub use outcome::{Debrief, ScoreBreakdown, Tag, debrief_text, grade_for};
pub use rng::{InitRoll, Mulberry32, roll_init};
pub use seams::{network_view, trace_from_alert};
pub use sim::GhostLobbySim;
pub use snapshot::{
    DebriefExport, DefinitionExport, RuntimeSnapshot, SNAPSHOT_FORMAT, ScenarioExport,
};
pub use state::{RuntimeState, Stats};
pub use tuning::{
    ActionKind, ActionSpec, DifficultyId, DifficultyPreset, GradeBands, ScoringDef, TuningConstants,
};
