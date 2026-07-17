//! The Exchange House / UMS building runtime, end to end: definition
//! round-trip + validation-error cases, reachability over the building graph,
//! the export-surface parity fixture, the UMS payload seam, and the Ghost
//! Lobby loaded as one floor of the building driving one ending.

use idaptik_core::netsim::{Effect, apply_actuation};
use idaptik_core::scenario::building::exchange_house::{
    EXCHANGE_HOUSE_JSON, EXCHANGE_HOUSE_RUN_JSON, exchange_house, exchange_house_run,
    exchange_house_script,
};
use idaptik_core::scenario::building::graph::{reachable_rooms, shortest_path};
use idaptik_core::scenario::building::sim::{
    BUILDING_SNAPSHOT_FORMAT, BuildingCommand, BuildingDenyReason, BuildingEvent, BuildingExport,
    BuildingSim,
};
use idaptik_core::scenario::building::{
    BUILDING_FORMAT, BuildingDefinition, BuildingPackError, BuildingValidationError, FloorKind,
    PortalKind, RoomRef, load_building,
};
use idaptik_core::scenario::command::Command;
use idaptik_core::scenario::common::{ExtractMethod, Outcome};
use idaptik_core::scenario::{GhostLobbySim, RunConfig, fold};

fn lobby() -> RoomRef {
    RoomRef::new("exchange", "lobby")
}

// --- definition: goldens + round-trip ---------------------------------------

#[test]
fn the_committed_golden_matches_the_authored_building() {
    let authored = exchange_house();
    let parsed: BuildingDefinition =
        serde_json::from_str(EXCHANGE_HOUSE_JSON).expect("the golden parses");
    assert_eq!(authored, parsed);
}

#[test]
fn the_committed_golden_is_serde_round_trip_stable() {
    let def: BuildingDefinition = serde_json::from_str(EXCHANGE_HOUSE_JSON).expect("parses");
    let round = serde_json::to_string_pretty(&def).expect("serialize");
    assert_eq!(round.trim(), EXCHANGE_HOUSE_JSON.trim());
}

#[test]
fn the_exchange_house_validates_clean() {
    let report = exchange_house().validate();
    assert!(
        report.passed(),
        "failed checks: {:?}",
        report
            .checks
            .iter()
            .filter(|c| !c.passed)
            .collect::<Vec<_>>()
    );
}

#[test]
fn the_exchange_house_is_genuinely_multi_floor() {
    let def = exchange_house();
    assert!(def.floors.len() >= 4, "four floors incl. service layer");
    assert_eq!(
        def.floors
            .iter()
            .filter(|f| f.kind == FloorKind::Billy)
            .count(),
        1,
        "exactly one Billy floor"
    );
    for kind in [
        PortalKind::Door,
        PortalKind::Stair,
        PortalKind::Lift,
        PortalKind::Ladder,
        PortalKind::Vent,
    ] {
        assert!(
            def.portals.iter().any(|p| p.kind == kind),
            "no {kind:?} portal"
        );
    }
    assert!(
        def.portals
            .iter()
            .any(|p| p.lock.is_some() && p.circuit.is_some()),
        "a locked portal with a powered controller"
    );
    assert!(def.circuits.len() >= 2, "at least two power circuits");
    assert!(def.zones.len() >= 2, "at least two network zones");
}

// --- validation-error cases ---------------------------------------------------

#[test]
fn a_wrong_format_tag_is_a_typed_error() {
    let mut def = exchange_house();
    def.format = "idaptik-scenario/999".to_owned();
    let errs = def.ok().expect_err("must fail");
    assert!(errs.iter().any(|e| matches!(
        e,
        BuildingValidationError::UnknownFormat { found } if found == "idaptik-scenario/999"
    )));
}

#[test]
fn a_dangling_portal_endpoint_is_a_typed_error() {
    let mut def = exchange_house();
    if let Some(p) = def.portals.first_mut() {
        p.to = RoomRef::new("exchange", "vault-that-does-not-exist");
    }
    let errs = def.ok().expect_err("must fail");
    // The bogus endpoint also strands nothing (lobby<->hall has other routes),
    // but the endpoint check itself must name the portal.
    assert!(errs.iter().any(|e| matches!(
        e,
        BuildingValidationError::PortalEndpointMissing { portal, .. } if portal.as_str() == "p-front"
    )));
}

