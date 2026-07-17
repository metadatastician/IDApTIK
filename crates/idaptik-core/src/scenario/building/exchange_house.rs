//! The canonical Exchange House: a modest but genuinely multi-floor building,
//! authored as data, with the Ghost Lobby scenario embedded as its Billy
//! floor. Two committed goldens pin it: [`EXCHANGE_HOUSE_JSON`] (the
//! definition) and [`EXCHANGE_HOUSE_RUN_JSON`] (the four export surfaces of a
//! canonical scripted run — the structural-parity fixture).

use crate::scenario::building::sim::{BuildingCommand, BuildingSim};
use crate::scenario::building::{
    BUILDING_FORMAT, BuildingDefinition, BuildingRoomDef, CircuitDef, FloorDef, FloorKind,
    PortalDef, PortalKind, PortalLock, RoomRef, ZoneDef, scenario_floor_portals,
};
use crate::scenario::ghost_lobby::ghost_lobby;

/// The stable building id.
pub const BUILDING_ID: &str = "exchange-house";

/// A committed pretty-printed JSON of [`exchange_house`]. Regenerate with the
/// ignored `regenerate_exchange_house_json` test if the building ever changes;
/// the round-trip test proves it parses back equal.
pub const EXCHANGE_HOUSE_JSON: &str = include_str!("exchange_house.json");

/// A committed pretty-printed JSON of [`exchange_house_run`]'s combined export
/// — the fixture proving the four export surfaces keep the prototype's shapes.
/// Regenerate with the ignored `regenerate_exchange_house_run_json` test.
pub const EXCHANGE_HOUSE_RUN_JSON: &str = include_str!("exchange_house_run.json");

fn room(id: &str, name: &str) -> BuildingRoomDef {
    BuildingRoomDef {
        id: id.into(),
        name: name.to_owned(),
    }
}

#[allow(clippy::too_many_arguments)]
fn portal(
    id: &str,
    kind: PortalKind,
    label: &str,
    from: RoomRef,
    to: RoomRef,
    travel_time: f64,
    lock: Option<&str>,
    circuit: Option<&str>,
) -> PortalDef {
    PortalDef {
        id: id.into(),
        kind,
        label: label.to_owned(),
        from,
        to,
        travel_time,
        lock: lock.map(|controller| PortalLock {
            controller: controller.to_owned(),
        }),
        circuit: circuit.map(Into::into),
    }
}

