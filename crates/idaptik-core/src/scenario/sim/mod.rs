//! The deterministic fixed-tick simulation: [`GhostLobbySim`].
//!
//! A [`GhostLobbySim`] owns the definition, the run config, the RNG, the tick
//! counter and the [`RuntimeState`]. Each call to [`GhostLobbySim::tick`] advances
//! exactly one 60 Hz frame — applying immediates before the systems, then running
//! the twelve systems in a fixed, load-bearing order (see `systems.rs`), then the
//! post-systems phase/lockdown checks — and returns the typed events it emitted.
//!
//! The code path is panic-free: no `unwrap`/`expect`/panicking index; fallible
//! construction returns `Result`; arithmetic is clamped; matches are exhaustive.

mod systems;

use crate::scenario::command::{RunConfig, TickInput};
use crate::scenario::common::{
    BillyMode, CrisisReason, ExtractMethod, FailReason, Outcome, Phase, ReportedTarget, Tone,
};
use crate::scenario::constants as c;
use crate::scenario::definition::{RoomDef, ScenarioDefinition, ValidationError};
use crate::scenario::event::Event;
use crate::scenario::ids::IdIndex;
use crate::scenario::mathf::{TICK_DT, clamp, js_round, sign_or};
use crate::scenario::outcome::{
    DEBRIEF_FORMAT, Debrief, ScoreBreakdown, Tag, debrief_text, grade_for,
};
use crate::scenario::rng::{Mulberry32, roll_init};
use crate::scenario::snapshot::{RuntimeSnapshot, SNAPSHOT_FORMAT};
use crate::scenario::state::{DoorState, RuntimeState};
use crate::scenario::tuning::{ActionKind, DifficultyPreset};

/// The playable Ghost Lobby simulation — forward-compatible as a UMS floor.
#[derive(Debug, Clone)]
pub struct GhostLobbySim {
    def: ScenarioDefinition,
    cfg: RunConfig,
    seed: u32,
    rng: Mulberry32,
    tick: u64,
    state: RuntimeState,
    idx: IdIndex,
    /// The active difficulty preset (resolved once; rebuilt on reset/restore).
    preset: DifficultyPreset,
    paused: bool,
    events: Vec<Event>,
}

/// A last-resort preset so construction stays total even if a preset is missing;
/// [`GhostLobbySim::new`] rejects such definitions via [`ScenarioDefinition::ok`]
/// first, so this is never actually observed by a valid scenario.
fn fallback_preset() -> DifficultyPreset {
    DifficultyPreset {
        label: "FALLBACK".to_owned(),
        arrival: (0.0, 0.0),
        player_speed: 0.0,
        sprint: 1.0,
        billy_speed: 0.0,
        billy_sight: 0.0,
        support_limit: 1.0,
        bandwidth_regen: 0.0,
        badge_delay: 1.0,
        usb_timer: 0.0,
        camera_lock: 1.0,
        alert_gain: 1.0,
        score_mult: 1.0,
        rescue: false,
    }
}

impl GhostLobbySim {
    /// Construct a new run. Validates the definition, seeds the RNG, performs the
    /// reset roll and emits `RunStarted` + `SeedAnnounced` (drain them with
    /// [`GhostLobbySim::drain_events`] before the first tick, or let the first
    /// tick return them ahead of its own events).
    pub fn new(
        def: ScenarioDefinition,
        cfg: RunConfig,
        seed: u32,
    ) -> Result<Self, Vec<ValidationError>> {
        def.ok()?;
        let preset = def
            .difficulty
            .get(&cfg.difficulty)
            .cloned()
            .unwrap_or_else(fallback_preset);
        let idx = IdIndex::resolve(&def);
        let mut rng = Mulberry32::new(seed);
        let roll = roll_init(&def, &mut rng, cfg.difficulty);
        let state = RuntimeState::initial(&def, &roll, cfg, &preset);
        let mut sim = Self {
            def,
            cfg,
            seed,
            rng,
            tick: 0,
            state,
            idx,
            preset,
            paused: false,
            events: Vec::new(),
        };
        sim.emit_start();
        Ok(sim)
    }

