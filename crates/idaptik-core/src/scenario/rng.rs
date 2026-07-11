//! Exact port of the prototype's `mulberry32` RNG and the reset roll.
//!
//! The JS accumulates the seed in a growing float but every downstream operator
//! (`>>>`, `^`, `|`, `Math.imul`) works on the low 32 bits, which equals wrapping
//! `u32` arithmetic — so this port is bit-identical (verified against seed
//! `123456`). The reset **draw order** is load-bearing: `note.x`, `usb.x`,
//! `arrival`, `stale_pulse`, per-door `route_delay` (with an Operator-only extra
//! draw), then `snack_x`.

use crate::scenario::definition::ScenarioDefinition;
use crate::scenario::mathf::lerp;
use crate::scenario::tuning::DifficultyId;
use serde::{Deserialize, Serialize};

/// `mulberry32` PRNG. The `state` is serialized so a snapshot resumes the exact
/// sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mulberry32 {
    pub state: u32,
}

impl Mulberry32 {
    /// Seed the generator.
    pub fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    /// Next 32-bit output.
    pub fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_add(0x6D2B_79F5);
        let mut t = self.state;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        t ^ (t >> 14)
    }

    /// Next float in `[0, 1)` — `u32 / 2^32`, exactly as the prototype.
    pub fn next_f64(&mut self) -> f64 {
        f64::from(self.next_u32()) / 4_294_967_296.0
    }
}

/// The values rolled once at reset from the seed and difficulty.
#[derive(Debug, Clone, PartialEq)]
pub struct InitRoll {
    /// Note spawn x (kitchen).
    pub note_x: f64,
    /// USB spawn x (office trap).
    pub usb_x: f64,
    /// Billy arrival time (crisis countdown).
    pub arrival: f64,
    /// Stale-camera pulse phase (drawn, but unused by the sim — kept for parity).
    pub stale_pulse: f64,
    /// Per-door routing delay before a hold takes effect.
    pub door_delay: Vec<f64>,
    /// Billy's snack fixation x.
    pub snack_x: f64,
}

/// Perform the reset roll. Consumes exactly 9 draws on Story/Standard and 13 on
/// Operator (each door draws one extra for the penalty check).
pub fn roll_init(
    def: &ScenarioDefinition,
    rng: &mut Mulberry32,
    difficulty: DifficultyId,
) -> InitRoll {
    let sp = &def.spawn;

    let note_x = sp.note_x.0 + rng.next_f64() * sp.note_x.1;
    let usb_x = sp.usb_x.0 + rng.next_f64() * sp.usb_x.1;

    let (a0, a1) = def
        .difficulty
        .get(&difficulty)
        .map(|p| p.arrival)
        .unwrap_or((0.0, 0.0));
    let arrival = lerp(a0, a1, rng.next_f64());

    let stale_pulse = sp.stale_pulse.0 + rng.next_f64() * sp.stale_pulse.1;

    let is_operator = difficulty == DifficultyId::Operator;
    let mut door_delay = Vec::with_capacity(def.doors.len());
    for i in 0..def.doors.len() {
        let mut delay = sp.door_delay.0 + rng.next_f64() * sp.door_delay.1;
        if is_operator {
            // JS: `i === Math.floor(rng() * 4)` — the `&&` left side is true on
            // Operator, so this draw ALWAYS happens. `4` is the door count.
            let pick = (rng.next_f64() * 4.0).floor();
            if (pick - i as f64).abs() < 0.5 {
                delay += sp.operator_door_penalty;
            }
        }
        door_delay.push(delay);
    }

    let snack_x = sp.snack_x.0 + rng.next_f64() * sp.snack_x.1;

    InitRoll {
        note_x,
        usb_x,
        arrival,
        stale_pulse,
        door_delay,
        snack_x,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_123456_u32_vector() {
        let mut r = Mulberry32::new(123456);
        assert_eq!(
            [r.next_u32(), r.next_u32(), r.next_u32(), r.next_u32()],
            [1642107918, 3424218114, 4280064779, 687244953]
        );
    }

    #[test]
    fn edge_seeds_do_not_panic() {
        for seed in [0u32, 1, u32::MAX] {
            let mut r = Mulberry32::new(seed);
            for _ in 0..1000 {
                let v = r.next_f64();
                assert!((0.0..1.0).contains(&v));
            }
        }
    }
}
