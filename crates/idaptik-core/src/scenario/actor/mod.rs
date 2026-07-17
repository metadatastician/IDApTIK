//! The actor archetype + modifier registry — enemies as data.
//!
//! This ports the **Enemy Actor Training Ground** prototype's `Registry` into
//! `idaptik-core`, generalising "Billy" into declarative base forms and
//! modified forms:
//!
//! * [`ActorArchetype`] — a base enemy: every FSM speed, timer, engagement
//!   distance, sight multiplier and belief parameter the reusable actor FSM in
//!   [`crate::scenario::sim`] consumes, plus per-object [`InterestProfile`]s.
//! * [`Modifier`] — a named, ordered list of [`StatOp`]s (`Add`/`Mul`/`Set`
//!   over a [`StatId`]). Applying a modifier set to a base archetype yields a
//!   deterministic [`ComposedActor`]: modifiers apply in the order given, and
//!   each modifier's ops apply in their listed order.
//! * [`ActorRegistry`] — the definition-as-data surface: pure serde, a
//!   committed golden JSON ([`ACTORS_JSON`]), and an Exchange-House-style
//!   [`ActorRegistry::validate`] report of named checks.
//!
//! The Ghost Lobby's Billy is the default archetype ([`billy_archetype`]),
//! projected from [`crate::scenario::constants`] exactly as
//! [`crate::scenario::ghost_lobby::ghost_lobby`] projects the scenario
//! definition — the same numbers, expressed as data, driving the same FSM.
//!
//! The note-vs-USB decoy generalises through the prototype's `valueSignal`
//! economy (see [`belief`]): objects carry a kind ([`ObjectClass`]) and a
//! `value_signal` in `0..=1`, and an actor's per-object interest is driven by
//! that signal plus the carrier's behavioural leakage ([`Leakage`]). Billy's
//! hand-tuned note/usb profiles pin the ported constants bit-for-bit;
//! [`InterestProfile::from_value_signal`] is the documented derivation for
//! content that declares only a `value_signal`.
//!
//! # UMS actor-pack seam
//!
//! [`load_actor_pack`] is where a UMS actor-pack DLC (payload format
//! [`ACTORS_FORMAT`], `"idaptik-actors/1"`) plugs in: hand it the payload JSON
//! and it returns a validated [`ActorRegistry`]. The DLC transport, merging
//! and licensing live above this crate; the format gate and validation live
//! here so every pack that loads is a pack the simulation can trust.

pub mod belief;

use crate::scenario::constants as c;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The actor-pack payload format this build reads and writes.
pub const ACTORS_FORMAT: &str = "idaptik-actors/1";

/// The default archetype's registry id.
pub const BILLY_ARCHETYPE_ID: &str = "billy";

/// The Ghost Lobby's two tracked-object ids, as interest-profile keys.
pub const NOTE_OBJECT: &str = "note";
/// See [`NOTE_OBJECT`].
pub const USB_OBJECT: &str = "usb";

/// A committed pretty-printed JSON of [`default_registry`]. Regenerate with
/// the ignored `regenerate_actors_json` test if the registry ever changes; the
/// `actor_registry` round-trip test proves it parses back equal.
pub const ACTORS_JSON: &str = include_str!("actors.json");

/// Object classes from the training-ground prototype's `valueSignal` economy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ObjectClass {
    Objective,
    Decoy,
    Tool,
    Trade,
}

/// Behavioural leakage: how loudly the observed party's movement advertises
/// the object they are near or carrying.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Leakage {
    Still,
    Moving,
    Sprinting,
}

