//! Stage B: the timer-crisis vs immediate-crisis Billy-movement asymmetry.

mod common;
use common::Runner;
use idaptik_core::scenario::command::Command;
use idaptik_core::scenario::common::Phase;
use idaptik_core::scenario::constants::CRISIS_BILLY_X;

#[test]
fn timer_crisis_billy_moves_next_tick() {
    // Run quiet until the timer flips into crisis (crisis_at ~26s on Standard).
    let mut r = Runner::standard();
    let mut transition = None;
    for _ in 0..3000u64 {
        r.step(&[]);
        if r.sim.state().phase == Phase::Crisis {
            transition = Some(());
            break;
        }
    }
    assert!(transition.is_some(), "timer crisis must fire");
    // On the transition tick Billy was placed at CRISIS_BILLY_X but the Billy
    // system already ran this tick, so he has NOT moved yet.
    assert_eq!(r.sim.state().billy.x, CRISIS_BILLY_X);
    // Next tick, Billy advances toward his snack.
    r.step(&[]);
    assert_ne!(r.sim.state().billy.x, CRISIS_BILLY_X);
}

#[test]
fn immediate_crisis_billy_moves_same_tick() {
    // ForceCrisis is applied pre-systems, so Billy moves on the crisis tick.
    let mut r = Runner::standard();
    r.step(&[Command::ForceCrisis]);
    assert_eq!(r.sim.state().phase, Phase::Crisis);
    assert_ne!(
        r.sim.state().billy.x,
        CRISIS_BILLY_X,
        "immediate crisis lets Billy move the same tick"
    );
}

#[test]
fn crisis_sets_alert_floor() {
    let mut r = Runner::standard();
    r.step(&[Command::ForceCrisis]);
    // Test/timer crises raise alert to 8 (usb to 14); one tick of crisis decay
    // (0.18/60) then nudges it just under.
    assert!(r.sim.state().alert > 7.9);
}
