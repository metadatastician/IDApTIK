//! Stage-B acceptance for the supervised-coverage slice: the VSM security
//! team's evidence-driven attention allocation is part of the deterministic
//! runtime state and *applies* to world coverage — Billy's Assess patrol band
//! bends toward the coverage target — while unsupervised runs stay inert.
//!
//! Choreography uses the public `snapshot()`/`restore()` reflective seam
//! exactly as `acceptance_misdirect_pickpocket.rs` does: state is pure data,
//! so preconditions are data too. No internal hooks.

mod common;
use common::Runner;
use idaptik_core::scenario::GhostLobbySim;
use idaptik_core::scenario::command::{Button, Buttons, Command};
use idaptik_core::scenario::common::BillyMode;
use idaptik_core::scenario::event::Event;

/// Reinstall `r.sim` from a mutated snapshot, resetting the held-button set.
fn restore_from<F: FnOnce(&mut idaptik_core::scenario::snapshot::RuntimeSnapshot)>(
    r: &mut Runner,
    mutate: F,
) {
    let mut snap = r.sim.snapshot();
    mutate(&mut snap);
    let def = r.sim.definition().clone();
    r.sim = GhostLobbySim::restore(def, snap).expect("snapshot restores");
    r.held = Buttons::default();
    let _ = r.sim.drain_events();
}

/// Shared choreography: crisis, a staged USB throw (the canonical evidence
/// event), then Billy reset to a clean mid-floor Assess patrol with the decoy
/// on the floor far to the right and the player hidden out of sight. Both the
/// supervised run and the control run follow it identically.
fn stage_decoy_patrol(r: &mut Runner) {
    r.step(&[Command::ForceCrisis]);
    restore_from(r, |s| {
        s.state.player.x = 600.0;
        s.state.player.vx = 0.0;
        s.state.player.has_usb = true;
        s.state.usb.held = true;
        s.state.usb.x = 600.0;
    });
    r.step(&[Command::ThrowUsb]);
    assert!(r.saw(|e| matches!(e, Event::UsbThrown)), "throw is logged");

    // The throw also pins Billy's belief; clear it so he returns to the Assess
    // patrol, and park the decoy inert on the floor well right of the raw
    // patrol band (the hall, right of door D1). `thrown` must come back off: `usb_interest` grows
    // *unobserved* while it is set, and would re-form the belief mid-test.
    // The supervision ledger inside the state is deliberately left as the
    // throw produced it.
    restore_from(r, |s| {
        s.state.billy.belief = None;
        s.state.billy.belief_announced = None;
        s.state.billy.target = None;
        s.state.billy.usb_interest = 0.0;
        s.state.billy.note_interest = 0.0;
        s.state.billy.player_interest = 0.0;
        s.state.billy.mode = BillyMode::Assess;
        s.state.billy.state_timer = 0.0;
        s.state.billy.x = 300.0;
        s.state.billy.vx = 0.0;
        // The sim's own post-Shock assess target. Billy cannot badge through
        // a closed door under the FSM port's bounce/reset cycle (pre-existing
        // behaviour every golden embeds), so the observable band for this test
        // must stay inside the hall, between D1 (270) and D2 (510).
        s.state.billy.patrol_target = 320.0;
        s.state.usb.x = 400.0;
        s.state.usb.y = 570.0;
        s.state.usb.vx = 0.0;
        s.state.usb.vy = 0.0;
        s.state.usb.timer = 0.0;
        s.state.usb.on_floor = true;
        s.state.usb.thrown = false;
        s.state.usb.wiped = true;
        s.state.usb.held = false;
        // The player parks inside the kitchen "counter" hide spot (x 205,
        // radius 58; the player's centre is x + 14). `hidden` is recomputed
        // every tick from crouch + spot + stillness, so the crouch itself is
        // held via the command stream below.
        s.state.player.x = 191.0;
        s.state.player.vx = 0.0;
        s.state.player.noise = 0.0;
        s.state.player.has_usb = false;
    });
    // Hold crouch from here on: with the player still, grounded and in the
    // spot, the sim keeps `hidden` true and Billy's sight/noise checks stay
    // cold for the whole patrol observation window.
    r.step(&[Command::SetButton {
        button: Button::Crouch,
        down: true,
    }]);
}