/// How one actor's attention couples to one object: the interest meter's
/// gains, its decay, and the guard timer once the object is seized.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InterestProfile {
    /// The object's class in the prototype's economy.
    pub kind: ObjectClass,
    /// The prototype's `valueSignal` (`0..=1`): how valuable the object
    /// *reads* to an enemy. Hand-tuned profiles may pin explicit numbers
    /// below; derived profiles scale from this signal alone.
    pub value_signal: f64,
    /// Attention radius: within this distance the object is "near".
    pub near: f64,
    /// Interest gain per second while observed, by behavioural leakage.
    pub urg_still: f64,
    /// See [`InterestProfile::urg_still`].
    pub urg_move: f64,
    /// See [`InterestProfile::urg_still`].
    pub urg_sprint: f64,
    /// Extra interest gain per second while the object is carried in view.
    pub carry: f64,
    /// Interest decay per second while unobserved and not pinned.
    pub decay: f64,
    /// How long the actor guards this object after seizing it.
    pub guard_t: f64,
}

impl InterestProfile {
    /// The observed interest gain per second for a given leakage.
    pub fn urgency(&self, leakage: Leakage) -> f64 {
        match leakage {
            Leakage::Still => self.urg_still,
            Leakage::Moving => self.urg_move,
            Leakage::Sprinting => self.urg_sprint,
        }
    }

    /// The derivation for content that declares only a `value_signal`: the
    /// urgencies and carry gain scale linearly with the signal (clamped to
    /// `0..=1`), the attention radius and decay stay at the reference values.
    /// Billy's committed note/usb profiles do **not** use this — they pin the
    /// ported Ghost Lobby constants exactly.
    pub fn from_value_signal(kind: ObjectClass, value_signal: f64) -> Self {
        let vs = value_signal.clamp(0.0, 1.0);
        Self {
            kind,
            value_signal: vs,
            near: 110.0,
            urg_still: 5.0 * vs,
            urg_move: 11.0 * vs,
            urg_sprint: 21.0 * vs,
            carry: 14.0 * vs,
            decay: 0.25,
            guard_t: 1.8,
        }
    }

    /// An all-zero profile: the object is invisible to the actor. Used as the
    /// panic-free fallback when a composed actor is asked about an object it
    /// carries no profile for (a content gap, not a play state).
    pub fn inert(kind: ObjectClass) -> Self {
        Self {
            kind,
            value_signal: 0.0,
            near: 0.0,
            urg_still: 0.0,
            urg_move: 0.0,
            urg_sprint: 0.0,
            carry: 0.0,
            decay: 0.0,
            guard_t: 0.0,
        }
    }
}

/// Every scalar stat a modifier can address. One variant per [`ActorStats`]
/// field, so a modifier is data, not code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum StatId {
    EnterSpeed,
    AssessSpeed,
    InvestSpeed,
    SecureSpeed,
    GuardSpeed,
    PursueSpeed,
    ShockT,
    CallT,
    PatrolLo,
    PatrolHi,
    PatrolPivot,
    GrabDist,
    PursueTrigger,
    PursueGiveupAgo,
    PursueGiveupDist,
    Accel,
    SightBack,
    SightCrouch,
    SightAlert,
    SightAlertHi,
    AlertBoostDiv,
    LightsBoostK,
    DistractDist,
    InvestNoise,
    InvestDist,
    BeliefThreshold,
    PiSeen,
    PiSprint,
    PiDecay,
}

impl StatId {
    /// Every stat, in [`ActorStats`] field order.
    pub const ALL: [StatId; 29] = [
        StatId::EnterSpeed,
        StatId::AssessSpeed,
        StatId::InvestSpeed,
        StatId::SecureSpeed,
        StatId::GuardSpeed,
        StatId::PursueSpeed,
        StatId::ShockT,
        StatId::CallT,
        StatId::PatrolLo,
        StatId::PatrolHi,
        StatId::PatrolPivot,
        StatId::GrabDist,
        StatId::PursueTrigger,
        StatId::PursueGiveupAgo,
        StatId::PursueGiveupDist,
        StatId::Accel,
        StatId::SightBack,
        StatId::SightCrouch,
        StatId::SightAlert,
        StatId::SightAlertHi,
        StatId::AlertBoostDiv,
        StatId::LightsBoostK,
        StatId::DistractDist,
        StatId::InvestNoise,
        StatId::InvestDist,
        StatId::BeliefThreshold,
        StatId::PiSeen,
        StatId::PiSprint,
        StatId::PiDecay,
    ];