/// The canonical Exchange House building definition: a ground exchange hall,
/// an upper office floor, the Ghost Lobby as the Billy floor, and a service
/// layer under it all — joined by stairs, two lift runs, a ladder and vents,
/// with one locked teller gate, one locked vent grille, two power circuits and
/// three network zones.
pub fn exchange_house() -> BuildingDefinition {
    let billy_floor_id = "ghost-lobby".into();
    let billy_scenario = ghost_lobby();
    let mut portals = vec![
        portal(
            "p-front",
            PortalKind::Door,
            "FRONT DOORS",
            RoomRef::new("exchange", "lobby"),
            RoomRef::new("exchange", "hall"),
            2.0,
            None,
            None,
        ),
        portal(
            "p-tellers",
            PortalKind::Door,
            "TELLER GATE",
            RoomRef::new("exchange", "hall"),
            RoomRef::new("exchange", "tellers"),
            2.0,
            Some("lock-tellers"),
            Some("main"),
        ),
        portal(
            "p-stair-1",
            PortalKind::Stair,
            "MAIN STAIR",
            RoomRef::new("exchange", "hall"),
            RoomRef::new("office", "landing"),
            6.0,
            None,
            None,
        ),
        portal(
            "p-bullpen",
            PortalKind::Door,
            "BULLPEN DOOR",
            RoomRef::new("office", "landing"),
            RoomRef::new("office", "bullpen"),
            2.0,
            None,
            None,
        ),
        portal(
            "p-manager",
            PortalKind::Door,
            "MANAGER'S DOOR",
            RoomRef::new("office", "bullpen"),
            RoomRef::new("office", "manager"),
            2.0,
            None,
            None,
        ),
        portal(
            "p-lift-01",
            PortalKind::Lift,
            "LIFT (GROUND-FIRST)",
            RoomRef::new("exchange", "lobby"),
            RoomRef::new("office", "landing"),
            8.0,
            None,
            Some("main"),
        ),
        portal(
            "p-lift-12",
            PortalKind::Lift,
            "LIFT (FIRST-SECOND)",
            RoomRef::new("office", "landing"),
            RoomRef::new("ghost-lobby", "kitchen"),
            8.0,
            None,
            Some("main"),
        ),
        portal(
            "p-stair-2",
            PortalKind::Stair,
            "BACK STAIR",
            RoomRef::new("office", "landing"),
            RoomRef::new("ghost-lobby", "kitchen"),
            6.0,
            None,
            None,
        ),
        portal(
            "p-hatch",
            PortalKind::Ladder,
            "SERVICE HATCH",
            RoomRef::new("exchange", "lobby"),
            RoomRef::new("service", "plant"),
            5.0,
            None,
            None,
        ),
        portal(
            "p-crawl",
            PortalKind::Door,
            "CRAWL DOOR",
            RoomRef::new("service", "plant"),
            RoomRef::new("service", "crawl"),
            3.0,
            None,
            None,
        ),
        portal(
            "p-vent-tellers",
            PortalKind::Vent,
            "TELLER VENT GRILLE",
            RoomRef::new("service", "crawl"),
            RoomRef::new("exchange", "tellers"),
            10.0,
            Some("lock-grille"),
            Some("service"),
        ),
        portal(
            "p-vent-laundry",
            PortalKind::Vent,
            "LAUNDRY RISER VENT",
            RoomRef::new("service", "crawl"),
            RoomRef::new("ghost-lobby", "laundry"),
            14.0,
            None,
            None,
        ),
    ];
    portals.extend(scenario_floor_portals(&billy_floor_id, &billy_scenario));

    BuildingDefinition {
        format: BUILDING_FORMAT.to_owned(),
        building_id: BUILDING_ID.to_owned(),
        name: "Exchange House".to_owned(),
        entry: RoomRef::new("exchange", "lobby"),
        floors: vec![
            FloorDef {
                id: "service".into(),
                name: "SERVICE LAYER".to_owned(),
                level: -1,
                kind: FloorKind::Service,
                rooms: vec![room("plant", "PLANT ROOM"), room("crawl", "CRAWLSPACE")],
                scenario: None,
            },
            FloorDef {
                id: "exchange".into(),
                name: "EXCHANGE HALL".to_owned(),
                level: 0,
                kind: FloorKind::Standard,
                rooms: vec![
                    room("lobby", "STREET LOBBY"),
                    room("hall", "EXCHANGE HALL"),
                    room("tellers", "TELLER ROW"),
                ],
                scenario: None,
            },
            FloorDef {
                id: "office".into(),
                name: "OFFICE FLOOR".to_owned(),
                level: 1,
                kind: FloorKind::Standard,
                rooms: vec![
                    room("landing", "STAIR LANDING"),
                    room("bullpen", "CLERKS' BULLPEN"),
                    room("manager", "MANAGER'S OFFICE"),
                ],
                scenario: None,
            },
            FloorDef::from_scenario(billy_floor_id, 2, FloorKind::Billy, billy_scenario),
        ],
        portals,
        circuits: vec![
            CircuitDef {
                id: "main".into(),
                name: "MAIN RISER".to_owned(),
                source: "feed-main".to_owned(),
                zone: "corp".into(),
            },
            CircuitDef {
                id: "service".into(),
                name: "SERVICE RISER".to_owned(),
                source: "feed-service".to_owned(),
                zone: "ot".into(),
            },
        ],
        zones: vec![
            ZoneDef {
                id: "corp".into(),
                name: "CORPORATE LAN".to_owned(),
                subnet: "10.60.10.".to_owned(),
                rooms: vec![
                    RoomRef::new("exchange", "lobby"),
                    RoomRef::new("exchange", "hall"),
                    RoomRef::new("exchange", "tellers"),
                    RoomRef::new("office", "landing"),
                    RoomRef::new("office", "bullpen"),
                    RoomRef::new("office", "manager"),
                ],
            },
            ZoneDef {
                id: "ops".into(),
                name: "OPERATIONS LAN".to_owned(),
                subnet: "10.60.20.".to_owned(),
                rooms: vec![
                    RoomRef::new("ghost-lobby", "kitchen"),
                    RoomRef::new("ghost-lobby", "hall"),
                    RoomRef::new("ghost-lobby", "office"),
                    RoomRef::new("ghost-lobby", "laundry"),
                    RoomRef::new("ghost-lobby", "exit"),
                ],
            },
            ZoneDef {
                id: "ot".into(),
                name: "BUILDING OT".to_owned(),
                subnet: "10.60.30.".to_owned(),
                rooms: vec![
                    RoomRef::new("service", "plant"),
                    RoomRef::new("service", "crawl"),
                ],
            },
        ],
    }
}

