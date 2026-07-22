//! The side-on 2.5D cross-section of the building, Gunpoint-like and very
//! basic: flat-shaded quads for readability over art.
//!
//! The scenario's coordinates are canvas-style (x rightwards, y *downwards*,
//! the floor line at `world.floor`); [`SceneMap`] converts them into Bevy's
//! y-up scene space. Rooms are laid out data-driven from the room definitions
//! as one horizontal row per floor; [`SceneMap::row_lift`] stacks additional
//! floor rows vertically, so the coming multi-floor Exchange House building
//! runtime slots in as extra rows without touching the mapping.

use bevy::camera::ScalingMode;
use bevy::prelude::*;
use idaptik_core::interp::VisualSlot;
use idaptik_core::scenario::camera_node_id;
use idaptik_core::scenario::common::BillyMode;
use idaptik_core::scenario::constants as c;

use crate::driver::{SimState, VisualBuffers};

/// How far through the current simulation tick this frame is drawn.
///
/// `0.0` is the tick that just completed, `1.0` the one about to. Bevy keeps
/// the leftover time that has not yet accumulated into a fixed step, which is
/// exactly the fraction we want; `_f64` because the simulation is `f64`
/// throughout and rounding to `f32` here would reintroduce the judder we are
/// removing.
///
/// This is the only place render timing enters the scene, and it is the seam to
/// cut when this lifts into a standalone engine — see `ENGINE_EXTRACTION_NOTES.md`.
fn render_alpha(fixed: &Time<Fixed>) -> f64 {
    fixed.overstep_fraction_f64()
}

/// World-y (canvas, downwards) of the room ceiling line for a floor row.
pub const CEILING_Y: f64 = 230.0;
/// Vertical scene-space gap between stacked floor rows (future multi-floor).
pub const ROW_STACK: f32 = 440.0;
/// How far the whole cross-section is shifted down so it centres on screen.
const VIEW_LIFT: f32 = 170.0;
/// Door slab visual size.
const DOOR_W: f32 = 12.0;
const DOOR_H: f32 = 96.0;

// Z layering: rooms behind everything, actors in front.
const Z_ROOM: f32 = 0.0;
const Z_FLOOR: f32 = 1.0;
const Z_HIDE: f32 = 1.5;
const Z_DOOR: f32 = 2.0;
const Z_PROP: f32 = 3.0;
const Z_ACTOR: f32 = 4.0;
const Z_LABEL: f32 = 5.0;

/// Canvas-space → scene-space mapping for one floor row.
#[derive(Resource, Clone, Copy)]
pub struct SceneMap {
    width: f32,
    floor: f32,
    row: u32,
}

impl SceneMap {
    /// Build the mapping for floor row `row` of a world `width` wide with its
    /// walk line at canvas-y `floor`.
    pub fn new(width: f64, floor: f64, row: u32) -> Self {
        Self {
            width: width as f32,
            floor: floor as f32,
            row,
        }
    }

    /// The extra scene-space height this floor row is lifted by.
    fn row_lift(&self) -> f32 {
        self.row as f32 * ROW_STACK
    }

    /// A canvas-space point (y downwards) in scene space (y upwards).
    pub fn point(&self, x: f64, y: f64) -> Vec2 {
        Vec2::new(
            x as f32 - self.width * 0.5,
            (self.floor - y as f32) - VIEW_LIFT + self.row_lift(),
        )
    }

    /// Centre of a canvas-space box given its top-left corner and size.
    pub fn box_center(&self, x: f64, y_top: f64, w: f64, h: f64) -> Vec2 {
        self.point(x + w * 0.5, y_top + h * 0.5)
    }
}

/// Marker: the infiltrator's body quad.
#[derive(Component)]
pub struct PlayerMarker;
/// Marker: the small facing-indicator quad parented to the player.
#[derive(Component)]
pub struct PlayerNose;
/// Marker: Billy's body quad.
#[derive(Component)]
pub struct BillyMarker;
/// Marker: Billy's facing-indicator quad.
#[derive(Component)]
pub struct BillyNose;
/// Marker: a door slab, by definition index.
#[derive(Component)]
pub struct DoorMarker(pub usize);
/// Marker: the contact note.
#[derive(Component)]
pub struct NoteMarker;
/// Marker: the USB trap.
#[derive(Component)]
pub struct UsbMarker;
/// Marker: the laundry chute.
#[derive(Component)]
pub struct ChuteMarker;
/// Marker: the robot vacuum.
#[derive(Component)]
pub struct VacuumMarker;