    /// The stats that are speed multipliers (validated non-negative).
    pub const SPEEDS: [StatId; 6] = [
        StatId::EnterSpeed,
        StatId::AssessSpeed,
        StatId::InvestSpeed,
        StatId::SecureSpeed,
        StatId::GuardSpeed,
        StatId::PursueSpeed,
    ];
}

/// The scalar half of an archetype: everything the reusable actor FSM reads
/// that is not per-object. Speeds are multipliers on the difficulty preset's
/// base actor speed, exactly as the ported constants were.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActorStats {
    pub enter_speed: f64,
    pub assess_speed: f64,
    pub invest_speed: f64,
    pub secure_speed: f64,
    pub guard_speed: f64,
    pub pursue_speed: f64,
    pub shock_t: f64,
    pub call_t: f64,
    pub patrol_lo: f64,
    pub patrol_hi: f64,
    pub patrol_pivot: f64,
    pub grab_dist: f64,
    pub pursue_trigger: f64,
    pub pursue_giveup_ago: f64,
    pub pursue_giveup_dist: f64,
    pub accel: f64,
    pub sight_back: f64,
    pub sight_crouch: f64,
    pub sight_alert: f64,
    pub sight_alert_hi: f64,
    pub alert_boost_div: f64,
    pub lights_boost_k: f64,
    pub distract_dist: f64,
    pub invest_noise: f64,
    pub invest_dist: f64,
    pub belief_threshold: f64,
    pub pi_seen: f64,
    pub pi_sprint: f64,
    pub pi_decay: f64,
}

impl ActorStats {
    /// Read one stat by id.
    pub fn get(&self, id: StatId) -> f64 {
        match id {
            StatId::EnterSpeed => self.enter_speed,
            StatId::AssessSpeed => self.assess_speed,
            StatId::InvestSpeed => self.invest_speed,
            StatId::SecureSpeed => self.secure_speed,
            StatId::GuardSpeed => self.guard_speed,
            StatId::PursueSpeed => self.pursue_speed,
            StatId::ShockT => self.shock_t,
            StatId::CallT => self.call_t,
            StatId::PatrolLo => self.patrol_lo,
            StatId::PatrolHi => self.patrol_hi,
            StatId::PatrolPivot => self.patrol_pivot,
            StatId::GrabDist => self.grab_dist,
            StatId::PursueTrigger => self.pursue_trigger,
            StatId::PursueGiveupAgo => self.pursue_giveup_ago,
            StatId::PursueGiveupDist => self.pursue_giveup_dist,
            StatId::Accel => self.accel,
            StatId::SightBack => self.sight_back,
            StatId::SightCrouch => self.sight_crouch,
            StatId::SightAlert => self.sight_alert,
            StatId::SightAlertHi => self.sight_alert_hi,
            StatId::AlertBoostDiv => self.alert_boost_div,
            StatId::LightsBoostK => self.lights_boost_k,
            StatId::DistractDist => self.distract_dist,
            StatId::InvestNoise => self.invest_noise,
            StatId::InvestDist => self.invest_dist,
            StatId::BeliefThreshold => self.belief_threshold,
            StatId::PiSeen => self.pi_seen,
            StatId::PiSprint => self.pi_sprint,
            StatId::PiDecay => self.pi_decay,
        }
    }

