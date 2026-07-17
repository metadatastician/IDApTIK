//! Moletaire companion — the archive test oracle, ported to Rust.
//!
//! Every assertion from the archive's five `__tests__/Moletaire*_test.res`
//! suites that maps onto the ported surface, plus behavioural fidelity tests,
//! a determinism test (same seed + command script → identical event log) and
//! a snapshot round-trip test.
//!
//! Not ported: the localStorage key-constant tests (`moletaire-unlocked` etc.)
//! — persistence here is the serde snapshot, not localStorage.

use idaptik_core::MoleEvent;
use idaptik_core::companion::{
    ALL_COPROCESSOR_TYPES, ALL_EQUIPMENT, CompanionValidationError, CoprocessorBay,
    CoprocessorType, EdibleObject, Equipment, Facing, HungerBehaviour, Level,
    MOLETAIRE_SNAPSHOT_FORMAT, MoleCommand, MoleParams, MoleRuntimeState, MoleState, MoletaireSim,
    MoletaireSnapshot, VibrationReading, hunger, moletaire, music,
};
use idaptik_core::scenario::rng::Mulberry32;

const DT: f64 = 1.0 / 60.0;

fn nearly_equal(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() < eps
}

fn make_mole() -> MoletaireSim {
    MoletaireSim::new(moletaire(), MoleParams::default(), 1).expect("valid definition")
}

fn make_mole_with(equipped: Option<Equipment>) -> MoletaireSim {
    MoletaireSim::new(
        moletaire(),
        MoleParams {
            equipped,
            ..MoleParams::default()
        },
        1,
    )
    .expect("valid definition")
}

// ===========================================================================
// Tuning constants (Moletaire_test.res + spec anchors)
// ===========================================================================

#[test]
fn tuning_constants_match_archive() {
    let t = moletaire().tuning;
    assert_eq!(t.underground_speed, 200.0);
    assert_eq!(t.above_ground_speed, 25.0);
    assert_eq!(t.skateboard_speed, 55.0);
    assert_eq!(t.trap_dig_duration_sec, 4.0);
    assert_eq!(t.cable_sabotage_duration_sec, 3.0);
    assert_eq!(t.distraction_duration_sec, 5.0);
    assert_eq!(t.item_eat_chance, 0.05);
    assert_eq!(t.base_carry_capacity, 1);
    assert_eq!(t.rucksack_carry_capacity, 3);
    assert_eq!(t.hunger_rate, 0.015);
    assert_eq!(t.dog_detection_depth, 0.3);
    assert_eq!(t.surface_threshold, 0.05);
    assert_eq!(t.hunger_fight_interval, 3.0);
    assert_eq!(t.hunger_fight_duration, 1.5);
    assert_eq!(t.hungry_threshold, 0.4);
    assert_eq!(t.starving_threshold, 0.8);
    assert_eq!(t.max_depth, 1.0);
    assert_eq!(t.dodge_window_sec, 0.5);
    assert_eq!(t.flash_stun_duration_sec, 2.5);
    assert_eq!(t.glider_height_multiplier, 3.0);
    assert_eq!(t.glider_fall_speed, 60.0);
    assert_eq!(t.hunger_eat_speed, 120.0);
    assert_eq!(t.training_hunger_rate, 0.035);
}

// ===========================================================================
// Equipment metadata (Moletaire_test.res)
// ===========================================================================

#[test]
fn equipment_name_skateboard() {
    assert_eq!(Equipment::Skateboard.name(), "SKATEBOARD");
}

#[test]
fn equipment_name_plasma_cutter() {
    assert_eq!(Equipment::PlasmaCutter.name(), "PLASMA CUTTER");
}

#[test]
fn equipment_descriptions_all_non_empty() {
    assert!(ALL_EQUIPMENT.iter().all(|eq| !eq.description().is_empty()));
}

#[test]
fn all_equipment_count_is_7() {
    assert_eq!(ALL_EQUIPMENT.len(), 7);
    assert_eq!(moletaire().equipment.len(), 7);
}

// ===========================================================================
// Carry capacity — base vs rucksack (Moletaire_test.res)
// ===========================================================================

#[test]
fn carry_capacity_base_is_1() {
    assert_eq!(make_mole().carry_capacity(), 1);
}

#[test]
fn carry_capacity_rucksack_is_3() {
    assert_eq!(
        make_mole_with(Some(Equipment::Rucksack)).carry_capacity(),
        3
    );
}

// ===========================================================================
// giveItem / dropItem (Moletaire_test.res)
// ===========================================================================

#[test]
fn give_item_success() {
    let mut mole = make_mole();
    assert!(mole.give_item("item-1"));
    assert_eq!(mole.state().carried_items.len(), 1);
    assert_eq!(mole.state().state, MoleState::CarryingItem);
}

#[test]
fn give_item_capacity_full_rejected() {
    let mut mole = make_mole();
    let _ = mole.give_item("item-1");
    assert!(!mole.give_item("item-2"));
    assert_eq!(mole.state().carried_items.len(), 1);
}

#[test]
fn give_item_rucksack_allows_3() {
    let mut mole = make_mole_with(Some(Equipment::Rucksack));
    let _ = mole.give_item("item-1");
    let _ = mole.give_item("item-2");
    assert!(mole.give_item("item-3"));
    assert_eq!(mole.state().carried_items.len(), 3);
}

#[test]
fn drop_item_success() {
    let mut mole = make_mole();
    let _ = mole.give_item("item-1");
    assert!(mole.drop_item("item-1"));
    assert_eq!(mole.state().carried_items.len(), 0);
    assert_eq!(mole.state().state, MoleState::Idle);
}

#[test]
fn drop_item_not_carried_is_false() {
    let mut mole = make_mole();
    assert!(!mole.drop_item("nonexistent"));
}

// ===========================================================================
// Event system — emit / drain (Moletaire_test.res)
// ===========================================================================

