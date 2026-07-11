//! Pure-data tuning tables. These carry the ported constants *inside* the
//! declarative [`crate::scenario::definition::ScenarioDefinition`] so a JSON
//! export is fully self-describing. There is **no logic here** — every field is
//! projected from [`crate::scenario::constants`] by
//! [`crate::scenario::ghost_lobby::ghost_lobby`].

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Selects a difficulty preset. Ordered so it is a stable `BTreeMap` key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DifficultyId {
    Story,
    Standard,
    Operator,
}

impl DifficultyId {
    /// All three ids in canonical order.
    pub const ALL: [DifficultyId; 3] = [
        DifficultyId::Story,
        DifficultyId::Standard,
        DifficultyId::Operator,
    ];

    /// The lowercase token used on the CLI and in script files.
    pub fn as_token(self) -> &'static str {
        match self {
            DifficultyId::Story => "story",
            DifficultyId::Standard => "standard",
            DifficultyId::Operator => "operator",
        }
    }
}

impl FromStr for DifficultyId {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "story" => Ok(DifficultyId::Story),
            "standard" => Ok(DifficultyId::Standard),
            "operator" => Ok(DifficultyId::Operator),
            other => Err(format!("unknown difficulty: {other:?}")),
        }
    }
}

/// The four uplink actions. Ordered so it is a stable `BTreeMap` key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ActionKind {
    Camera,
    Door,
    Vacuum,
    Lights,
}

impl ActionKind {
    /// All four actions in canonical order (matches the 1–4 key bindings).
    pub const ALL: [ActionKind; 4] = [
        ActionKind::Camera,
        ActionKind::Door,
        ActionKind::Vacuum,
        ActionKind::Lights,
    ];

    /// Stable index used by the fixed-length throttle/cooldown arrays.
    pub fn index(self) -> usize {
        match self {
            ActionKind::Camera => 0,
            ActionKind::Door => 1,
            ActionKind::Vacuum => 2,
            ActionKind::Lights => 3,
        }
    }
}

/// One uplink action's economics.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActionSpec {
    pub cost: f64,
    pub cooldown: f64,
    pub alert_gain: f64,
}

/// A full difficulty preset (`story` / `standard` / `operator`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DifficultyPreset {
    pub label: String,
    pub arrival: (f64, f64),
    pub player_speed: f64,
    pub sprint: f64,
    pub billy_speed: f64,
    pub billy_sight: f64,
    pub support_limit: f64,
    pub bandwidth_regen: f64,
    pub badge_delay: f64,
    pub usb_timer: f64,
    pub camera_lock: f64,
    pub alert_gain: f64,
    pub score_mult: f64,
    pub rescue: bool,
}