    /// Write one stat by id.
    pub fn set(&mut self, id: StatId, value: f64) {
        match id {
            StatId::EnterSpeed => self.enter_speed = value,
            StatId::AssessSpeed => self.assess_speed = value,
            StatId::InvestSpeed => self.invest_speed = value,
            StatId::SecureSpeed => self.secure_speed = value,
            StatId::GuardSpeed => self.guard_speed = value,
            StatId::PursueSpeed => self.pursue_speed = value,
            StatId::ShockT => self.shock_t = value,
            StatId::CallT => self.call_t = value,
            StatId::PatrolLo => self.patrol_lo = value,
            StatId::PatrolHi => self.patrol_hi = value,
            StatId::PatrolPivot => self.patrol_pivot = value,
            StatId::GrabDist => self.grab_dist = value,
            StatId::PursueTrigger => self.pursue_trigger = value,
            StatId::PursueGiveupAgo => self.pursue_giveup_ago = value,
            StatId::PursueGiveupDist => self.pursue_giveup_dist = value,
            StatId::Accel => self.accel = value,
            StatId::SightBack => self.sight_back = value,
            StatId::SightCrouch => self.sight_crouch = value,
            StatId::SightAlert => self.sight_alert = value,
            StatId::SightAlertHi => self.sight_alert_hi = value,
            StatId::AlertBoostDiv => self.alert_boost_div = value,
            StatId::LightsBoostK => self.lights_boost_k = value,
            StatId::DistractDist => self.distract_dist = value,
            StatId::InvestNoise => self.invest_noise = value,
            StatId::InvestDist => self.invest_dist = value,
            StatId::BeliefThreshold => self.belief_threshold = value,
            StatId::PiSeen => self.pi_seen = value,
            StatId::PiSprint => self.pi_sprint = value,
            StatId::PiDecay => self.pi_decay = value,
        }
    }
}

/// A base enemy form: the scalar stats plus per-object interest profiles,
/// keyed by object id (a `BTreeMap`, not a `HashMap` — the composition and
/// any iteration over it must be deterministic).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorArchetype {
    pub id: String,
    pub name: String,
    pub stats: ActorStats,
    pub interests: BTreeMap<String, InterestProfile>,
}

/// How one op transforms a stat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatOpKind {
    Add,
    Mul,
    Set,
}

/// One declarative stat transformation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StatOp {
    pub stat: StatId,
    pub op: StatOpKind,
    pub value: f64,
}

impl StatOp {
    fn apply(&self, stats: &mut ActorStats) {
        let cur = stats.get(self.stat);
        let next = match self.op {
            StatOpKind::Add => cur + self.value,
            StatOpKind::Mul => cur * self.value,
            StatOpKind::Set => self.value,
        };
        stats.set(self.stat, next);
    }
}

/// A named modified form: an ordered list of stat ops. Modifiers act on the
/// scalar stats; per-object interest profiles are archetype data (v1 of the
/// registry format).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Modifier {
    pub id: String,
    pub name: String,
    pub ops: Vec<StatOp>,
}

/// The deterministic result of applying a modifier set to a base archetype.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComposedActor {
    /// The base archetype's id.
    pub archetype: String,
    /// The applied modifier ids, in application order.
    pub modifiers: Vec<String>,
    pub stats: ActorStats,
    pub interests: BTreeMap<String, InterestProfile>,
}

impl ComposedActor {
    /// The unmodified form of an archetype.
    pub fn from_archetype(archetype: &ActorArchetype) -> Self {
        Self {
            archetype: archetype.id.clone(),
            modifiers: Vec::new(),
            stats: archetype.stats,
            interests: archetype.interests.clone(),
        }
    }

    /// The interest profile for `object`, or the inert profile if the actor
    /// carries none — a content gap behaves as "invisible", never a panic.
    pub fn interest_or_inert(&self, object: &str) -> InterestProfile {
        self.interests
            .get(object)
            .copied()
            .unwrap_or_else(|| InterestProfile::inert(ObjectClass::Tool))
    }
}

/// A typed composition failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComposeError {
    UnknownArchetype(String),
    UnknownModifier(String),
}

/// A typed registry validation failure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActorValidationError {
    UnknownFormat {
        found: String,
    },
    EmptyArchetypes,
    ArchetypeKeyMismatch {
        key: String,
        id: String,
    },
    ModifierKeyMismatch {
        key: String,
        id: String,
    },
    ValueSignalOutOfRange {
        archetype: String,
        object: String,
        value: f64,
    },
    NonFiniteStat {
        archetype: String,
        stat: StatId,
    },
    NegativeSpeed {
        archetype: String,
        stat: StatId,
    },
    BeliefThresholdOutOfRange {
        archetype: String,
        value: f64,
    },
    NonFiniteModifierOp {
        modifier: String,
        stat: StatId,
    },
}

