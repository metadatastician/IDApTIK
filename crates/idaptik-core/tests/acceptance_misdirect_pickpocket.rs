//! Stage-B acceptance: the two "extra" mechanics the ending suite did not cover
//! as standalone scripted runs — the deliberate **USB-throw misdirection** and
//! the lights-out **pickpocket** — each driven through the real command pipeline
//! and asserted on the resulting [`Debrief`] (reason / grade / score / tags).
//!
//! Both set up a precise pre-condition via the public `snapshot()`/`restore()`
//! surface (a faithful reflective seam: state is pure data), then let the sim's
//! own systems run the mechanic. No internal hooks are used.

mod common;
use common::Runner;
use idaptik_core::scenario::GhostLobbySim;
use idaptik_core::scenario::command::{Button, Buttons, Command};
use idaptik_core::scenario::common::{
    BillyMode, ExtractMethod, FailReason, Grade, ObjectKind, Outcome,
};
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
    // `restore` emits no events; drop any stragglers so the log reflects the run.
    let _ = r.sim.drain_events();
}

/// Throwing the carried USB is the canonical misdirection: it pins Billy's USB
/// belief and, on a subsequent note extraction, earns the decoy tag/score while
/// the debrief prose stays in the *narrower* "not fully controlled" register
/// (Billy neither holds nor reported the drive).
#[test]
fn misdirection_throw_then_note_extract_debrief() {
    let mut r = Runner::standard();
    r.step(&[Command::ForceCrisis]);
    // Secure the note and arm the player with the USB, mid-floor and clear of any
    // interaction volume, then let the sim run the throw.
    restore_from(&mut r, |s| {
        s.state.player.x = 400.0;
        s.state.player.vx = 0.0;
        s.state.player.has_note = true;
        s.state.note.held = true;
        s.state.player.has_usb = true;
        s.state.usb.held = true;
        s.state.usb.x = 400.0;
    });

    r.step(&[Command::ThrowUsb]);
    assert!(r.saw(|e| matches!(e, Event::UsbThrown)), "throw is logged");
    assert_eq!(r.sim.state().billy.usb_interest, 100.0);
    assert_eq!(r.sim.state().billy.belief, Some(ObjectKind::Usb));
    assert!(!r.sim.state().player.has_usb);
    assert!(!r.sim.is_ended());

    // Extract via the service exit with the note in hand.
    r.step(&[Command::ForceExtract {
        method: ExtractMethod::ServiceExit,
    }]);
    let d = r.sim.debrief().expect("debrief").clone();

    assert!(d.success);
    assert_eq!(d.reason, Outcome::Extracted);
    assert!(
        d.breakdown.misdirect > 0.0,
        "decoy earns the misdirect term"
    );
    assert!(d.breakdown.note > 0.0, "note secured earns the note term");
    assert!(d.score > 0);
    assert_ne!(d.grade, Grade::D, "a clean note+decoy extract out-grades D");
    assert!(
        d.tags.iter().any(|t| t.text.contains("Decoy success")),
        "decoy tag present: {:?}",
        d.tags
    );
    assert!(
        d.tags
            .iter()
            .any(|t| t.text.contains("contact lead secured")),
        "note tag present"
    );
    // Fix: Billy never held nor reported the drive, so the prose is the narrower
    // "interpretation was not fully controlled" paragraph, not the strong branch.
    assert!(
        d.debrief_text.contains("not fully controlled"),
        "narrow debrief paragraph, got: {}",
        d.debrief_text
    );
}