#[test]
fn drain_events_empty_initially() {
    let mut mole = make_mole();
    assert!(mole.drain_events().is_empty());
}

#[test]
fn emit_and_drain_collects_and_clears() {
    let mut mole = make_mole();
    mole.emit(MoleEvent::FlashFired);
    mole.emit(MoleEvent::FoodEaten);
    let events = mole.drain_events();
    let events_after = mole.drain_events();
    assert_eq!(events.len(), 2);
    assert!(events_after.is_empty());
}

// ===========================================================================
// orderMoveTo (Moletaire_test.res)
// ===========================================================================

#[test]
fn order_move_to_underground() {
    let mut mole = make_mole();
    mole.order_move_to(200.0, true);
    assert_eq!(mole.state().state, MoleState::MovingUnderground);
    assert_eq!(mole.state().target_x, Some(200.0));
    // Auto-dive from the surface.
    assert_eq!(mole.state().target_depth, Some(0.5));
}

#[test]
fn order_move_to_above_ground() {
    let mut mole = make_mole();
    mole.order_move_to(100.0, false);
    assert_eq!(mole.state().state, MoleState::MovingAboveGround);
}

#[test]
fn order_move_to_dead_mole_ignored() {
    let mut mole = make_mole();
    mole.state_mut().alive = false;
    mole.state_mut().state = MoleState::Dead;
    mole.order_move_to(100.0, false);
    assert_eq!(mole.state().state, MoleState::Dead);
}

// ===========================================================================
// orderDigTrap (Moletaire_test.res)
// ===========================================================================

#[test]
fn order_dig_trap_underground() {
    let mut mole = make_mole();
    mole.state_mut().depth = 0.5;
    mole.order_dig_trap();
    assert_eq!(mole.state().state, MoleState::DiggingTrap);
    assert_eq!(mole.state().action_timer, 4.0);
}

#[test]
fn order_dig_trap_surface_rejected() {
    let mut mole = make_mole();
    mole.state_mut().depth = 0.0;
    mole.order_dig_trap();
    assert_eq!(mole.state().state, MoleState::Idle);
}

// ===========================================================================
// equip / unequip (Moletaire_test.res)
// ===========================================================================

#[test]
fn equip_sets_equipment() {
    let mut mole = make_mole();
    mole.equip(Equipment::Glider);
    assert_eq!(mole.state().equipped, Some(Equipment::Glider));
}

#[test]
fn equip_replaces_existing() {
    let mut mole = make_mole_with(Some(Equipment::Skateboard));
    mole.equip(Equipment::Beacon);
    assert_eq!(mole.state().equipped, Some(Equipment::Beacon));
}

#[test]
fn unequip_rucksack_sheds_excess_items() {
    let mut mole = make_mole_with(Some(Equipment::Rucksack));
    let _ = mole.give_item("a");
    let _ = mole.give_item("b");
    let _ = mole.give_item("c");
    mole.unequip();
    assert_eq!(mole.state().equipped, None);
    assert_eq!(mole.state().carried_items, vec!["a".to_owned()]);
}

// ===========================================================================
// Construction defaults (Moletaire_test.res)
// ===========================================================================

#[test]
fn make_correct_defaults() {
    let mole = MoletaireSim::new(
        moletaire(),
        MoleParams {
            id: "test-mole".to_owned(),
            x: 50.0,
            y: 100.0,
            ..MoleParams::default()
        },
        1,
    )
    .expect("valid definition");
    let s = mole.state();
    assert_eq!(s.id, "test-mole");
    assert_eq!(s.x, 50.0);
    assert_eq!(s.y, 100.0);
    assert_eq!(s.state, MoleState::Idle);
    assert!(s.alive);
    assert_eq!(s.hunger, 0.0);
    assert_eq!(s.depth, 0.0);
    assert_eq!(s.equipped, None);
    assert_eq!(s.facing, Facing::Right);
    assert!(s.carried_items.is_empty());
    assert_eq!(s.coprocessors, CoprocessorBay::new());
}

// ===========================================================================
// Hunger thresholds and configs (MoletaireHunger_test.res)
// ===========================================================================

#[test]
fn hunger_thresholds() {
    assert_eq!(hunger::PECKISH_THRESHOLD, 0.4);
    assert_eq!(hunger::HUNGRY_THRESHOLD, 0.4);
    assert_eq!(hunger::STARVING_THRESHOLD, 0.8);
    assert_eq!(hunger::OBJECTIVE_EAT_THRESHOLD, 0.9);
    // Mirrored into the definition data.
    let h = moletaire().hunger;
    assert_eq!(h.peckish_threshold, 0.4);
    assert_eq!(h.hungry_threshold, 0.4);
    assert_eq!(h.starving_threshold, 0.8);
    assert_eq!(h.objective_eat_threshold, 0.9);
}

#[test]
fn hunger_default_config() {
    let c = hunger::DEFAULT_CONFIG;
    assert_eq!(c.starting_hunger, 0.0);
    assert_eq!(c.level_hunger_multiplier, 1.0);
    assert!(!c.ravenous_mode);
}

#[test]
fn hunger_ravenous_config() {
    let c = hunger::RAVENOUS_CONFIG;
    assert_eq!(c.starting_hunger, 0.7);
    assert!(c.ravenous_mode);
}

#[test]
fn hunger_hard_config() {
    let c = hunger::HARD_CONFIG;
    assert_eq!(c.starting_hunger, 0.3);
    assert_eq!(c.level_hunger_multiplier, 1.5);
    assert!(!c.ravenous_mode);
}

// ===========================================================================
// calculatePull (MoletaireHunger_test.res)
// ===========================================================================

#[test]
fn pull_zero_hunger_near_zero() {
    let pull = hunger::calculate_pull(0.0, 0.0, 100.0, 0.0, 0.0);
    assert!(pull.abs() < 1.0);
}

