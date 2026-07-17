//! The actor archetype + modifier registry: round-trip, validation errors,
//! deterministic composition, the valueSignal belief economy, and the
//! regression pin that Billy-as-data is the ported Billy.

use idaptik_core::scenario::actor::{
    ACTORS_FORMAT, ACTORS_JSON, ActorPackError, ActorRegistry, ActorValidationError,
    BILLY_ARCHETYPE_ID, ComposeError, InterestProfile, Leakage, NOTE_OBJECT, ObjectClass, StatId,
    USB_OBJECT, belief, billy_actor, billy_archetype, default_registry, load_actor_pack,
};
use idaptik_core::scenario::constants as c;
use idaptik_core::scenario::mathf::TICK_DT;

// --- round-trip -----------------------------------------------------------

#[test]
fn committed_golden_parses_back_to_the_canonical_registry() {
    let parsed: ActorRegistry = serde_json::from_str(ACTORS_JSON).expect("golden parses");
    assert_eq!(parsed, default_registry());
}

#[test]
fn registry_round_trips_through_json_unchanged() {
    let reg = default_registry();
    let json = serde_json::to_string(&reg).expect("serialize");
    let back: ActorRegistry = serde_json::from_str(&json).expect("parse");
    assert_eq!(back, reg);
}

#[test]
fn canonical_registry_validates_clean() {
    let report = default_registry().validate();
    assert!(report.passed(), "failed checks: {:#?}", report);
    assert!(default_registry().ok().is_ok());
}

// --- validation errors ------------------------------------------------------

#[test]
fn a_wrong_format_tag_is_a_typed_error() {
    let mut reg = default_registry();
    reg.format = "idaptik-actors/999".to_owned();
    let errs = reg.ok().expect_err("wrong format must fail");
    assert!(errs.contains(&ActorValidationError::UnknownFormat {
        found: "idaptik-actors/999".to_owned(),
    }));
}

#[test]
fn an_empty_registry_is_a_typed_error() {
    let mut reg = default_registry();
    reg.archetypes.clear();
    let errs = reg.ok().expect_err("no archetypes must fail");
    assert!(errs.contains(&ActorValidationError::EmptyArchetypes));
}

#[test]
fn a_value_signal_outside_the_unit_interval_is_a_typed_error() {
    let mut reg = default_registry();
    if let Some(a) = reg.archetypes.get_mut(BILLY_ARCHETYPE_ID)
        && let Some(p) = a.interests.get_mut(USB_OBJECT)
    {
        p.value_signal = 1.5;
    }
    let errs = reg.ok().expect_err("valueSignal 1.5 must fail");
    assert!(errs.iter().any(|e| matches!(
        e,
        ActorValidationError::ValueSignalOutOfRange { archetype, object, value }
            if archetype == BILLY_ARCHETYPE_ID && object == USB_OBJECT && *value == 1.5
    )));
}

#[test]
fn an_archetype_keyed_by_another_id_is_a_typed_error() {
    let mut reg = default_registry();
    let billy = reg
        .archetypes
        .remove(BILLY_ARCHETYPE_ID)
        .expect("billy is canonical");
    reg.archetypes.insert("impostor".to_owned(), billy);
    let errs = reg.ok().expect_err("key/id mismatch must fail");
    assert!(errs.iter().any(|e| matches!(
        e,
        ActorValidationError::ArchetypeKeyMismatch { key, id }
            if key == "impostor" && id == BILLY_ARCHETYPE_ID
    )));
}

#[test]
fn a_non_finite_stat_is_a_typed_error() {
    let mut reg = default_registry();
    if let Some(a) = reg.archetypes.get_mut(BILLY_ARCHETYPE_ID) {
        a.stats.pursue_speed = f64::NAN;
    }
    let errs = reg.ok().expect_err("NaN stat must fail");
    assert!(errs.iter().any(|e| matches!(
        e,
        ActorValidationError::NonFiniteStat {
            stat: StatId::PursueSpeed,
            ..
        }
    )));
}

// --- the DLC seam ------------------------------------------------------------

#[test]
fn the_actor_pack_loader_accepts_the_canonical_payload() {
    let reg = load_actor_pack(ACTORS_JSON).expect("canonical pack loads");
    assert_eq!(reg.format, ACTORS_FORMAT);
    assert!(reg.archetypes.contains_key(BILLY_ARCHETYPE_ID));
}

#[test]
fn the_actor_pack_loader_refuses_garbage_and_invalid_payloads() {
    assert!(matches!(
        load_actor_pack("not json"),
        Err(ActorPackError::Parse(_))
    ));
    let mut reg = default_registry();
    reg.format = "something-else/1".to_owned();
    let json = serde_json::to_string(&reg).expect("serialize");
    assert!(matches!(
        load_actor_pack(&json),
        Err(ActorPackError::Invalid(_))
    ));
}

// --- composition ---------------------------------------------------------------