    /// Rebuild an equivalent sim from a snapshot and its definition.
    pub fn restore(
        def: ScenarioDefinition,
        snap: RuntimeSnapshot,
    ) -> Result<Self, Vec<ValidationError>> {
        def.ok()?;
        let preset = def
            .difficulty
            .get(&snap.cfg.difficulty)
            .cloned()
            .unwrap_or_else(fallback_preset);
        let idx = IdIndex::resolve(&def);
        Ok(Self {
            def,
            cfg: snap.cfg,
            seed: snap.seed,
            rng: snap.rng,
            tick: snap.tick,
            state: snap.state,
            idx,
            preset,
            paused: snap.paused,
            events: Vec::new(),
        })
    }

    /// Restart from `seed`, re-rolling the reset and emitting the start events.
    pub fn reset(&mut self, seed: u32) {
        self.seed = seed;
        self.rng = Mulberry32::new(seed);
        let roll = roll_init(&self.def, &mut self.rng, self.cfg.difficulty);
        self.tick = 0;
        self.state = RuntimeState::initial(&self.def, &roll, self.cfg, &self.preset);
        self.paused = false;
        self.emit_start();
    }

    fn emit_start(&mut self) {
        self.events.push(Event::RunStarted {
            seed: self.seed,
            difficulty: self.cfg.difficulty,
            reduced_motion: self.cfg.reduced_motion,
        });
        self.events.push(Event::SeedAnnounced { seed: self.seed });
    }

    // --- accessors ---------------------------------------------------------

    /// Borrow the runtime state.
    pub fn state(&self) -> &RuntimeState {
        &self.state
    }

    /// Borrow the definition.
    pub fn definition(&self) -> &ScenarioDefinition {
        &self.def
    }

    /// The resolved id index.
    pub fn id_index(&self) -> &IdIndex {
        &self.idx
    }

    /// The run config.
    pub fn config(&self) -> RunConfig {
        self.cfg
    }

    /// The current tick number.
    pub fn current_tick(&self) -> u64 {
        self.tick
    }

    /// The run seed.
    pub fn seed(&self) -> u32 {
        self.seed
    }

    /// The current physics time (accumulated `t`).
    pub fn time(&self) -> f64 {
        self.state.t
    }

    /// Whether the run has ended.
    pub fn is_ended(&self) -> bool {
        self.state.ended
    }

    /// Whether the run is paused.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// The after-action debrief, if the run has ended.
    pub fn debrief(&self) -> Option<&Debrief> {
        self.state.result.as_ref()
    }

    /// Take the pending events (the `RunStarted`/`SeedAnnounced` after `new`).
    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// The definition export surface (definition + validation report).
    pub fn definition_export(&self) -> crate::scenario::snapshot::DefinitionExport {
        crate::scenario::snapshot::DefinitionExport {
            format: self.def.format.clone(),
            definition: self.def.clone(),
            validation: self.def.validate(),
        }
    }

    /// The debrief export surface (present once the run has ended).
    pub fn debrief_export(&self) -> crate::scenario::snapshot::DebriefExport {
        crate::scenario::snapshot::DebriefExport {
            format: DEBRIEF_FORMAT.to_owned(),
            debrief: self.state.result.clone(),
        }
    }

    /// The combined Exchange-House-style export: definition + snapshot + debrief
    /// + the caller-supplied canonical event log.
    pub fn export(&self, event_log: Vec<Event>) -> crate::scenario::snapshot::ScenarioExport {
        crate::scenario::snapshot::ScenarioExport {
            format: crate::scenario::snapshot::EXPORT_FORMAT.to_owned(),
            definition: self.def.clone(),
            snapshot: self.snapshot(),
            debrief: self.state.result.clone(),
            event_log,
        }
    }

    /// A full, restorable snapshot at the current tick.
    pub fn snapshot(&self) -> RuntimeSnapshot {
        RuntimeSnapshot {
            format: SNAPSHOT_FORMAT.to_owned(),
            definition: self.def.clone(),
            cfg: self.cfg,
            seed: self.seed,
            tick: self.tick,
            rng: self.rng.clone(),
            state: self.state.clone(),
            paused: self.paused,
            validation: self.def.validate(),
        }
    }

