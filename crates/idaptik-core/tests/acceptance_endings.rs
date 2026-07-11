//! Stage B: drive all five endings and assert the debrief shape.

mod common;
use common::Runner;
use idaptik_core::scenario::command::Command;
use idaptik_core::scenario::common::{ExtractMethod, FailReason, Outcome};
use idaptik_core::scenario::event::Event;

fn force_extract(method: ExtractMethod) -> Runner {
    let mut r = Runner::standard();
    r.step(&[Command::ForceExtract { method }]);
    r
}

fn force_fail(reason: FailReason) -> Runner {
    let mut r = Runner::standard();
    r.step(&[Command::ForceFail { reason }]);
    r
}

#[test]
fn service_exit_extraction() {
    let r = force_extract(ExtractMethod::ServiceExit);
    assert!(r.sim.is_ended());
    let d = r.sim.debrief().expect("debrief");
    assert!(d.success);
    assert_eq!(d.reason, Outcome::Extracted);
    assert!(r.saw(|e| matches!(
        e,
        Event::Extracted {
            method: ExtractMethod::ServiceExit
        }
    )));
    assert!(r.saw(|e| matches!(
        e,
        Event::RunEnded {
            outcome: Outcome::Extracted
        }
    )));
}

#[test]
fn laundry_chute_extraction() {
    let r = force_extract(ExtractMethod::LaundryChute);
    let d = r.sim.debrief().expect("debrief");
    assert!(d.success);
    assert_eq!(d.reason, Outcome::Extracted);
    // Chute route awards the humiliation bonus.
    assert!(d.breakdown.chute > 0.0);
}

#[test]
fn caught_ending() {
    let r = force_fail(FailReason::Caught);
    let d = r.sim.debrief().expect("debrief");
    assert!(!d.success);
    assert_eq!(d.reason, Outcome::Caught);
    assert!(r.saw(|e| matches!(
        e,
        Event::MissionFailed {
            reason: FailReason::Caught
        }
    )));
}

#[test]
fn partition_ending() {
    let r = force_fail(FailReason::Partition);
    let d = r.sim.debrief().expect("debrief");
    assert!(!d.success);
    assert_eq!(d.reason, Outcome::Partition);
}

#[test]
fn lockdown_ending() {
    let r = force_fail(FailReason::Lockdown);
    let d = r.sim.debrief().expect("debrief");
    assert!(!d.success);
    assert_eq!(d.reason, Outcome::Lockdown);
}

#[test]
fn ended_is_terminal_no_further_events() {
    let mut r = force_extract(ExtractMethod::ServiceExit);
    let before = r.log.len();
    // Ticks after ending produce no further events and do not panic.
    r.idle(120);
    assert_eq!(r.log.len(), before);
    assert!(r.sim.is_ended());
}

#[test]
fn natural_crisis_then_extract() {
    // Force crisis, run a while, then force a service-exit extract.
    let mut r = Runner::standard();
    r.step(&[Command::ForceCrisis]);
    assert!(r.saw(|e| matches!(e, Event::CrisisBegan { .. })));
    r.idle(60);
    r.step(&[Command::ForceExtract {
        method: ExtractMethod::ServiceExit,
    }]);
    assert!(r.sim.debrief().expect("debrief").success);
}