#[test]
fn a_negative_travel_time_is_a_typed_error() {
    let mut def = exchange_house();
    if let Some(p) = def.portals.first_mut() {
        p.travel_time = -1.0;
    }
    let errs = def.ok().expect_err("must fail");
    assert!(errs.iter().any(|e| matches!(
        e,
        BuildingValidationError::PortalTravelTimeInvalid(p) if p.as_str() == "p-front"
    )));
}

#[test]
fn a_duplicate_portal_id_is_a_typed_error() {
    let mut def = exchange_house();
    let dup = def.portals[0].clone();
    def.portals.push(dup);
    let errs = def.ok().expect_err("must fail");
    assert!(errs.iter().any(|e| matches!(
        e,
        BuildingValidationError::DuplicatePortalId(p) if p.as_str() == "p-front"
    )));
}

#[test]
fn a_missing_billy_floor_is_a_typed_error() {
    let mut def = exchange_house();
    for f in &mut def.floors {
        if f.kind == FloorKind::Billy {
            f.kind = FloorKind::Standard;
        }
    }
    let errs = def.ok().expect_err("must fail");
    assert!(
        errs.iter()
            .any(|e| matches!(e, BuildingValidationError::NoBillyFloor))
    );
}

#[test]
fn a_stranded_room_is_a_typed_error() {
    let mut def = exchange_house();
    // Cut every way into the service layer.
    def.portals
        .retain(|p| p.from.floor.as_str() != "service" && p.to.floor.as_str() != "service");
    let errs = def.ok().expect_err("must fail");
    assert!(errs.iter().any(|e| matches!(
        e,
        BuildingValidationError::RoomUnreachable(r) if r.floor.as_str() == "service"
    )));
}

#[test]
fn a_room_in_two_zones_is_a_typed_error() {
    let mut def = exchange_house();
    if let Some(z) = def.zones.iter_mut().find(|z| z.id.as_str() == "ot") {
        z.rooms.push(lobby());
    }
    let errs = def.ok().expect_err("must fail");
    assert!(errs.iter().any(|e| matches!(
        e,
        BuildingValidationError::RoomInMultipleZones(r) if *r == lobby()
    )));
}

#[test]
fn a_drifted_scenario_floor_is_a_typed_error() {
    let mut def = exchange_house();
    if let Some(f) = def.floors.iter_mut().find(|f| f.scenario.is_some()) {
        f.rooms.remove(0);
    }
    let errs = def.ok().expect_err("must fail");
    assert!(errs.iter().any(|e| matches!(
        e,
        BuildingValidationError::ScenarioRoomsMismatch { floor } if floor.as_str() == "ghost-lobby"
    )));
}

// --- the UMS payload seam ----------------------------------------------------

#[test]
fn load_building_accepts_the_committed_payload() {
    let def = load_building(EXCHANGE_HOUSE_JSON).expect("the golden loads");
    assert_eq!(def.format, BUILDING_FORMAT);
    assert_eq!(def.building_id, "exchange-house");
}

#[test]
fn load_building_gates_on_the_payload_format() {
    let hostile = EXCHANGE_HOUSE_JSON.replacen("idaptik-scenario/1", "idaptik-scenario/2", 1);
    let err = load_building(&hostile).expect_err("wrong format must be refused");
    assert!(matches!(err, BuildingPackError::Invalid(_)));
}

#[test]
fn load_building_refuses_non_building_json() {
    assert!(matches!(
        load_building("{\"format\": 3}"),
        Err(BuildingPackError::Parse(_))
    ));
}

// --- reachability over the building graph -------------------------------------

#[test]
fn every_room_is_reachable_ignoring_locks_and_power() {
    let def = exchange_house();
    let reach = reachable_rooms(&def, &lobby(), |_| true);
    assert_eq!(reach.len(), def.all_rooms().len());
}