    // --- the tick ----------------------------------------------------------

    /// Advance exactly one fixed 60 Hz frame, returning the emitted events.
    pub fn tick(&mut self, input: &TickInput) -> Vec<Event> {
        // PRE — P0: pause / restart (mirror synchronous keydown handlers).
        for imm in &input.immediates {
            match imm {
                crate::scenario::command::Command::Pause { on } if *on != self.paused => {
                    self.paused = *on;
                    self.events
                        .push(if *on { Event::Paused } else { Event::Resumed });
                }
                crate::scenario::command::Command::Restart => {
                    self.events.push(Event::Restarted { seed: self.seed });
                    self.reset(self.seed);
                    return std::mem::take(&mut self.events);
                }
                _ => {}
            }
        }
        if self.paused {
            return std::mem::take(&mut self.events);
        }

        // P2 — other immediates at the pre-increment `t`.
        if !self.state.ended {
            for imm in &input.immediates {
                if self.state.ended {
                    break;
                }
                match imm {
                    crate::scenario::command::Command::Uplink { kind } => {
                        self.perform_action(*kind);
                    }
                    crate::scenario::command::Command::ForceCrisis => {
                        self.begin_crisis(CrisisReason::Test);
                    }
                    crate::scenario::command::Command::ForceExtract { method } => {
                        self.extract(*method);
                    }
                    crate::scenario::command::Command::ForceFail { reason } => {
                        self.fail_mission(*reason);
                    }
                    _ => {}
                }
            }
        }

        // P3 — if ended, do not run the update.
        if !self.state.ended {
            self.update(input);
        }

        std::mem::take(&mut self.events)
    }

    /// The system pipeline for one non-ended, non-paused frame.
    fn update(&mut self, input: &TickInput) {
        self.state.t += TICK_DT;
        self.tick += 1;

        self.system_timers();
        self.system_player(input);
        self.system_interactions(input);
        if self.state.ended {
            return;
        }
        self.system_usb();
        self.system_vacuum();
        self.system_cameras();
        self.system_support();
        if self.state.ended {
            return;
        }
        self.system_behaviour();
        self.system_billy();
        self.system_collisions();
        if self.state.ended {
            return;
        }
        self.system_objectives();

        // POST — timer crisis (billy first moves NEXT tick) then lockdown.
        if self.state.phase == Phase::Quiet {
            self.state.quiet_remaining = (self.state.crisis_at - self.state.t).max(0.0);
            if self.state.quiet_remaining <= 0.0 {
                self.begin_crisis(CrisisReason::Timer);
            }
        }
        if self.state.alert >= c::LOCKDOWN && !self.state.ended {
            self.fail_mission(FailReason::Lockdown);
        }
    }

    // --- shared helpers ----------------------------------------------------

    /// Room index containing world-x (uses the `+ROOM_OFFSET` centre; defaults to
    /// the last room, as the prototype does).
    pub(crate) fn room_index_at(def: &ScenarioDefinition, x: f64) -> usize {
        let cx = x + def.world.room_offset;
        def.rooms
            .iter()
            .position(|r| r.contains(cx))
            .unwrap_or(def.rooms.len().saturating_sub(1))
    }

    /// The id string of the room containing world-x, if any.
    pub(crate) fn room_id_at(def: &ScenarioDefinition, x: f64) -> Option<&str> {
        def.rooms
            .get(Self::room_index_at(def, x))
            .map(|r| r.id.as_str())
    }

    /// The room def containing world-x, if any.
    pub(crate) fn room_at(def: &ScenarioDefinition, x: f64) -> Option<&RoomDef> {
        def.rooms.get(Self::room_index_at(def, x))
    }

    /// Index of the nearest door to `x` (first wins ties).
    pub(crate) fn nearest_door(doors: &[DoorState], x: f64) -> Option<usize> {
        let mut best: Option<(usize, f64)> = None;
        for (i, d) in doors.iter().enumerate() {
            let gap = (d.x - x).abs();
            match best {
                Some((_, bg)) if gap >= bg => {}
                _ => best = Some((i, gap)),
            }
        }
        best.map(|(i, _)| i)
    }

