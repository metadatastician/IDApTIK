//! Owned, serde runtime state — the mutable half of an event-sourced run.
//!
//! Every field is a faithful transliteration of the prototype's `game.*` object.
//! The state is pure data: the systems in [`crate::scenario::sim`] mutate it in a
//! fixed order and emit typed events; [`RuntimeState`] itself contains no logic
//! beyond [`RuntimeState::initial`], which builds the quiet-phase start from a
//! reset roll.

use crate::scenario::command::RunConfig;
use crate::scenario::common::{
    BillyMode, ChuteMethod, ExtractMethod, ObjectKind, ObjectiveStatus, Phase, ReportedTarget,
};
use crate::scenario::constants as c;
use crate::scenario::definition::ScenarioDefinition;
use crate::scenario::ids::DoorId;
use crate::scenario::outcome::Debrief;
use crate::scenario::rng::InitRoll;
use crate::scenario::tuning::{ActionKind, DifficultyPreset};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The infiltrator's physical and interaction state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerState {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub facing: f64,
    pub stamina: f64,
    pub interaction_progress: f64,
    pub noise: f64,
    pub caught_grace: f64,
    pub grounded: bool,
    pub crouching: bool,
    pub hidden: bool,
    pub sprinting: bool,
    pub has_note: bool,
    pub has_usb: bool,
    /// Index of the nearby hide spot, if the player is within one.
    pub hide_spot: Option<usize>,
}

/// Billy's physical state, FSM bookkeeping and belief meters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BillyState {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub facing: f64,
    pub state_timer: f64,
    pub note_interest: f64,
    pub usb_interest: f64,
    pub player_interest: f64,
    pub last_known_x: f64,
    pub last_seen_ago: f64,
    pub patrol_target: f64,
    pub door_wait: f64,
    pub stun: f64,
    pub guard_timer: f64,
    pub call_timer: f64,
    pub snack_x: f64,
    pub mode: BillyMode,
    pub target: Option<ObjectKind>,
    pub belief: Option<ObjectKind>,
    pub belief_announced: Option<ObjectKind>,
    pub blocked_door: Option<usize>,
    pub reported_target: Option<ReportedTarget>,
    pub called: bool,
    pub has_note: bool,
    pub has_usb: bool,
}

/// A door's runtime state (route countdown, hold, badge latch).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoorState {
    pub id: DoorId,
    pub x: f64,
    pub open: f64,
    pub pending: f64,
    pub route_delay: f64,
    pub route_duration: f64,
    pub badge_logged: bool,
}

/// The contact note.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteState {
    pub x: f64,
    pub y: f64,
    pub progress: f64,
    pub held: bool,
    pub billy_has: bool,
    pub exposed: bool,
}

/// The USB trap (ballistics + self-wipe).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsbState {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub timer: f64,
    pub held: bool,
    pub billy_has: bool,
    pub thrown: bool,
    pub wiped: bool,
    pub on_floor: bool,
}

/// The laundry chute.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChuteState {
    pub x: f64,
    pub y: f64,
    pub progress: f64,
    pub revealed: bool,
    pub used: bool,
}

/// The robot vacuum distraction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VacuumState {
    pub x: f64,
    pub y: f64,
    pub control: f64,
    pub target: f64,
    pub active: bool,
    pub fallen: bool,
    pub lag_warned: bool,
}

/// One uplink action's cooldown state.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActionState {
    pub cd: f64,
    pub max: f64,
}

/// Fixed-key log-throttle timestamps (never a `String` map — determinism).
/// Slots default to `-999.0` so the first emit of each key always passes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Throttles {
    /// Per-action cooldown-denial throttle, indexed by [`ActionKind::index`].
    pub cd: [f64; 4],
    /// Per-action bandwidth-denial throttle.
    pub bw: [f64; 4],
    /// Support-fraying throttle.
    pub support_fray: f64,
    /// Per-door badge throttle.
    pub badge: [f64; 4],
}

impl Default for Throttles {
    fn default() -> Self {
        Self {
            cd: [-999.0; 4],
            bw: [-999.0; 4],
            support_fray: -999.0,
            badge: [-999.0; 4],
        }
    }
}

/// Run statistics — the raw counters the debrief scores.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    pub hacker_actions: u32,
    pub failed_actions: u32,
    pub camera_detections: u32,
    pub doors_held: u32,
    pub light_flickers: u32,
    pub jumps: u32,
    /// Accumulated sprint seconds (a float, as in the prototype).
    pub sprints: f64,
    pub hidden_seconds: f64,
    pub support_broken_time: f64,
    pub max_isolation: f64,
    pub rescue_used: bool,
    pub usb_trace: bool,
    pub boss_called: bool,
    pub note_exposed: bool,
    pub vacuum_used: bool,
    pub chute_revealed_by: Option<ChuteMethod>,
    pub extraction: Option<ExtractMethod>,
}