#[test]
fn pull_high_hunger_stronger() {
    let low = hunger::calculate_pull(0.0, 0.0, 100.0, 0.0, 0.2);
    let high = hunger::calculate_pull(0.0, 0.0, 100.0, 0.0, 0.9);
    assert!(high.abs() > low.abs());
}

#[test]
fn pull_direction_right() {
    assert!(hunger::calculate_pull(0.0, 0.0, 100.0, 0.0, 0.5) > 0.0);
}

#[test]
fn pull_direction_left() {
    assert!(hunger::calculate_pull(100.0, 0.0, 0.0, 0.0, 0.5) < 0.0);
}

#[test]
fn pull_capped_at_200() {
    let pull = hunger::calculate_pull(0.0, 0.0, 1.0, 0.0, 1.0);
    assert!(pull.abs() <= 200.0);
}

// ===========================================================================
// calculateTotalPull (MoletaireHunger_test.res)
// ===========================================================================

fn wire(id: &str, x: f64, consumed: bool) -> EdibleObject {
    EdibleObject {
        id: id.to_owned(),
        x,
        y: 0.0,
        is_mission_objective: false,
        nutrition_value: 0.3,
        consumed,
    }
}

#[test]
fn total_pull_no_edibles() {
    let (force, nearest_id, _) = hunger::calculate_total_pull(0.0, 0.0, 0.5, &[]);
    assert_eq!(force, 0.0);
    assert_eq!(nearest_id, None);
}

#[test]
fn total_pull_one_edible() {
    let edibles = [wire("wire-1", 100.0, false)];
    let (force, nearest_id, _) = hunger::calculate_total_pull(0.0, 0.0, 0.5, &edibles);
    assert!(force > 0.0);
    assert_eq!(nearest_id, Some("wire-1".to_owned()));
}

#[test]
fn total_pull_consumed_ignored() {
    let edibles = [wire("wire-1", 100.0, true)];
    let (force, nearest_id, _) = hunger::calculate_total_pull(0.0, 0.0, 0.5, &edibles);
    assert_eq!(force, 0.0);
    assert_eq!(nearest_id, None);
}

// ===========================================================================
// calculateHungerRate (MoletaireHunger_test.res)
// ===========================================================================

#[test]
fn hunger_rate_stationary() {
    let rate = hunger::calculate_hunger_rate(0.01, false, 0.0, 1.0, 1.0);
    // 0.01 * 0.5 (stationary) * 1.0 * 1.0 * 1.0 = 0.005
    assert!(nearly_equal(rate, 0.005, 1e-12));
}

#[test]
fn hunger_rate_moving() {
    let rate = hunger::calculate_hunger_rate(0.01, true, 0.0, 1.0, 1.0);
    // 0.01 * 1.5 (moving) * 1.0 * 1.0 * 1.0 = 0.015
    assert!(nearly_equal(rate, 0.015, 1e-12));
}

#[test]
fn hunger_rate_accelerates_with_hunger() {
    let low = hunger::calculate_hunger_rate(0.01, true, 0.0, 1.0, 1.0);
    let high = hunger::calculate_hunger_rate(0.01, true, 0.8, 1.0, 1.0);
    assert!(high > low);
}

// ===========================================================================
// getBehaviour (MoletaireHunger_test.res)
// ===========================================================================

#[test]
fn behaviour_bands() {
    assert_eq!(hunger::behaviour(0.2, true), HungerBehaviour::Cooperative);
    assert_eq!(hunger::behaviour(0.6, true), HungerBehaviour::Resisting);
    assert_eq!(hunger::behaviour(0.9, true), HungerBehaviour::Devouring);
    assert_eq!(hunger::behaviour(0.9, false), HungerBehaviour::Wandering);
}

// ===========================================================================
// hungerDisplayString / hungerColor (MoletaireHunger_test.res)
// ===========================================================================

#[test]
fn hunger_display_strings() {
    assert_eq!(hunger::display_string(0.05), "FULL");
    assert_eq!(hunger::display_string(0.85), "STARVING");
    assert_eq!(hunger::display_string(0.95), "RAVENOUS");
}

#[test]
fn hunger_colors() {
    assert_eq!(hunger::color(0.1), 0x0044_ff44);
    assert_eq!(hunger::color(0.9), 0x00ff_4444);
}

// ===========================================================================
// Mission objective risk (MoletaireHunger_test.res)
// ===========================================================================

#[test]
fn will_eat_objective_bands() {
    assert!(!hunger::will_eat_objective(0.85));
    assert!(hunger::will_eat_objective(0.95));
}

#[test]
fn objective_at_risk_bands() {
    assert!(!hunger::objective_at_risk(0.5));
    assert!(hunger::objective_at_risk(0.85));
}

// ===========================================================================
// Coprocessor levels (MoletaireCoprocessors_test.res)
// ===========================================================================

#[test]
fn level_values() {
    assert_eq!(Level::Stock.value(), 0);
    assert_eq!(Level::Overclocked.value(), 3);
}

#[test]
fn level_display_names() {
    assert_eq!(Level::Stock.display_name(), "STOCK");
    assert_eq!(Level::Basic.display_name(), "MK-I");
    assert_eq!(Level::Enhanced.display_name(), "MK-II");
    assert_eq!(Level::Overclocked.display_name(), "MK-III");
}

#[test]
fn coprocessor_names_non_empty() {
    assert!(ALL_COPROCESSOR_TYPES.iter().all(|ct| !ct.name().is_empty()));
}

#[test]
fn all_types_count_is_5() {
    assert_eq!(ALL_COPROCESSOR_TYPES.len(), 5);
}

#[test]
fn make_bay_all_stock() {
    let bay = CoprocessorBay::new();
    assert!(
        ALL_COPROCESSOR_TYPES
            .iter()
            .all(|&ct| bay.level(ct) == Level::Stock)
    );
}

#[test]
fn upgrade_stock_to_basic() {
    let mut bay = CoprocessorBay::new();
    assert!(bay.upgrade(CoprocessorType::AudioSynthesiser));
    assert_eq!(bay.level(CoprocessorType::AudioSynthesiser), Level::Basic);
}