#[test]
fn locks_and_power_prune_the_reachable_set() {
    let def = exchange_house();
    let sim = BuildingSim::new(exchange_house()).expect("valid");
    // As the run starts: locked portals and unpowered portals are impassable.
    let live = |p: &idaptik_core::scenario::building::PortalDef| {
        !sim.is_locked(&p.id) && p.circuit.as_ref().is_none_or(|c| sim.is_powered(c))
    };
    let reach = reachable_rooms(&def, &lobby(), live);
    // The teller row's gate is locked and its vent grille is locked, so it is
    // out of reach until a controller is actuated.
    assert!(!reach.contains(&RoomRef::new("exchange", "tellers")));
    // But the Ghost Lobby floor is reachable over the stairs.
    assert!(reach.contains(&RoomRef::new("ghost-lobby", "exit")));
}

#[test]
fn shortest_path_prefers_the_stairs_when_the_lifts_are_dead() {
    let def = exchange_house();
    // With everything passable the lift run wins nothing over the stairs from
    // the lobby (2+6 via hall = 8 == lift 8): ties break deterministically.
    let all = shortest_path(
        &def,
        &lobby(),
        &RoomRef::new("ghost-lobby", "kitchen"),
        |_| true,
    )
    .expect("route exists");
    assert!((all.travel_time - 14.0).abs() < 1e-9, "{}", all.travel_time);
    // With the main circuit dead, the lifts are impassable and the stairs are
    // the only way up; the cost is unchanged (8 to the landing, 6 more up).
    let no_lifts = shortest_path(
        &def,
        &lobby(),
        &RoomRef::new("ghost-lobby", "kitchen"),
        |p| p.kind != PortalKind::Lift,
    )
    .expect("route exists");
    assert_eq!(
        no_lifts
            .portals
            .iter()
            .map(|p| p.as_str())
            .collect::<Vec<_>>(),
        vec!["p-front", "p-stair-1", "p-stair-2"],
    );
    assert!((no_lifts.travel_time - 14.0).abs() < 1e-9);
    // Unreachable is None, not a panic.
    assert!(
        shortest_path(&def, &lobby(), &RoomRef::new("exchange", "tellers"), |p| p
            .lock
            .is_none())
        .is_none()
    );
}

// --- the runtime + export-surface parity fixture --------------------------------

#[test]
fn the_scripted_run_replays_the_committed_parity_fixture() {
    let sim = exchange_house_run().expect("the committed building validates");
    let export = sim.export();
    let committed: BuildingExport =
        serde_json::from_str(EXCHANGE_HOUSE_RUN_JSON).expect("the fixture parses");
    assert_eq!(export, committed);
}

#[test]
fn the_parity_fixture_is_serde_round_trip_stable() {
    let export: BuildingExport = serde_json::from_str(EXCHANGE_HOUSE_RUN_JSON).expect("parses");
    let round = serde_json::to_string_pretty(&export).expect("serialize");
    assert_eq!(round.trim(), EXCHANGE_HOUSE_RUN_JSON.trim());
}

#[test]
fn the_run_is_deterministic() {
    let a = exchange_house_run().expect("valid").export();
    let b = exchange_house_run().expect("valid").export();
    assert_eq!(a, b);
}

#[test]
fn the_script_hits_the_lock_the_controller_and_the_dead_lift() {
    let mut sim = BuildingSim::new(exchange_house()).expect("valid");
    let mut log = Vec::new();
    for cmd in exchange_house_script() {
        log.extend(sim.apply(&cmd));
    }
    assert!(log.iter().any(|e| matches!(
        e,
        BuildingEvent::TraversalDenied {
            reason: BuildingDenyReason::Locked,
            ..
        }
    )));
    assert!(log.iter().any(|e| matches!(
        e,
        BuildingEvent::Unlocked { controller, .. } if controller == "lock-tellers"
    )));
    assert!(log.iter().any(|e| matches!(
        e,
        BuildingEvent::PowerCut { circuit } if circuit.as_str() == "main"
    )));
    assert!(log.iter().any(|e| matches!(
        e,
        BuildingEvent::TraversalDenied {
            reason: BuildingDenyReason::Unpowered,
            ..
        }
    )));
    assert_eq!(sim.state().at, RoomRef::new("ghost-lobby", "kitchen"));
}