/// The full mutable runtime state of a run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeState {
    pub phase: Phase,
    pub t: f64,
    pub quiet_remaining: f64,
    pub crisis_at: f64,
    pub support: f64,
    pub isolation: f64,
    pub bandwidth: f64,
    pub alert: f64,
    pub max_alert: f64,
    pub camera_ping: f64,
    pub lights_flicker: f64,
    pub camera_detection: f64,
    pub camera_lockout: f64,
    pub lights_uses: u32,
    pub camera_seen_count: u32,
    pub player: PlayerState,
    pub billy: BillyState,
    pub doors: Vec<DoorState>,
    pub note: NoteState,
    pub usb: UsbState,
    pub chute: ChuteState,
    pub vacuum: VacuumState,
    pub actions: BTreeMap<ActionKind, ActionState>,
    pub throttles: Throttles,
    pub stats: Stats,
    /// The last emitted objective ledger — a change emits `ObjectivesUpdated`.
    pub objective_ledger: (ObjectiveStatus, ObjectiveStatus, ObjectiveStatus),
    pub ended: bool,
    pub result: Option<Debrief>,
}

impl RuntimeState {
    /// Build the quiet-phase start from a reset roll and difficulty preset.
    pub fn initial(
        def: &ScenarioDefinition,
        roll: &InitRoll,
        _cfg: RunConfig,
        _preset: &DifficultyPreset,
    ) -> Self {
        let doors = def
            .doors
            .iter()
            .enumerate()
            .map(|(i, d)| DoorState {
                id: d.id.clone(),
                x: d.x,
                open: 0.0,
                pending: 0.0,
                route_delay: roll.door_delay.get(i).copied().unwrap_or(0.0),
                route_duration: c::ROUTE_DURATION,
                badge_logged: false,
            })
            .collect();

        let actions = ActionKind::ALL
            .iter()
            .map(|k| {
                let max = def.actions.get(k).map(|s| s.cooldown).unwrap_or(0.0);
                (*k, ActionState { cd: 0.0, max })
            })
            .collect();

        RuntimeState {
            phase: Phase::Quiet,
            t: 0.0,
            quiet_remaining: roll.arrival,
            crisis_at: roll.arrival,
            support: 1.0,
            isolation: 0.0,
            bandwidth: 100.0,
            alert: 0.0,
            max_alert: 0.0,
            camera_ping: 0.0,
            lights_flicker: 0.0,
            camera_detection: 0.0,
            camera_lockout: 0.0,
            lights_uses: 0,
            camera_seen_count: 0,
            player: PlayerState {
                x: def.player.spawn_x,
                y: def.player.spawn_y,
                vx: 0.0,
                vy: 0.0,
                facing: 1.0,
                stamina: 100.0,
                interaction_progress: 0.0,
                noise: 0.0,
                caught_grace: 0.0,
                grounded: true,
                crouching: false,
                hidden: false,
                sprinting: false,
                has_note: false,
                has_usb: false,
                hide_spot: None,
            },
            billy: BillyState {
                x: def.billy.spawn_x,
                y: def.billy.spawn_y,
                vx: 0.0,
                facing: 1.0,
                state_timer: 0.0,
                note_interest: 0.0,
                usb_interest: 0.0,
                player_interest: 0.0,
                last_known_x: c::BILLY_LAST_KNOWN_X,
                last_seen_ago: c::BILLY_LAST_SEEN_AGO,
                patrol_target: c::BILLY_PATROL_TARGET,
                door_wait: 0.0,
                stun: 0.0,
                guard_timer: 0.0,
                call_timer: 0.0,
                snack_x: roll.snack_x,
                mode: BillyMode::Offsite,
                target: None,
                belief: None,
                belief_announced: None,
                blocked_door: None,
                reported_target: None,
                called: false,
                has_note: false,
                has_usb: false,
            },
            doors,
            note: NoteState {
                x: roll.note_x,
                y: def.props.note.y,
                progress: 0.0,
                held: false,
                billy_has: false,
                exposed: false,
            },
            usb: UsbState {
                x: roll.usb_x,
                y: def.props.usb.y,
                vx: 0.0,
                vy: 0.0,
                timer: 0.0,
                held: false,
                billy_has: false,
                thrown: false,
                wiped: false,
                on_floor: true,
            },
            chute: ChuteState {
                x: def.props.chute.x,
                y: def.props.chute.y,
                progress: 0.0,
                revealed: false,
                used: false,
            },
            vacuum: VacuumState {
                x: def.props.vacuum.x,
                y: def.props.vacuum.y,
                control: 1.0,
                target: c::VAC_TARGET,
                active: false,
                fallen: false,
                lag_warned: false,
            },
            actions,
            throttles: Throttles::default(),
            stats: Stats::default(),
            objective_ledger: (
                ObjectiveStatus::Open,
                ObjectiveStatus::Open,
                ObjectiveStatus::Locked,
            ),
            ended: false,
            result: None,
        }
    }
}