#[test]
fn base_plus_two_modifiers_composes_to_the_expected_stats() {
    let reg = default_registry();
    let composed = reg
        .compose(BILLY_ARCHETYPE_ID, &["veteran", "skittish"])
        .expect("both modifiers are canonical");

    assert_eq!(composed.archetype, BILLY_ARCHETYPE_ID);
    assert_eq!(composed.modifiers, vec!["veteran", "skittish"]);

    // veteran
    assert_eq!(composed.stats.pursue_speed, c::PURSUE_SPEED * 1.15);
    assert_eq!(composed.stats.sight_crouch, c::SIGHT_CROUCH * 1.15);
    assert_eq!(composed.stats.pursue_giveup_ago, c::PURSUE_GIVEUP_AGO + 1.4);
    // skittish
    assert_eq!(composed.stats.assess_speed, c::ASSESS_SPEED * 1.2);
    assert_eq!(composed.stats.pi_decay, c::PI_DECAY + 0.45);
    assert_eq!(
        composed.stats.pursue_giveup_dist,
        c::PURSUE_GIVEUP_DIST * 1.5
    );
    // both touch the belief threshold: veteran's Add lands before skittish's
    // Mul, so the result is (48 - 6) * 0.5.
    assert_eq!(
        composed.stats.belief_threshold,
        (c::BELIEF_THRESHOLD - 6.0) * 0.5
    );
    // untouched stats pass through unchanged, as do the interest profiles.
    assert_eq!(composed.stats.enter_speed, c::ENTER_SPEED);
    assert_eq!(composed.interests, billy_archetype().interests);
}

#[test]
fn composition_is_deterministic_and_order_sensitive() {
    let reg = default_registry();
    let a = reg
        .compose(BILLY_ARCHETYPE_ID, &["veteran", "skittish"])
        .expect("compose");
    let b = reg
        .compose(BILLY_ARCHETYPE_ID, &["veteran", "skittish"])
        .expect("compose again");
    assert_eq!(a, b, "the same inputs compose to the same actor");

    let reversed = reg
        .compose(BILLY_ARCHETYPE_ID, &["skittish", "veteran"])
        .expect("compose reversed");
    assert_eq!(
        reversed.stats.belief_threshold,
        c::BELIEF_THRESHOLD * 0.5 - 6.0,
        "application order is part of the definition, not an accident"
    );
}

#[test]
fn composing_unknown_ids_is_a_typed_error_not_a_panic() {
    let reg = default_registry();
    assert_eq!(
        reg.compose("nobody", &[]),
        Err(ComposeError::UnknownArchetype("nobody".to_owned()))
    );
    assert_eq!(
        reg.compose(BILLY_ARCHETYPE_ID, &["haunted"]),
        Err(ComposeError::UnknownModifier("haunted".to_owned()))
    );
}

// --- the valueSignal belief economy ---------------------------------------------

/// A high-valueSignal decoy draws the actor's belief: two objects observed
/// identically, profiled only by their `valueSignal`, and the louder signal
/// crosses the belief threshold first — the note-vs-USB decoy, generalised.
#[test]
fn a_high_value_signal_decoy_draws_belief() {
    let threshold = c::BELIEF_THRESHOLD;
    let objective = InterestProfile::from_value_signal(ObjectClass::Objective, 0.35);
    let decoy = InterestProfile::from_value_signal(ObjectClass::Decoy, 0.95);

    let mut interest_objective = 0.0;
    let mut interest_decoy = 0.0;
    let mut first_belief: Option<&str> = None;

    // Ten simulated seconds of the actor watching a moving carrier near both.
    for _ in 0..600 {
        interest_objective = belief::interest_observed(
            interest_objective,
            &objective,
            Leakage::Moving,
            false,
            TICK_DT,
        );
        interest_decoy =
            belief::interest_observed(interest_decoy, &decoy, Leakage::Moving, false, TICK_DT);
        if first_belief.is_none() {
            first_belief = belief::belief_over(
                &[("objective", interest_objective), ("decoy", interest_decoy)],
                threshold,
            );
        }
    }

    assert!(
        interest_decoy > interest_objective,
        "the louder valueSignal accrues interest faster"
    );
    assert_eq!(
        first_belief,
        Some("decoy"),
        "belief forms on the high-valueSignal decoy first"
    );
}

#[test]
fn unobserved_interest_decays_unless_pinned() {
    let profile = InterestProfile::from_value_signal(ObjectClass::Decoy, 0.8);
    let decayed = belief::interest_unobserved(50.0, &profile, false, TICK_DT);
    assert!(decayed < 50.0, "an unpinned object fades");
    let pinned = belief::interest_unobserved(50.0, &profile, true, TICK_DT);
    assert_eq!(pinned, 50.0, "a pinned object is not forgotten");
    assert_eq!(
        belief::interest_unobserved(0.0, &profile, false, TICK_DT),
        0.0,
        "interest never goes negative"
    );
}