#[test]
fn a_dead_controller_cannot_unlock_its_portal() {
    let mut sim = BuildingSim::new(exchange_house()).expect("valid");
    // Cut the main riser first: the teller gate's controller draws from it.
    sim.apply(&BuildingCommand::Actuate("feed-main".to_owned()));
    let ev = sim.apply(&BuildingCommand::Actuate("lock-tellers".to_owned()));
    assert!(matches!(&ev[..], [BuildingEvent::ActuationFailed { .. }]));
    assert!(sim.is_locked(&"p-tellers".into()));
}

#[test]
fn snapshots_restore_exactly_and_gate_on_format() {
    let sim = exchange_house_run().expect("valid");
    let snap = sim.snapshot();
    assert_eq!(snap.format, BUILDING_SNAPSHOT_FORMAT);
    let restored = BuildingSim::restore(snap.clone()).expect("restores");
    assert_eq!(restored.state(), sim.state());
    assert_eq!(restored.snapshot(), snap);

    let mut stale = snap;
    stale.format = "idaptik-building-runtime-v0".to_owned();
    let errs = BuildingSim::restore(stale).expect_err("stale format refused");
    assert!(errs.iter().any(|e| matches!(
        e,
        BuildingValidationError::UnsupportedSnapshotFormat { found } if found == "idaptik-building-runtime-v0"
    )));
}

// --- the grounded network reuses netsim ----------------------------------------

#[test]
fn zones_map_onto_netsim_segments_and_power_cascades() {
    let sim = BuildingSim::new(exchange_house()).expect("valid");
    let net = sim.network();
    // One segment per zone, carrying the zone's subnet.
    for zone in &sim.definition().zones {
        let seg = net.segment(zone.id.as_str()).expect("zone segment exists");
        assert_eq!(seg.subnet, zone.subnet);
    }
    // Cutting the main feed takes the teller gate's controller with it —
    // straight through netsim's dependency cascade, no bespoke graph code.
    let effects = apply_actuation(net, "feed-main");
    assert!(effects.contains(&Effect::PowerCut("feed-main".into())));
    assert!(effects.contains(&Effect::DevicePowerLost("lock-tellers".into())));
}

// --- Ghost Lobby as one floor ----------------------------------------------------

#[test]
fn the_ghost_lobby_loads_as_the_billy_floor_and_drives_one_ending() {
    let def = exchange_house();
    let (floor, scenario) = def.scenario_floor().expect("a scenario floor exists");
    assert_eq!(floor.kind, FloorKind::Billy);
    assert_eq!(floor.id.as_str(), "ghost-lobby");
    // The embedded definition is the canonical one, bit for bit.
    assert_eq!(scenario, &idaptik_core::scenario::ghost_lobby());

    // The scenario sim's public API is untouched: construct it from the
    // floor's content exactly as from the standalone definition, and drive a
    // service-exit ending.
    let mut sim = GhostLobbySim::new(scenario.clone(), RunConfig::standard(), 123456)
        .expect("the embedded scenario validates");
    sim.drain_events();
    let mut held = Default::default();
    let input = fold(
        &[Command::ForceExtract {
            method: ExtractMethod::ServiceExit,
        }],
        &mut held,
    );
    sim.tick(&input);
    assert!(sim.is_ended());
    let debrief = sim.debrief().expect("debrief present");
    assert!(debrief.success);
    assert_eq!(debrief.reason, Outcome::Extracted);
}

#[test]
fn the_scenario_doors_project_into_the_floor_portals() {
    let def = exchange_house();
    // All four Ghost Lobby doors sit on interior boundaries, so all four
    // project; the floor's five rooms are threaded kitchen->...->exit.
    let gl: Vec<_> = def
        .portals
        .iter()
        .filter(|p| p.from.floor.as_str() == "ghost-lobby" && p.kind == PortalKind::Door)
        .collect();
    assert_eq!(gl.len(), 4);
    assert!(gl.iter().all(|p| p.to.floor.as_str() == "ghost-lobby"));
    let last = def.portal(&"ghost-lobby-d4".into()).expect("D4 projects");
    assert_eq!(last.from, RoomRef::new("ghost-lobby", "laundry"));
    assert_eq!(last.to, RoomRef::new("ghost-lobby", "exit"));
}
