//! Versioned UMS package boundary.
//!
//! IDApTIK owns this envelope and the embedded [`ScenarioDefinition`]. Editors
//! may produce the JSON, but they do not define the game vocabulary or bypass
//! the same semantic validation used by native content.

use crate::scenario::{
    Buttons, Command, Event, GhostLobbySim, RunConfig, RuntimeSnapshot, SNAPSHOT_FORMAT,
    ScenarioDefinition, ValidationError, fold,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const PACKAGE_FORMAT: &str = "idaptik-package/v1";
pub const CONTRACT_ID: &str = "dev.metadatastician.idaptik.package";
pub const CONTRACT_VERSION: &str = "1.0.0";
pub const GAME_ID: &str = "idaptik";
pub const GAME_VERSION: &str = "0.1.0";
pub const PROFILE_ID: &str = "idaptik";
pub const PROFILE_VERSION: &str = "1.0.0";

const TAXONOMY_TERMS: [&str; 9] = [
    "room",
    "traversal",
    "camera_device",
    "security_actor",
    "non_security_actor",
    "prop",
    "objective",
    "network_action",
    "physical_effect",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractRef {
    pub id: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Compatibility {
    pub game: String,
    pub game_version: String,
    pub profile: String,
    pub profile_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorRole {
    Security,
    NonSecurity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageActor {
    pub id: String,
    pub role: ActorRole,
    pub runtime_binding: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledCommand {
    pub tick: u64,
    pub command: Command,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageGuarantees {
    pub deterministic: bool,
    pub snapshot_format: String,
    pub required_events: Vec<String>,
}

/// The stable package envelope. The scenario field is the game's real type,
/// not an editor-side imitation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GamePackage {
    pub format: String,
    pub contract: ContractRef,
    pub compatibility: Compatibility,
    pub scenario: ScenarioDefinition,
    pub scenario_id: String,
    pub seed: u32,
    pub run_ticks: u64,
    pub snapshot_tick: u64,
    pub taxonomy: BTreeMap<String, String>,
    pub actors: Vec<PackageActor>,
    pub commands: Vec<ScheduledCommand>,
    pub guarantees: PackageGuarantees,
}

/// A package accepted by both envelope and gameplay-semantic validation.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedPackage {
    pub package: GamePackage,
    pub semantic_validation_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageError {
    Json(String),
    UnsupportedFormat { found: String },
    UnsupportedContract { id: String, version: String },
    IncompatibleGame { id: String, version: String },
    IncompatibleProfile { id: String, version: String },
    ScenarioIdMismatch { envelope: String, scenario: String },
    ScenarioFormatMismatch { found: String },
    ScenarioInvalid(Vec<ValidationError>),
    MissingTaxonomyTerm(String),
    MissingActorRole(ActorRole),
    InvalidRuntimeBinding(String),
    InvalidSchedule(String),
    InvalidGuarantee(String),
    MissingRequiredEvent(String),
    ReplayMismatch,
}

impl fmt::Display for PackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for PackageError {}

/// Parse and validate a package through the actual game-owned types.
pub fn load_package(json: &str) -> Result<LoadedPackage, PackageError> {
    let package: GamePackage =
        serde_json::from_str(json).map_err(|e| PackageError::Json(e.to_string()))?;

    if package.format != PACKAGE_FORMAT {
        return Err(PackageError::UnsupportedFormat {
            found: package.format,
        });
    }
    if package.contract.id != CONTRACT_ID || package.contract.version != CONTRACT_VERSION {
        return Err(PackageError::UnsupportedContract {
            id: package.contract.id,
            version: package.contract.version,
        });
    }
    if package.compatibility.game != GAME_ID || package.compatibility.game_version != GAME_VERSION {
        return Err(PackageError::IncompatibleGame {
            id: package.compatibility.game,
            version: package.compatibility.game_version,
        });
    }
    if package.compatibility.profile != PROFILE_ID
        || package.compatibility.profile_version != PROFILE_VERSION
    {
        return Err(PackageError::IncompatibleProfile {
            id: package.compatibility.profile,
            version: package.compatibility.profile_version,
        });
    }
    if package.scenario_id != package.scenario.scenario_id {
        return Err(PackageError::ScenarioIdMismatch {
            envelope: package.scenario_id,
            scenario: package.scenario.scenario_id,
        });
    }
    if package.scenario.format != "idaptik-ghost-lobby-v2" {
        return Err(PackageError::ScenarioFormatMismatch {
            found: package.scenario.format,
        });
    }

    let report = package.scenario.validate();
    let semantic_validation_ids = report.checks.iter().map(|check| check.id.clone()).collect();
    report.ok().map_err(PackageError::ScenarioInvalid)?;

    for term in TAXONOMY_TERMS {
        if package.taxonomy.get(term).is_none_or(String::is_empty) {
            return Err(PackageError::MissingTaxonomyTerm(term.to_owned()));
        }
    }

    for role in [ActorRole::Security, ActorRole::NonSecurity] {
        if !package.actors.iter().any(|actor| actor.role == role) {
            return Err(PackageError::MissingActorRole(role));
        }
    }
    if !package.actors.iter().any(|actor| {
        actor.role == ActorRole::Security && actor.runtime_binding.as_deref() == Some("billy")
    }) {
        return Err(PackageError::InvalidRuntimeBinding(
            "one security actor must bind to the game-owned `billy` runtime actor".into(),
        ));
    }

    if package.run_ticks < 2
        || package.snapshot_tick == 0
        || package.snapshot_tick >= package.run_ticks
        || package
            .commands
            .windows(2)
            .any(|pair| pair[0].tick > pair[1].tick)
        || package
            .commands
            .iter()
            .any(|scheduled| scheduled.tick >= package.run_ticks)
    {
        return Err(PackageError::InvalidSchedule(
            "ticks must be sorted and bounded; snapshot_tick must split the run".into(),
        ));
    }

    if !package.guarantees.deterministic {
        return Err(PackageError::InvalidGuarantee(
            "deterministic must be true".into(),
        ));
    }
    if package.guarantees.snapshot_format != SNAPSHOT_FORMAT {
        return Err(PackageError::InvalidGuarantee(format!(
            "snapshot format must be {SNAPSHOT_FORMAT}"
        )));
    }
    let required: BTreeSet<_> = package.guarantees.required_events.iter().collect();
    if required.len() != package.guarantees.required_events.len() || required.is_empty() {
        return Err(PackageError::InvalidGuarantee(
            "required_events must be non-empty and unique".into(),
        ));
    }

    Ok(LoadedPackage {
        package,
        semantic_validation_ids,
    })
}

/// IDApTIK's game-specific stages in the bounded guard appraisal.
///
/// This is intentionally not a copy of Enaction's general domain enum. A
/// future adapter can map these game words onto that versioned public type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardTraceStage {
    CameraSignalFailed,
    InterferenceAppraised,
    AnxietyUpdated,
    VerificationGoalRaised,
    VerifyFeedSelected,
    TeamProtectionContext,
}

/// One inspectable step in a deterministic appraisal chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacTrace {
    pub tick: u64,
    pub sequence: u8,
    pub actor: String,
    pub stage: GuardTraceStage,
    pub state: String,
    pub value_milli: i32,
}

/// Headless evidence returned at the package boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoundTripResult {
    pub package_format: String,
    pub scenario_id: String,
    pub seed: u32,
    pub semantic_validation_ids: Vec<String>,
    pub events: Vec<Event>,
    pub cognitive_trace: Vec<CacTrace>,
    pub snapshot: RuntimeSnapshot,
    pub final_snapshot: RuntimeSnapshot,
    pub replay_equal: bool,
    pub package_guarantees_met: bool,
}

fn event_name(event: &Event) -> Option<String> {
    serde_json::to_value(event)
        .ok()?
        .get("event")?
        .as_str()
        .map(str::to_owned)
}

fn trace_camera_failure(tick: u64) -> Vec<CacTrace> {
    [
        (
            GuardTraceStage::CameraSignalFailed,
            "camera_feed_failed",
            1_000,
        ),
        (
            GuardTraceStage::InterferenceAppraised,
            "possible_interference_not_confirmed",
            550,
        ),
        (GuardTraceStage::AnxietyUpdated, "anxiety_delta", 180),
        (
            GuardTraceStage::VerificationGoalRaised,
            "verify_feed_priority",
            800,
        ),
        (
            GuardTraceStage::VerifyFeedSelected,
            "verify_camera_feed",
            1_000,
        ),
        (
            GuardTraceStage::TeamProtectionContext,
            "protect_security_team",
            650,
        ),
    ]
    .into_iter()
    .enumerate()
    .map(|(sequence, (stage, state, value_milli))| CacTrace {
        tick,
        sequence: sequence as u8,
        actor: "billy".into(),
        stage,
        state: state.into(),
        value_milli,
    })
    .collect()
}

fn commands_at(package: &GamePackage, tick: u64) -> Vec<Command> {
    package
        .commands
        .iter()
        .filter(|scheduled| scheduled.tick == tick)
        .map(|scheduled| scheduled.command)
        .collect()
}

/// Execute, snapshot, restore and replay one accepted package.
pub fn run_package(loaded: LoadedPackage) -> Result<RoundTripResult, PackageError> {
    let package = loaded.package;
    let cfg = RunConfig::standard();
    let mut sim = GhostLobbySim::new(package.scenario.clone(), cfg, package.seed)
        .map_err(PackageError::ScenarioInvalid)?;
    let mut held = Buttons::default();
    let mut events = Vec::new();
    let mut cognitive_trace = Vec::new();
    let mut saved: Option<(RuntimeSnapshot, Buttons, usize)> = None;

    for tick in 0..package.run_ticks {
        let input = fold(&commands_at(&package, tick), &mut held);
        let tick_events = sim.tick(&input);
        if tick_events
            .iter()
            .any(|event| matches!(event, Event::CameraPinged { .. }))
        {
            cognitive_trace.extend(trace_camera_failure(tick));
        }
        events.extend(tick_events);
        if tick + 1 == package.snapshot_tick {
            saved = Some((sim.snapshot(), held, events.len()));
        }
    }

    let (snapshot, mut replay_held, tail_start) = saved.ok_or_else(|| {
        PackageError::InvalidSchedule("snapshot was not reached during execution".into())
    })?;
    let final_snapshot = sim.snapshot();
    let original_tail = events[tail_start..].to_vec();

    let mut restored = GhostLobbySim::restore(package.scenario.clone(), snapshot.clone())
        .map_err(PackageError::ScenarioInvalid)?;
    let mut replay_tail = Vec::new();
    for tick in package.snapshot_tick..package.run_ticks {
        let input = fold(&commands_at(&package, tick), &mut replay_held);
        replay_tail.extend(restored.tick(&input));
    }
    let replay_equal = original_tail == replay_tail && final_snapshot == restored.snapshot();
    if !replay_equal {
        return Err(PackageError::ReplayMismatch);
    }

    let observed: BTreeSet<String> = events.iter().filter_map(event_name).collect();
    for required in &package.guarantees.required_events {
        if !observed.contains(required) {
            return Err(PackageError::MissingRequiredEvent(required.clone()));
        }
    }

    Ok(RoundTripResult {
        package_format: package.format,
        scenario_id: package.scenario_id,
        seed: package.seed,
        semantic_validation_ids: loaded.semantic_validation_ids,
        events,
        cognitive_trace,
        snapshot,
        final_snapshot,
        replay_equal,
        package_guarantees_met: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::{ActionKind, PivotTarget, ghost_lobby};

    fn package() -> GamePackage {
        GamePackage {
            format: PACKAGE_FORMAT.into(),
            contract: ContractRef {
                id: CONTRACT_ID.into(),
                version: CONTRACT_VERSION.into(),
            },
            compatibility: Compatibility {
                game: GAME_ID.into(),
                game_version: GAME_VERSION.into(),
                profile: PROFILE_ID.into(),
                profile_version: PROFILE_VERSION.into(),
            },
            scenario: ghost_lobby(),
            scenario_id: "envelope-001-ghost-lobby".into(),
            seed: 0x01DA_771C,
            run_ticks: 8,
            snapshot_tick: 2,
            taxonomy: TAXONOMY_TERMS
                .into_iter()
                .map(|term| (term.into(), format!("idaptik.{term}")))
                .collect(),
            actors: vec![
                PackageActor {
                    id: "billy".into(),
                    role: ActorRole::Security,
                    runtime_binding: Some("billy".into()),
                },
                PackageActor {
                    id: "night-cleaner".into(),
                    role: ActorRole::NonSecurity,
                    runtime_binding: None,
                },
            ],
            commands: vec![
                ScheduledCommand {
                    tick: 0,
                    command: Command::Pivot {
                        target: PivotTarget::Bridge,
                    },
                },
                ScheduledCommand {
                    tick: 2,
                    command: Command::Uplink {
                        kind: ActionKind::Camera,
                    },
                },
            ],
            guarantees: PackageGuarantees {
                deterministic: true,
                snapshot_format: SNAPSHOT_FORMAT.into(),
                required_events: vec![
                    "PivotOpened".into(),
                    "UplinkAction".into(),
                    "CameraPinged".into(),
                ],
            },
        }
    }

    #[test]
    fn package_loader_and_replay_use_the_real_game_types() {
        let json = serde_json::to_string(&package()).expect("serialize package");
        let loaded = load_package(&json).expect("load package");
        let result = run_package(loaded).expect("run package");
        assert!(result.replay_equal);
        assert!(result.package_guarantees_met);
        assert_eq!(result.cognitive_trace.len(), 6);
        assert_eq!(
            result
                .cognitive_trace
                .iter()
                .map(|trace| trace.stage)
                .collect::<Vec<_>>(),
            vec![
                GuardTraceStage::CameraSignalFailed,
                GuardTraceStage::InterferenceAppraised,
                GuardTraceStage::AnxietyUpdated,
                GuardTraceStage::VerificationGoalRaised,
                GuardTraceStage::VerifyFeedSelected,
                GuardTraceStage::TeamProtectionContext,
            ]
        );
    }

    #[test]
    fn wrong_profile_is_rejected_before_simulation() {
        let mut wrong = package();
        wrong.compatibility.profile = "slavia".into();
        let json = serde_json::to_string(&wrong).expect("serialize package");
        assert!(matches!(
            load_package(&json),
            Err(PackageError::IncompatibleProfile { .. })
        ));
    }
}