fn label(text: &str, size: f32, at: Vec2) -> impl Bundle {
    (
        Text2d::new(text),
        TextFont::from_font_size(size),
        TextColor(Color::srgba(0.75, 0.78, 0.85, 0.9)),
        Transform::from_translation(at.extend(Z_LABEL)),
    )
}

/// Spawn the whole static scene from the scenario definition, plus the dynamic
/// entities the sync systems drive. Everything is data-driven: rooms, doors,
/// cameras, hide spots and props come from the definition, never from names.
pub fn setup_scene(mut commands: Commands, sim: Res<SimState>) {
    let def = sim.sim.definition();
    let map = SceneMap::new(def.world.width, def.world.floor, def.floor_id);
    let floor = def.world.floor;
    let room_h = floor - CEILING_Y;

    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: map.width + 120.0,
                min_height: 820.0,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
    commands.insert_resource(ClearColor(Color::srgb(0.05, 0.06, 0.08)));

    // The walk line the whole row stands on.
    commands.spawn((
        Sprite::from_color(
            Color::srgb(0.35, 0.37, 0.42),
            Vec2::new(map.width + 8.0, 8.0),
        ),
        Transform::from_translation(
            map.point(def.world.width * 0.5, floor + 4.0)
                .extend(Z_FLOOR),
        ),
    ));

    // Rooms: a row of side-view boxes, one per definition.
    for (i, room) in def.rooms.iter().enumerate() {
        let color = if room.lit {
            Color::srgb(0.23, 0.21, 0.15)
        } else if i % 2 == 0 {
            Color::srgb(0.12, 0.14, 0.18)
        } else {
            Color::srgb(0.15, 0.17, 0.22)
        };
        commands.spawn((
            Sprite::from_color(color, Vec2::new(room.w as f32 - 6.0, room_h as f32)),
            Transform::from_translation(
                map.box_center(room.x, CEILING_Y, room.w, room_h)
                    .extend(Z_ROOM),
            ),
        ));
        commands.spawn(label(
            &room.name,
            13.0,
            map.point(room.x + room.w * 0.5, CEILING_Y + 22.0),
        ));
    }

    // Hide spots: faint bands at the walk line.
    for spot in &def.hide_spots {
        commands.spawn((
            Sprite::from_color(
                Color::srgba(0.25, 0.35, 0.55, 0.35),
                Vec2::new(spot.radius as f32 * 2.0, 44.0),
            ),
            Transform::from_translation(map.point(spot.x, floor - 22.0).extend(Z_HIDE)),
        ));
    }

    // Camera mounts at the ceiling; the sweep cones are drawn with gizmos.
    for cam in &def.cameras {
        commands.spawn((
            Sprite::from_color(Color::srgb(0.55, 0.58, 0.65), Vec2::new(18.0, 10.0)),
            Transform::from_translation(map.point(cam.x, CEILING_Y + 10.0).extend(Z_PROP)),
        ));
    }

    // Door slabs (positions/colours are synced from state every frame).
    for (i, _door) in def.doors.iter().enumerate() {
        commands.spawn((
            DoorMarker(i),
            Sprite::from_color(Color::srgb(0.5, 0.5, 0.55), Vec2::new(DOOR_W, DOOR_H)),
            Transform::from_translation(Vec3::new(0.0, 0.0, Z_DOOR)),
        ));
    }

    // The extraction edge: the right end of the row.
    commands.spawn((
        Sprite::from_color(
            Color::srgba(0.2, 0.9, 0.4, 0.5),
            Vec2::new(8.0, room_h as f32),
        ),
        Transform::from_translation(
            map.point(def.world.width - 4.0, (CEILING_Y + floor) * 0.5)
                .extend(Z_DOOR),
        ),
    ));
    commands.spawn(label(
        "EXIT",
        13.0,
        map.point(def.world.width - 30.0, CEILING_Y + 40.0),
    ));

    // Props.
    commands.spawn((
        NoteMarker,
        Sprite::from_color(Color::srgb(0.95, 0.85, 0.35), Vec2::new(12.0, 9.0)),
        Transform::from_translation(Vec3::new(0.0, 0.0, Z_PROP)),
    ));
    commands.spawn((
        UsbMarker,
        Sprite::from_color(Color::srgb(0.3, 0.85, 0.9), Vec2::new(10.0, 7.0)),
        Transform::from_translation(Vec3::new(0.0, 0.0, Z_PROP)),
    ));
    commands.spawn((
        ChuteMarker,
        Sprite::from_color(
            Color::srgb(0.1, 0.1, 0.12),
            Vec2::new(30.0, (floor - def.props.chute.y) as f32),
        ),
        Transform::from_translation(
            map.box_center(
                def.props.chute.x - 15.0,
                def.props.chute.y,
                30.0,
                floor - def.props.chute.y,
            )
            .extend(Z_PROP),
        ),
    ));
    commands.spawn((
        VacuumMarker,
        Sprite::from_color(Color::srgb(0.45, 0.45, 0.5), Vec2::new(30.0, 12.0)),
        Transform::from_translation(Vec3::new(0.0, 0.0, Z_PROP)),
    ));

    // The infiltrator and Billy, with small facing-indicator noses.
    commands
        .spawn((
            PlayerMarker,
            Sprite::from_color(
                Color::srgb(0.9, 0.92, 0.98),
                Vec2::new(def.player.w as f32, def.player.h as f32),
            ),
            Transform::from_translation(Vec3::new(0.0, 0.0, Z_ACTOR)),
        ))
        .with_children(|parent| {
            parent.spawn((
                PlayerNose,
                Sprite::from_color(Color::srgb(0.35, 0.75, 1.0), Vec2::new(7.0, 7.0)),
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.5)),
            ));
        });
    commands
        .spawn((
            BillyMarker,
            Sprite::from_color(
                Color::srgb(0.95, 0.6, 0.25),
                Vec2::new(def.billy.w as f32, def.billy.h as f32),
            ),
            Transform::from_translation(Vec3::new(0.0, 0.0, Z_ACTOR)),
        ))
        .with_children(|parent| {
            parent.spawn((
                BillyNose,
                Sprite::from_color(Color::srgb(1.0, 0.35, 0.25), Vec2::new(8.0, 8.0)),
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.5)),
            ));
        });

    commands.insert_resource(map);
}

