//! Moletaire's coprocessor bay — computational augments, not chassis upgrades.
//!
//! Ported from the archive's `MoletaireCoprocessors.res` (the pure ReScript
//! ladder, **not** the wasm "coprocessor bridge" mirror — the bridge carries a
//! deliberately replicated `can_carry_fragile` bug; the canonical rule is
//! `level >= Enhanced`). Five coprocessors, each on a four-step GURPS-style
//! ladder: Stock / MK-I / MK-II / MK-III.
//!
//! The numeric effect ladders live as data on
//! [`crate::companion::definition::CoprocessorLadders`]; the structural rules
//! (voice mimicry and vault weak points only at Overclocked, fragile carry at
//! Enhanced or better) live here on the bay, exactly as in the archive.

use crate::Deserialize;
use serde::Serialize;

/// Upgrade level for a coprocessor. Declaration order is the upgrade order, so
/// the derived `Ord` gives the archive's `level >= Enhanced` comparison.
// Creusot needs a *logical* model of that order before it will admit a `<` on
// this type. `derive(DeepModel)` is not it: the derive emits the deep-model
// type with no impls at all, so the ordering site then fails with
// `LevelDeepModel: OrdLogic is not satisfied`. The model is written by hand
// below, mapping each variant to its `value()`, which is the same order the
// derived `Ord` uses (no explicit discriminants, so declaration order rules).
// That is a specification, not proof debt -- a `logic` function is a
// definition and asserts nothing unproven.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default, Hash,
)]
pub enum Level {
    /// No coprocessor installed. Baseline capability.
    #[default]
    Stock,
    /// Basic augment (MK-I). Modest improvement, no trade-offs.
    Basic,
    /// Enhanced augment (MK-II). Significant improvement.
    Enhanced,
    /// Overclocked (MK-III). Maximum capability. May introduce quirks.
    Overclocked,
}

/// Logical model of [`Level`] for Creusot: the upgrade ladder as an integer,
/// identical to [`Level::value`]. Ordering comparisons on `Level` are verified
/// against this, so `level >= Level::Enhanced` means `deep_model >= 2` in the
/// proof. Kept adjacent to `value()` deliberately: if one changes the other
/// must, and `level_model_matches_value` in this module's tests is the guard.
#[cfg(creusot)]
impl creusot_std::model::DeepModel for Level {
    type DeepModelTy = creusot_std::logic::Int;

    #[creusot_std::macros::logic(open)]
    fn deep_model(self) -> creusot_std::logic::Int {
        match self {
            Level::Stock => 0int,
            Level::Basic => 1int,
            Level::Enhanced => 2int,
            Level::Overclocked => 3int,
        }
    }
}

impl Level {
    /// Numeric value for calculations and ladder indexing (archive `levelValue`).
    pub fn value(self) -> usize {
        match self {
            Level::Stock => 0,
            Level::Basic => 1,
            Level::Enhanced => 2,
            Level::Overclocked => 3,
        }
    }

    /// Display name for the upgrade UI (archive `levelDisplayName`).
    pub fn display_name(self) -> &'static str {
        match self {
            Level::Stock => "STOCK",
            Level::Basic => "MK-I",
            Level::Enhanced => "MK-II",
            Level::Overclocked => "MK-III",
        }
    }

    /// Colour for the upgrade level badge (archive `levelColor`).
    pub fn color(self) -> u32 {
        match self {
            Level::Stock => 0x0055_5555,
            Level::Basic => 0x0044_aa44,
            Level::Enhanced => 0x0044_88ff,
            Level::Overclocked => 0x00ff_8844,
        }
    }

    /// The next level up, or `None` at the Overclocked cap.
    pub fn next(self) -> Option<Level> {
        match self {
            Level::Stock => Some(Level::Basic),
            Level::Basic => Some(Level::Enhanced),
            Level::Enhanced => Some(Level::Overclocked),
            Level::Overclocked => None,
        }
    }
}