/// One named check in the Exchange-House-style registry validation report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorCheck {
    pub id: String,
    pub label: String,
    pub passed: bool,
    pub detail: String,
    pub family: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ActorValidationError>,
}

/// The result of [`ActorRegistry::validate`] — a list of named checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorValidationReport {
    pub checks: Vec<ActorCheck>,
}

impl ActorValidationReport {
    /// Whether every check passed.
    pub fn passed(&self) -> bool {
        self.checks.iter().all(|c| c.passed)
    }

    /// The typed errors of all failed checks, or `Ok(())` if all passed.
    pub fn ok(&self) -> Result<(), Vec<ActorValidationError>> {
        let errs: Vec<ActorValidationError> = self
            .checks
            .iter()
            .filter(|c| !c.passed)
            .filter_map(|c| c.error.clone())
            .collect();
        if errs.is_empty() { Ok(()) } else { Err(errs) }
    }
}

/// Push one check. `err` is `Some` only when the condition failed.
fn check(
    checks: &mut Vec<ActorCheck>,
    family: &str,
    id: &str,
    label: &str,
    passed: bool,
    detail: String,
    err: Option<ActorValidationError>,
) {
    checks.push(ActorCheck {
        id: id.to_owned(),
        label: label.to_owned(),
        passed,
        detail,
        family: family.to_owned(),
        error: if passed { None } else { err },
    });
}

/// The complete, self-describing actor registry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorRegistry {
    pub format: String,
    pub archetypes: BTreeMap<String, ActorArchetype>,
    pub modifiers: BTreeMap<String, Modifier>,
}