/// Door slabs: colour says closed / routed / held-open, and an opening door
/// slides up out of the walk line, Gunpoint-style.
pub fn sync_doors(
    sim: Res<SimState>,
    map: Res<SceneMap>,
    visual: Res<VisualBuffers>,
    fixed: Res<Time<Fixed>>,
    mut doors: Query<(&DoorMarker, &mut Transform, &mut Sprite)>,
) {
    let s = sim.sim.state();
    let alpha = render_alpha(&fixed);
    for (marker, mut tf, mut sprite) in &mut doors {
        let Some(door) = s.doors.get(marker.0) else {
            continue;
        };
        // The slab slides open over several ticks, so its openness interpolates
        // like a position. Colour keys off the live tick.
        let open = visual.doors.sample(marker.0, alpha).clamp(0.0, 1.0) as f32;
        let base = map.point(
            door.x,
            sim.sim.definition().world.floor - f64::from(DOOR_H) * 0.5,
        );
        tf.translation.x = base.x;
        tf.translation.y = base.y + open * DOOR_H * 0.92;
        sprite.color = if open > 0.05 {
            Color::srgb(0.25, 0.85, 0.4)
        } else if door.pending > 0.0 {
            Color::srgb(0.95, 0.75, 0.25)
        } else {
            Color::srgb(0.55, 0.3, 0.3)
        };
    }
}