#[test]
fn upgrade_to_enhanced() {
    let mut bay = CoprocessorBay::new();
    let _ = bay.upgrade(CoprocessorType::PathOptimiser);
    assert!(bay.upgrade(CoprocessorType::PathOptimiser));
    assert_eq!(bay.level(CoprocessorType::PathOptimiser), Level::Enhanced);
}

#[test]
fn upgrade_overclocked_cap() {
    let mut bay = CoprocessorBay::new();
    let _ = bay.upgrade(CoprocessorType::AudioSynthesiser);
    let _ = bay.upgrade(CoprocessorType::AudioSynthesiser);
    let _ = bay.upgrade(CoprocessorType::AudioSynthesiser);
    assert!(!bay.upgrade(CoprocessorType::AudioSynthesiser));
    assert_eq!(
        bay.level(CoprocessorType::AudioSynthesiser),
        Level::Overclocked
    );
}

// ===========================================================================
// Coprocessor effect ladders (MoletaireCoprocessors_test.res)
// ===========================================================================

#[test]
fn audio_sound_count_ladder() {
    let l = moletaire().coprocessors;
    assert_eq!(l.audio_sound_count(Level::Stock), 0);
    assert_eq!(l.audio_sound_count(Level::Basic), 3);
    assert_eq!(l.audio_sound_count(Level::Enhanced), 6);
    assert_eq!(l.audio_sound_count(Level::Overclocked), 10);
}

#[test]
fn can_mimic_voice_only_overclocked() {
    let mut bay = CoprocessorBay::new();
    assert!(!bay.can_mimic_voice());
    let _ = bay.upgrade(CoprocessorType::AudioSynthesiser);
    let _ = bay.upgrade(CoprocessorType::AudioSynthesiser);
    assert!(!bay.can_mimic_voice());
    let _ = bay.upgrade(CoprocessorType::AudioSynthesiser);
    assert!(bay.can_mimic_voice());
}

#[test]
fn path_efficiency_ladder() {
    let l = moletaire().coprocessors;
    assert!(nearly_equal(l.path_efficiency(Level::Stock), 1.0, 0.001));
    assert!(nearly_equal(l.path_efficiency(Level::Basic), 1.15, 0.001));
    assert!(nearly_equal(
        l.path_efficiency(Level::Enhanced),
        1.30,
        0.001
    ));
    assert!(nearly_equal(
        l.path_efficiency(Level::Overclocked),
        1.5,
        0.001
    ));
}

#[test]
fn sensor_range_ladder() {
    let l = moletaire().coprocessors;
    assert_eq!(l.sensor_range(Level::Stock), 1);
    assert_eq!(l.sensor_range(Level::Basic), 3);
    assert_eq!(l.sensor_range(Level::Enhanced), 5);
    assert_eq!(l.sensor_range(Level::Overclocked), 8);
}

#[test]
fn can_detect_vault_weak_points_only_overclocked() {
    let mut bay = CoprocessorBay::new();
    assert!(!bay.can_detect_vault_weak_points());
    let _ = bay.upgrade(CoprocessorType::SignalProcessor);
    let _ = bay.upgrade(CoprocessorType::SignalProcessor);
    let _ = bay.upgrade(CoprocessorType::SignalProcessor);
    assert!(bay.can_detect_vault_weak_points());
}

#[test]
fn vibration_quality_ladder() {
    let l = moletaire().coprocessors;
    assert_eq!(l.vibration_quality(Level::Stock), VibrationReading::NoData);
    assert_eq!(
        l.vibration_quality(Level::Basic),
        VibrationReading::DirectionOnly
    );
    assert_eq!(
        l.vibration_quality(Level::Enhanced),
        VibrationReading::DirectionAndIntent
    );
    assert_eq!(
        l.vibration_quality(Level::Overclocked),
        VibrationReading::FullProfile
    );
}

#[test]
fn eat_chance_multiplier_ladder() {
    let l = moletaire().coprocessors;
    assert!(nearly_equal(
        l.eat_chance_multiplier(Level::Stock),
        1.0,
        0.001
    ));
    assert!(nearly_equal(
        l.eat_chance_multiplier(Level::Basic),
        0.6,
        0.001
    ));
    assert!(nearly_equal(
        l.eat_chance_multiplier(Level::Enhanced),
        0.2,
        0.001
    ));
    assert!(nearly_equal(
        l.eat_chance_multiplier(Level::Overclocked),
        0.05,
        0.001
    ));
}

#[test]
fn can_carry_fragile_at_enhanced_or_better() {
    // The canonical ReScript rule: `level >= Enhanced` (NOT the wasm bridge's
    // deliberately replicated bug).
    let mut bay = CoprocessorBay::new();
    assert!(!bay.can_carry_fragile()); // Stock
    let _ = bay.upgrade(CoprocessorType::StabilisationCore);
    assert!(!bay.can_carry_fragile()); // Basic
    let _ = bay.upgrade(CoprocessorType::StabilisationCore);
    assert!(bay.can_carry_fragile()); // Enhanced
    let _ = bay.upgrade(CoprocessorType::StabilisationCore);
    assert!(bay.can_carry_fragile()); // Overclocked
}

// ===========================================================================
// Music constants and pattern data (MoletaireMusic_test.res)
// ===========================================================================

#[test]
fn music_constants() {
    assert!(nearly_equal(music::BPM, 114.0, 0.001));
    assert_eq!(music::PATTERN_LENGTH, 16);
    assert_eq!(music::SCHEDULER_INTERVAL_MS, 25);
    // 60 / 114 ≈ 0.5263
    assert!(music::seconds_per_beat() > 0.52 && music::seconds_per_beat() < 0.53);
}

#[test]
fn music_pattern_lengths() {
    assert_eq!(music::MELODY_NOTES.len(), 16);
    assert_eq!(music::BASS_NOTES.len(), 16);
}