/// The five coprocessor categories (archive `coprocessorType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum CoprocessorType {
    /// Synthesises environmental sounds; MK-III adds voice mimicry.
    AudioSynthesiser,
    /// Calculates optimal tunnelling routes. Same drill, smarter pathing.
    PathOptimiser,
    /// Underground sensing; MK-III detects vault weak points.
    SignalProcessor,
    /// Reads ground vibrations to infer movement above.
    VibrationAnalyser,
    /// Better grip on carried objects; reduces the item-eating behaviour.
    StabilisationCore,
}

/// All coprocessor types in the archive's display order.
pub const ALL_COPROCESSOR_TYPES: [CoprocessorType; 5] = [
    CoprocessorType::AudioSynthesiser,
    CoprocessorType::PathOptimiser,
    CoprocessorType::SignalProcessor,
    CoprocessorType::VibrationAnalyser,
    CoprocessorType::StabilisationCore,
];

impl CoprocessorType {
    /// Display name for the coprocessor slot (archive `coprocessorName`).
    pub fn name(self) -> &'static str {
        match self {
            CoprocessorType::AudioSynthesiser => "AUDIO SYNTHESISER",
            CoprocessorType::PathOptimiser => "PATH OPTIMISER",
            CoprocessorType::SignalProcessor => "SIGNAL PROCESSOR",
            CoprocessorType::VibrationAnalyser => "VIBRATION ANALYSER",
            CoprocessorType::StabilisationCore => "STABILISATION CORE",
        }
    }

    /// Description for the upgrade screen (archive `coprocessorDescription`).
    pub fn description(self) -> &'static str {
        match self {
            CoprocessorType::AudioSynthesiser => {
                "Synthesise environmental sounds to confuse guards. MK-III: ventriloquism."
            }
            CoprocessorType::PathOptimiser => {
                "Calculate smarter tunnelling routes. Same drill, better pathing."
            }
            CoprocessorType::SignalProcessor => {
                "Detect cables, items, and vault walls further ahead underground."
            }
            CoprocessorType::VibrationAnalyser => {
                "Read footstep vibrations. Know when guards approach or walk away."
            }
            CoprocessorType::StabilisationCore => {
                "Better grip on carried items. Dramatically reduces fumble and eat chance."
            }
        }
    }
}

/// Vibration analyser reading quality (archive `vibrationReading`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
pub enum VibrationReading {
    /// No data — coprocessor not installed.
    NoData,
    /// Something is moving nearby (direction only: left/right).
    DirectionOnly,
    /// Moving entity with intent (approaching, receding, stationary).
    DirectionAndIntent,
    /// Full reading: direction, intent, weight class and pace.
    FullProfile,
}

/// Logical model of [`VibrationReading`] for Creusot: the reading-quality
/// ladder as an integer, in declaration order. Unlike [`Level`] this enum has
/// no `value()` to mirror, so the ladder *is* the derived `Ord` and nothing
/// else -- which is exactly what a comparison such as `reading > NoData` means
/// in a proof. `vibration_reading_model_matches_order` in this module's tests
/// is what keeps the two from drifting.
#[cfg(creusot)]
impl creusot_std::model::DeepModel for VibrationReading {
    type DeepModelTy = creusot_std::logic::Int;

    #[creusot_std::macros::logic(open)]
    fn deep_model(self) -> creusot_std::logic::Int {
        match self {
            VibrationReading::NoData => 0int,
            VibrationReading::DirectionOnly => 1int,
            VibrationReading::DirectionAndIntent => 2int,
            VibrationReading::FullProfile => 3int,
        }
    }
}

/// Moletaire's coprocessor bay — one slot per type, five total, all starting
/// at [`Level::Stock`] (archive `makeBay`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CoprocessorBay {
    pub audio_synthesiser: Level,
    pub path_optimiser: Level,
    pub signal_processor: Level,
    pub vibration_analyser: Level,
    pub stabilisation_core: Level,
}

impl CoprocessorBay {
    /// A stock bay (all slots uninstalled).
    pub fn new() -> CoprocessorBay {
        CoprocessorBay::default()
    }

