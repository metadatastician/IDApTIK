//! Shared helpers for the Stage-B acceptance tests: a small runner that owns a
//! sim, its persistent held-button set, and the accumulated event log.
#![allow(dead_code)]

use idaptik_core::scenario::command::{Buttons, Command};
use idaptik_core::scenario::event::Event;
use idaptik_core::scenario::{DifficultyId, GhostLobbySim, RunConfig, fold, ghost_lobby};

/// A scripted run: fold command streams into ticks, accumulate every event.
pub struct Runner {
    pub sim: GhostLobbySim,
    pub held: Buttons,
    pub log: Vec<Event>,
}

impl Runner {
    /// Start a run at `seed` on `diff`, capturing the startup events.
    pub fn start(diff: DifficultyId, reduced_motion: bool, seed: u32) -> Self {
        let cfg = RunConfig {
            difficulty: diff,
            reduced_motion,
        };
        let mut sim = GhostLobbySim::new(ghost_lobby(), cfg, seed).expect("valid definition");
        let log = sim.drain_events();
        Self {
            sim,
            held: Buttons::default(),
            log,
        }
    }

    /// The canonical default: Standard, full motion, seed 123456.
    pub fn standard() -> Self {
        Self::start(DifficultyId::Standard, false, 123456)
    }

    /// Advance one tick with the given command stream.
    pub fn step(&mut self, cmds: &[Command]) {
        let input = fold(cmds, &mut self.held);
        let ev = self.sim.tick(&input);
        self.log.extend(ev);
    }

    /// Advance `n` idle ticks (or until the run ends).
    pub fn idle(&mut self, n: u64) {
        for _ in 0..n {
            if self.sim.is_ended() {
                break;
            }
            self.step(&[]);
        }
    }

    /// Advance until the run ends or `max` ticks elapse.
    pub fn run_to_end(&mut self, max: u64) {
        for _ in 0..max {
            if self.sim.is_ended() {
                break;
            }
            self.step(&[]);
        }
    }

    /// Whether the log contains an event matching `pred`.
    pub fn saw(&self, pred: impl Fn(&Event) -> bool) -> bool {
        self.log.iter().any(pred)
    }

    /// Count events matching `pred`.
    pub fn count(&self, pred: impl Fn(&Event) -> bool) -> usize {
        self.log.iter().filter(|e| pred(e)).count()
    }
}