// --- Tuning sub-tables --------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeometryTuning {
    pub floor: f64,
    pub room_offset: f64,
    pub player_w: f64,
    pub player_h: f64,
    pub billy_w: f64,
    pub billy_h: f64,
    pub collision_radius: f64,
    pub world_width: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MovementTuning {
    pub gravity: f64,
    pub jump_vy: f64,
    pub accel_drive: f64,
    pub accel_brake: f64,
    pub crouch_speed: f64,
    pub stam_drain: f64,
    pub stam_regen: f64,
    pub stam_regen_crouch: f64,
    pub sprint_min_stam: f64,
    pub hide_max_vx: f64,
    pub crouch_cam_vx: f64,
    pub moving_vx: f64,
    pub clamp_lo: f64,
    pub clamp_hi: f64,
    pub land_vy: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NoiseTuning {
    pub jump: f64,
    pub land: f64,
    pub decay: f64,
    pub sprint_base: f64,
    pub sprint_k: f64,
    pub walk_k: f64,
    pub crouch: f64,
    pub move_min_vx: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FsmTuning {
    pub start_x: f64,
    pub crisis_x: f64,
    pub enter_speed: f64,
    pub shock_t: f64,
    pub assess_speed: f64,
    pub patrol_lo: f64,
    pub patrol_hi: f64,
    pub patrol_pivot: f64,
    pub invest_speed: f64,
    pub secure_speed: f64,
    pub grab_dist: f64,
    pub guard_speed: f64,
    pub guard_t_note: f64,
    pub guard_t_usb: f64,
    pub call_t: f64,
    pub pursue_speed: f64,
    pub pursue_trigger: f64,
    pub pursue_giveup_ago: f64,
    pub pursue_giveup_dist: f64,
    pub accel: f64,
    pub clamp_lo: f64,
    pub clamp_hi: f64,
    pub badge_open: f64,
    pub snack_reach: f64,
    pub alert_boost_div: f64,
    pub lights_boost_k: f64,
    pub vac_distract: f64,
    pub sight_back: f64,
    pub sight_crouch: f64,
    pub sight_office: f64,
    pub sight_alert: f64,
    pub sight_alert_hi: f64,
    pub invest_noise: f64,
    pub invest_dist: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BeliefTuning {
    pub threshold: f64,
    pub misdirect_threshold: f64,
    pub moving_vx: f64,
    pub pi_seen: f64,
    pub pi_sprint: f64,
    pub note_near: f64,
    pub usb_near: f64,
    pub note_urg_sprint: f64,
    pub note_urg_move: f64,
    pub note_urg_still: f64,
    pub usb_urg_sprint: f64,
    pub usb_urg_move: f64,
    pub usb_urg_still: f64,
    pub note_carry: f64,
    pub usb_carry: f64,
    pub prog_gate: f64,
    pub pi_decay: f64,
    pub note_decay: f64,
    pub usb_decay: f64,
    pub peel_interest: f64,
    pub secure_note_seen: f64,
    pub take_usb_seen: f64,
    pub take_usb_unseen: f64,
    pub camera_player_int: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SupportTuning {
    pub ping_hall: f64,
    pub ping_office: f64,
    pub ping_laundry: f64,
    pub ping_exit: f64,
    pub lowbw_x: f64,
    pub lowbw_pen: f64,
    pub alert_pen: f64,
    pub hidden: f64,
    pub flicker: f64,
    pub clamp_min: f64,
    pub approach: f64,
    pub iso_gate: f64,
    pub iso_pressure_div: f64,
    pub fray_frac: f64,
    pub iso_decay: f64,
    pub iso_decay_hidden: f64,
    pub ping_dur: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BandwidthTuning {
    pub regen_a: f64,
    pub regen_b: f64,
    pub alert_decay_quiet: f64,
    pub alert_decay_crisis: f64,
    pub lockdown: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActionTiming {
    pub lights_dur: f64,
    pub lights_dur_rm: f64,
    pub lights_stun: f64,
    pub route_duration: f64,
    pub throttle_cdbw: f64,
    pub throttle_fray: f64,
    pub throttle_badge: f64,
    pub call_alert: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CameraTuning {
    pub sweep_w: f64,
    pub sweep_a: f64,
    pub lockout: f64,
    pub alert: f64,
    pub decay_unseen: f64,
    pub decay_off: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UsbTuning {
    pub throw_vx: f64,
    pub throw_vy: f64,
    pub throw_off: f64,
    pub held_off: f64,
    pub grav: f64,
    pub drag: f64,
    pub floor: f64,
    pub bounce: f64,
    pub friction: f64,
    pub rest_vx: f64,
    pub rest_vy: f64,
    pub clamp_lo: f64,
    pub clamp_hi: f64,
    pub wipe_alert: f64,
    pub throw_alert: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VacuumTuning {
    pub lag_x: f64,
    pub ctrl_loss: f64,
    pub ctrl_min: f64,
    pub lag_warn: f64,
    pub ctrl_gain: f64,
    pub speed: f64,
    pub move_min: f64,
    pub target: f64,
    pub fall_off: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InteractionTuning {
    pub note_dist: f64,
    pub note_hold: f64,
    pub usb_dist: f64,
    pub chute_enter: f64,
    pub chute_search_off: f64,
    pub chute_search_dist: f64,
    pub chute_hold: f64,
    pub pickpocket_dist: f64,
    pub pickpocket_hold: f64,
    pub exit_prompt_x: f64,
    pub exit_x: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RescueTuning {
    pub support_min: f64,
    pub bw_min: f64,
    pub bw_cost: f64,
    pub stun: f64,
    pub grace: f64,
    pub displace: f64,
}

/// The full bundle of ported constants, carried inside the definition.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TuningConstants {
    pub geometry: GeometryTuning,
    pub movement: MovementTuning,
    pub noise: NoiseTuning,
    pub fsm: FsmTuning,
    pub belief: BeliefTuning,
    pub support: SupportTuning,
    pub bandwidth: BandwidthTuning,
    pub action: ActionTiming,
    pub camera: CameraTuning,
    pub usb: UsbTuning,
    pub vacuum: VacuumTuning,
    pub interaction: InteractionTuning,
    pub rescue: RescueTuning,
}

/// Grade thresholds (raw score, pre score-multiplier).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GradeBands {
    pub s: f64,
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub fail_c: f64,
}

/// The scoring/debrief weight table.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScoringDef {
    pub base: f64,
    pub note: f64,
    pub note_lost: f64,
    pub note_none: f64,
    pub misdir: f64,
    pub misdir_leak: f64,
    pub noboss: f64,
    pub boss: f64,
    pub nocam: f64,
    pub cam_each: f64,
    pub iso_ok: f64,
    pub iso_snap: f64,
    pub iso_snap_frac: f64,
    pub chute: f64,
    pub usbtrace: f64,
    pub rescue: f64,
    pub time_base: f64,
    pub time_k: f64,
    pub alert_k: f64,
    pub fail_act: f64,
    pub fail_note: f64,
    pub fail_base: f64,
    pub fail_usb: f64,
    pub fail_alert_k: f64,
    pub grades: GradeBands,
}