#[test]
fn music_pattern_notes() {
    assert!(nearly_equal(music::MELODY_NOTES[0], 523.25, 0.001)); // C5
    assert!(nearly_equal(music::MELODY_NOTES[1], 0.0, 0.001)); // rest
    assert!(nearly_equal(music::BASS_NOTES[0], 130.81, 0.001)); // C3
    // The full archive arrays, verbatim.
    assert_eq!(
        music::MELODY_NOTES,
        [
            523.25, 0.0, 659.26, 0.0, 783.99, 698.46, 659.26, 0.0, 523.25, 587.33, 659.26, 523.25,
            440.00, 0.0, 523.25, 0.0,
        ]
    );
    assert_eq!(
        music::BASS_NOTES,
        [
            130.81, 0.0, 0.0, 0.0, 164.81, 0.0, 0.0, 0.0, 130.81, 0.0, 146.83, 0.0, 110.00, 0.0,
            0.0, 0.0,
        ]
    );
}

#[test]
fn music_scheduler_emits_step_zero_pair_then_advances() {
    let mut state = music::SchedulerState::new(0.0);
    // Look-ahead 0.1 s covers step 0 only (a step is ~0.1316 s).
    let notes = music::schedule_notes(&mut state, 0.0);
    assert_eq!(notes.len(), 2); // C5 melody + C3 bass at step 0.
    assert_eq!(notes[0].wave, music::WaveType::Square);
    assert!(nearly_equal(notes[0].freq, 523.25, 0.001));
    assert!(nearly_equal(notes[0].gain, 0.08, 1e-12));
    assert_eq!(notes[1].wave, music::WaveType::Triangle);
    assert!(nearly_equal(notes[1].freq, 130.81, 0.001));
    assert!(nearly_equal(notes[1].gain, 0.12, 1e-12));
    assert_eq!(state.current_step, 1);
    // Steps 1..=3 are melody rests and bass rests: advancing the clock one
    // full step emits nothing.
    let notes = music::schedule_notes(&mut state, music::seconds_per_step());
    assert!(notes.is_empty());
    assert_eq!(state.current_step, 2);
}

// ===========================================================================
// Equipment codec (MoletairePersistence_test.res)
// ===========================================================================

#[test]
fn equipment_to_code() {
    assert_eq!(Equipment::Skateboard.code(), "skateboard");
    assert_eq!(Equipment::Glider.code(), "glider");
    assert_eq!(Equipment::PlasmaCutter.code(), "plasma-cutter");
}

#[test]
fn equipment_from_code() {
    assert_eq!(
        Equipment::from_code("skateboard"),
        Some(Equipment::Skateboard)
    );
}

#[test]
fn equipment_from_code_legacy_miniglider() {
    assert_eq!(Equipment::from_code("miniglider"), Some(Equipment::Glider));
}

#[test]
fn equipment_from_code_legacy_flash_camera() {
    assert_eq!(Equipment::from_code("flash"), Some(Equipment::FlashCamera));
    assert_eq!(Equipment::from_code("camera"), Some(Equipment::FlashCamera));
}

#[test]
fn equipment_from_code_unknown_is_none() {
    assert_eq!(Equipment::from_code("laserbeam"), None);
}

#[test]
fn equipment_from_code_empty_is_none() {
    assert_eq!(Equipment::from_code(""), None);
}

#[test]
fn equipment_codec_round_trips_all() {
    for eq in ALL_EQUIPMENT {
        assert_eq!(Equipment::from_code(eq.code()), Some(eq));
    }
}

#[test]
fn equipment_serde_uses_codec_with_legacy_aliases() {
    // Canonical serialization is the code string.
    assert_eq!(
        serde_json::to_string(&Equipment::FlashCamera).expect("serialize"),
        "\"flash-camera\""
    );
    // Legacy save strings deserialize.
    let legacy: Equipment = serde_json::from_str("\"miniglider\"").expect("legacy alias parses");
    assert_eq!(legacy, Equipment::Glider);
    // Unknown codes are rejected.
    assert!(serde_json::from_str::<Equipment>("\"laserbeam\"").is_err());
}

// ===========================================================================
// Behavioural fidelity — the archive state machine in motion
// ===========================================================================

fn tick_n(mole: &mut MoletaireSim, n: usize, edibles: &[EdibleObject]) -> Vec<MoleEvent> {
    let mut log = Vec::new();
    for _ in 0..n {
        log.extend(mole.tick(DT, edibles));
    }
    log
}

#[test]
fn dig_trap_completes_after_4_seconds() {
    let mut mole = make_mole();
    mole.state_mut().depth = 0.5;
    mole.order_dig_trap();
    let log = tick_n(&mut mole, 241, &[]);
    assert!(
        log.iter()
            .any(|e| matches!(e, MoleEvent::TrapTriggered { .. })),
        "trap should trigger after 4 s: {log:?}"
    );
    assert_eq!(mole.state().state, MoleState::Idle);
}

#[test]
fn sabotage_completes_with_cable_id() {
    let mut mole = make_mole();
    mole.state_mut().depth = 0.5;
    mole.state_mut().x = 40.0;
    mole.order_sabotage_cable();
    assert_eq!(mole.state().action_timer, 3.0);
    let log = tick_n(&mut mole, 181, &[]);
    assert!(
        log.iter()
            .any(|e| matches!(e, MoleEvent::CableSabotaged { cable_id } if cable_id == "cable_40")),
        "cable id should be cable_40: {log:?}"
    );
}

#[test]
fn move_arrives_and_reaches_destination() {
    let mut mole = make_mole();
    mole.order_move_to(10.0, false);
    let log = tick_n(&mut mole, 60, &[]);
    assert!(log.contains(&MoleEvent::ReachedDestination));
    assert_eq!(mole.state().x, 10.0);
    assert_eq!(mole.state().state, MoleState::Idle);
}

