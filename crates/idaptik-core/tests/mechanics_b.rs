//! Stage B: reduced-motion sim effect, uplink throttling, Billy FSM progression.

mod common;
use common::Runner;
use idaptik_core::scenario::ActionKind;
use idaptik_core::scenario::command::Command;
use idaptik_core::scenario::common::BillyMode;
use idaptik_core::scenario::event::Event;

fn uplink(kind: ActionKind) -> Command {
    Command::Uplink { kind }
}

#[test]
fn reduced_motion_shortens_lights_flicker() {
    let mut full = Runner::start(
        idaptik_core::scenario::DifficultyId::Standard,
        false,
        123456,
    );
    let mut reduced = Runner::start(idaptik_core::scenario::DifficultyId::Standard, true, 123456);
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
    // Spend bandwidth, then let it regenerate over a second of quiet.
    r.step(&[uplink(ActionKind::Vacuum)]);
    let low = r.sim.state().bandwidth;
    r.idle(120);
    assert!(r.sim.state().bandwidth > low);
    assert!(r.sim.state().bandwidth <= 100.0);
}