    /// The level installed in a slot (archive `get`).
    pub fn level(&self, ctype: CoprocessorType) -> Level {
        match ctype {
            CoprocessorType::AudioSynthesiser => self.audio_synthesiser,
            CoprocessorType::PathOptimiser => self.path_optimiser,
            CoprocessorType::SignalProcessor => self.signal_processor,
            CoprocessorType::VibrationAnalyser => self.vibration_analyser,
            CoprocessorType::StabilisationCore => self.stabilisation_core,
        }
    }

    fn level_mut(&mut self, ctype: CoprocessorType) -> &mut Level {
        match ctype {
            CoprocessorType::AudioSynthesiser => &mut self.audio_synthesiser,
            CoprocessorType::PathOptimiser => &mut self.path_optimiser,
            CoprocessorType::SignalProcessor => &mut self.signal_processor,
            CoprocessorType::VibrationAnalyser => &mut self.vibration_analyser,
            CoprocessorType::StabilisationCore => &mut self.stabilisation_core,
        }
    }

    /// Upgrade a slot to the next level. Returns `true` if it upgraded,
    /// `false` at the Overclocked cap (archive `upgrade`).
    pub fn upgrade(&mut self, ctype: CoprocessorType) -> bool {
        let slot = self.level_mut(ctype);
        match slot.next() {
            Some(next) => {
                *slot = next;
                true
            }
            None => false,
        }
    }

    /// Whether ventriloquism (voice mimicry) is available — MK-III
    /// AudioSynthesiser only (archive `canMimicVoice`).
    pub fn can_mimic_voice(&self) -> bool {
        self.audio_synthesiser == Level::Overclocked
    }

    /// Whether vault wall weak points are detectable — MK-III SignalProcessor
    /// only (archive `canDetectVaultWeakPoints`).
    pub fn can_detect_vault_weak_points(&self) -> bool {
        self.signal_processor == Level::Overclocked
    }

    /// Whether Moletaire can carry fragile items without breaking them.
    ///
    /// The canonical ReScript rule: `stabilisationCore.level >= Enhanced`.
    /// (The archive's wasm bridge mirror carries a deliberately replicated bug
    /// here — this port follows the ReScript.)
    pub fn can_carry_fragile(&self) -> bool {
        self.stabilisation_core >= Level::Enhanced
    }
}

#[cfg(test)]
mod creusot_model_tests {
    use super::{Level, VibrationReading};

    /// The Creusot deep model of [`Level`] is written by hand as
    /// `Stock => 0 .. Overclocked => 3`. Nothing in a normal build type-checks
    /// it, so this test is what stops the model and the code drifting apart:
    /// it pins `value()` to the same ladder and pins the derived `Ord` to it
    /// too, which is the property the proof actually relies on.
    #[test]
    fn level_model_matches_value() {
        let ladder = [
            Level::Stock,
            Level::Basic,
            Level::Enhanced,
            Level::Overclocked,
        ];
        for (i, lvl) in ladder.iter().enumerate() {
            assert_eq!(lvl.value(), i, "deep_model maps {lvl:?} to {i}");
        }
        for w in ladder.windows(2) {
            assert!(w[0] < w[1], "derived Ord must agree with the deep model");
        }
    }

    /// [`VibrationReading`]'s deep model is the declaration-order ladder
    /// `NoData => 0 .. FullProfile => 3`. There is no `value()` to check it
    /// against, so this test pins the only thing the proof relies on: that the
    /// derived `Ord` really is that ladder. Reordering the variants would
    /// silently change what `reading > NoData` proves; this fails first.
    #[test]
    fn vibration_reading_model_matches_order() {
        let ladder = [
            VibrationReading::NoData,
            VibrationReading::DirectionOnly,
            VibrationReading::DirectionAndIntent,
            VibrationReading::FullProfile,
        ];
        for w in ladder.windows(2) {
            assert!(w[0] < w[1], "derived Ord must agree with the deep model");
        }
        assert_eq!(ladder.iter().min(), Some(&VibrationReading::NoData));
        assert_eq!(ladder.iter().max(), Some(&VibrationReading::FullProfile));
    }
}