#[test]
fn above_ground_speed_is_25_and_skateboard_55() {
    let mut slow = make_mole();
    slow.order_move_to(1000.0, false);
    let _ = tick_n(&mut slow, 60, &[]);
    assert!(nearly_equal(slow.state().x, 25.0, 0.5));

    let mut fast = make_mole_with(Some(Equipment::Skateboard));
    fast.order_move_to(1000.0, false);
    let _ = tick_n(&mut fast, 60, &[]);
    assert!(nearly_equal(fast.state().x, 55.0, 0.5));
}

#[test]
fn underground_speed_is_200_after_dive() {
    let mut mole = make_mole();
    mole.order_move_to(10_000.0, true);
    let _ = tick_n(&mut mole, 60, &[]);
    // The first ~0.625 s dives to 0.5 depth; movement runs the whole second.
    assert!(nearly_equal(mole.state().depth, 0.5, 0.01));
    assert!(nearly_equal(mole.state().x, 200.0, 1.0));
}

#[test]
fn delivery_emits_delivered_or_eaten_by_seeded_roll() {
    // The eat roll is the first RNG draw. Pick one seed whose first draw is
    // >= 0.05 (delivered) and one < 0.05 (eaten): the outcome is then exact.
    let mut delivered_seed = None;
    let mut eaten_seed = None;
    for seed in 0..10_000u32 {
        let roll = Mulberry32::new(seed).next_f64();
        if roll >= 0.05 && delivered_seed.is_none() {
            delivered_seed = Some(seed);
        }
        if roll < 0.05 && eaten_seed.is_none() {
            eaten_seed = Some(seed);
        }
        if delivered_seed.is_some() && eaten_seed.is_some() {
            break;
        }
    }
    let run = |seed: u32| -> Vec<MoleEvent> {
        let mut mole =
            MoletaireSim::new(moletaire(), MoleParams::default(), seed).expect("valid definition");
        let _ = mole.give_item("gem");
        mole.order_deliver(10.0);
        tick_n(&mut mole, 120, &[])
    };
    let seed = delivered_seed.expect("some seed rolls >= 0.05");
    assert!(
        run(seed)
            .iter()
            .any(|e| matches!(e, MoleEvent::ItemDelivered { item_id } if item_id == "gem"))
    );
    let seed = eaten_seed.expect("some seed rolls < 0.05");
    assert!(
        run(seed)
            .iter()
            .any(|e| matches!(e, MoleEvent::ItemEaten { item_id } if item_id == "gem"))
    );
}

#[test]
fn stabilisation_core_overclocked_makes_marginal_roll_a_delivery() {
    // Find a seed whose first draw lands in [0.0025, 0.05): eaten at Stock,
    // delivered at Overclocked (eat chance 0.05 * 0.05 = 0.0025).
    let seed = (0..100_000u32)
        .find(|&s| {
            let roll = Mulberry32::new(s).next_f64();
            (0.0025..0.05).contains(&roll)
        })
        .expect("such a seed exists");
    let run = |upgrade: bool| -> Vec<MoleEvent> {
        let mut mole =
            MoletaireSim::new(moletaire(), MoleParams::default(), seed).expect("valid definition");
        if upgrade {
            for _ in 0..3 {
                let _ = mole
                    .state_mut()
                    .coprocessors
                    .upgrade(CoprocessorType::StabilisationCore);
            }
        }
        let _ = mole.give_item("gem");
        mole.order_deliver(10.0);
        tick_n(&mut mole, 120, &[])
    };
    assert!(
        run(false)
            .iter()
            .any(|e| matches!(e, MoleEvent::ItemEaten { .. }))
    );
    assert!(
        run(true)
            .iter()
            .any(|e| matches!(e, MoleEvent::ItemDelivered { .. }))
    );
}

#[test]
fn glider_requires_equipment_and_completes() {
    let mut bare = make_mole();
    assert!(!bare.launch_glider(100.0));

    let mut mole = make_mole_with(Some(Equipment::Glider));
    assert!(mole.launch_glider(100.0));
    assert_eq!(mole.state().state, MoleState::Gliding);
    assert_eq!(mole.state().glide_distance, 300.0); // height * 3
    let log = tick_n(&mut mole, 121, &[]); // progress 0.5/s → 2 s
    assert!(log.contains(&MoleEvent::GlideStarted { height: 100.0 }));
    assert!(
        log.iter()
            .any(|e| matches!(e, MoleEvent::GlideComplete { .. }))
    );
    assert_eq!(mole.state().state, MoleState::Idle);
    assert_eq!(mole.state().depth, 0.0);
}

#[test]
fn wire_distraction_gates() {
    let mut mole = make_mole();
    assert!(!mole.distract_by_wire(300.0)); // beyond 200 px
    assert!(mole.distract_by_wire(150.0));
    assert_eq!(mole.state().state, MoleState::Distracted);
    assert_eq!(mole.state().distraction_timer, 5.0);

    let mut digging = make_mole();
    digging.state_mut().depth = 0.5;
    digging.order_dig_trap();
    assert!(!digging.distract_by_wire(10.0)); // never while digging
}

#[test]
fn distraction_expires_back_to_idle() {
    let mut mole = make_mole();
    let _ = mole.distract_by_wire(150.0);
    let _ = tick_n(&mut mole, 301, &[]); // 5 s
    assert_eq!(mole.state().state, MoleState::Idle);
    assert_eq!(mole.state().distraction_x, None);
}

#[test]
fn use_flash_requires_flash_camera() {
    assert!(!make_mole().use_flash());
    assert!(make_mole_with(Some(Equipment::FlashCamera)).use_flash());
}

