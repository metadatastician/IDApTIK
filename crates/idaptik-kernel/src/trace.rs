//! The intrusion trace — the pressure the hacker plays against — and the alerts
//! that noisy actions raise on a defended network.
//!
//! Deterministic and tick-based (no wall clock) so the core stays engine- and
//! transport-agnostic; the frontend and the Elixir session layer decide how
//! often to tick it.

#[cfg(not(creusot))]
use serde::{Deserialize, Serialize};

#[cfg(creusot)]
use creusot_std::model::DeepModel;

/// A live trace clock. It fills as the hacker works; when it reaches its
/// threshold the intrusion is traced.
///
/// The serde derives are compiled out under `cfg(creusot)`: serde's `Serialize`
/// translates but does not prove, and serialization is outside the proof
/// boundary either way. See the crate docs and `CREUSOT-PROOF-DEBT.tsv`.
#[derive(Debug, Clone, Eq)]
#[cfg_attr(not(creusot), derive(Serialize, Deserialize))]
pub struct Trace {
    progress: u32,
    threshold: u32,
}

/// Logical model of [`Trace`]: the pair of counters as unbounded integers.
#[cfg(creusot)]
impl DeepModel for Trace {
    type DeepModelTy = (creusot_std::logic::Int, creusot_std::logic::Int);

    #[creusot_std::macros::logic(open(self))]
    fn deep_model(self) -> (creusot_std::logic::Int, creusot_std::logic::Int) {
        (self.progress.deep_model(), self.threshold.deep_model())
    }
}

/// Hand-written rather than derived. A derived `PartialEq` cannot be proved
/// under Creusot: `creusot-std` specs the trait, a derived impl supplies no
/// contract of its own, and the resulting refinement goal is false by
/// construction. Spelling the impl out lets it carry the contract the trait
/// asks for, and this is the impl that actually ships — not a proof-only twin.
impl PartialEq for Trace {
    #[cfg_attr(creusot, creusot_std::macros::ensures(result == (self.deep_model() == rhs.deep_model())))]
    fn eq(&self, rhs: &Trace) -> bool {
        self.progress == rhs.progress && self.threshold == rhs.threshold
    }
}

impl Trace {
    /// A fresh trace that trips once `threshold` progress has accumulated.
    #[cfg_attr(creusot, creusot_std::macros::ensures(result.deep_model() == (0int, threshold@)))]
    pub fn new(threshold: u32) -> Self {
        Self {
            progress: 0,
            threshold,
        }
    }

    /// Advance the trace by one tick. `base` is the raw trace rate; `hops` is how
    /// many machines the hacker is bouncing through. Each extra hop divides the
    /// rate, so pivoting through intermediate boxes buys time — the classic
    /// bounce-to-slow-the-trace mechanic. Bouncing never speeds the trace up.
    ///
    /// Bouncing *slows* the trace but never *freezes* it: any active intrusion
    /// (`base > 0`) advances the trace by at least 1, however many hops it is
    /// bounced through. Only genuine inactivity (`base == 0`) yields no progress
    /// — otherwise a hacker could pivot through enough machines to divide the
    /// integer rate down to 0 and stall the trace indefinitely.
    ///
    /// The three `ensures` below are that paragraph, machine-checked: the
    /// threshold is never moved, progress never passes it, and an active
    /// intrusion short of the threshold always gains ground. The third is the
    /// anti-stall guarantee, and deleting the `.max(1)` in the body — the exact
    /// bug the paragraph warns about — makes it fail.
    #[cfg_attr(creusot, creusot_std::macros::ensures((^self).threshold@ == (*self).threshold@))]
    #[cfg_attr(creusot, creusot_std::macros::ensures((^self).progress@ <= (*self).threshold@))]
    #[cfg_attr(creusot, creusot_std::macros::ensures(
        base@ > 0 && (*self).progress@ < (*self).threshold@
            ==> (^self).progress@ > (*self).progress@))]
    pub fn advance(&mut self, base: u32, hops: u32) {
        let rate = if base == 0 {
            0
        } else {
            (base / hops.max(1)).max(1)
        };
        self.progress = self.progress.saturating_add(rate).min(self.threshold);
    }

    /// Whether the intrusion has been traced.
    #[cfg_attr(creusot, creusot_std::macros::ensures(result == (self.progress@ >= self.threshold@)))]
    pub fn traced(&self) -> bool {
        self.progress >= self.threshold
    }

    /// Progress so far.
    #[cfg_attr(creusot, creusot_std::macros::ensures(result@ == self.progress@))]
    pub fn progress(&self) -> u32 {
        self.progress
    }

    /// The threshold at which the trace trips.
    #[cfg_attr(creusot, creusot_std::macros::ensures(result@ == self.threshold@))]
    pub fn threshold(&self) -> u32 {
        self.threshold
    }

    /// Fraction complete, in `0.0..=1.0` — handy for a progress bar.
    ///
    /// Display-only, and the one float in this crate. Creusot has no logical
    /// model of IEEE-754, so it is `#[trusted]`: an empty contract, which
    /// assumes nothing and therefore stays sound — callers in proved code learn
    /// nothing about the result rather than learning something false. Ledgered.
    #[cfg_attr(creusot, creusot_std::macros::trusted)]
    pub fn fraction(&self) -> f32 {
        if self.threshold == 0 {
            1.0
        } else {
            self.progress as f32 / self.threshold as f32
        }
    }
}

