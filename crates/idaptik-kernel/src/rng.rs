//! Exact port of the prototype's `mulberry32` RNG.
//!
//! The JS accumulates the seed in a growing float but every downstream operator
//! (`>>>`, `^`, `|`, `Math.imul`) works on the low 32 bits, which equals wrapping
//! `u32` arithmetic — so this port is bit-identical (verified against seed
//! `123456`).
//!
//! The reset roll that draws from this stream (`roll_init`, `InitRoll`) stays in
//! `idaptik-core`: it is float-valued throughout, so it cannot be proved, and
//! keeping it out of this crate is what makes the kernel float-free.

#[cfg(not(creusot))]
use serde::{Deserialize, Serialize};

#[cfg(creusot)]
use creusot_std::model::DeepModel;

/// `mulberry32` PRNG. The `state` is serialized so a snapshot resumes the exact
/// sequence.
#[derive(Debug, Clone, Eq)]
#[cfg_attr(not(creusot), derive(Serialize, Deserialize))]
pub struct Mulberry32 {
    pub state: u32,
}

/// Logical model of [`Mulberry32`]: its state as an unbounded integer.
#[cfg(creusot)]
impl DeepModel for Mulberry32 {
    type DeepModelTy = creusot_std::logic::Int;

    #[creusot_std::macros::logic(open)]
    fn deep_model(self) -> creusot_std::logic::Int {
        self.state.deep_model()
    }
}

/// Hand-written rather than derived — see [`crate::trace::Trace`]'s impl for
/// why a derived `PartialEq` cannot be proved under Creusot.
impl PartialEq for Mulberry32 {
    #[cfg_attr(creusot, creusot_std::macros::ensures(result == (self.deep_model() == rhs.deep_model())))]
    fn eq(&self, rhs: &Mulberry32) -> bool {
        self.state == rhs.state
    }
}

impl Mulberry32 {
    /// Seed the generator.
    #[cfg_attr(creusot, creusot_std::macros::ensures(result.deep_model() == seed@))]
    pub fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    /// Next 32-bit output.
    ///
    /// Deliberately uncontracted beyond what Creusot checks unconditionally.
    /// A PRNG step has no useful functional postcondition — its whole purpose
    /// is to be hard to characterise — but the proof is not vacuous: every
    /// operation here is wrapping or masking, and discharging this function
    /// proves it is panic- and overflow-free for all 2^32 states, which is the
    /// property that actually matters for a bit-exact port.
    ///
    /// An earlier draft specced this as `result@ == state@ + 1`, which is false
    /// at `u32::MAX`. A false contract is a mutant that passes for the wrong
    /// reason; it is recorded here so it is not reintroduced.
    pub fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_add(0x6D2B_79F5);
        let mut t = self.state;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        t ^ (t >> 14)
    }

    /// Next float in `[0, 1)` — `u32 / 2^32`, exactly as the prototype.
    ///
    /// `#[trusted]` for the same reason as `Trace::fraction`: Creusot 0.13 has
    /// no logical model of IEEE-754. The contract is empty, so it assumes
    /// nothing and stays sound. Ledgered.
    #[cfg_attr(creusot, creusot_std::macros::trusted)]
    pub fn next_f64(&mut self) -> f64 {
        f64::from(self.next_u32()) / 4_294_967_296.0
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

    #[test]
    fn equality_compares_state() {
        // `PartialEq` is hand-written here (see the impl), so it needs pinning.
        let a = Mulberry32::new(7);
        let b = Mulberry32::new(7);
        let mut c = Mulberry32::new(7);
        c.next_u32();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