#[test]
fn play_synth_sound_respects_ladder() {
    let mut mole = make_mole();
    assert_eq!(mole.play_synth_sound(0), None); // Stock: no sounds
    let _ = mole
        .state_mut()
        .coprocessors
        .upgrade(CoprocessorType::AudioSynthesiser);
    assert_eq!(mole.play_synth_sound(0), Some("footstep".to_owned()));
    assert_eq!(mole.play_synth_sound(3), None); // Basic: only 3 sounds
    let _ = mole
        .state_mut()
        .coprocessors
        .upgrade(CoprocessorType::AudioSynthesiser);
    let _ = mole
        .state_mut()
        .coprocessors
        .upgrade(CoprocessorType::AudioSynthesiser);
    assert_eq!(mole.play_synth_sound(9), Some("voice_mimicry".to_owned()));
    let log = mole.drain_events();
    assert_eq!(
        log,
        vec![
            MoleEvent::SynthSoundPlayed {
                sound: "footstep".to_owned()
            },
            MoleEvent::SynthSoundPlayed {
                sound: "voice_mimicry".to_owned()
            },
        ]
    );
}

#[test]
fn scans_only_underground() {
    let mut mole = make_mole();
    assert_eq!(mole.scan_ahead(), 0); // surface
    assert_eq!(mole.read_vibrations(), VibrationReading::NoData);
    assert!(!mole.can_detect_vault_weak_points());

    mole.state_mut().depth = 0.5;
    assert_eq!(mole.scan_ahead(), 1); // Stock sensor range
    let _ = mole
        .state_mut()
        .coprocessors
        .upgrade(CoprocessorType::VibrationAnalyser);
    assert_eq!(mole.read_vibrations(), VibrationReading::DirectionOnly);
    for _ in 0..3 {
        let _ = mole
            .state_mut()
            .coprocessors
            .upgrade(CoprocessorType::SignalProcessor);
    }
    assert_eq!(mole.scan_ahead(), 8);
    assert!(mole.can_detect_vault_weak_points());
}

#[test]
fn periodic_scan_emits_vibration_and_scan_events() {
    let mut mole = make_mole();
    mole.state_mut().depth = 0.5;
    let _ = mole
        .state_mut()
        .coprocessors
        .upgrade(CoprocessorType::VibrationAnalyser);
    mole.order_move_to(10_000.0, true);
    let log = tick_n(&mut mole, 60, &[]); // two 0.5 s scan windows
    let vibrations = log
        .iter()
        .filter(|e| {
            matches!(
                e,
                MoleEvent::VibrationDetected {
                    reading: VibrationReading::DirectionOnly
                }
            )
        })
        .count();
    let scans = log
        .iter()
        .filter(|e| matches!(e, MoleEvent::UndergroundScanComplete { objects: 1 }))
        .count();
    assert_eq!(vibrations, 2, "log: {log:?}");
    assert_eq!(scans, 2, "log: {log:?}");
}

#[test]
fn hunger_resistance_episodes_emit_transitions() {
    let mut mole = make_mole();
    mole.state_mut().hunger = 0.6; // hungry band, below starving
    // 3 s until the first episode, 1.5 s episode length.
    let log = tick_n(&mut mole, 200, &[]); // ~3.33 s
    assert!(log.contains(&MoleEvent::HungerResistanceStarted));
    assert!(mole.is_resisting_control());
    let log = tick_n(&mut mole, 90, &[]); // past interval + duration
    assert!(log.contains(&MoleEvent::HungerResistanceEnded));
    assert!(!mole.is_resisting_control());
}

#[test]
fn feed_resets_hunger() {
    let mut mole = make_mole();
    mole.state_mut().hunger = 0.9;
    mole.state_mut().is_resisting_control = true;
    mole.feed();
    assert_eq!(mole.state().hunger, 0.0);
    assert!(!mole.is_resisting_control());
    assert_eq!(mole.drain_events(), vec![MoleEvent::FoodEaten]);
}

#[test]
fn mole_died_event_carries_dead_state_archive_quirk() {
    // The archive reassigns state to Dead before building the event, so
    // MoleDied carries Dead — and only fires when the fatal state is entered
    // while still alive.
    let mut mole = make_mole();
    mole.state_mut().state = MoleState::Crushed; // set directly, alive still true
    let log = tick_n(&mut mole, 1, &[]);
    assert!(log.contains(&MoleEvent::MoleDied {
        state: MoleState::Dead
    }));
    assert!(mole.is_dead());
    assert_eq!(mole.state().state, MoleState::Dead);
}

#[test]
fn crush_command_kills_without_mole_died_event_archive_parity() {
    let mut mole = make_mole();
    mole.crush();
    assert!(mole.is_dead());
    assert_eq!(mole.state().state, MoleState::Crushed);
    let log = tick_n(&mut mole, 10, &[]);
    assert!(
        !log.iter().any(|e| matches!(e, MoleEvent::MoleDied { .. })),
        "crush() clears alive, so the archive's MoleDied arm never runs"
    );
}

#[test]
fn dead_mole_ignores_items_and_glider() {
    let mut mole = make_mole_with(Some(Equipment::Glider));
    mole.catch_by_dog();
    assert!(!mole.give_item("x"));
    assert!(!mole.launch_glider(50.0));
    assert_eq!(mole.play_synth_sound(0), None);
    assert_eq!(mole.scan_ahead(), 0);
}

// ===========================================================================
// View state and hitbox
// ===========================================================================

#[test]
fn state_ordinals_and_labels() {
    let states = [
        (MoleState::Idle, 0, "IDLE"),
        (MoleState::MovingUnderground, 1, "TUNNELLING"),
        (MoleState::MovingAboveGround, 2, "SURFACE"),
        (MoleState::DiggingTrap, 3, "DIGGING TRAP"),
        (MoleState::SabotagingCable, 4, "SABOTAGING"),
        (MoleState::CarryingItem, 5, "CARRYING"),
        (MoleState::Distracted, 6, "DISTRACTED"),
        (MoleState::Gliding, 7, "GLIDING"),
        (MoleState::Crushed, 8, "CRUSHED"),
        (MoleState::CaughtByDog, 9, "CAUGHT"),
        (MoleState::Dead, 10, "DEAD"),
    ];
    for (state, ordinal, label) in states {
        assert_eq!(state.ordinal(), ordinal);
        assert_eq!(state.label(), label);
    }
    assert_eq!(Facing::Left.ordinal(), 0);
    assert_eq!(Facing::Right.ordinal(), 1);
}

