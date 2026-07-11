//! The canonical definition validates clean, and each single-field corruption
//! produces exactly one typed `ValidationError`.

use idaptik_core::ScenarioDefinition;
use idaptik_core::ghost_lobby;
use idaptik_core::scenario::definition::{RoomDef, ValidationError};
use idaptik_core::scenario::ids::{CameraId, DoorId, ObjectiveId, RoomId};
use idaptik_core::scenario::tuning::{ActionKind, DifficultyId};

#[test]
fn canonical_definition_validates_clean() {
    let def = ghost_lobby();
    assert!(def.validate().passed());
    assert_eq!(def.ok(), Ok(()));
}

/// Assert a mutated definition yields exactly the one expected error.
#[track_caller]
fn expect_one(def: &ScenarioDefinition, want: ValidationError) {
    match def.ok() {
        Ok(()) => panic!("expected {want:?}, but validation passed"),
        Err(errs) => {
            assert_eq!(errs.len(), 1, "expected exactly one error, got {errs:?}");
            assert_eq!(errs[0], want);
        }
    }
}

#[test]
fn empty_rooms_reports_only_empty_rooms() {
    let mut def = ghost_lobby();
    def.rooms.clear();
    expect_one(&def, ValidationError::EmptyRooms);
}

#[test]
fn duplicate_room_id() {
    let mut def = ghost_lobby();
    // Append a contiguous extra room that reuses an existing id.
    def.rooms.push(RoomDef {
        id: "kitchen".into(),
        name: "DUP".into(),
        x: 1260.0,
        w: 100.0,
        support: 0.5,
        ping_support_bonus: 0.0,
        sight_multiplier: 1.0,
        lit: false,
    });
    expect_one(
        &def,
        ValidationError::DuplicateRoomId(RoomId::from("kitchen")),
    );
}

#[test]
fn duplicate_door_id() {
    let mut def = ghost_lobby();
    def.doors[1].id = "D1".into();
    expect_one(&def, ValidationError::DuplicateDoorId(DoorId::from("D1")));
}

#[test]
fn rooms_not_contiguous() {
    let mut def = ghost_lobby();
    def.rooms[2].x += 5.0; // office slides right, opening a gap after the hall
    expect_one(
        &def,
        ValidationError::RoomsNotContiguous {
            gap_after: RoomId::from("hall"),
        },
    );
}

#[test]
fn door_off_room_boundary() {
    let mut def = ghost_lobby();
    def.doors[0].x = 999.0;
    expect_one(
        &def,
        ValidationError::DoorOffRoomBoundary(DoorId::from("D1")),
    );
}

#[test]
fn camera_range_inverted() {
    let mut def = ghost_lobby();
    def.cameras[0].range = (500.0, 100.0);
    expect_one(
        &def,
        ValidationError::CameraRangeInverted(CameraId::from("cam-hall")),
    );
}

#[test]
fn arrival_inverted() {
    let mut def = ghost_lobby();
    if let Some(p) = def.difficulty.get_mut(&DifficultyId::Standard) {
        p.arrival = (30.0, 10.0);
    }
    expect_one(
        &def,
        ValidationError::ArrivalInverted(DifficultyId::Standard),
    );
}

#[test]
fn difficulty_missing() {
    let mut def = ghost_lobby();
    def.difficulty.remove(&DifficultyId::Operator);
    expect_one(
        &def,
        ValidationError::DifficultyMissing(DifficultyId::Operator),
    );
}

#[test]
fn negative_action_cost() {
    let mut def = ghost_lobby();
    if let Some(a) = def.actions.get_mut(&ActionKind::Camera) {
        a.cost = -1.0;
    }
    expect_one(
        &def,
        ValidationError::NegativeActionCost(ActionKind::Camera),
    );
}

#[test]
fn support_out_of_range() {
    let mut def = ghost_lobby();
    def.rooms[0].support = 2.0;
    expect_one(
        &def,
        ValidationError::SupportOutOfRange(RoomId::from("kitchen")),
    );
}

#[test]
fn player_spawn_out_of_bounds() {
    let mut def = ghost_lobby();
    def.player.spawn_x = 99_999.0;
    expect_one(&def, ValidationError::PlayerSpawnOutOfBounds);
}

#[test]
fn objective_room_missing() {
    let mut def = ghost_lobby();
    def.objectives[0].room = Some("nowhere".into());
    expect_one(
        &def,
        ValidationError::ObjectiveRoomMissing(ObjectiveId::from("note")),
    );
}

#[test]
fn no_extraction_room() {
    let mut def = ghost_lobby();
    def.objectives
        .retain(|o| o.kind != idaptik_core::scenario::definition::ObjectiveKind::Exit);
    expect_one(&def, ValidationError::NoExtractionRoom);
}

#[test]
fn object_spawn_out_of_bounds() {
    let mut def = ghost_lobby();
    def.props.note.x = 99_999.0;
    expect_one(
        &def,
        ValidationError::ObjectSpawnOutOfBounds { obj: "note".into() },
    );
}
