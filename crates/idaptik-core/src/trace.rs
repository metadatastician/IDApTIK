//! The intrusion trace — the pressure the hacker plays against — and the alerts
//! that noisy actions raise on a defended network.
//!
//! Deterministic and tick-based (no wall clock) so the core stays engine- and
//! transport-agnostic; the frontend and the Elixir session layer decide how
//! often to tick it.

use serde::{Deserialize, Serialize};

/// A live trace clock. It fills as the hacker works; when it reaches its
/// threshold the intrusion is traced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trace {
    progress: u32,
    threshold: u32,
}

impl Trace {
    /// A fresh trace that trips once `threshold` progress has accumulated.
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
    pub fn advance(&mut self, base: u32, hops: u32) {
        let rate = if base == 0 {
            0
        } else {
            (base / hops.max(1)).max(1)
        };
        self.progress = self.progress.saturating_add(rate).min(self.threshold);
    }

    /// Whether the intrusion has been traced.
    pub fn traced(&self) -> bool {
        self.progress >= self.threshold
    }

    /// Progress so far.
    pub fn progress(&self) -> u32 {
        self.progress
    }

    /// The threshold at which the trace trips.
    pub fn threshold(&self) -> u32 {
        self.threshold
    }

    /// Fraction complete, in `0.0..=1.0` — handy for a progress bar.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Alert {
    FailedLogin,
    PortScan,
    FirewallTrip,
    PowerCut,
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
        let mut t = Trace::new(100);
        for _ in 0..100 {
            t.advance(3, 16); // 3/16 == 0 under plain integer division
        }
        assert_eq!(t.progress(), 100, "active intrusion must keep advancing");
        assert!(t.traced());
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
}