    /// Push a new `x` back inside `[old_x, new_x]` when a closed door lies
    /// between the old and new centre; returns the constrained x and whether a
    /// door blocked the move.
    pub(crate) fn constrain_by_doors(
        doors: &[DoorState],
        old_x: f64,
        new_x: f64,
        w: f64,
    ) -> (f64, bool) {
        let old_center = old_x + w / 2.0;
        let mut x = new_x;
        let mut new_center = x + w / 2.0;
        let mut blocked = false;
        for door in doors {
            if door.open > 0.0 {
                continue;
            }
            let crossed_right = old_center < door.x && new_center >= door.x;
            let crossed_left = old_center > door.x && new_center <= door.x;
            if !crossed_right && !crossed_left {
                continue;
            }
            if crossed_right {
                x = door.x - w / 2.0 - 3.0;
            } else {
                x = door.x - w / 2.0 + 3.0;
            }
            new_center = x + w / 2.0;
            blocked = true;
        }
        (x, blocked)
    }

    /// Whether Billy can currently see the player (recomputed on demand — never
    /// cached for the whole tick).
    pub(crate) fn can_billy_see_player(&self) -> bool {
        let s = &self.state;
        if s.phase != Phase::Crisis || s.billy.mode == BillyMode::Offsite {
            return false;
        }
        let p = &s.player;
        let b = &s.billy;
        if p.hidden || s.lights_flicker > 0.0 || b.stun > 0.0 {
            return false;
        }
        if Self::room_index_at(&self.def, b.x) != Self::room_index_at(&self.def, p.x) {
            return false;
        }
        let dx = (p.x + self.def.player.w / 2.0) - (b.x + self.def.billy.w / 2.0);
        let in_front = sign_or(dx, b.facing) == b.facing;
        let mut range = self.preset.billy_sight * if in_front { 1.0 } else { c::SIGHT_BACK };
        if p.crouching {
            range *= c::SIGHT_CROUCH;
        }
        if let Some(room) = Self::room_at(&self.def, p.x) {
            range *= room.sight_multiplier;
        }
        if s.alert > c::SIGHT_ALERT_HI {
            range *= c::SIGHT_ALERT;
        }
        dx.abs() <= range
    }

    /// Change Billy's FSM mode, emitting `BillyStateChanged` on an actual change.
    pub(crate) fn set_billy_mode(&mut self, to: BillyMode) {
        let from = self.state.billy.mode;
        if from != to {
            self.state.billy.mode = to;
            self.events.push(Event::BillyStateChanged { from, to });
        }
    }

    /// Tip the floor into crisis (idempotent; only from the quiet phase).
    pub(crate) fn begin_crisis(&mut self, reason: CrisisReason) {
        if self.state.ended || self.state.phase != Phase::Quiet {
            return;
        }
        let from = self.state.phase;
        self.state.phase = Phase::Crisis;
        self.state.billy.x = c::CRISIS_BILLY_X;
        self.state.billy.state_timer = 1.1;
        let boost = if reason == CrisisReason::Usb {
            14.0
        } else {
            8.0
        };
        self.state.alert = self.state.alert.max(boost);
        self.events.push(Event::PhaseChanged {
            from,
            to: Phase::Crisis,
            reason: Some(reason),
        });
        self.set_billy_mode(BillyMode::Entering);
        self.events.push(Event::CrisisBegan { reason });
    }