/// Noisy actions that raise alerts on a defended network. Passive logging and
/// active response both key off these.
///
/// The serde derives sit outside the proof boundary: they translate but cannot
/// be proved. `Hash` is outside it too, by the same ruling — `Hasher` is an
/// external trait with no contract, and Creusot warns that calling a
/// contractless external function "will yield an impossible precondition", so a
/// green result there would be vacuous rather than meaningful. Measured:
/// `vc_hash_Alert` was the single failing goal of 23.
///
/// `Hash` takes the form of a hand-written `#[cfg(not(creusot))] impl` below
/// rather than a derive, because `clippy::derived_hash_with_manual_eq` will not
/// have a derived `Hash` beside the hand-written `eq` this type needs. The
/// ledger keys `cfg-out` on the trait name, not the mechanism, so the swap
/// keeps the row it already had.
#[derive(Debug, Clone, Copy, Eq)]
#[cfg_attr(not(creusot), derive(Serialize, Deserialize))]
#[cfg_attr(creusot, derive(creusot_std::model::DeepModel))]
pub enum Alert {
    FailedLogin,
    PortScan,
    FirewallTrip,
    PowerCut,
}

/// Hand-written for the same reason as [`Trace`]'s.
impl PartialEq for Alert {
    #[cfg_attr(creusot, creusot_std::macros::ensures(result == (self.deep_model() == rhs.deep_model())))]
    fn eq(&self, rhs: &Alert) -> bool {
        // Spelled out rather than `core::mem::discriminant`, which is an
        // external function with no contract: Creusot warns that calling one
        // "will yield an impossible precondition", i.e. it would make this impl
        // vacuously provable. An explicit match is the honest version.
        matches!(
            (self, rhs),
            (Alert::FailedLogin, Alert::FailedLogin)
                | (Alert::PortScan, Alert::PortScan)
                | (Alert::FirewallTrip, Alert::FirewallTrip)
                | (Alert::PowerCut, Alert::PowerCut)
        )
    }
}

