//! Gravity-based hunger system for Moletaire.
//!
//! Exact port of the archive's `MoletaireHunger.res`. Moletaire is attracted
//! to edible objects along an inverse-square law (`pull = G * hungerMult /
//! distSq`, X-axis only — the mole can't fly), with a piecewise hunger
//! multiplier (peckish 0.2x → hungry 1.0x → starving 3.0x) and a hard force
//! cap of 200 to prevent teleporting.
//!
//! Three hunger bands: Peckish (0.0–0.4) mild pull; Hungry (0.4–0.8) strong
//! pull plus periodic control resistance; Starving (0.8–1.0) devours anything
//! nearby, wanders aimlessly otherwise, and above 0.9 will even eat the
//! mission objective.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Thresholds — the canonical constants (mirrored into the definition data).
// ---------------------------------------------------------------------------

/// Below this: mild pull, easily overridden. Moletaire is cooperative.
pub const PECKISH_THRESHOLD: f64 = 0.4;
/// Above this: strong pull, periodic resistance, slowed movement.
pub const HUNGRY_THRESHOLD: f64 = 0.4;
/// Above this: eats anything on contact, wanders if nothing nearby.
pub const STARVING_THRESHOLD: f64 = 0.8;
/// Above this while carrying a mission objective: Moletaire WILL eat it.
pub const OBJECTIVE_EAT_THRESHOLD: f64 = 0.9;

/// Gravitational constant — tuned for gameplay feel.
pub const GRAVITY_G: f64 = 5000.0;
/// Minimum squared distance, preventing infinite force at zero distance.
pub const MIN_DIST_SQ: f64 = 100.0;
/// Maximum pull force, preventing teleporting.
pub const MAX_PULL: f64 = 200.0;
/// The "nearest distance" sentinel when no edible exists (archive `99999.0`).
pub const NO_EDIBLE_DIST: f64 = 99999.0;

// ---------------------------------------------------------------------------
// Edible objects and per-level configs
// ---------------------------------------------------------------------------

/// An object in the world that Moletaire finds edible.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdibleObject {
    /// Unique identifier for this object.
    pub id: String,
    /// World X position.
    pub x: f64,
    /// World Y position (dangling cables: pull without reachability).
    pub y: f64,
    /// Whether this object is the mission objective. Eating it = mission fail.
    pub is_mission_objective: bool,
    /// How much hunger is reduced when eaten (0.0–1.0).
    pub nutrition_value: f64,
    /// Whether this object has already been eaten or removed.
    pub consumed: bool,
}

/// Per-level hunger configuration (archive `hungerConfig`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HungerConfig {
    /// Starting hunger level (0.0 = full).
    pub starting_hunger: f64,
    /// Base hunger rate multiplier for this level (1.0 = normal).
    pub level_hunger_multiplier: f64,
    /// Whether Ravenous Mode is active (game setting override).
    pub ravenous_mode: bool,
}

/// Default config for early/tutorial levels.
pub const DEFAULT_CONFIG: HungerConfig = HungerConfig {
    starting_hunger: 0.0,
    level_hunger_multiplier: 1.0,
    ravenous_mode: false,
};

/// Config for harder levels — less forgiving.
pub const HARD_CONFIG: HungerConfig = HungerConfig {
    starting_hunger: 0.3,
    level_hunger_multiplier: 1.5,
    ravenous_mode: false,
};

/// Config for Ravenous Mode (difficulty setting).
pub const RAVENOUS_CONFIG: HungerConfig = HungerConfig {
    starting_hunger: 0.7,
    level_hunger_multiplier: 3.0,
    ravenous_mode: true,
};

// ---------------------------------------------------------------------------
// Gravity-based pull
// ---------------------------------------------------------------------------

/// The gravitational pull toward one edible (archive `calculatePull`).
/// Signed: positive pulls right, negative pulls left, zero when `dx == 0`.
pub fn calculate_pull(mole_x: f64, mole_y: f64, object_x: f64, object_y: f64, hunger: f64) -> f64 {
    let dx = object_x - mole_x;
    let dy = object_y - mole_y;
    let dist_sq = dx * dx + dy * dy;

    let clamped_dist_sq = if dist_sq < MIN_DIST_SQ {
        MIN_DIST_SQ
    } else {
        dist_sq
    };

    // Hunger multiplier: peckish 0.2x → hungry 1.0x → starving 3.0x, piecewise.
    let hunger_mult = if hunger < PECKISH_THRESHOLD {
        hunger / PECKISH_THRESHOLD * 0.2
    } else if hunger < STARVING_THRESHOLD {
        0.2 + (hunger - PECKISH_THRESHOLD) / (STARVING_THRESHOLD - PECKISH_THRESHOLD) * 0.8
    } else {
        1.0 + (hunger - STARVING_THRESHOLD) / (1.0 - STARVING_THRESHOLD) * 2.0
    };

    let direction = if dx > 0.0 {
        1.0
    } else if dx < 0.0 {
        -1.0
    } else {
        0.0
    };

    let magnitude = GRAVITY_G * hunger_mult / clamped_dist_sq;
    let capped = if magnitude > MAX_PULL {
        MAX_PULL
    } else {
        magnitude
    };

    direction * capped
}