    /// Perform an uplink action (immediate; self-guards on paused/ended).
    pub(crate) fn perform_action(&mut self, kind: ActionKind) {
        if self.paused || self.state.ended {
            return;
        }
        let spec = match self.def.actions.get(&kind) {
            Some(s) => *s,
            None => return,
        };
        let t = self.state.t;
        let i = kind.index();

        // cooldown gate
        let cd = self.state.actions.get(&kind).map(|a| a.cd).unwrap_or(0.0);
        if cd > 0.0 {
            self.state.stats.failed_actions += 1;
            if let Some(slot) = self.state.throttles.cd.get_mut(i)
                && t - *slot >= c::THROTTLE_CDBW
            {
                *slot = t;
                self.events.push(Event::UplinkDenied {
                    kind,
                    reason: crate::scenario::common::DenyReason::Cooldown,
                });
            }
            return;
        }
        // bandwidth gate
        if self.state.bandwidth < spec.cost {
            self.state.stats.failed_actions += 1;
            if let Some(slot) = self.state.throttles.bw.get_mut(i)
                && t - *slot >= c::THROTTLE_CDBW
            {
                *slot = t;
                self.events.push(Event::UplinkDenied {
                    kind,
                    reason: crate::scenario::common::DenyReason::Bandwidth,
                });
            }
            return;
        }

        self.state.bandwidth -= spec.cost;
        if let Some(a) = self.state.actions.get_mut(&kind) {
            a.cd = spec.cooldown;
        }
        self.state.stats.hacker_actions += 1;
        let ag = self.preset.alert_gain;
        self.events.push(Event::UplinkAction { kind });

        match kind {
            ActionKind::Camera => {
                self.state.camera_ping = c::PING_DUR;
                self.state.alert = clamp(self.state.alert + spec.alert_gain * ag, 0.0, 100.0);
                let laundry = Self::room_id_at(&self.def, self.state.player.x) == Some("laundry");
                self.events.push(Event::CameraPinged {
                    laundry_view: laundry,
                });
            }
            ActionKind::Door => {
                let center = self.state.player.x + self.def.player.w / 2.0;
                if let Some(di) = Self::nearest_door(&self.state.doors, center)
                    && let Some(d) = self.state.doors.get_mut(di)
                {
                    d.pending = d.route_delay;
                    d.badge_logged = false;
                    let (door_id, delay) = (d.id.clone(), d.route_delay);
                    self.state.stats.doors_held += 1;
                    self.state.alert = clamp(self.state.alert + spec.alert_gain * ag, 0.0, 100.0);
                    self.events.push(Event::DoorRouted {
                        door: door_id,
                        delay,
                    });
                }
            }
            ActionKind::Vacuum => {
                if self.state.vacuum.fallen {
                    self.state.stats.failed_actions += 1;
                    self.events.push(Event::UplinkDenied {
                        kind,
                        reason: crate::scenario::common::DenyReason::VacuumFallen,
                    });
                    return;
                }
                self.state.vacuum.active = true;
                self.state.vacuum.target = c::VAC_TARGET;
                self.state.stats.vacuum_used = true;
                self.state.alert = clamp(self.state.alert + spec.alert_gain * ag, 0.0, 100.0);
                self.events.push(Event::VacuumRouted);
            }
            ActionKind::Lights => {
                self.state.lights_flicker = if self.cfg.reduced_motion {
                    c::LIGHTS_DUR_RM
                } else {
                    c::LIGHTS_DUR
                };
                self.state.lights_uses += 1;
                self.state.stats.light_flickers += 1;
                self.state.billy.stun = self.state.billy.stun.max(c::LIGHTS_STUN);
                let extra = if self.state.lights_uses >= 3 {
                    6.0 + f64::from(self.state.lights_uses) * 1.5
                } else {
                    0.0
                };
                self.state.alert = clamp(
                    self.state.alert + (spec.alert_gain + extra) * ag,
                    0.0,
                    100.0,
                );
                self.events.push(Event::LightsFlickered {
                    third_use: self.state.lights_uses == 3,
                });
            }
        }
    }