/// An actor's canvas-space body box (top-left corner plus size).
struct BodyBox {
    x: f64,
    y_top: f64,
    w: f64,
    h: f64,
}

/// Position/size an actor's body quad from canvas-space state.
/// `crouch_scale` shrinks the drawn height while keeping the feet planted.
fn place_body(
    map: &SceneMap,
    tf: &mut Transform,
    sprite: &mut Sprite,
    body: BodyBox,
    crouch_scale: f64,
) {
    let dh = body.h * crouch_scale;
    let bottom = body.y_top + body.h;
    let center = map.box_center(body.x, bottom - dh, body.w, dh);
    tf.translation.x = center.x;
    tf.translation.y = center.y;
    sprite.custom_size = Some(Vec2::new(body.w as f32, dh as f32));
}

/// The infiltrator: body, crouch squash, hidden fade, facing nose.
pub fn sync_player(
    sim: Res<SimState>,
    map: Res<SceneMap>,
    visual: Res<VisualBuffers>,
    fixed: Res<Time<Fixed>>,
    mut body: Query<(&mut Transform, &mut Sprite), With<PlayerMarker>>,
    mut nose: Query<&mut Transform, (With<PlayerNose>, Without<PlayerMarker>)>,
) {
    let def = sim.sim.definition();
    // Position is interpolated; every discrete flag is read live from the
    // current tick. Blending `crouching` or `hidden` would be meaningless.
    let p = &sim.sim.state().player;
    let pose = visual
        .poses
        .sample(VisualSlot::Player.index(), render_alpha(&fixed));
    let Ok((mut tf, mut sprite)) = body.single_mut() else {
        return;
    };
    let crouch = if p.crouching { 0.62 } else { 1.0 };
    place_body(
        &map,
        &mut tf,
        &mut sprite,
        BodyBox {
            x: pose.x,
            y_top: pose.y,
            w: def.player.w,
            h: def.player.h,
        },
        crouch,
    );
    sprite.color = sprite.color.with_alpha(if p.hidden { 0.35 } else { 1.0 });
    if let Ok(mut nose_tf) = nose.single_mut() {
        nose_tf.translation.x = pose.facing as f32 * (def.player.w as f32 * 0.5 + 5.0);
        nose_tf.translation.y = def.player.h as f32 * crouch as f32 * 0.22;
    }
}