/// The total pull from all non-consumed edibles (archive `calculateTotalPull`).
/// Returns `(total_force_x, nearest_id, nearest_distance)`; the distance is
/// [`NO_EDIBLE_DIST`] when nothing is edible.
pub fn calculate_total_pull(
    mole_x: f64,
    mole_y: f64,
    hunger: f64,
    edibles: &[EdibleObject],
) -> (f64, Option<String>, f64) {
    let mut total_force = 0.0;
    let mut nearest_id: Option<String> = None;
    let mut nearest_dist = NO_EDIBLE_DIST;

    for obj in edibles {
        if !obj.consumed {
            let dx = obj.x - mole_x;
            let dy = obj.y - mole_y;
            let dist = libm::sqrt(dx * dx + dy * dy);

            if dist < nearest_dist {
                nearest_dist = dist;
                nearest_id = Some(obj.id.clone());
            }

            // X-axis only — the mole can't fly.
            total_force += calculate_pull(mole_x, mole_y, obj.x, obj.y, hunger);
        }
    }

    (total_force, nearest_id, nearest_dist)
}

// ---------------------------------------------------------------------------
// Hunger rate
// ---------------------------------------------------------------------------

/// Hunger increase per second (archive `calculateHungerRate`): base rate times
/// movement (moving 1.5x / stationary 0.5x), acceleration (`1 + hunger * 0.5`),
/// coprocessor multiplier, and per-level multiplier.
pub fn calculate_hunger_rate(
    base_rate: f64,
    is_moving: bool,
    current_hunger: f64,
    coprocessor_multiplier: f64,
    level_multiplier: f64,
) -> f64 {
    let movement_mult = if is_moving { 1.5 } else { 0.5 };
    let acceleration_mult = 1.0 + current_hunger * 0.5;
    base_rate * movement_mult * acceleration_mult * coprocessor_multiplier * level_multiplier
}

// ---------------------------------------------------------------------------
// Behaviour bands and display
// ---------------------------------------------------------------------------

/// Current hunger behaviour state (archive `hungerBehaviour`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HungerBehaviour {
    /// Normal operation: mild pull, full player control.
    Cooperative,
    /// Periodically ignores player input toward the nearest edible.
    Resisting,
    /// Locked onto the nearest edible; eats on contact.
    Devouring,
    /// Starving with nothing nearby: wanders, needs a rescue pickup.
    Wandering,
}

/// The behaviour band for a hunger level (archive `getBehaviour`).
pub fn behaviour(hunger: f64, has_nearby_edible: bool) -> HungerBehaviour {
    if hunger > STARVING_THRESHOLD {
        if has_nearby_edible {
            HungerBehaviour::Devouring
        } else {
            HungerBehaviour::Wandering
        }
    } else if hunger > HUNGRY_THRESHOLD {
        HungerBehaviour::Resisting
    } else {
        HungerBehaviour::Cooperative
    }
}

/// HUD/debug display string (archive `hungerDisplayString`).
pub fn display_string(hunger: f64) -> &'static str {
    if hunger < 0.1 {
        "FULL"
    } else if hunger < PECKISH_THRESHOLD {
        "CONTENT"
    } else if hunger < 0.6 {
        "PECKISH"
    } else if hunger < STARVING_THRESHOLD {
        "HUNGRY"
    } else if hunger < OBJECTIVE_EAT_THRESHOLD {
        "STARVING"
    } else {
        "RAVENOUS"
    }
}

/// Colour for the hunger indicator, green → yellow → red (archive `hungerColor`).
pub fn color(hunger: f64) -> u32 {
    if hunger < PECKISH_THRESHOLD {
        0x0044_ff44 // Green — well-fed
    } else if hunger < 0.6 {
        0x00aa_ff44 // Yellow-green — peckish
    } else if hunger < STARVING_THRESHOLD {
        0x00ff_aa44 // Orange — hungry
    } else {
        0x00ff_4444 // Red — starving
    }
}

/// Whether Moletaire will eat a mission objective on contact
/// (archive `willEatObjective`) — slightly above the starving threshold.
pub fn will_eat_objective(hunger: f64) -> bool {
    hunger > OBJECTIVE_EAT_THRESHOLD
}

/// Whether the objective is at risk soon, for warning indicators
/// (archive `objectiveAtRisk`).
pub fn objective_at_risk(hunger: f64) -> bool {
    hunger > STARVING_THRESHOLD
}