    /// Move Billy toward a target x at `speed`, handling door-blocking and the
    /// badge-through behaviour (emitting `BillyBadgedDoor` under an 8 s throttle).
    pub(crate) fn move_billy_toward(&mut self, target_x: f64, speed: f64) {
        let dt = TICK_DT;
        let old_x = self.state.billy.x;
        let dx = target_x - old_x;
        let dir = if dx.abs() < 2.0 {
            0.0
        } else {
            crate::scenario::mathf::sign(dx)
        };
        if dir != 0.0 {
            self.state.billy.facing = dir;
        }
        self.state.billy.vx =
            crate::scenario::mathf::approach(self.state.billy.vx, dir * speed, c::BILLY_ACCEL * dt);
        self.state.billy.x += self.state.billy.vx * dt;
        self.state.billy.x = clamp(self.state.billy.x, c::BILLY_CLAMP_LO, c::BILLY_CLAMP_HI);
        let before = self.state.billy.x;
        let (nx, _) = Self::constrain_by_doors(
            &self.state.doors,
            old_x,
            self.state.billy.x,
            self.def.billy.w,
        );
        self.state.billy.x = nx;
        let blocked = (before - self.state.billy.x).abs() > 0.1;
        if !blocked {
            self.state.billy.door_wait = 0.0;
            self.state.billy.blocked_door = None;
            return;
        }
        self.state.billy.vx = 0.0;
        let center = self.state.billy.x + self.def.billy.w / 2.0;
        let di = match Self::nearest_door(&self.state.doors, center) {
            Some(d) => d,
            None => return,
        };
        if self.state.billy.blocked_door != Some(di) {
            self.state.billy.blocked_door = Some(di);
            self.state.billy.door_wait = 0.0;
        }
        self.state.billy.door_wait += dt;
        if self.state.billy.door_wait >= self.preset.badge_delay {
            self.state.billy.door_wait = 0.0;
            let t = self.state.t;
            if let Some(d) = self.state.doors.get_mut(di) {
                d.open = d.open.max(c::BADGE_OPEN);
                if !d.badge_logged {
                    d.badge_logged = true;
                    let door_id = d.id.clone();
                    if let Some(slot) = self.state.throttles.badge.get_mut(di)
                        && t - *slot >= c::THROTTLE_BADGE
                    {
                        *slot = t;
                        self.events.push(Event::BillyBadgedDoor { door: door_id });
                    }
                }
            }
        }
    }

    // --- endings + scoring -------------------------------------------------

    /// Whether Billy ended believing the USB mattered (has it, reported it, or
    /// interest crossed the misdirect threshold).
    fn misled(&self) -> bool {
        self.state.billy.has_usb
            || self.state.billy.reported_target == Some(ReportedTarget::Usb)
            || self.state.billy.usb_interest >= c::MISDIRECT_THRESHOLD
    }

