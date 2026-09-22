//! The declarative Moletaire companion definition — content as data.
//!
//! A [`CompanionDefinition`] is pure serde data: the full tuning table from
//! the archive's `Moletaire.res` `Tuning` module (plus the inline literals its
//! logic used, promoted to named fields), the hunger model constants and
//! per-level configs, the seven equipment items, the five coprocessor effect
//! ladders, the ten synthesised sounds, and the chiptune pattern data. It
//! round-trips through JSON unchanged ([`MOLETAIRE_JSON`] is the committed
//! golden — same pattern as `scenario/ghost_lobby.json`) and self-validates
//! via [`CompanionDefinition::validate`] / [`CompanionDefinition::ok`].

use crate::companion::coprocessors::{Level, VibrationReading};
use crate::companion::equipment::{ALL_EQUIPMENT, Equipment};
use crate::companion::hunger::{self, HungerConfig};
use crate::companion::music;
use serde::{Deserialize, Serialize};

/// The companion definition format tag.
pub const COMPANION_FORMAT: &str = "idaptik-moletaire/1";
/// The stable companion id.
pub const COMPANION_ID: &str = "moletaire";
/// The runtime-snapshot export format tag (see
/// [`crate::companion::sim::MoletaireSnapshot`]).
pub const MOLETAIRE_SNAPSHOT_FORMAT: &str = "idaptik-moletaire-runtime-v1";

/// A committed pretty-printed JSON of [`moletaire`]. Regenerate with the
/// ignored `regenerate_golden_json` test if the definition ever changes; the
/// `json_roundtrip` test proves it parses back equal to [`moletaire`].
pub const MOLETAIRE_JSON: &str = include_str!("moletaire.json");

/// The full movement / action / hunger tuning table. Fields up to
/// `training_hunger_rate` are the archive `Tuning` module verbatim; the rest
/// are the inline literals from `Moletaire.res` logic, promoted to named data
/// so no magic number hides in the simulation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoleTuning {
    /// Fast tunnelling speed, px/s.
    pub underground_speed: f64,
    /// Very slow surface speed, px/s.
    pub above_ground_speed: f64,
    /// Surface speed with the skateboard equipped, px/s.
    pub skateboard_speed: f64,
    /// Deepest depth (0.0 = surface, 1.0 = deepest).
    pub max_depth: f64,
    /// Below this depth the mole is effectively on the surface.
    pub surface_threshold: f64,
    /// Dogs can detect the mole at this depth or shallower.
    pub dog_detection_depth: f64,
    /// Trap dig duration, seconds.
    pub trap_dig_duration_sec: f64,
    /// Cable sabotage (chew) duration, seconds.
    pub cable_sabotage_duration_sec: f64,
    /// Wire distraction duration, seconds.
    pub distraction_duration_sec: f64,
    /// Time to dodge after a trap triggers, seconds.
    pub dodge_window_sec: f64,
    /// Flash stun duration, seconds.
    pub flash_stun_duration_sec: f64,
    /// Chance the mole eats a carried item on delivery.
    pub item_eat_chance: f64,
    /// Default carry capacity.
    pub base_carry_capacity: u32,
    /// Carry capacity with the rucksack.
    pub rucksack_carry_capacity: u32,
    /// Glide horizontal distance = launch height × this.
    pub glider_height_multiplier: f64,
    /// Glider fall speed, px/s.
    pub glider_fall_speed: f64,
    /// Body width, px (rendering + hitbox).
    pub body_width: f64,
    /// Body height, px (rendering + hitbox).
    pub body_height: f64,
    /// Nose radius, px (rendering).
    pub nose_radius: f64,
    /// Surface dirt-mound indicator width, px.
    pub dirt_mound_width: f64,
    /// Surface dirt-mound indicator height, px.
    pub dirt_mound_height: f64,
    /// Base hunger units per second (~67 s to max).
    pub hunger_rate: f64,
    /// Hunger level above which the mole periodically resists control.
    pub hungry_threshold: f64,
    /// Hunger level above which the mole is starving.
    pub starving_threshold: f64,
    /// Seconds between hunger-resistance episodes.
    pub hunger_fight_interval: f64,
    /// Seconds the mole fights the controller per episode.
    pub hunger_fight_duration: f64,
    /// Speed when hunger-driven toward food, px/s.
    pub hunger_eat_speed: f64,
    /// Faster hunger rate for training-mode visibility.
    pub training_hunger_rate: f64,
    // --- inline literals from the archive logic, promoted to data ---
    /// Depth change speed, depth units per second (archive `depthSpeed`).
    pub depth_speed: f64,
    /// Depth snap epsilon (archive `absFloat(depthDiff) < 0.02`).
    pub depth_epsilon: f64,
    /// Arrival snap distance in px (archive `dist < 5.0`).
    pub arrive_distance: f64,
    /// Underground speed multiplier while carrying (archive `*. 0.7`).
    pub carrying_underground_multiplier: f64,
    /// Wire pull radius in px (archive `dist < 200.0`).
    pub wire_distraction_range: f64,
    /// Coprocessor scan period, seconds (archive `scanTimer = 0.5`).
    pub scan_interval_sec: f64,
    /// Glide progress per second (archive `glideProgress +. dt *. 0.5`).
    pub glide_progress_rate: f64,
    /// Partial hunger drag factor when not resisting (archive `*. 0.3`).
    pub hunger_drag_multiplier: f64,
    /// Starving-wander speed factor of `above_ground_speed` (archive `*. 0.3`).
    pub wander_speed_multiplier: f64,
    /// Starving-wander triggers when the nearest edible is further than this.
    pub wander_min_distance: f64,
    /// Auto-dive target when ordered underground from near the surface.
    pub dive_target_depth: f64,
    /// Depth below which an underground order triggers the auto-dive.
    pub dive_trigger_depth: f64,
    /// Minimum depth required to dig a trap or sabotage a cable.
    pub dig_min_depth: f64,
    /// Rendering: `visual_y = y + depth * this` (archive `depth *. 60.0`).
    pub visual_depth_offset: f64,
    /// Hitbox: `top_y = y - body_height - depth * this` (archive `*. 10.0`).
    pub hitbox_depth_offset: f64,
}

