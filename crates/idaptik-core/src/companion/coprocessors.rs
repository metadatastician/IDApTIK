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

use serde::{Deserialize, Serialize};

/// Upgrade level for a coprocessor. Declaration order is the upgrade order, so
/// the derived `Ord` gives the archive's `level >= Enhanced` comparison.
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