    /// Extract via `method`, building the success debrief and ending the run.
    pub(crate) fn extract(&mut self, method: ExtractMethod) {
        if self.state.ended {
            return;
        }
        self.state.chute.used = method == ExtractMethod::LaundryChute;
        self.state.stats.extraction = Some(method);
        let misled = self.misled();
        let has_note = self.state.player.has_note;
        let mut tags: Vec<Tag> = Vec::new();
        let mut bd = ScoreBreakdown::default();
        let sc = self.def.scoring;
        let mut score = sc.base;
        bd.base = sc.base;

        if has_note {
            tags.push(Tag::new(
                "Strategic success: contact lead secured",
                Tone::Good,
            ));
            score += sc.note;
            bd.note = sc.note;
        } else if self.state.note.billy_has {
            tags.push(Tag::new("Compromised: Billy has the real lead", Tone::Bad));
            score += sc.note_lost;
            bd.note = sc.note_lost;
        } else {
            tags.push(Tag::new("Survival only: real lead not secured", Tone::Warn));
            score += sc.note_none;
            bd.note = sc.note_none;
        }

        if misled {
            tags.push(Tag::new(
                "Decoy success: Billy thinks the USB mattered",
                Tone::Good,
            ));
            score += sc.misdir;
            bd.misdirect = sc.misdir;
        } else if self.state.billy.reported_target == Some(ReportedTarget::Note) {
            tags.push(Tag::new(
                "Behavioural leak: Billy identified the note",
                Tone::Bad,
            ));
            score += sc.misdir_leak;
            bd.misdirect = sc.misdir_leak;
        } else {
            tags.push(Tag::new("Billy remained uncertain", Tone::Warn));
        }

        if self.state.billy.called {
            tags.push(Tag::new("Boss heard Billy's report", Tone::Warn));
            score += sc.boss;
            bd.boss = sc.boss;
        } else {
            tags.push(Tag::new("No report completed", Tone::Good));
            score += sc.noboss;
            bd.boss = sc.noboss;
        }

        let cams = self.state.stats.camera_detections;
        if cams == 0 {
            tags.push(Tag::new("No confirmed camera motion flags", Tone::Good));
            score += sc.nocam;
            bd.camera = sc.nocam;
        } else {
            let pen = sc.cam_each * f64::from(cams);
            let plural = if cams == 1 { "" } else { "s" };
            tags.push(Tag {
                text: format!("{cams} camera motion flag{plural}"),
                tone: Tone::Warn,
            });
            score += pen;
            bd.camera = pen;
        }

        if self.state.stats.max_isolation > self.preset.support_limit * sc.iso_snap_frac {
            tags.push(Tag::new("Support envelope nearly snapped", Tone::Warn));
            score += sc.iso_snap;
            bd.isolation = sc.iso_snap;
        } else {
            tags.push(Tag::new("Support relationship preserved", Tone::Good));
            score += sc.iso_ok;
            bd.isolation = sc.iso_ok;
        }

        if method == ExtractMethod::LaundryChute {
            tags.push(Tag::new(
                "Humiliating extraction route validated",
                Tone::Good,
            ));
            score += sc.chute;
            bd.chute = sc.chute;
        } else {
            tags.push(Tag::new("Service exit extraction", Tone::Good));
        }
        if self.state.stats.usb_trace {
            tags.push(Tag::new("USB trace handshake completed", Tone::Bad));
            score += sc.usbtrace;
            bd.usb_trace = sc.usbtrace;
        }
        if self.state.stats.rescue_used {
            tags.push(Tag::new("Automatic rescue consumed", Tone::Warn));
            score += sc.rescue;
            bd.rescue = sc.rescue;
        }
        tags.push(Tag::new("Both agents remain playable", Tone::Good));

        let time_bonus = (sc.time_base - self.state.t * sc.time_k).max(0.0);
        score += time_bonus;
        bd.time_bonus = time_bonus;
        let alert_pen = -self.state.max_alert * sc.alert_k;
        score += alert_pen;
        bd.alert_penalty = alert_pen;
        let fa_pen = -f64::from(self.state.stats.failed_actions) * sc.fail_act;
        score += fa_pen;
        bd.failed_actions_penalty = fa_pen;

        bd.raw = score;
        bd.score_mult = self.preset.score_mult;
        let final_f = js_round(score * self.preset.score_mult).max(0.0);
        let final_u = final_f as u32;
        bd.final_score = final_u;

        let grade = grade_for(final_u, true, &sc.grades);
        let title = if has_note {
            "Extraction reached"
        } else {
            "Extraction without the answer"
        };
        let summary = self.build_summary(method, misled);
        // HTML debrief() keys the strong paragraph on `hasUSB || reportedTarget=="usb"`
        // only — NOT the full misdirect predicate (which also counts usb_interest>=72).
        let debrief_misled = self.state.billy.has_usb
            || self.state.billy.reported_target == Some(ReportedTarget::Usb);
        let text = debrief_text(Outcome::Extracted, has_note, debrief_misled);
        let debrief = Debrief {
            format: DEBRIEF_FORMAT.to_owned(),
            success: true,
            reason: Outcome::Extracted,
            title: title.to_owned(),
            summary,
            grade,
            score: final_u,
            tags,
            breakdown: bd,
            debrief_text: text,
            stats: self.state.stats.clone(),
            time_s: self.state.t,
            max_alert: self.state.max_alert,
        };
        self.events.push(Event::Extracted { method });
        self.finish(debrief, Outcome::Extracted);
    }

    fn build_summary(&self, method: ExtractMethod, misled: bool) -> String {
        let method_s = match method {
            ExtractMethod::ServiceExit => "service exit",
            ExtractMethod::LaundryChute => "laundry chute",
        };
        let lead = if self.state.player.has_note {
            "The contact lead is secured."
        } else if self.state.note.billy_has {
            "Billy retains the real lead."
        } else {
            "The real lead remains unresolved."
        };
        let theory = if misled {
            "Billy's story centres on the USB."
        } else if self.state.billy.reported_target == Some(ReportedTarget::Note) {
            "Billy learned what truly mattered."
        } else {
            "Billy never formed a stable object theory."
        };
        format!("Both agents remain playable after a {method_s} extraction. {lead} {theory}")
    }

