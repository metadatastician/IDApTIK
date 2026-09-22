//! The deterministic building runtime and its JSON export surfaces.
//!
//! [`BuildingSim`] owns a validated [`BuildingDefinition`], its derived
//! grounded network, and a [`BuildingState`]. Commands are typed
//! ([`BuildingCommand`]), results are typed events ([`BuildingEvent`]), and the
//! whole run is a pure function of `(definition, command stream)` — there is no
//! RNG and no wall clock, so equal inputs always produce equal exports.
//!
//! The export surfaces mirror the Exchange House prototype's four:
//!
//! * **definition** — [`BuildingDefinitionExport`] (definition + validation);
//! * **runtime** — [`BuildingSnapshot`], a full restorable snapshot;
//! * **legacy levelconfig** — [`LegacyLevelConfig`], the flat pre-building
//!   level shape older tooling consumes;
//! * **after-action** — [`BuildingDebriefExport`].
//!
//! The code path is panic-free: no `unwrap`/`expect`/panicking index; fallible
//! construction returns `Result`; unknown ids are denied events, never panics.

use crate::netsim::graph::{Actuation, GroundedGraph};
use crate::scenario::building::network::building_network;
use crate::scenario::building::{
    BuildingDefinition, BuildingValidationError, BuildingValidationReport, PortalKind, RoomRef,
};
use crate::scenario::ids::{CircuitId, PortalId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The runtime-snapshot export format tag.
pub const BUILDING_SNAPSHOT_FORMAT: &str = "idaptik-building-runtime-v1";
/// The after-action export format tag.
pub const BUILDING_DEBRIEF_FORMAT: &str = "idaptik-building-after-action-v1";
/// The legacy flat levelconfig export format tag.
pub const LEGACY_LEVELCONFIG_FORMAT: &str = "idaptik-legacy-levelconfig-v1";
/// The combined-export format tag (all four surfaces of one run).
pub const BUILDING_EXPORT_FORMAT: &str = "idaptik-building-export-v1";

/// A typed command the building runtime consumes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildingCommand {
    /// Traverse a portal adjacent to the agent's room.
    Traverse(PortalId),
    /// Actuate a node in the building's grounded network by id (a lock
    /// controller disengages its portal; a circuit feed cuts its circuit).
    Actuate(String),
}

/// Why a traversal was denied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildingDenyReason {
    UnknownPortal,
    NotAdjacent,
    Locked,
    Unpowered,
}

/// A typed event the building runtime emits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BuildingEvent {
    Entered {
        room: RoomRef,
        via: PortalId,
        travel_time: f64,
    },
    TraversalDenied {
        portal: PortalId,
        reason: BuildingDenyReason,
    },
    Unlocked {
        portal: PortalId,
        controller: String,
    },
    PowerCut {
        circuit: CircuitId,
    },
    ActuationFailed {
        node: String,
    },
}

/// The complete runtime state — pure serde, so a snapshot restores exactly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildingState {
    /// The room the agent stands in.
    pub at: RoomRef,
    /// Seconds elapsed traversing portals.
    pub clock: f64,
    /// Successful traversals.
    pub traversals: u64,
    /// Rooms in first-visit order (the entry is first).
    pub visited: Vec<RoomRef>,
    /// Portals in first-use order.
    pub portals_used: Vec<PortalId>,
    /// Live power state per circuit.
    pub powered: BTreeMap<CircuitId, bool>,
    /// Live lock state per locked portal (absent portals are unlocked).
    pub locked: BTreeMap<PortalId, bool>,
    /// Circuits cut, in cut order.
    pub circuits_cut: Vec<CircuitId>,
}

impl BuildingState {
    fn initial(def: &BuildingDefinition) -> Self {
        Self {
            at: def.entry.clone(),
            clock: 0.0,
            traversals: 0,
            visited: vec![def.entry.clone()],
            portals_used: Vec::new(),
            powered: def.circuits.iter().map(|c| (c.id.clone(), true)).collect(),
            locked: def
                .portals
                .iter()
                .filter(|p| p.lock.is_some())
                .map(|p| (p.id.clone(), true))
                .collect(),
            circuits_cut: Vec::new(),
        }
    }
}

/// The definition export surface (definition + validation report), mirroring
/// the scenario's `DefinitionExport`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildingDefinitionExport {
    pub format: String,
    pub definition: BuildingDefinition,
    pub validation: BuildingValidationReport,
}

/// A full, restorable snapshot of a run — the runtime export surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildingSnapshot {
    pub format: String,
    pub definition: BuildingDefinition,
    pub state: BuildingState,
    pub validation: BuildingValidationReport,
}

/// One room in the legacy flat levelconfig.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegacyRoom {
    /// `"floor/room"`.
    pub id: String,
    pub name: String,
    pub floor: String,
    pub level: i32,
}