#[test]
fn view_state_projection() {
    let mut mole = MoletaireSim::new(
        moletaire(),
        MoleParams {
            x: 50.0,
            y: 100.0,
            ..MoleParams::default()
        },
        1,
    )
    .expect("valid definition");
    mole.state_mut().depth = 0.5;
    mole.state_mut().state = MoleState::MovingUnderground;
    let v = mole.view();
    assert_eq!(v.state_ordinal, 1);
    assert_eq!(v.facing_ordinal, 1); // Right
    assert_eq!(v.x, 50.0);
    assert_eq!(v.visual_y, 100.0 + 0.5 * 60.0); // y + depth * 60
    assert_eq!(v.body_color, 0x005a_3518); // darker underground brown
    assert!(v.underground);
    assert!(v.alive);
}

#[test]
fn body_colors_match_archive_palette() {
    assert_eq!(MoleState::Idle.body_color(), 0x006b_4226);
    assert_eq!(MoleState::CarryingItem.body_color(), 0x006b_4226);
    assert_eq!(MoleState::MovingAboveGround.body_color(), 0x007a_5230);
    assert_eq!(MoleState::DiggingTrap.body_color(), 0x008b_6914);
    assert_eq!(MoleState::SabotagingCable.body_color(), 0x0088_4422);
    assert_eq!(MoleState::Distracted.body_color(), 0x00aa_7744);
    assert_eq!(MoleState::Gliding.body_color(), 0x0066_88aa);
    assert_eq!(MoleState::Crushed.body_color(), 0x0033_3333);
    assert_eq!(MoleState::Dead.body_color(), 0x0033_3333);
}

#[test]
fn body_rect_matches_archive() {
    let mut mole = MoletaireSim::new(
        moletaire(),
        MoleParams {
            x: 100.0,
            y: 50.0,
            ..MoleParams::default()
        },
        1,
    )
    .expect("valid definition");
    mole.state_mut().depth = 0.4;
    let r = mole.body_rect();
    assert_eq!(r.x, 100.0 - 20.0 / 2.0);
    assert_eq!(r.y, 50.0 - 14.0 - 0.4 * 10.0);
    assert_eq!(r.w, 20.0);
    assert_eq!(r.h, 14.0);
}

// ===========================================================================
// Determinism — same seed + command script → identical event log
// ===========================================================================

fn scripted_run(seed: u32) -> (Vec<MoleEvent>, MoleRuntimeState, u64) {
    let mut sim =
        MoletaireSim::new(moletaire(), MoleParams::default(), seed).expect("valid definition");
    // Starving with the only edible far away → a wander RNG draw every tick.
    sim.state_mut().hunger = 0.95;
    let edibles = vec![wire("snack", 900.0, false)];
    let mut log = Vec::new();
    for i in 0..600u32 {
        match i {
            50 => {
                sim.apply(MoleCommand::GiveItem {
                    item_id: "gem".to_owned(),
                });
                let x = sim.state().x;
                sim.apply(MoleCommand::Deliver {
                    delivery_x: x + 20.0,
                });
            }
            300 => sim.apply(MoleCommand::MoveTo {
                target_x: 40.0,
                underground: true,
            }),
            400 => sim.apply(MoleCommand::DigTrap),
            520 => sim.apply(MoleCommand::UseFlash),
            _ => {}
        }
        log.extend(sim.tick(DT, &edibles));
    }
    (log, sim.state().clone(), sim.ticks())
}

#[test]
fn determinism_same_seed_same_script_identical_logs() {
    let (log_a, state_a, ticks_a) = scripted_run(0xC0FFEE);
    let (log_b, state_b, ticks_b) = scripted_run(0xC0FFEE);
    assert!(!log_a.is_empty());
    assert_eq!(log_a, log_b);
    assert_eq!(state_a, state_b);
    assert_eq!(ticks_a, ticks_b);
}

// ===========================================================================
// Snapshot round-trip
// ===========================================================================

#[test]
fn snapshot_round_trips_and_resumes_identically() {
    let mut original =
        MoletaireSim::new(moletaire(), MoleParams::default(), 42).expect("valid definition");
    original.state_mut().hunger = 0.95; // keep the RNG stream hot (wander draws)
    let edibles = [wire("snack", 900.0, false)];
    let _ = tick_n(&mut original, 200, &edibles);

    // Serialize → deserialize → restore.
    let snap = original.snapshot();
    assert_eq!(snap.format, MOLETAIRE_SNAPSHOT_FORMAT);
    assert_eq!(snap.format, "idaptik-moletaire-runtime-v1");
    let json = serde_json::to_string(&snap).expect("snapshot serializes");
    let parsed: MoletaireSnapshot = serde_json::from_str(&json).expect("snapshot parses");
    assert_eq!(parsed, snap);
    let mut restored = MoletaireSim::restore(parsed).expect("snapshot restores");

    // Both copies must continue tick-for-tick identically (RNG resumed
    // mid-sequence).
    assert_eq!(restored.state(), original.state());
    for _ in 0..200 {
        assert_eq!(restored.tick(DT, &edibles), original.tick(DT, &edibles));
    }
    assert_eq!(restored.state(), original.state());
    assert_eq!(restored.ticks(), original.ticks());
}

#[test]
fn snapshot_rejects_wrong_format_tag() {
    let mole = make_mole();
    let mut snap = mole.snapshot();
    snap.format = "idaptik-moletaire-runtime-v0".to_owned();
    let err = MoletaireSim::restore(snap).expect_err("wrong tag must be rejected");
    assert!(matches!(
        err.as_slice(),
        [CompanionValidationError::UnsupportedSnapshotFormat { found }]
            if found == "idaptik-moletaire-runtime-v0"
    ));
}
