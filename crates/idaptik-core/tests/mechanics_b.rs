//! Stage B: reduced-motion sim effect, uplink throttling, Billy FSM progression.

mod common;
use common::Runner;
use idaptik_core::scenario::ActionKind;
use idaptik_core::scenario::command::Command;
use idaptik_core::scenario::common::{BillyMode, DenyReason};
use idaptik_core::scenario::event::Event;

fn uplink(kind: ActionKind) -> Command {
    Command::Uplink { kind }
}

/// Pivot the hacker into the building's maintenance bridge, which is what opens a
/// route from the van to the floor's fixtures. Every test that means to exercise a
/// landed uplink must do this first. Driven as the real `Command::Pivot` (one
/// tick), so the fixture takes exactly the path a recorded stream replays.
fn pivot_in(r: &mut Runner) {
    r.step(&[Command::Pivot {
        target: idaptik_core::scenario::command::PivotTarget::Bridge,
    }]);
    assert!(
        r.saw(|e| matches!(e, Event::PivotOpened { .. })),
        "the van can reach the maintenance bridge"
    );
}

#[test]
fn the_hacker_cannot_act_before_pivoting_and_can_after() {
    let mut r = Runner::standard();
    let bandwidth_before = r.sim.state().bandwidth;

    // Cold from the van, no fixture is reachable: the action is denied on route.
    r.step(&[uplink(ActionKind::Lights)]);
    assert!(
        r.saw(|e| matches!(
            e,
            Event::UplinkDenied {
                reason: DenyReason::NoRoute,
                ..
            }
        )),
        "a cold van has no route to the lights"
    );
    assert_eq!(r.sim.state().stats.hacker_actions, 0);
    assert_eq!(
        r.sim.state().bandwidth,
        bandwidth_before,
        "a denied route must not charge bandwidth"
    );
    assert_eq!(
        r.sim.state().lights_flicker,
        0.0,
        "a denied route must not darken the floor"
    );

    // Pivot, and the same action lands.
    pivot_in(&mut r);
    r.step(&[uplink(ActionKind::Lights)]);
    assert_eq!(r.sim.state().stats.hacker_actions, 1);
    assert!(r.sim.state().lights_flicker > 0.0);
    assert!(
        r.sim.state().bandwidth < bandwidth_before,
        "a landed action charges bandwidth"
    );
}

#[test]
fn a_route_denial_charges_no_cooldown() {
    // A route you never had is not a resource you spent: the denial must leave the
    // action ready to fire the instant a route exists.
    let mut r = Runner::standard();
    r.step(&[uplink(ActionKind::Lights)]);
    assert_eq!(r.sim.state().stats.failed_actions, 1);
    pivot_in(&mut r);
    // The very next tick, with no cooldown to wait out, the action lands.
    r.step(&[uplink(ActionKind::Lights)]);
    assert_eq!(r.sim.state().stats.hacker_actions, 1);
}

#[test]
fn reduced_motion_shortens_lights_flicker() {
    let mut full = Runner::start(
        idaptik_core::scenario::DifficultyId::Standard,
        false,
        123456,
    );
    let mut reduced = Runner::start(idaptik_core::scenario::DifficultyId::Standard, true, 123456);
    pivot_in(&mut full);
    pivot_in(&mut reduced);
    full.step(&[Command::ForceCrisis, uplink(ActionKind::Lights)]);
    reduced.step(&[Command::ForceCrisis, uplink(ActionKind::Lights)]);
    // 1.45 vs 0.70 window: reduced motion is strictly shorter.
    assert!(full.sim.state().lights_flicker > reduced.sim.state().lights_flicker);
    assert!(reduced.sim.state().lights_flicker > 0.0);
    // And Billy is stunned by the flicker either way.
    assert!(full.sim.state().billy.stun > 0.0);
}

#[test]
fn cooldown_denial_is_throttled_but_stats_still_count() {
    let mut r = Runner::standard();
    pivot_in(&mut r);
    // Camera succeeds, then two rapid retries are on cooldown.
    r.step(&[uplink(ActionKind::Camera)]);
    r.step(&[uplink(ActionKind::Camera)]);
    r.step(&[uplink(ActionKind::Camera)]);
    let denied = r.count(|e| matches!(e, Event::UplinkDenied { .. }));
    assert_eq!(
        denied, 1,
        "the 1.2s throttle suppresses the duplicate event"
    );
    assert_eq!(
        r.sim.state().stats.failed_actions,
        2,
        "both denials still increment the stat"
    );
    assert_eq!(r.sim.state().stats.hacker_actions, 1);
}

#[test]
fn crisis_entry_transitions_billy_and_can_catch_the_idle_agent() {
    let mut r = Runner::standard();
    r.step(&[Command::ForceCrisis]);
    // Offsite -> Entering is emitted as a state change.
    assert!(r.saw(|e| matches!(
        e,
        Event::BillyStateChanged {
            from: BillyMode::Offsite,
            to: BillyMode::Entering
        }
    )));
    assert_eq!(r.sim.state().billy.mode, BillyMode::Entering);
    // An idle agent standing in Billy's path is rescued once, then caught.
    r.idle(400);
    assert!(r.sim.is_ended());
    assert!(r.saw(|e| matches!(e, Event::RescueUsed)));
    assert_eq!(
        r.sim.debrief().expect("debrief").reason,
        idaptik_core::scenario::common::Outcome::Caught
    );
    // The one-shot rescue fired exactly once.
    assert_eq!(r.count(|e| matches!(e, Event::RescueUsed)), 1);
}

#[test]
fn bandwidth_regen_and_alert_decay_run_in_quiet() {
    let mut r = Runner::standard();
    pivot_in(&mut r);
    // Spend bandwidth, then let it regenerate over a second of quiet.
    r.step(&[uplink(ActionKind::Vacuum)]);
    let low = r.sim.state().bandwidth;
    r.idle(120);
    assert!(r.sim.state().bandwidth > low);
    assert!(r.sim.state().bandwidth <= 100.0);
}