impl ActorRegistry {
    /// Run the full validation suite, returning a report of named checks.
    /// Each check reports the *first* violation in its family, so a single
    /// malformed field yields exactly one failed check.
    pub fn validate(&self) -> ActorValidationReport {
        let mut checks: Vec<ActorCheck> = Vec::new();

        // --- format ---------------------------------------------------------
        let format_ok = self.format == ACTORS_FORMAT;
        check(
            &mut checks,
            "format",
            "format.recognised",
            "The payload format tag is the one this build reads",
            format_ok,
            format!("found {:?}, expected {ACTORS_FORMAT:?}", self.format),
            Some(ActorValidationError::UnknownFormat {
                found: self.format.clone(),
            }),
        );

        // --- archetypes present ----------------------------------------------
        let present = !self.archetypes.is_empty();
        check(
            &mut checks,
            "archetypes",
            "archetypes.present",
            "At least one archetype is defined",
            present,
            format!("{} archetype(s)", self.archetypes.len()),
            Some(ActorValidationError::EmptyArchetypes),
        );

        // --- keys match ids ---------------------------------------------------
        let arch_mismatch = self
            .archetypes
            .iter()
            .find(|(k, a)| **k != a.id)
            .map(|(k, a)| (k.clone(), a.id.clone()));
        check(
            &mut checks,
            "archetypes",
            "archetypes.key_matches_id",
            "Every archetype is keyed by its own id",
            arch_mismatch.is_none(),
            arch_mismatch
                .as_ref()
                .map(|(k, id)| format!("key {k:?} holds id {id:?}"))
                .unwrap_or_else(|| "all keyed by id".into()),
            arch_mismatch.map(|(key, id)| ActorValidationError::ArchetypeKeyMismatch { key, id }),
        );
        let mod_mismatch = self
            .modifiers
            .iter()
            .find(|(k, m)| **k != m.id)
            .map(|(k, m)| (k.clone(), m.id.clone()));
        check(
            &mut checks,
            "modifiers",
            "modifiers.key_matches_id",
            "Every modifier is keyed by its own id",
            mod_mismatch.is_none(),
            mod_mismatch
                .as_ref()
                .map(|(k, id)| format!("key {k:?} holds id {id:?}"))
                .unwrap_or_else(|| "all keyed by id".into()),
            mod_mismatch.map(|(key, id)| ActorValidationError::ModifierKeyMismatch { key, id }),
        );

        // --- value signals in range -------------------------------------------
        let vs_bad = self
            .archetypes
            .values()
            .flat_map(|a| {
                a.interests
                    .iter()
                    .map(move |(obj, p)| (a.id.clone(), obj.clone(), p.value_signal))
            })
            .find(|(_, _, vs)| !(0.0..=1.0).contains(vs));
        check(
            &mut checks,
            "interests",
            "interests.value_signal_range",
            "Every valueSignal is within [0, 1]",
            vs_bad.is_none(),
            vs_bad
                .as_ref()
                .map(|(a, o, v)| format!("{a}/{o} valueSignal={v}"))
                .unwrap_or_else(|| "all in range".into()),
            vs_bad.map(
                |(archetype, object, value)| ActorValidationError::ValueSignalOutOfRange {
                    archetype,
                    object,
                    value,
                },
            ),
        );

        // --- stats finite -------------------------------------------------------
        let non_finite = self.archetypes.values().find_map(|a| {
            StatId::ALL
                .iter()
                .find(|s| !a.stats.get(**s).is_finite())
                .map(|s| (a.id.clone(), *s))
        });
        check(
            &mut checks,
            "stats",
            "stats.finite",
            "Every archetype stat is a finite number",
            non_finite.is_none(),
            non_finite
                .as_ref()
                .map(|(a, s)| format!("{a} {s:?} is not finite"))
                .unwrap_or_else(|| "all finite".into()),
            non_finite
                .map(|(archetype, stat)| ActorValidationError::NonFiniteStat { archetype, stat }),
        );

        // --- speed multipliers non-negative --------------------------------------
        let neg_speed = self.archetypes.values().find_map(|a| {
            StatId::SPEEDS
                .iter()
                .find(|s| a.stats.get(**s) < 0.0)
                .map(|s| (a.id.clone(), *s))
        });
        check(
            &mut checks,
            "stats",
            "stats.speeds_non_negative",
            "Every speed multiplier is non-negative",
            neg_speed.is_none(),
            neg_speed
                .as_ref()
                .map(|(a, s)| format!("{a} {s:?} is negative"))
                .unwrap_or_else(|| "all non-negative".into()),
            neg_speed
                .map(|(archetype, stat)| ActorValidationError::NegativeSpeed { archetype, stat }),
        );

        // --- belief threshold in range ---------------------------------------------
        let thr_bad = self
            .archetypes
            .values()
            .find(|a| !(a.stats.belief_threshold > 0.0 && a.stats.belief_threshold <= 100.0))
            .map(|a| (a.id.clone(), a.stats.belief_threshold));
        check(
            &mut checks,
            "stats",
            "stats.belief_threshold_range",
            "Every belief threshold is within (0, 100]",
            thr_bad.is_none(),
            thr_bad
                .as_ref()
                .map(|(a, v)| format!("{a} threshold={v}"))
                .unwrap_or_else(|| "all in range".into()),
            thr_bad.map(
                |(archetype, value)| ActorValidationError::BeliefThresholdOutOfRange {
                    archetype,
                    value,
                },
            ),
        );

        // --- modifier ops finite -------------------------------------------------
        let op_bad = self.modifiers.values().find_map(|m| {
            m.ops
                .iter()
                .find(|o| !o.value.is_finite())
                .map(|o| (m.id.clone(), o.stat))
        });
        check(
            &mut checks,
            "modifiers",
            "modifiers.ops_finite",
            "Every modifier op value is a finite number",
            op_bad.is_none(),
            op_bad
                .as_ref()
                .map(|(m, s)| format!("{m} op on {s:?} is not finite"))
                .unwrap_or_else(|| "all finite".into()),
            op_bad.map(
                |(modifier, stat)| ActorValidationError::NonFiniteModifierOp { modifier, stat },
            ),
        );

        ActorValidationReport { checks }
    }