/// One connection in the legacy flat levelconfig.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegacyConnection {
    pub from: String,
    pub to: String,
    pub kind: PortalKind,
    pub travel_time: f64,
    pub locked: bool,
}

/// The legacy pre-building levelconfig export surface: the building flattened
/// to the flat room/connection lists older tooling consumes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegacyLevelConfig {
    pub format: String,
    pub name: String,
    pub rooms: Vec<LegacyRoom>,
    pub connections: Vec<LegacyConnection>,
}

/// The after-action summary of a run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildingDebrief {
    pub format: String,
    /// Seconds elapsed traversing portals.
    pub elapsed: f64,
    pub traversals: u64,
    /// `"floor/room"`, in first-visit order.
    pub rooms_visited: Vec<String>,
    pub portals_used: Vec<PortalId>,
    pub circuits_cut: Vec<CircuitId>,
}

/// The after-action export surface, mirroring the scenario's `DebriefExport`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildingDebriefExport {
    pub format: String,
    pub debrief: Option<BuildingDebrief>,
}

/// The combined export of one run: all four surfaces, in the prototype's
/// order. This is the shape the committed parity fixture pins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildingExport {
    pub format: String,
    pub definition: BuildingDefinitionExport,
    pub runtime: BuildingSnapshot,
    pub levelconfig: LegacyLevelConfig,
    pub after_action: BuildingDebriefExport,
}

/// The deterministic building runtime.
#[derive(Debug, Clone)]
pub struct BuildingSim {
    def: BuildingDefinition,
    net: GroundedGraph,
    state: BuildingState,
}

impl BuildingSim {
    /// Construct a run at the building's entry. Validates the definition and
    /// derives the grounded network once (it is a pure function of the
    /// definition, so snapshots need not carry it).
    pub fn new(def: BuildingDefinition) -> Result<Self, Vec<BuildingValidationError>> {
        def.ok()?;
        let net = building_network(&def);
        let state = BuildingState::initial(&def);
        Ok(Self { def, net, state })
    }

    /// Restore a run from a snapshot. Gates on the snapshot format tag and
    /// re-validates the carried definition.
    pub fn restore(snapshot: BuildingSnapshot) -> Result<Self, Vec<BuildingValidationError>> {
        if snapshot.format != BUILDING_SNAPSHOT_FORMAT {
            return Err(vec![BuildingValidationError::UnsupportedSnapshotFormat {
                found: snapshot.format,
            }]);
        }
        snapshot.definition.ok()?;
        let net = building_network(&snapshot.definition);
        Ok(Self {
            def: snapshot.definition,
            net,
            state: snapshot.state,
        })
    }

    /// The definition the run plays.
    pub fn definition(&self) -> &BuildingDefinition {
        &self.def
    }

    /// The current runtime state.
    pub fn state(&self) -> &BuildingState {
        &self.state
    }

    /// The building's grounded network (zones as segments, feeds and lock
    /// controllers as nodes).
    pub fn network(&self) -> &GroundedGraph {
        &self.net
    }

    /// Whether a circuit is currently powered (unknown ids read as unpowered).
    pub fn is_powered(&self, circuit: &CircuitId) -> bool {
        self.state.powered.get(circuit).copied().unwrap_or(false)
    }

    /// Whether a portal is currently locked.
    pub fn is_locked(&self, portal: &PortalId) -> bool {
        self.state.locked.get(portal).copied().unwrap_or(false)
    }

    /// Apply one command, returning the events it produced.
    pub fn apply(&mut self, cmd: &BuildingCommand) -> Vec<BuildingEvent> {
        match cmd {
            BuildingCommand::Traverse(portal) => self.traverse(portal),
            BuildingCommand::Actuate(node) => self.actuate(node),
        }
    }

    fn traverse(&mut self, id: &PortalId) -> Vec<BuildingEvent> {
        let deny = |reason| {
            vec![BuildingEvent::TraversalDenied {
                portal: id.clone(),
                reason,
            }]
        };
        let Some(portal) = self.def.portal(id) else {
            return deny(BuildingDenyReason::UnknownPortal);
        };
        let dest = if portal.from == self.state.at {
            portal.to.clone()
        } else if portal.to == self.state.at {
            portal.from.clone()
        } else {
            return deny(BuildingDenyReason::NotAdjacent);
        };
        if self.is_locked(id) {
            return deny(BuildingDenyReason::Locked);
        }
        if let Some(circuit) = &portal.circuit
            && !self.is_powered(circuit)
        {
            return deny(BuildingDenyReason::Unpowered);
        }
        let travel_time = portal.travel_time;
        self.state.clock += travel_time;
        self.state.traversals += 1;
        self.state.at = dest.clone();
        if !self.state.visited.contains(&dest) {
            self.state.visited.push(dest.clone());
        }
        if !self.state.portals_used.contains(id) {
            self.state.portals_used.push(id.clone());
        }
        vec![BuildingEvent::Entered {
            room: dest,
            via: id.clone(),
            travel_time,
        }]
    }

