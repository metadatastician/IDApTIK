//! The sim driver: [`GhostLobbySim`] as a Bevy resource, stepped at a fixed
//! 60 Hz from a queued [`Command`] stream — the same wire API the TUI speaks.
//!
//! This module is deliberately render-free: a headless test `App` can add
//! [`SimDriverPlugin`] alone and drive [`FixedUpdate`] by hand, which is how
//! the frontend-parity test in `tests/parity.rs` works.

use bevy::prelude::*;
use idaptik_core::RunConfig;
use idaptik_core::scenario::command::{Buttons, Command, fold};
use idaptik_core::scenario::event::{Event as SimEvent, LogLine};
use idaptik_core::scenario::{GhostLobbySim, ghost_lobby, log_view};

/// The simulation tick rate, matching [`idaptik_core::scenario::TICK_DT`].
pub const SIM_HZ: f64 = 60.0;

/// How many rendered log lines the HUD keeps.
const LOG_CAP: usize = 200;

/// The authoritative simulation plus its pure event views.
#[derive(Resource)]
pub struct SimState {
    /// The gameplay truth. The frontend only ever calls `tick` on it.
    pub sim: GhostLobbySim,
    /// Rendered log lines (a pure view of the events; cleared on restart).
    pub log: Vec<LogLine>,
    /// Every event the run has emitted, in order (the deterministic artifact).
    pub event_log: Vec<SimEvent>,
}

impl SimState {
    /// Build a fresh run and ingest its startup events.
    pub fn new(cfg: RunConfig, seed: u32) -> Result<Self, String> {
        let mut sim = GhostLobbySim::new(ghost_lobby(), cfg, seed)
            .map_err(|e| format!("invalid scenario: {e:?}"))?;
        let startup = sim.drain_events();
        let mut state = Self {
            sim,
            log: Vec::new(),
            event_log: Vec::new(),
        };
        state.ingest(startup);
        Ok(state)
    }

    /// Append fresh events to the event log and render them into log lines.
    fn ingest(&mut self, events: Vec<SimEvent>) {
        let tick = self.sim.current_tick();
        let t = self.sim.time();
        for e in &events {
            if let Some(line) = log_view(e, tick, t) {
                self.log.push(line);
            }
        }
        if self.log.len() > LOG_CAP {
            let drop = self.log.len() - LOG_CAP;
            self.log.drain(0..drop);
        }
        self.event_log.extend(events);
    }
}

/// Commands queued by the frontend between ticks, plus the persistent
/// held-button set [`fold`] mutates — the exact seam the TUI's `InputState`
/// occupies.
#[derive(Resource, Default)]
pub struct CommandQueue {
    /// Commands accumulated since the last tick, in stream order.
    pub pending: Vec<Command>,
    /// The held movement buttons, carried across ticks.
    pub held: Buttons,
    /// Frontend-side pause latch (the sim is told via `Command::Pause`).
    pub paused: bool,
}

impl CommandQueue {
    /// Queue one command for the next tick.
    pub fn push(&mut self, cmd: Command) {
        self.pending.push(cmd);
    }
}

/// Inserts the sim, the command queue and the 60 Hz fixed timestep, and steps
/// the simulation every [`FixedUpdate`].
pub struct SimDriverPlugin {
    pub cfg: RunConfig,
    pub seed: u32,
}

impl Plugin for SimDriverPlugin {
    fn build(&self, app: &mut App) {
        let state = SimState::new(self.cfg, self.seed)
            .expect("the canonical Ghost Lobby definition is valid");
        app.insert_resource(state)
            .init_resource::<CommandQueue>()
            .insert_resource(Time::<Fixed>::from_hz(SIM_HZ))
            .add_systems(FixedUpdate, step_sim);
    }
}

/// One fixed tick: drain the queued commands, fold them into a `TickInput`,
/// advance the sim, and ingest the events it emitted.
pub fn step_sim(mut queue: ResMut<CommandQueue>, mut state: ResMut<SimState>) {
    let cmds = std::mem::take(&mut queue.pending);
    let input = fold(&cmds, &mut queue.held);
    let events = state.sim.tick(&input);
    if events
        .iter()
        .any(|e| matches!(e, SimEvent::Restarted { .. }))
    {
        state.log.clear();
    }
    state.ingest(events);
}