    /// Convenience: `validate().ok()`.
    pub fn ok(&self) -> Result<(), Vec<ActorValidationError>> {
        self.validate().ok()
    }

    /// Apply a modifier set to a base archetype, yielding a deterministic
    /// composed actor: modifiers apply in the order given, and each modifier's
    /// ops apply in their listed order. Unknown ids are typed errors, never
    /// panics.
    pub fn compose(&self, base: &str, mods: &[&str]) -> Result<ComposedActor, ComposeError> {
        let archetype = self
            .archetypes
            .get(base)
            .ok_or_else(|| ComposeError::UnknownArchetype(base.to_owned()))?;
        let mut composed = ComposedActor::from_archetype(archetype);
        for id in mods {
            let modifier = self
                .modifiers
                .get(*id)
                .ok_or_else(|| ComposeError::UnknownModifier((*id).to_owned()))?;
            for op in &modifier.ops {
                op.apply(&mut composed.stats);
            }
            composed.modifiers.push(modifier.id.clone());
        }
        Ok(composed)
    }
}

/// Parse and validate an actor-pack payload — the seam a UMS actor-pack DLC
/// (payload format `"idaptik-actors/1"`) plugs into. The transport and merge
/// policy live above this crate; this function guarantees that whatever comes
/// through is a registry the simulation can trust.
pub fn load_actor_pack(json: &str) -> Result<ActorRegistry, ActorPackError> {
    let registry: ActorRegistry =
        serde_json::from_str(json).map_err(|e| ActorPackError::Parse(e.to_string()))?;
    registry.ok().map_err(ActorPackError::Invalid)?;
    Ok(registry)
}

/// Why an actor pack was refused.
#[derive(Debug, Clone, PartialEq)]
pub enum ActorPackError {
    /// The payload is not the JSON shape of an [`ActorRegistry`].
    Parse(String),
    /// The payload parsed but failed validation.
    Invalid(Vec<ActorValidationError>),
}

// ---------------------------------------------------------------------------
// The canonical registry — Billy as data, plus two canonical modified forms.
// ---------------------------------------------------------------------------

/// Billy, the Ghost Lobby's guard, as the default archetype: every number is
/// projected from [`crate::scenario::constants`], so the compile-time audit
/// and the data surface stay in lock-step (the `actor_registry` tests assert
/// the projection equality, as `json_roundtrip` does for the scenario).
pub fn billy_archetype() -> ActorArchetype {
    ActorArchetype {
        id: BILLY_ARCHETYPE_ID.to_owned(),
        name: "Billy".to_owned(),
        stats: ActorStats {
            enter_speed: c::ENTER_SPEED,
            assess_speed: c::ASSESS_SPEED,
            invest_speed: c::INVEST_SPEED,
            secure_speed: c::SECURE_SPEED,
            guard_speed: c::GUARD_SPEED,
            pursue_speed: c::PURSUE_SPEED,
            shock_t: c::SHOCK_T,
            call_t: c::CALL_T,
            patrol_lo: c::PATROL_LO,
            patrol_hi: c::PATROL_HI,
            patrol_pivot: c::PATROL_PIVOT,
            grab_dist: c::GRAB_DIST,
            pursue_trigger: c::PURSUE_TRIGGER,
            pursue_giveup_ago: c::PURSUE_GIVEUP_AGO,
            pursue_giveup_dist: c::PURSUE_GIVEUP_DIST,
            accel: c::BILLY_ACCEL,
            sight_back: c::SIGHT_BACK,
            sight_crouch: c::SIGHT_CROUCH,
            sight_alert: c::SIGHT_ALERT,
            sight_alert_hi: c::SIGHT_ALERT_HI,
            alert_boost_div: c::ALERT_BOOST_DIV,
            lights_boost_k: c::LIGHTS_BOOST_K,
            distract_dist: c::VAC_DISTRACT,
            invest_noise: c::NOISE_INVEST_NOISE,
            invest_dist: c::NOISE_INVEST_DIST,
            belief_threshold: c::BELIEF_THRESHOLD,
            pi_seen: c::PI_SEEN,
            pi_sprint: c::PI_SPRINT,
            pi_decay: c::PI_DECAY,
        },
        interests: BTreeMap::from([
            (
                NOTE_OBJECT.to_owned(),
                InterestProfile {
                    kind: ObjectClass::Objective,
                    value_signal: 0.9,
                    near: c::NOTE_NEAR,
                    urg_still: c::NOTE_URG_STILL,
                    urg_move: c::NOTE_URG_MOVE,
                    urg_sprint: c::NOTE_URG_SPRINT,
                    carry: c::NOTE_CARRY,
                    decay: c::NOTE_DECAY,
                    guard_t: c::GUARD_T_NOTE,
                },
            ),
            (
                USB_OBJECT.to_owned(),
                InterestProfile {
                    kind: ObjectClass::Decoy,
                    value_signal: 0.75,
                    near: c::USB_NEAR,
                    urg_still: c::USB_URG_STILL,
                    urg_move: c::USB_URG_MOVE,
                    urg_sprint: c::USB_URG_SPRINT,
                    carry: c::USB_CARRY,
                    decay: c::USB_DECAY,
                    guard_t: c::GUARD_T_USB,
                },
            ),
        ]),
    }
}