    fn actuate(&mut self, node_id: &str) -> Vec<BuildingEvent> {
        let failed = vec![BuildingEvent::ActuationFailed {
            node: node_id.to_owned(),
        }];
        let Some(node) = self.net.node(node_id) else {
            return failed;
        };
        // A node whose upstream circuit is cut cannot act.
        let node_powered = node.deps.iter().all(|dep| {
            self.def
                .circuits
                .iter()
                .find(|c| c.source == dep.on)
                .is_some_and(|c| self.is_powered(&c.id))
        });
        if !node_powered {
            return failed;
        }
        match node.actuation {
            Some(Actuation::DisengageLock) => {
                let portal = self
                    .def
                    .portals
                    .iter()
                    .find(|p| p.lock.as_ref().is_some_and(|l| l.controller == node.id));
                let Some(portal) = portal else {
                    return failed;
                };
                if !self.is_locked(&portal.id) {
                    return failed;
                }
                self.state.locked.insert(portal.id.clone(), false);
                vec![BuildingEvent::Unlocked {
                    portal: portal.id.clone(),
                    controller: node_id.to_owned(),
                }]
            }
            Some(Actuation::CutPower) => {
                let circuit = self.def.circuits.iter().find(|c| c.source == node.id);
                let Some(circuit) = circuit else {
                    return failed;
                };
                if !self.is_powered(&circuit.id) {
                    return failed;
                }
                self.state.powered.insert(circuit.id.clone(), false);
                self.state.circuits_cut.push(circuit.id.clone());
                vec![BuildingEvent::PowerCut {
                    circuit: circuit.id.clone(),
                }]
            }
            _ => failed,
        }
    }

    // --- export surfaces -----------------------------------------------------

    /// The definition export surface.
    pub fn definition_export(&self) -> BuildingDefinitionExport {
        BuildingDefinitionExport {
            format: self.def.format.clone(),
            definition: self.def.clone(),
            validation: self.def.validate(),
        }
    }

    /// A full, restorable snapshot — the runtime export surface.
    pub fn snapshot(&self) -> BuildingSnapshot {
        BuildingSnapshot {
            format: BUILDING_SNAPSHOT_FORMAT.to_owned(),
            definition: self.def.clone(),
            state: self.state.clone(),
            validation: self.def.validate(),
        }
    }

    /// The legacy flat levelconfig export surface.
    pub fn legacy_levelconfig(&self) -> LegacyLevelConfig {
        let rooms = self
            .def
            .floors
            .iter()
            .flat_map(|f| {
                f.rooms.iter().map(|r| LegacyRoom {
                    id: format!("{}/{}", f.id, r.id),
                    name: r.name.clone(),
                    floor: f.id.as_str().to_owned(),
                    level: f.level,
                })
            })
            .collect();
        let connections = self
            .def
            .portals
            .iter()
            .map(|p| LegacyConnection {
                from: p.from.to_string(),
                to: p.to.to_string(),
                kind: p.kind,
                travel_time: p.travel_time,
                locked: p.lock.is_some(),
            })
            .collect();
        LegacyLevelConfig {
            format: LEGACY_LEVELCONFIG_FORMAT.to_owned(),
            name: self.def.name.clone(),
            rooms,
            connections,
        }
    }

    /// The after-action summary of the run so far.
    pub fn debrief(&self) -> BuildingDebrief {
        BuildingDebrief {
            format: BUILDING_DEBRIEF_FORMAT.to_owned(),
            elapsed: self.state.clock,
            traversals: self.state.traversals,
            rooms_visited: self.state.visited.iter().map(RoomRef::to_string).collect(),
            portals_used: self.state.portals_used.clone(),
            circuits_cut: self.state.circuits_cut.clone(),
        }
    }

    /// The after-action export surface.
    pub fn debrief_export(&self) -> BuildingDebriefExport {
        BuildingDebriefExport {
            format: BUILDING_DEBRIEF_FORMAT.to_owned(),
            debrief: Some(self.debrief()),
        }
    }

    /// The combined export: all four surfaces of this run.
    pub fn export(&self) -> BuildingExport {
        BuildingExport {
            format: BUILDING_EXPORT_FORMAT.to_owned(),
            definition: self.definition_export(),
            runtime: self.snapshot(),
            levelconfig: self.legacy_levelconfig(),
            after_action: self.debrief_export(),
        }
    }
}