/// The canonical scripted run the parity fixture pins: walk in, bounce off
/// the locked teller gate, unlock it, tour the tellers, climb to the office,
/// cut the main riser (stranding the lifts), and take the back stair up to
/// the Ghost Lobby floor.
pub fn exchange_house_script() -> Vec<BuildingCommand> {
    vec![
        BuildingCommand::Traverse("p-front".into()),
        BuildingCommand::Traverse("p-tellers".into()), // denied: locked
        BuildingCommand::Actuate("lock-tellers".to_owned()),
        BuildingCommand::Traverse("p-tellers".into()),
        BuildingCommand::Traverse("p-tellers".into()), // back to the hall
        BuildingCommand::Traverse("p-stair-1".into()),
        BuildingCommand::Actuate("feed-main".to_owned()),
        BuildingCommand::Traverse("p-lift-12".into()), // denied: unpowered
        BuildingCommand::Traverse("p-stair-2".into()),
    ]
}

/// Run the canonical script over a fresh Exchange House, returning the sim in
/// its final state. Infallible by construction: the committed definition
/// validates (the goldens and tests pin it), so the error arm is unreachable
/// content-wise but still typed.
pub fn exchange_house_run()
-> Result<BuildingSim, Vec<crate::scenario::building::BuildingValidationError>> {
    let mut sim = BuildingSim::new(exchange_house())?;
    for cmd in exchange_house_script() {
        sim.apply(&cmd);
    }
    Ok(sim)
}

#[cfg(test)]
mod regen {
    use super::*;

    /// Rewrites the committed definition golden from the current authoring.
    /// Run explicitly after an intentional change:
    /// `cargo test -p idaptik-core regenerate_exchange_house_json -- --ignored`.
    #[test]
    #[ignore = "regenerates the committed golden; run manually"]
    fn regenerate_exchange_house_json() {
        let def = exchange_house();
        let mut json = serde_json::to_string_pretty(&def).expect("serialize");
        json.push('\n');
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/scenario/building/exchange_house.json"
        );
        std::fs::write(path, json.as_bytes()).expect("write golden json");
    }

    /// Rewrites the committed run-export parity fixture. Run explicitly after
    /// an intentional change to the building, the script or the surfaces:
    /// `cargo test -p idaptik-core regenerate_exchange_house_run_json -- --ignored`.
    #[test]
    #[ignore = "regenerates the committed golden; run manually"]
    fn regenerate_exchange_house_run_json() {
        let sim = exchange_house_run().expect("the committed building validates");
        let mut json = serde_json::to_string_pretty(&sim.export()).expect("serialize");
        json.push('\n');
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/scenario/building/exchange_house_run.json"
        );
        std::fs::write(path, json.as_bytes()).expect("write golden json");
    }
}
