//! The reusable interest/belief engine — the arithmetic behind an actor's
//! object theory, factored out of Billy's `system_behaviour` so that every
//! archetype (and every scenario built on the registry) shares one
//! implementation instead of a copy.
//!
//! The model is the Ghost Lobby's, generalised to the training-ground
//! prototype's `valueSignal` economy: each tracked object holds an interest
//! meter in `0..=100`. While the actor observes the object's holder (or the
//! object's neighbourhood), the meter climbs at the profile's urgency for the
//! observed [`Leakage`](super::Leakage), plus a carry bonus when the object is
//! being carried in view. While unobserved, it decays — unless the object is
//! *pinned* (exposed, thrown, or known-carried), in which case the actor does
//! not forget. Belief forms over whichever meter is highest once it crosses
//! the archetype's threshold; earlier entries win ties, so callers list
//! objects in a deterministic order.
//!
//! Every function is total: no panics, no allocation, clamped arithmetic.

use super::{InterestProfile, Leakage};
use crate::scenario::mathf::clamp;

/// One observed tick of interest: the meter climbs by the profile's urgency
/// for `leakage`, plus the carry bonus when the object is carried in view.
/// Result is clamped to `0..=100`.
pub fn interest_observed(
    current: f64,
    profile: &InterestProfile,
    leakage: Leakage,
    carried: bool,
    dt: f64,
) -> f64 {
    let carry = if carried { profile.carry * dt } else { 0.0 };
    clamp(current + profile.urgency(leakage) * dt + carry, 0.0, 100.0)
}

/// One unobserved tick of interest: the meter decays toward zero unless the
/// object is pinned (the actor has no reason to forget it).
pub fn interest_unobserved(current: f64, profile: &InterestProfile, pinned: bool, dt: f64) -> f64 {
    if pinned {
        current
    } else {
        (current - profile.decay * dt).max(0.0)
    }
}

/// The actor's object theory: the first entry holding the highest interest,
/// if that interest has crossed `threshold`; `None` while everything is below
/// it. Later entries replace the front-runner only when *strictly* greater,
/// so ties resolve to the earlier entry — callers must list objects in a
/// deterministic order (the Ghost Lobby lists the note before the usb, which
/// reproduces the prototype's `note >= usb` tie-break exactly).
pub fn belief_over<K: Clone>(interests: &[(K, f64)], threshold: f64) -> Option<K> {
    let mut best: Option<(&K, f64)> = None;
    for (k, v) in interests {
        match best {
            Some((_, bv)) if *v <= bv => {}
            _ => best = Some((k, *v)),
        }
    }
    best.and_then(|(k, v)| (v >= threshold).then(|| k.clone()))
}
