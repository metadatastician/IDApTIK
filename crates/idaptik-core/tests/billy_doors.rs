//! Billy's badge: a closed door costs him `badge_delay` seconds, it does not
//! stop him. Doors buy the infiltrator time; they are not walls.
//!
//! Regression cover for the bounce/reset limit cycle that made
//! `Event::BillyBadgedDoor` unreachable in play: `constrain_by_doors` snaps a
//! blocked Billy a few pixels clear of the door plane, so most ticks of a door
//! wait are *not* collision ticks — the wait timer used to reset on every one
//! of them and never reached `badge_delay` at any speed.

mod common;
use common::Runner;
use idaptik_core::scenario::GhostLobbySim;
use idaptik_core::scenario::command::{Buttons, Command};
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

/// Billy assessing in the hall, patrolling toward a target in the kitchen —
/// the kitchen/hall door (D1, x = 270) stands between them, closed. The
/// player is parked in the laundry, a different room, so sight and noise stay
/// cold and Billy keeps his Assess patrol for the whole window.
fn stage_billy_at_d1(r: &mut Runner, patrol_target: f64) {
    r.step(&[Command::ForceCrisis]);
    restore_from(r, |s| {
        s.state.billy.mode = BillyMode::Assess;
        s.state.billy.state_timer = 0.0;
        s.state.billy.x = 300.0;
        s.state.billy.vx = 0.0;
        s.state.billy.patrol_target = patrol_target;
        s.state.billy.belief = None;
        s.state.billy.belief_announced = None;
        s.state.billy.target = None;
        s.state.billy.note_interest = 0.0;
        s.state.billy.usb_interest = 0.0;
        s.state.billy.player_interest = 0.0;
        s.state.player.x = 900.0;
        s.state.player.vx = 0.0;
        s.state.player.noise = 0.0;
        // Both props out of play so no belief forms mid-window.
        s.state.usb.x = 900.0;
        s.state.note.x = 900.0;
    });
}

/// Billy's centre, the coordinate the door constraint actually compares.
fn centre(r: &Runner) -> f64 {
    r.sim.state().billy.x + r.sim.definition().billy.w / 2.0
}

#[test]
fn billy_badges_through_a_closed_door_on_his_patrol() {
    let mut r = Runner::standard();
    stage_billy_at_d1(&mut r, 155.0);

    assert_eq!(r.sim.state().doors[0].open, 0.0, "D1 starts closed");
    let start = centre(&r);
    assert!(start > 270.0, "Billy starts east of D1: {start}");

    // badge_delay is 1.32 s on Standard (~80 ticks); 600 ticks is ten seconds,
    // ample for the wait plus the walk through. The Assess patrol ping-pongs
    // across the whole band, so track the westmost reach rather than where he
    // happens to stand at the end.
    let mut westmost = start;
    for _ in 0..600u64 {
        r.step(&[]);
        westmost = westmost.min(centre(&r));
        if r.sim.is_ended() {
            break;
        }
    }

    assert!(
        r.saw(|e| matches!(e, Event::BillyBadgedDoor { .. })),
        "Billy badges through the closed door instead of being walled by it"
    );
    assert!(
        westmost < 270.0,
        "and passes the door plane: westmost centre {westmost}"
    );
}

#[test]
fn the_badge_wait_resets_when_billy_no_longer_wants_through() {
    let mut r = Runner::standard();
    stage_billy_at_d1(&mut r, 155.0);

    // Walk him up to D1 and stop at the first tick the wait is running — well
    // short of badge_delay (1.32 s), and independent of speed tuning.
    let mut pressed = false;
    for _ in 0..200u64 {
        r.step(&[]);
        if r.sim.state().billy.door_wait > 0.0 {
            pressed = true;
            break;
        }
    }
    assert!(pressed, "Billy reaches D1 and the wait starts");
    assert_eq!(r.sim.state().billy.blocked_door, Some(0));
    let badges_before = r.count(|e| matches!(e, Event::BillyBadgedDoor { .. }));
    assert_eq!(badges_before, 0, "not yet badged this early");

    // He changes his mind: the target is now on his own side.
    restore_from(&mut r, |s| s.state.billy.patrol_target = 420.0);
    r.idle(10);

    assert_eq!(
        r.sim.state().billy.door_wait,
        0.0,
        "walking away abandons the wait"
    );
    assert_eq!(r.sim.state().billy.blocked_door, None);
    assert_eq!(
        r.count(|e| matches!(e, Event::BillyBadgedDoor { .. })),
        badges_before,
        "and no badge fires for a door he gave up on"
    );
}