/// The hunger model constants and per-level configs (`MoletaireHunger.res`),
/// mirrored from the canonical constants in [`crate::companion::hunger`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HungerDef {
    pub peckish_threshold: f64,
    pub hungry_threshold: f64,
    pub starving_threshold: f64,
    pub objective_eat_threshold: f64,
    /// Gravitational constant for the inverse-square pull.
    pub gravity_g: f64,
    /// Minimum squared distance clamp.
    pub min_dist_sq: f64,
    /// Maximum pull force cap.
    pub max_pull: f64,
    pub default_config: HungerConfig,
    pub hard_config: HungerConfig,
    pub ravenous_config: HungerConfig,
}

/// One equipment item, as data (kind serializes as its save-string code).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquipmentDef {
    pub kind: Equipment,
    pub name: String,
    pub description: String,
}

/// The five coprocessor effect ladders, indexed by [`Level`] (Stock, MK-I,
/// MK-II, MK-III). Exactly the archive's pure-ReScript ladders.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoprocessorLadders {
    /// AudioSynthesiser: distinct sounds available.
    pub audio_sound_count: [u32; 4],
    /// PathOptimiser: underground travel efficiency multiplier.
    pub path_efficiency: [f64; 4],
    /// SignalProcessor: detection range ahead underground, grid cells.
    pub sensor_range: [u32; 4],
    /// VibrationAnalyser: information quality about movement above.
    pub vibration_quality: [VibrationReading; 4],
    /// StabilisationCore: multiplier on `item_eat_chance`.
    pub eat_chance_multiplier: [f64; 4],
}

/// Pick a ladder rung by level — constant indices, cannot panic.
fn rung<T: Copy>(arr: &[T; 4], level: Level) -> T {
    match level {
        Level::Stock => arr[0],
        Level::Basic => arr[1],
        Level::Enhanced => arr[2],
        Level::Overclocked => arr[3],
    }
}

impl CoprocessorLadders {
    /// Sounds available at an AudioSynthesiser level (archive `audioSoundCount`).
    pub fn audio_sound_count(&self, level: Level) -> u32 {
        rung(&self.audio_sound_count, level)
    }

    /// Travel efficiency at a PathOptimiser level (archive `pathEfficiency`).
    pub fn path_efficiency(&self, level: Level) -> f64 {
        rung(&self.path_efficiency, level)
    }

    /// Scan range at a SignalProcessor level (archive `sensorRange`).
    pub fn sensor_range(&self, level: Level) -> u32 {
        rung(&self.sensor_range, level)
    }