    /// Fail the run for `reason`, building the failure debrief and ending it.
    pub(crate) fn fail_mission(&mut self, reason: FailReason) {
        if self.state.ended {
            return;
        }
        let has_note = self.state.player.has_note;
        let usb_believed =
            self.state.billy.has_usb || self.state.billy.usb_interest >= c::MISDIRECT_THRESHOLD;
        let sc = self.def.scoring;
        let mut bd = ScoreBreakdown::default();
        let note_term = if has_note { sc.fail_note } else { sc.fail_base };
        // HTML fail score adds the USB term only on `usbInterest >= 72` — Billy
        // physically holding the drive does NOT contribute to the failure score
        // (it only earns the "believed the USB mattered" tag below).
        let usb_term = if self.state.billy.usb_interest >= c::MISDIRECT_THRESHOLD {
            sc.fail_usb
        } else {
            0.0
        };
        let alert_term = -self.state.alert * sc.fail_alert_k;
        bd.note = note_term;
        bd.misdirect = usb_term;
        bd.alert_penalty = alert_term;
        let raw = note_term + usb_term + alert_term;
        bd.raw = raw;
        bd.score_mult = 1.0;
        let final_u = js_round(raw).max(0.0) as u32;
        bd.final_score = final_u;

        let (outcome, title, first_tag) = match reason {
            FailReason::Caught => (
                Outcome::Caught,
                "Caught by Billy",
                Tag::new("Isolation failure", Tone::Bad),
            ),
            FailReason::Partition => (
                Outcome::Partition,
                "Security partition trap",
                Tag::new("Team-support failure", Tone::Bad),
            ),
            FailReason::Lockdown => (
                Outcome::Lockdown,
                "Building-wide lockdown",
                Tag::new("Infrastructure fully awake", Tone::Bad),
            ),
        };
        let mut tags = vec![first_tag];
        if has_note {
            tags.push(Tag::new(
                "Contact lead was secured before failure",
                Tone::Warn,
            ));
        }
        if usb_believed {
            tags.push(Tag::new("Billy believed the USB mattered", Tone::Good));
        }
        if self.state.billy.called {
            tags.push(Tag::new("Boss heard Billy's report", Tone::Warn));
        }
        tags.push(Tag::new("Field agent no longer operational", Tone::Bad));

        let summary = self.fail_summary(reason);
        let grade = grade_for(final_u, false, &sc.grades);
        let text = debrief_text(outcome, has_note, self.misled());
        let debrief = Debrief {
            format: DEBRIEF_FORMAT.to_owned(),
            success: false,
            reason: outcome,
            title: title.to_owned(),
            summary,
            grade,
            score: final_u,
            tags,
            breakdown: bd,
            debrief_text: text,
            stats: self.state.stats.clone(),
            time_s: self.state.t,
            max_alert: self.state.max_alert,
        };
        self.events.push(Event::MissionFailed { reason });
        self.finish(debrief, outcome);
    }

    fn fail_summary(&self, reason: FailReason) -> String {
        match reason {
            FailReason::Caught => "Billy catches the field agent outside a viable support response. Solo brilliance was not enough.".to_owned(),
            FailReason::Partition => "A security partition slams down. You are trapped. This is a team-support failure, not a solo death.".to_owned(),
            FailReason::Lockdown => "The building reaches a complete administrative opinion. Every partition closes at once.".to_owned(),
        }
    }

    fn finish(&mut self, debrief: Debrief, outcome: Outcome) {
        let from = self.state.phase;
        self.state.ended = true;
        self.state.phase = Phase::Result;
        self.state.result = Some(debrief);
        self.events.push(Event::PhaseChanged {
            from,
            to: Phase::Result,
            reason: None,
        });
        self.events.push(Event::RunEnded { outcome });
    }
}
