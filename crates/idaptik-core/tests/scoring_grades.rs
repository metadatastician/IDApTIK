//! Stage B: grade bands, `js_round` parity, and a golden extract score.

mod common;
use common::Runner;
use idaptik_core::scenario::command::Command;
use idaptik_core::scenario::common::{ExtractMethod, Grade};
use idaptik_core::scenario::grade_for;
use idaptik_core::scenario::mathf::js_round;

#[test]
fn grade_bands_success_and_fail() {
    let bands = idaptik_core::ghost_lobby().scoring.grades;
    assert_eq!(grade_for(1850, true, &bands), Grade::S);
    assert_eq!(grade_for(1450, true, &bands), Grade::A);
    assert_eq!(grade_for(1050, true, &bands), Grade::B);
    assert_eq!(grade_for(700, true, &bands), Grade::C);
    assert_eq!(grade_for(699, true, &bands), Grade::D);
    assert_eq!(grade_for(500, false, &bands), Grade::C);
    assert_eq!(grade_for(499, false, &bands), Grade::D);
}

#[test]
fn js_round_is_half_to_plus_infinity() {
    assert_eq!(js_round(2.5), 3.0);
    assert_eq!(js_round(-0.5), 0.0);
    assert_eq!(js_round(0.5), 1.0);
    assert_eq!(js_round(1220.4999), 1220.0);
}

#[test]
fn golden_service_exit_score_at_t0() {
    // ForceExtract runs pre-systems at t = 0: a fully deterministic base score.
    let mut r = Runner::standard();
    r.step(&[Command::ForceExtract {
        method: ExtractMethod::ServiceExit,
    }]);
    let d = r.sim.debrief().expect("debrief");
    // 700 base - 80 no-note + 150 no-boss + 120 no-cam + 100 iso-ok + 220 time.
    assert_eq!(d.score, 1210);
    assert_eq!(d.grade, Grade::B);
    assert_eq!(d.breakdown.raw, 1210.0);
    assert_eq!(d.breakdown.score_mult, 1.0);
}

#[test]
fn golden_chute_score_adds_bonus() {
    let mut r = Runner::standard();
    r.step(&[Command::ForceExtract {
        method: ExtractMethod::LaundryChute,
    }]);
    let d = r.sim.debrief().expect("debrief");
    assert_eq!(d.breakdown.chute, 110.0);
    assert_eq!(d.score, 1320);
}

#[test]
fn operator_multiplier_scales_score() {
    let mut r = Runner::start(
        idaptik_core::scenario::DifficultyId::Operator,
        false,
        123456,
    );
    r.step(&[Command::ForceExtract {
        method: ExtractMethod::ServiceExit,
    }]);
    let d = r.sim.debrief().expect("debrief");
    // raw 1210 * 1.25 = 1512.5 -> js_round -> 1513.
    assert_eq!(d.breakdown.score_mult, 1.25);
    assert_eq!(d.score, 1513);
}