    /// Reading quality at a VibrationAnalyser level (archive `vibrationQuality`).
    pub fn vibration_quality(&self, level: Level) -> VibrationReading {
        rung(&self.vibration_quality, level)
    }

    /// Eat-chance multiplier at a StabilisationCore level
    /// (archive `eatChanceMultiplier`).
    pub fn eat_chance_multiplier(&self, level: Level) -> f64 {
        rung(&self.eat_chance_multiplier, level)
    }
}

/// The chiptune loop as pure data (`MoletaireMusic.res` minus the Web Audio
/// I/O), mirrored from the canonical constants in [`crate::companion::music`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MusicDef {
    pub bpm: f64,
    pub pattern_length: usize,
    pub schedule_ahead_time_sec: f64,
    pub scheduler_interval_ms: u32,
    pub melody_notes: Vec<f64>,
    pub bass_notes: Vec<f64>,
    pub melody_gain: f64,
    pub bass_gain: f64,
    pub melody_duration_steps: f64,
    pub bass_duration_steps: f64,
}

/// The complete declarative Moletaire companion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompanionDefinition {
    pub format: String,
    pub id: String,
    pub tuning: MoleTuning,
    pub hunger: HungerDef,
    pub equipment: Vec<EquipmentDef>,
    pub coprocessors: CoprocessorLadders,
    /// Synthesised sounds in AudioSynthesiser index order (archive
    /// `availableSounds`).
    pub available_sounds: Vec<String>,
    pub music: MusicDef,
}

/// A typed validation failure from [`CompanionDefinition::validate`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompanionValidationError {
    /// The format tag is not the one this build understands.
    UnsupportedFormat { found: String },
    /// The equipment list is not the seven-item single-slot set.
    WrongEquipmentCount { found: usize },
    /// Two equipment entries share a save-string code.
    DuplicateEquipmentCode { code: String },
    /// An equipment entry has an empty display name.
    EmptyEquipmentName { code: String },
    /// Hunger thresholds are not ordered `0 < hungry <= starving < eat <= 1`.
    ThresholdsUnordered,
    /// A movement speed is not strictly positive.
    NonPositiveSpeed { which: String },
    /// The delivery eat chance is outside `[0, 1]`.
    EatChanceOutOfRange { value: f64 },
    /// Carry capacities are not `1 <= base <= rucksack`.
    CarryCapacityInvalid { base: u32, rucksack: u32 },
    /// A coprocessor ladder decreases where it must not (or vice versa).
    LadderNotMonotonic { which: String },
    /// A music pattern's length does not match `pattern_length`.
    MusicPatternLengthMismatch { which: String, found: usize },
    /// The tempo is not strictly positive.
    NonPositiveBpm { value: f64 },
    /// Depth bands are not `0 < surface < dog_detection <= max`.
    DepthBandsUnordered,
    /// Fewer sounds than the deepest AudioSynthesiser ladder rung offers.
    NotEnoughSounds { found: usize, needed: u32 },
    /// A snapshot whose format tag is not the one this build restores.
    UnsupportedSnapshotFormat { found: String },
}