/// Hand-written for the same reason `PartialEq` is, and to keep the two
/// provably in step.
///
/// `#[derive(Hash)]` alongside a hand-written `eq` is
/// `clippy::derived_hash_with_manual_eq`, and the lint is right to ask: `Hash`
/// and `Eq` must agree, or a `HashMap` silently loses entries. Here they do
/// agree — `Alert` is field-less and the hand-written `eq` matches exactly the
/// variant pairs a derive would — but *stating* the agreement is better than
/// asserting it in a comment and silencing the lint with an `allow`. Each
/// variant hashes to its own discriminant, so equal values hash equally and
/// unequal ones are distinguished.
///
/// It stays outside the proof boundary with the serde derives, by the same
/// ruling recorded above: `Hasher` is an external trait with no contract, so a
/// green Creusot result here would be vacuous rather than meaningful.
#[cfg(not(creusot))]
impl core::hash::Hash for Alert {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        state.write_u8(match self {
            Alert::FailedLogin => 0,
            Alert::PortScan => 1,
            Alert::FirewallTrip => 2,
            Alert::PowerCut => 3,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_intrusion_trips_the_trace() {
        let mut t = Trace::new(100);
        for _ in 0..10 {
            t.advance(10, 1); // no bouncing
        }
        assert!(t.traced());
        assert_eq!(t.progress(), 100); // clamped, not overrun
    }

    #[test]
    fn bouncing_through_hops_slows_the_trace() {
        let mut direct = Trace::new(100);
        let mut bounced = Trace::new(100);
        for _ in 0..5 {
            direct.advance(10, 1); // straight in
            bounced.advance(10, 4); // via 4 hops -> rate 2
        }
        assert!(bounced.progress() < direct.progress());
        assert!((bounced.fraction() - 0.10).abs() < f32::EPSILON);
    }

    #[test]
    fn bouncing_slows_but_never_freezes_the_trace() {
        // base < hops would integer-divide the rate to 0 — a hacker must not be
        // able to pivot through enough machines to stall the trace outright.
        // This is the same property `advance`'s third `ensures` states, checked
        // here by example and there for every input.
        let mut t = Trace::new(100);
        for _ in 0..100 {
            t.advance(3, 16); // 3/16 == 0 under plain integer division
        }
        assert_eq!(t.progress(), 100, "active intrusion must keep advancing");
        assert!(t.traced());
    }

    #[test]
    fn zero_hops_does_not_divide_by_zero() {
        // `hops` should never be 0, but if a caller passes it the guard must
        // treat it as a direct (unbounced) intrusion rather than panicking.
        let mut t = Trace::new(100);
        t.advance(5, 0); // hops.max(1) -> divide by 1, no panic
        assert_eq!(t.progress(), 5); // same as advance(5, 1)
    }

    #[test]
    fn inactivity_makes_no_progress() {
        // No activity (base == 0) is the only thing that yields no progress.
        let mut t = Trace::new(100);
        for _ in 0..100 {
            t.advance(0, 1);
        }
        assert_eq!(t.progress(), 0);
        assert!(!t.traced());
    }

    #[test]
    fn alert_equality_distinguishes_every_variant() {
        // `PartialEq` is hand-written here, so unlike a derive it is not
        // automatically right. This pins it against the variant set.
        let all = [
            Alert::FailedLogin,
            Alert::PortScan,
            Alert::FirewallTrip,
            Alert::PowerCut,
        ];
        for (i, a) in all.iter().enumerate() {
            for (j, b) in all.iter().enumerate() {
                assert_eq!(a == b, i == j, "{a:?} vs {b:?}");
            }
        }
    }

    #[test]
    fn trace_equality_compares_both_fields() {
        // Same reason as above: a hand-written `eq` that forgot a field would
        // still compile and still pass every test that only varies one.
        let a = Trace::new(100);
        let b = Trace::new(100);
        let c = Trace::new(50);
        let mut d = Trace::new(100);
        d.advance(10, 1);
        assert_eq!(a, b);
        assert_ne!(a, c, "threshold must participate");
        assert_ne!(a, d, "progress must participate");
    }

    #[test]
    fn alert_hash_agrees_with_alert_eq() {
        // The obligation `clippy::derived_hash_with_manual_eq` names. Silencing
        // that lint with an `allow`, or answering it in a comment, leaves the
        // agreement unchecked -- and a `Hash` that disagrees with `Eq` does not
        // fail loudly, it makes a `HashMap` lose entries. So assert it over the
        // whole cross product, in both directions: equal values must hash
        // equally, and unequal values must not collide.
        use core::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        fn digest(a: &Alert) -> u64 {
            let mut h = DefaultHasher::new();
            a.hash(&mut h);
            h.finish()
        }

        let all = [
            Alert::FailedLogin,
            Alert::PortScan,
            Alert::FirewallTrip,
            Alert::PowerCut,
        ];
        for a in &all {
            for b in &all {
                assert_eq!(
                    a == b,
                    digest(a) == digest(b),
                    "Hash and Eq disagree for {a:?} vs {b:?}"
                );
            }
        }
    }
}