/// Billy: body, offsite hides him, facing nose.
pub fn sync_billy(
    sim: Res<SimState>,
    map: Res<SceneMap>,
    visual: Res<VisualBuffers>,
    fixed: Res<Time<Fixed>>,
    mut body: Query<(&mut Transform, &mut Sprite, &mut Visibility), With<BillyMarker>>,
    mut nose: Query<&mut Transform, (With<BillyNose>, Without<BillyMarker>)>,
) {
    let def = sim.sim.definition();
    // `mode` is discrete and read live; only the pose interpolates.
    let b = &sim.sim.state().billy;
    let pose = visual
        .poses
        .sample(VisualSlot::Billy.index(), render_alpha(&fixed));
    let Ok((mut tf, mut sprite, mut vis)) = body.single_mut() else {
        return;
    };
    *vis = if b.mode == BillyMode::Offsite {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    place_body(
        &map,
        &mut tf,
        &mut sprite,
        BodyBox {
            x: pose.x,
            y_top: pose.y,
            w: def.billy.w,
            h: def.billy.h,
        },
        1.0,
    );
    if let Ok(mut nose_tf) = nose.single_mut() {
        nose_tf.translation.x = pose.facing as f32 * (def.billy.w as f32 * 0.5 + 5.0);
        nose_tf.translation.y = def.billy.h as f32 * 0.25;
    }
}

/// Props: the note and USB track their state positions and vanish when
/// carried; the chute darkens open when revealed; the vacuum lights up active.
// A Bevy system takes one parameter per resource and per query, so arity here
// tracks how many things the prop layer draws, not how complex it is. Bundling
// them into a `SystemParam` would satisfy the lint and tell the reader less.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn sync_props(
    sim: Res<SimState>,
    map: Res<SceneMap>,
    visual: Res<VisualBuffers>,
    fixed: Res<Time<Fixed>>,
    mut note: Query<
        (&mut Transform, &mut Visibility),
        (With<NoteMarker>, Without<UsbMarker>, Without<VacuumMarker>),
    >,
    mut usb: Query<
        (&mut Transform, &mut Visibility, &mut Sprite),
        (With<UsbMarker>, Without<NoteMarker>, Without<VacuumMarker>),
    >,
    mut chute: Query<&mut Sprite, (With<ChuteMarker>, Without<UsbMarker>, Without<VacuumMarker>)>,
    mut vacuum: Query<
        (&mut Transform, &mut Visibility, &mut Sprite),
        (With<VacuumMarker>, Without<NoteMarker>, Without<UsbMarker>),
    >,
) {
    let s = sim.sim.state();
    let alpha = render_alpha(&fixed);
    // Positions interpolate; `held`/`billy_has`/`wiped`/`fallen` are discrete
    // and read live.

    if let Ok((mut tf, mut vis)) = note.single_mut() {
        let pose = visual.poses.sample(VisualSlot::Note.index(), alpha);
        let p = map.point(pose.x, pose.y);
        tf.translation.x = p.x;
        tf.translation.y = p.y;
        *vis = if s.note.held || s.note.billy_has {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }

    if let Ok((mut tf, mut vis, mut sprite)) = usb.single_mut() {
        let pose = visual.poses.sample(VisualSlot::Usb.index(), alpha);
        let p = map.point(pose.x, pose.y);
        tf.translation.x = p.x;
        tf.translation.y = p.y;
        *vis = if s.usb.held || s.usb.billy_has {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        sprite.color = if s.usb.wiped {
            Color::srgb(0.35, 0.45, 0.48)
        } else {
            Color::srgb(0.3, 0.85, 0.9)
        };
    }

    if let Ok(mut sprite) = chute.single_mut() {
        sprite.color = if s.chute.revealed {
            Color::srgb(0.02, 0.02, 0.03)
        } else {
            Color::srgb(0.18, 0.19, 0.22)
        };
    }

    if let Ok((mut tf, mut vis, mut sprite)) = vacuum.single_mut() {
        let pose = visual.poses.sample(VisualSlot::Vacuum.index(), alpha);
        let p = map.point(pose.x, pose.y + 6.0);
        tf.translation.x = p.x;
        tf.translation.y = p.y;
        *vis = if s.vacuum.fallen {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        sprite.color = if s.vacuum.active {
            Color::srgb(0.4, 0.9, 0.6)
        } else {
            Color::srgb(0.45, 0.45, 0.5)
        };
    }
}

/// Camera view cones, drawn as gizmo wireframes each frame using the exact
/// sweep the simulation's camera system uses (a pure view; nothing here can
/// disagree with detection because the formula and constants are the sim's).
pub fn draw_camera_cones(sim: Res<SimState>, map: Res<SceneMap>, mut gizmos: Gizmos) {
    let def = sim.sim.definition();
    let s = sim.sim.state();
    let floor = def.world.floor;
    let all_off = s.camera_lockout > 0.0 || s.lights_flicker > 0.0;
    for (i, cam) in def.cameras.iter().enumerate() {
        let sweep = (s.t * c::CAM_SWEEP_W + cam.phase).sin() * c::CAM_SWEEP_A;
        let lo = cam.range.0 + sweep.min(0.0);
        let hi = cam.range.1 + sweep.max(0.0);
        let looped = s.camera_looped.get(i).is_some_and(|left| *left > 0.0);
        let dead = s.dead_nodes.contains(&camera_node_id(i));
        let live = !(cam.stale || looped || dead || all_off);
        let color = if live {
            Color::srgba(1.0, 0.35, 0.25, 0.55)
        } else {
            Color::srgba(0.4, 0.5, 0.7, 0.25)
        };
        let mount = map.point(cam.x, CEILING_Y + 14.0);
        let a = map.point(lo, floor);
        let b = map.point(hi, floor);
        gizmos.line_2d(mount, a, color);
        gizmos.line_2d(mount, b, color);
        gizmos.line_2d(a, b, color);
    }
}