impl CompanionDefinition {
    /// Run the validation suite, returning every failure (empty = valid).
    pub fn validate(&self) -> Vec<CompanionValidationError> {
        use CompanionValidationError as E;
        let mut errs = Vec::new();
        let t = &self.tuning;

        if self.format != COMPANION_FORMAT {
            errs.push(E::UnsupportedFormat {
                found: self.format.clone(),
            });
        }

        if self.equipment.len() != ALL_EQUIPMENT.len() {
            errs.push(E::WrongEquipmentCount {
                found: self.equipment.len(),
            });
        }
        let mut seen: Vec<&str> = Vec::new();
        for eq in &self.equipment {
            let code = eq.kind.code();
            if seen.contains(&code) {
                errs.push(E::DuplicateEquipmentCode {
                    code: code.to_owned(),
                });
            }
            seen.push(code);
            if eq.name.is_empty() {
                errs.push(E::EmptyEquipmentName {
                    code: code.to_owned(),
                });
            }
        }

        let h = &self.hunger;
        if !(0.0 < h.hungry_threshold
            && h.hungry_threshold <= h.starving_threshold
            && h.starving_threshold < h.objective_eat_threshold
            && h.objective_eat_threshold <= 1.0)
        {
            errs.push(E::ThresholdsUnordered);
        }

        for (which, v) in [
            ("underground_speed", t.underground_speed),
            ("above_ground_speed", t.above_ground_speed),
            ("skateboard_speed", t.skateboard_speed),
            ("hunger_eat_speed", t.hunger_eat_speed),
            ("depth_speed", t.depth_speed),
        ] {
            if v <= 0.0 {
                errs.push(E::NonPositiveSpeed {
                    which: which.to_owned(),
                });
            }
        }

        if !(0.0..=1.0).contains(&t.item_eat_chance) {
            errs.push(E::EatChanceOutOfRange {
                value: t.item_eat_chance,
            });
        }

        if t.base_carry_capacity < 1 || t.rucksack_carry_capacity < t.base_carry_capacity {
            errs.push(E::CarryCapacityInvalid {
                base: t.base_carry_capacity,
                rucksack: t.rucksack_carry_capacity,
            });
        }

        if !(0.0 < t.surface_threshold
            && t.surface_threshold < t.dog_detection_depth
            && t.dog_detection_depth <= t.max_depth)
        {
            errs.push(E::DepthBandsUnordered);
        }

        let c = &self.coprocessors;
        if !c.audio_sound_count.is_sorted() {
            errs.push(E::LadderNotMonotonic {
                which: "audio_sound_count".to_owned(),
            });
        }
        if !c.path_efficiency.is_sorted() {
            errs.push(E::LadderNotMonotonic {
                which: "path_efficiency".to_owned(),
            });
        }
        if !c.sensor_range.is_sorted() {
            errs.push(E::LadderNotMonotonic {
                which: "sensor_range".to_owned(),
            });
        }
        if !c.vibration_quality.is_sorted() {
            errs.push(E::LadderNotMonotonic {
                which: "vibration_quality".to_owned(),
            });
        }
        // Eat chance improves (decreases) with level.
        if !c.eat_chance_multiplier.iter().rev().is_sorted() {
            errs.push(E::LadderNotMonotonic {
                which: "eat_chance_multiplier".to_owned(),
            });
        }

        let needed = c.audio_sound_count(Level::Overclocked);
        if (self.available_sounds.len() as u32) < needed {
            errs.push(E::NotEnoughSounds {
                found: self.available_sounds.len(),
                needed,
            });
        }

        let m = &self.music;
        if m.bpm <= 0.0 {
            errs.push(E::NonPositiveBpm { value: m.bpm });
        }
        if m.melody_notes.len() != m.pattern_length {
            errs.push(E::MusicPatternLengthMismatch {
                which: "melody_notes".to_owned(),
                found: m.melody_notes.len(),
            });
        }
        if m.bass_notes.len() != m.pattern_length {
            errs.push(E::MusicPatternLengthMismatch {
                which: "bass_notes".to_owned(),
                found: m.bass_notes.len(),
            });
        }

        errs
    }

    /// `Ok(())` if the definition validates, else every failure.
    pub fn ok(&self) -> Result<(), Vec<CompanionValidationError>> {
        let errs = self.validate();
        if errs.is_empty() { Ok(()) } else { Err(errs) }
    }
}