/// The observable slice: with supervision on, the Assess ping-pong band bends
/// toward the decoy; the identical unsupervised run patrols the raw band.
#[test]
fn supervised_run_shifts_billy_patrol_toward_decoy_target() {
    let mut supervised = Runner::supervised();
    let mut control = Runner::standard();
    stage_decoy_patrol(&mut supervised);
    stage_decoy_patrol(&mut control);

    let mut supervised_targets = Vec::new();
    let mut control_targets = Vec::new();
    for _ in 0..1200u64 {
        supervised.step(&[]);
        control.step(&[]);
        supervised_targets.push(supervised.sim.state().billy.patrol_target);
        control_targets.push(control.sim.state().billy.patrol_target);
        if supervised.sim.is_ended() || control.sim.is_ended() {
            break;
        }
    }

    let sup = supervised.sim.state();
    assert_eq!(
        sup.supervision.supervisor.coverage_target.as_deref(),
        Some("usb"),
        "the throw evidence selects the usb coverage target"
    );
    assert!(
        sup.supervision.supervisor.team.attention > 0,
        "evidence allocated attention"
    );

    let usb_x = sup.usb.x;
    let sup_max = supervised_targets.iter().cloned().fold(f64::MIN, f64::max);
    let ctl_max = control_targets.iter().cloned().fold(f64::MIN, f64::max);
    assert!(
        sup_max > ctl_max + 50.0,
        "the supervised patrol band reaches toward the decoy: \
         supervised max {sup_max}, control max {ctl_max}, usb at {usb_x}"
    );
    assert!(
        (sup_max - usb_x).abs() < 200.0,
        "the supervised band tracks the decoy's position: max {sup_max}, usb {usb_x}"
    );
}

/// Regression lock: an unsupervised run never emits supervision events and its
/// supervision state stays exactly at its initial value — the flag defaulting
/// off is what keeps every existing fixture and golden byte-identical.
#[test]
fn unsupervised_run_keeps_supervision_inert() {
    let mut r = Runner::standard();
    stage_decoy_patrol(&mut r);
    r.idle(600);
    assert_eq!(
        r.count(|e| matches!(
            e,
            Event::TeamAttentionAllocated { .. } | Event::CoverageRetargeted { .. }
        )),
        0,
        "no supervision events in an unsupervised run"
    );
    let sup = &r.sim.state().supervision.supervisor;
    assert_eq!(sup.team.attention, 0);
    assert_eq!(sup.observed_events, 0);
    assert!(sup.coverage_target.is_none());
}

/// The allocation is visible in the canonical event log, announce-once: a
/// steady evidence picture does not re-announce every tick.
#[test]
fn supervision_announces_allocation_once_per_change() {
    let mut r = Runner::supervised();
    stage_decoy_patrol(&mut r);
    r.idle(600);
    let allocations = r.count(|e| matches!(e, Event::TeamAttentionAllocated { .. }));
    let retargets = r.count(|e| matches!(e, Event::CoverageRetargeted { .. }));
    assert_eq!(
        allocations, 1,
        "one attention change announces exactly once, not per tick"
    );
    assert_eq!(retargets, 1, "one coverage target announces exactly once");
}

/// Supervision state survives the snapshot/restore seam: a restored supervised
/// run continues tick-identical to the uninterrupted one (the live-seat resync
/// leg of the loopback gate depends on exactly this).
#[test]
fn supervised_snapshot_restore_continues_identically() {
    let mut whole = Runner::supervised();
    stage_decoy_patrol(&mut whole);
    whole.idle(120);

    // Fork a restored copy at this point.
    let snap = whole.sim.snapshot();
    let def = whole.sim.definition().clone();
    let mut forked = Runner {
        sim: GhostLobbySim::restore(def, snap).expect("snapshot restores"),
        // Held buttons are client-side input state, not snapshot state: a
        // rejoining seat re-derives them from its command stream, and this
        // fork does the same by inheriting them.
        held: whole.held,
        log: Vec::new(),
    };
    let _ = forked.sim.drain_events();

    let mut whole_events = Vec::new();
    let mut forked_events = Vec::new();
    for _ in 0..300u64 {
        whole.step(&[]);
        forked.step(&[]);
        if whole.sim.is_ended() || forked.sim.is_ended() {
            break;
        }
    }
    whole_events.extend(whole.log.iter().skip_while(|_| false));
    forked_events.extend(forked.log.iter());
    // Compare only the continuation: the fork's log starts at the fork point.
    let tail = &whole_events[whole_events.len() - forked_events.len()..];
    assert_eq!(
        serde_json::to_string(&tail).unwrap(),
        serde_json::to_string(&forked_events).unwrap(),
        "restored supervised run continues byte-identically"
    );
    assert_eq!(
        serde_json::to_string(&whole.sim.snapshot()).unwrap(),
        serde_json::to_string(&forked.sim.snapshot()).unwrap(),
        "final snapshots agree"
    );
}