#[test]
fn belief_ties_resolve_to_the_earlier_entry() {
    // The Ghost Lobby lists the note first, so `note >= usb` picks the note —
    // the engine must reproduce that tie-break for the regression to hold.
    assert_eq!(
        belief::belief_over(&[("note", 60.0), ("usb", 60.0)], 48.0),
        Some("note")
    );
    assert_eq!(
        belief::belief_over(&[("note", 40.0), ("usb", 41.0)], 48.0),
        None,
        "no belief below the threshold"
    );
}

// --- the regression pin ----------------------------------------------------------

/// Billy expressed as data is the ported Billy: every stat and interest number
/// in the default archetype equals the constant the FSM used to read. With this
/// pin, the Ghost Lobby acceptance/determinism suites passing over the
/// archetype-driven sim *is* the "endings unchanged" regression.
#[test]
fn billy_archetype_matches_the_ported_constants() {
    let billy = billy_actor();
    let s = &billy.stats;
    assert_eq!(s.enter_speed, c::ENTER_SPEED);
    assert_eq!(s.assess_speed, c::ASSESS_SPEED);
    assert_eq!(s.invest_speed, c::INVEST_SPEED);
    assert_eq!(s.secure_speed, c::SECURE_SPEED);
    assert_eq!(s.guard_speed, c::GUARD_SPEED);
    assert_eq!(s.pursue_speed, c::PURSUE_SPEED);
    assert_eq!(s.shock_t, c::SHOCK_T);
    assert_eq!(s.call_t, c::CALL_T);
    assert_eq!(s.patrol_lo, c::PATROL_LO);
    assert_eq!(s.patrol_hi, c::PATROL_HI);
    assert_eq!(s.patrol_pivot, c::PATROL_PIVOT);
    assert_eq!(s.grab_dist, c::GRAB_DIST);
    assert_eq!(s.pursue_trigger, c::PURSUE_TRIGGER);
    assert_eq!(s.pursue_giveup_ago, c::PURSUE_GIVEUP_AGO);
    assert_eq!(s.pursue_giveup_dist, c::PURSUE_GIVEUP_DIST);
    assert_eq!(s.accel, c::BILLY_ACCEL);
    assert_eq!(s.sight_back, c::SIGHT_BACK);
    assert_eq!(s.sight_crouch, c::SIGHT_CROUCH);
    assert_eq!(s.sight_alert, c::SIGHT_ALERT);
    assert_eq!(s.sight_alert_hi, c::SIGHT_ALERT_HI);
    assert_eq!(s.alert_boost_div, c::ALERT_BOOST_DIV);
    assert_eq!(s.lights_boost_k, c::LIGHTS_BOOST_K);
    assert_eq!(s.distract_dist, c::VAC_DISTRACT);
    assert_eq!(s.invest_noise, c::NOISE_INVEST_NOISE);
    assert_eq!(s.invest_dist, c::NOISE_INVEST_DIST);
    assert_eq!(s.belief_threshold, c::BELIEF_THRESHOLD);
    assert_eq!(s.pi_seen, c::PI_SEEN);
    assert_eq!(s.pi_sprint, c::PI_SPRINT);
    assert_eq!(s.pi_decay, c::PI_DECAY);

    let note = billy.interest_or_inert(NOTE_OBJECT);
    assert_eq!(note.kind, ObjectClass::Objective);
    assert_eq!(note.near, c::NOTE_NEAR);
    assert_eq!(note.urg_still, c::NOTE_URG_STILL);
    assert_eq!(note.urg_move, c::NOTE_URG_MOVE);
    assert_eq!(note.urg_sprint, c::NOTE_URG_SPRINT);
    assert_eq!(note.carry, c::NOTE_CARRY);
    assert_eq!(note.decay, c::NOTE_DECAY);
    assert_eq!(note.guard_t, c::GUARD_T_NOTE);

    let usb = billy.interest_or_inert(USB_OBJECT);
    assert_eq!(usb.kind, ObjectClass::Decoy);
    assert_eq!(usb.near, c::USB_NEAR);
    assert_eq!(usb.urg_still, c::USB_URG_STILL);
    assert_eq!(usb.urg_move, c::USB_URG_MOVE);
    assert_eq!(usb.urg_sprint, c::USB_URG_SPRINT);
    assert_eq!(usb.carry, c::USB_CARRY);
    assert_eq!(usb.decay, c::USB_DECAY);
    assert_eq!(usb.guard_t, c::GUARD_T_USB);
}

#[test]
fn an_unprofiled_object_is_inert_not_a_panic() {
    let billy = billy_actor();
    let ghost = billy.interest_or_inert("no-such-object");
    assert_eq!(ghost.urgency(Leakage::Sprinting), 0.0);
    assert_eq!(ghost.decay, 0.0);
    assert_eq!(
        belief::interest_observed(0.0, &ghost, Leakage::Sprinting, true, TICK_DT),
        0.0,
        "the actor never notices an object it has no profile for"
    );
}