/// Build the canonical Moletaire definition by projecting the archive's
/// constants into data (the companion analogue of `scenario::ghost_lobby`).
pub fn moletaire() -> CompanionDefinition {
    CompanionDefinition {
        format: COMPANION_FORMAT.to_owned(),
        id: COMPANION_ID.to_owned(),
        tuning: MoleTuning {
            underground_speed: 200.0,
            above_ground_speed: 25.0,
            skateboard_speed: 55.0,
            max_depth: 1.0,
            surface_threshold: 0.05,
            dog_detection_depth: 0.3,
            trap_dig_duration_sec: 4.0,
            cable_sabotage_duration_sec: 3.0,
            distraction_duration_sec: 5.0,
            dodge_window_sec: 0.5,
            flash_stun_duration_sec: 2.5,
            item_eat_chance: 0.05,
            base_carry_capacity: 1,
            rucksack_carry_capacity: 3,
            glider_height_multiplier: 3.0,
            glider_fall_speed: 60.0,
            body_width: 20.0,
            body_height: 14.0,
            nose_radius: 3.0,
            dirt_mound_width: 16.0,
            dirt_mound_height: 6.0,
            hunger_rate: 0.015,
            hungry_threshold: hunger::HUNGRY_THRESHOLD,
            starving_threshold: hunger::STARVING_THRESHOLD,
            hunger_fight_interval: 3.0,
            hunger_fight_duration: 1.5,
            hunger_eat_speed: 120.0,
            training_hunger_rate: 0.035,
            depth_speed: 0.8,
            depth_epsilon: 0.02,
            arrive_distance: 5.0,
            carrying_underground_multiplier: 0.7,
            wire_distraction_range: 200.0,
            scan_interval_sec: 0.5,
            glide_progress_rate: 0.5,
            hunger_drag_multiplier: 0.3,
            wander_speed_multiplier: 0.3,
            wander_min_distance: 500.0,
            dive_target_depth: 0.5,
            dive_trigger_depth: 0.3,
            dig_min_depth: 0.1,
            visual_depth_offset: 60.0,
            hitbox_depth_offset: 10.0,
        },
        hunger: HungerDef {
            peckish_threshold: hunger::PECKISH_THRESHOLD,
            hungry_threshold: hunger::HUNGRY_THRESHOLD,
            starving_threshold: hunger::STARVING_THRESHOLD,
            objective_eat_threshold: hunger::OBJECTIVE_EAT_THRESHOLD,
            gravity_g: hunger::GRAVITY_G,
            min_dist_sq: hunger::MIN_DIST_SQ,
            max_pull: hunger::MAX_PULL,
            default_config: hunger::DEFAULT_CONFIG,
            hard_config: hunger::HARD_CONFIG,
            ravenous_config: hunger::RAVENOUS_CONFIG,
        },
        equipment: ALL_EQUIPMENT
            .iter()
            .map(|&kind| EquipmentDef {
                kind,
                name: kind.name().to_owned(),
                description: kind.description().to_owned(),
            })
            .collect(),
        coprocessors: CoprocessorLadders {
            audio_sound_count: [0, 3, 6, 10],
            path_efficiency: [1.0, 1.15, 1.30, 1.50],
            sensor_range: [1, 3, 5, 8],
            vibration_quality: [
                VibrationReading::NoData,
                VibrationReading::DirectionOnly,
                VibrationReading::DirectionAndIntent,
                VibrationReading::FullProfile,
            ],
            eat_chance_multiplier: [1.0, 0.6, 0.2, 0.05],
        },
        available_sounds: [
            "footstep",
            "door_creak",
            "radio_static",
            "alarm_beep",
            "dog_bark",
            "guard_cough",
            "intercom_buzz",
            "phone_ring",
            "generator_hum",
            "voice_mimicry",
        ]
        .iter()
        .map(|s| (*s).to_owned())
        .collect(),
        music: MusicDef {
            bpm: music::BPM,
            pattern_length: music::PATTERN_LENGTH,
            schedule_ahead_time_sec: music::SCHEDULE_AHEAD_TIME_SEC,
            scheduler_interval_ms: music::SCHEDULER_INTERVAL_MS,
            melody_notes: music::MELODY_NOTES.to_vec(),
            bass_notes: music::BASS_NOTES.to_vec(),
            melody_gain: music::MELODY_GAIN,
            bass_gain: music::BASS_GAIN,
            melody_duration_steps: music::MELODY_DURATION_STEPS,
            bass_duration_steps: music::BASS_DURATION_STEPS,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rewrites the committed golden JSON from the current definition. Ignored
    /// by default; run explicitly after an intentional definition change:
    /// `cargo test -p idaptik-core companion::definition::tests::regenerate_golden_json -- --ignored`.
    #[test]
    #[ignore = "regenerates the committed golden; run manually"]
    fn regenerate_golden_json() {
        let def = moletaire();
        let json = serde_json::to_string_pretty(&def).expect("serialize");
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/companion/moletaire.json");
        std::fs::write(path, json.as_bytes()).expect("write golden json");
    }

    /// The committed golden parses back equal to the Rust-authored definition
    /// and validates clean.
    #[test]
    fn json_roundtrip() {
        let parsed: CompanionDefinition =
            serde_json::from_str(MOLETAIRE_JSON).expect("golden parses");
        assert_eq!(parsed, moletaire(), "golden drifted from moletaire()");
        assert_eq!(parsed.validate(), Vec::new());
    }

    #[test]
    fn canonical_definition_validates() {
        assert_eq!(moletaire().ok(), Ok(()));
    }
}