/// Billy's unmodified composed form — what [`crate::scenario::sim`] runs.
/// Infallible by construction, so the sim's no-panic guarantee holds.
pub fn billy_actor() -> ComposedActor {
    ComposedActor::from_archetype(&billy_archetype())
}

/// The canonical registry: Billy plus two canonical modified forms from the
/// training-ground prototype's economy ("veteran" and "skittish").
pub fn default_registry() -> ActorRegistry {
    ActorRegistry {
        format: ACTORS_FORMAT.to_owned(),
        archetypes: BTreeMap::from([(BILLY_ARCHETYPE_ID.to_owned(), billy_archetype())]),
        modifiers: BTreeMap::from([
            (
                "veteran".to_owned(),
                Modifier {
                    id: "veteran".to_owned(),
                    name: "Veteran".to_owned(),
                    ops: vec![
                        StatOp {
                            stat: StatId::PursueSpeed,
                            op: StatOpKind::Mul,
                            value: 1.15,
                        },
                        StatOp {
                            stat: StatId::BeliefThreshold,
                            op: StatOpKind::Add,
                            value: -6.0,
                        },
                        StatOp {
                            stat: StatId::SightCrouch,
                            op: StatOpKind::Mul,
                            value: 1.15,
                        },
                        StatOp {
                            stat: StatId::PursueGiveupAgo,
                            op: StatOpKind::Add,
                            value: 1.4,
                        },
                    ],
                },
            ),
            (
                "skittish".to_owned(),
                Modifier {
                    id: "skittish".to_owned(),
                    name: "Skittish".to_owned(),
                    ops: vec![
                        StatOp {
                            stat: StatId::AssessSpeed,
                            op: StatOpKind::Mul,
                            value: 1.2,
                        },
                        StatOp {
                            stat: StatId::BeliefThreshold,
                            op: StatOpKind::Mul,
                            value: 0.5,
                        },
                        StatOp {
                            stat: StatId::PiDecay,
                            op: StatOpKind::Add,
                            value: 0.45,
                        },
                        StatOp {
                            stat: StatId::PursueGiveupDist,
                            op: StatOpKind::Mul,
                            value: 1.5,
                        },
                    ],
                },
            ),
        ]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rewrites the committed golden JSON from the current registry. Ignored
    /// by default; run explicitly after an intentional registry change:
    /// `cargo test -p idaptik-core regenerate_actors_json -- --ignored`.
    #[test]
    #[ignore = "regenerates the committed golden; run manually"]
    fn regenerate_actors_json() {
        let reg = default_registry();
        let json = serde_json::to_string_pretty(&reg).expect("serialize");
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/scenario/actor/actors.json"
        );
        std::fs::write(path, json.as_bytes()).expect("write golden json");
    }
}