/// Lifting the note from Billy's pocket during a lights-out window flips the note
/// to the player, spikes player-interest to 100 and drives Billy into `Pursue`.
/// A follow-up extraction then debriefs as a secured-lead success.
#[test]
fn pickpocket_lights_out_then_extract_debrief() {
    let mut r = Runner::standard();
    r.step(&[Command::ForceCrisis]);
    // Billy holds the real note; the player stands on him under a long flicker,
    // with Billy stunned so he cannot drift out of pickpocket range.
    restore_from(&mut r, |s| {
        s.state.billy.x = 400.0;
        s.state.billy.vx = 0.0;
        s.state.billy.stun = 10.0;
        s.state.billy.has_note = true;
        s.state.note.billy_has = true;
        s.state.note.held = false;
        s.state.player.x = 400.0;
        s.state.player.vx = 0.0;
        s.state.player.grounded = true;
        s.state.lights_flicker = 50.0;
    });

    // Hold interact until the 0.72 s pickpocket completes.
    r.step(&[Command::SetButton {
        button: Button::Interact,
        down: true,
    }]);
    for _ in 0..80 {
        if r.saw(|e| matches!(e, Event::PickpocketSucceeded)) {
            break;
        }
        r.step(&[]);
    }

    assert!(
        r.saw(|e| matches!(e, Event::PickpocketSucceeded)),
        "pickpocket completes within the flicker window"
    );
    assert!(r.sim.state().player.has_note, "note is now the player's");
    assert!(!r.sim.state().note.billy_has);
    // Pickpocket pins player-interest to 100; later systems this tick may shave a
    // hair of decay, so assert the spike rather than an exact float.
    assert!(r.sim.state().billy.player_interest >= 99.0);
    assert_eq!(r.sim.state().billy.mode, BillyMode::Pursue);
    assert!(!r.sim.is_ended());

    // Extract with the lifted note.
    r.step(&[Command::ForceExtract {
        method: ExtractMethod::ServiceExit,
    }]);
    let d = r.sim.debrief().expect("debrief").clone();

    assert!(d.success);
    assert_eq!(d.reason, Outcome::Extracted);
    assert!(d.breakdown.note > 0.0);
    assert!(d.score > 0);
    assert_ne!(d.grade, Grade::D);
    assert!(
        d.tags
            .iter()
            .any(|t| t.text.contains("contact lead secured")),
        "strategic-success tag present: {:?}",
        d.tags
    );
}

/// Regression for the failure-score USB term: the HTML adds the +USB bonus only
/// when `usbInterest >= 72`. Billy physically holding the drive with interest
/// *below* the threshold must NOT contribute to the fail score (it only earns
/// the "believed the USB mattered" tag).
#[test]
fn fail_score_usb_term_keys_on_interest_not_possession() {
    // Billy holds the drive but interest is sub-threshold: tag yes, score no.
    let mut held = Runner::standard();
    held.step(&[Command::ForceCrisis]);
    restore_from(&mut held, |s| {
        s.state.billy.has_usb = true;
        s.state.billy.usb_interest = 40.0;
        s.state.alert = 0.0;
    });
    held.step(&[Command::ForceFail {
        reason: FailReason::Caught,
    }]);
    let dh = held.sim.debrief().expect("debrief").clone();
    assert_eq!(dh.reason, Outcome::Caught);
    assert_eq!(
        dh.breakdown.misdirect, 0.0,
        "possession with sub-72 interest adds no fail-score USB term"
    );
    assert!(
        dh.tags
            .iter()
            .any(|t| t.text.contains("believed the USB mattered")),
        "possession still earns the belief tag"
    );

    // Interest at/above threshold: the +USB term is applied.
    let mut hot = Runner::standard();
    hot.step(&[Command::ForceCrisis]);
    restore_from(&mut hot, |s| {
        s.state.billy.has_usb = false;
        s.state.billy.usb_interest = 80.0;
        s.state.alert = 0.0;
    });
    hot.step(&[Command::ForceFail {
        reason: FailReason::Caught,
    }]);
    let dhot = hot.sim.debrief().expect("debrief").clone();
    assert!(
        dhot.breakdown.misdirect > 0.0,
        "interest >= 72 applies the fail-score USB term"
    );
    // Same alert/note baseline, so the hot run scores strictly higher.
    assert!(dhot.score > dh.score);
}
