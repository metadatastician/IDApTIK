//! Moletaire's single-slot equipment: seven items, one carried at a time.
//!
//! Ported from the archive's `Moletaire.res` (the `equipment` type and its
//! metadata) and `MoletairePersistence.res` (the string codec). The codec is
//! the serde surface: [`Equipment`] serializes as its code string and
//! deserializes through [`Equipment::from_code`], which accepts the archive's
//! legacy aliases (`miniglider` → Glider, `flash` / `camera` → FlashCamera).

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Equipment items — Moletaire carries exactly one of these (or nothing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Equipment {
    /// Faster surface movement (55 px/s instead of 25 px/s). Useless underground.
    Skateboard,
    /// Uncontrolled glide from heights. Emergency escape. No steering.
    Glider,
    /// Break windows and thin barriers on impact. One use per approach.
    BatteringRam,
    /// Mark a location for satellite imagery or precision strike.
    Beacon,
    /// Extra carry capacity (3 items instead of 1). Essential for retrieval.
    Rucksack,
    /// Dazzle living things (guards, dogs) + enhanced night vision camera.
    FlashCamera,
    /// Cut into underground vaults. Extremely slow, dangerous, and loud.
    /// Requires the MK-III SignalProcessor to find vault weak points.
    PlasmaCutter,
}

/// All equipment options in the archive's display order.
pub const ALL_EQUIPMENT: [Equipment; 7] = [
    Equipment::Skateboard,
    Equipment::Glider,
    Equipment::BatteringRam,
    Equipment::Beacon,
    Equipment::Rucksack,
    Equipment::FlashCamera,
    Equipment::PlasmaCutter,
];

impl Equipment {
    /// Display name for the equipment selection UI (archive `equipmentName`).
    pub fn name(self) -> &'static str {
        match self {
            Equipment::Skateboard => "SKATEBOARD",
            Equipment::Glider => "GLIDER",
            Equipment::BatteringRam => "BATTERING RAM",
            Equipment::Beacon => "TARGET BEACON",
            Equipment::Rucksack => "RUCKSACK",
            Equipment::FlashCamera => "FLASH CAMERA",
            Equipment::PlasmaCutter => "PLASMA CUTTER",
        }
    }

    /// Description for the selection screen (archive `equipmentDescription`).
    pub fn description(self) -> &'static str {
        match self {
            Equipment::Skateboard => "Faster surface movement. Useless underground.",
            Equipment::Glider => "Emergency glide from heights. Uncontrollable.",
            Equipment::BatteringRam => "Break windows and thin barriers on impact.",
            Equipment::Beacon => "Mark a target for satellite imagery or precision strike.",
            Equipment::Rucksack => "Carry 3 items instead of 1. Essential for retrieval.",
            Equipment::FlashCamera => {
                "Dazzle guards and dogs. Night vision. Collect evidence for Q."
            }
            Equipment::PlasmaCutter => {
                "Cut into underground vaults. Slow, loud, dangerous. Requires MK-III Signal Processor."
            }
        }
    }

    /// Canonical save-string code (archive `equipmentToString`).
    pub fn code(self) -> &'static str {
        match self {
            Equipment::Skateboard => "skateboard",
            Equipment::Glider => "glider",
            Equipment::BatteringRam => "battering-ram",
            Equipment::Beacon => "beacon",
            Equipment::Rucksack => "rucksack",
            Equipment::FlashCamera => "flash-camera",
            Equipment::PlasmaCutter => "plasma-cutter",
        }
    }

    /// Parse a save-string code (archive `equipmentFromString`). Returns `None`
    /// for unknown/empty values. Accepts the legacy aliases from the old
    /// head+body save format: `miniglider` → [`Equipment::Glider`], `flash` and
    /// `camera` → [`Equipment::FlashCamera`].
    pub fn from_code(code: &str) -> Option<Equipment> {
        match code {
            "skateboard" => Some(Equipment::Skateboard),
            "glider" | "miniglider" => Some(Equipment::Glider),
            "battering-ram" => Some(Equipment::BatteringRam),
            "beacon" => Some(Equipment::Beacon),
            "rucksack" => Some(Equipment::Rucksack),
            "flash-camera" | "flash" | "camera" => Some(Equipment::FlashCamera),
            "plasma-cutter" => Some(Equipment::PlasmaCutter),
            _ => None,
        }
    }
}

impl Serialize for Equipment {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.code())
    }
}

impl<'de> Deserialize<'de> for Equipment {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Equipment::from_code(&s)
            .ok_or_else(|| D::Error::custom(format!("unknown Moletaire equipment code {s:?}")))
    }
}
