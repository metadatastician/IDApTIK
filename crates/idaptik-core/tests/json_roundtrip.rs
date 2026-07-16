//! The definition round-trips through JSON unchanged, the committed golden
//! parses back equal, and the tuning/actions/scoring/spawn tables are a faithful
//! projection of `constants.rs`.

use idaptik_core::scenario::constants as c;
use idaptik_core::scenario::tuning::ActionKind;
use idaptik_core::{GHOST_LOBBY_JSON, ScenarioDefinition, ghost_lobby};

#[test]
fn definition_round_trips_through_json() {
    let def = ghost_lobby();
    let json = serde_json::to_string(&def).unwrap();
    let back: ScenarioDefinition = serde_json::from_str(&json).unwrap();
    assert_eq!(def, back);

    let pretty = serde_json::to_string_pretty(&def).unwrap();
    let back2: ScenarioDefinition = serde_json::from_str(&pretty).unwrap();
    assert_eq!(def, back2);
}

#[test]
fn committed_golden_parses_equal() {
    let def = ghost_lobby();
    let from_golden: ScenarioDefinition =
        serde_json::from_str(GHOST_LOBBY_JSON).expect("golden json parses");
    assert_eq!(
        def, from_golden,
        "GHOST_LOBBY_JSON is stale; run `cargo test -p idaptik-core \
         regenerate_golden_json -- --ignored`"
    );
}

#[test]
fn tuning_is_projected_from_constants() {
    let t = ghost_lobby().tuning;

    // geometry
    assert_eq!(t.geometry.floor, c::FLOOR);
    assert_eq!(t.geometry.world_width, c::WORLD_WIDTH);
    assert_eq!(t.geometry.collision_radius, c::COLLISION_RADIUS);

    // movement
    assert_eq!(t.movement.gravity, c::GRAVITY);
    assert_eq!(t.movement.jump_vy, c::JUMP_VY);
    assert_eq!(t.movement.clamp_lo, c::PLAYER_CLAMP_LO);
    assert_eq!(t.movement.clamp_hi, c::PLAYER_CLAMP_HI);

    // noise
    assert_eq!(t.noise.decay, c::NOISE_DECAY);
    assert_eq!(t.noise.sprint_base, c::NOISE_SPRINT_BASE);

    // fsm / billy
    assert_eq!(t.fsm.pursue_trigger, c::PURSUE_TRIGGER);
    assert_eq!(t.fsm.badge_open, c::BADGE_OPEN);
    assert_eq!(t.fsm.sight_office, c::SIGHT_OFFICE);
    assert_eq!(t.fsm.vac_distract, c::VAC_DISTRACT);

    // belief
    assert_eq!(t.belief.threshold, c::BELIEF_THRESHOLD);
    assert_eq!(t.belief.misdirect_threshold, c::MISDIRECT_THRESHOLD);
    assert_eq!(t.belief.take_usb_seen, c::TAKE_USB_SEEN);

    // support / bandwidth
    assert_eq!(t.support.approach, c::SUPPORT_APPROACH);
    assert_eq!(t.support.iso_pressure_div, c::ISO_PRESSURE_DIV);
    assert_eq!(t.support.hop_pen, c::SUPPORT_HOP_PEN);
    assert_eq!(t.bandwidth.alert_decay_crisis, c::ALERT_DECAY_CRISIS);
    assert_eq!(t.bandwidth.lockdown, c::LOCKDOWN);

    // action timing / camera / usb / vacuum / interaction / rescue
    assert_eq!(t.action.lights_dur, c::LIGHTS_DUR);
    assert_eq!(t.action.route_duration, c::ROUTE_DURATION);
    assert_eq!(t.camera.lockout, c::CAM_LOCKOUT);
    assert_eq!(t.usb.floor, c::USB_FLOOR);
    assert_eq!(t.usb.drag, c::USB_DRAG);
    assert_eq!(t.vacuum.target, c::VAC_TARGET);
    assert_eq!(t.interaction.exit_x, c::EXIT_X);
    assert_eq!(t.rescue.support_min, c::RESCUE_SUPPORT_MIN);
}

#[test]
fn actions_are_projected_from_constants() {
    let a = ghost_lobby().actions;
    let cam = a[&ActionKind::Camera];
    assert_eq!(cam.cost, c::ACT_CAMERA_COST);
    assert_eq!(cam.cooldown, c::ACT_CAMERA_CD);
    assert_eq!(cam.alert_gain, c::ACT_CAMERA_ALERT);

    let lights = a[&ActionKind::Lights];
    assert_eq!(lights.cost, c::ACT_LIGHTS_COST);
    assert_eq!(lights.alert_gain, c::ACT_LIGHTS_ALERT);
}

#[test]
fn scoring_and_spawn_are_projected_from_constants() {
    let def = ghost_lobby();
    let s = def.scoring;
    assert_eq!(s.base, c::SCORE_BASE);
    assert_eq!(s.note, c::SCORE_NOTE);
    assert_eq!(s.fail_note, c::FAIL_NOTE);
    assert_eq!(s.grades.s, c::GRADE_S);
    assert_eq!(s.grades.fail_c, c::GRADE_FAIL_C);

    let sp = def.spawn;
    assert_eq!(sp.note_x, (c::SPAWN_NOTE_X_BASE, c::SPAWN_NOTE_X_SPAN));
    assert_eq!(sp.usb_x, (c::SPAWN_USB_X_BASE, c::SPAWN_USB_X_SPAN));
    assert_eq!(
        sp.door_delay,
        (c::SPAWN_DOOR_DELAY_BASE, c::SPAWN_DOOR_DELAY_SPAN)
    );
    assert_eq!(sp.operator_door_penalty, c::OPERATOR_DOOR_PENALTY);
}
