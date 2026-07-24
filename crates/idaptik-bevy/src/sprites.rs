//! Vector-sprite composites ported from IDApixiTIK's `PlayerGraphics.res` and
//! `WorldScreen.res`'s `WorldDeviceIcon`: layered flat-colour shapes (rects
//! and, where a circle is needed, a `Mesh2d` disc) rather than a single flat
//! quad per actor.

use bevy::prelude::*;

//## Player

/// The player body colour and whether the laptop shows, from the same two
/// state flags Ghost Lobby actually tracks (`crouching`, `sprinting`) --
/// collapsed from `PlayerGraphics.res`'s six-state `visualState`, since this
/// sim has no separate Jumping/ChargingJump visual state.
pub fn player_colours(crouching: bool, sprinting: bool) -> (Color, bool) {
    if crouching {
        (Color::srgb(0.267, 0.333, 0.6), false)
    } else if sprinting {
        (Color::srgb(0.133, 0.267, 0.533), true)
    } else {
        (Color::srgb(0.2, 0.4, 0.8), true)
    }
}

/// Marker: the player's circular head.
#[derive(Component)]
pub struct PlayerHead;
/// Marker: the player's laptop base.
#[derive(Component)]
pub struct PlayerLaptop;
/// Marker: the player's laptop screen.
#[derive(Component)]
pub struct PlayerLaptopScreen;

/// Head radius, laptop size and vertical offsets, transliterated from
/// `PlayerGraphics.res`'s `Dimensions` module (its `y` is canvas-down; these
/// are Bevy scene-space, so the sign is flipped once here rather than at
/// every call site).
const HEAD_RADIUS: f32 = 6.0;
const HEAD_Y: f32 = 24.0;
const LAPTOP_W: f32 = 10.0;
const LAPTOP_H: f32 = 6.0;
const LAPTOP_Y: f32 = 12.0;
const LAPTOP_SCREEN_PAD: f32 = 1.0;

/// Spawn the player's head, laptop and laptop-screen as children of the
/// entity this is called from within (the existing `PlayerMarker` body).
/// Colours/visibility are then driven each frame by `sync_player`, so this
/// only needs to exist -- its first-frame look is set below and refined there.
pub fn spawn_player_children(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
) {
    let head_mesh = meshes.add(Circle::new(HEAD_RADIUS));
    let head_material = materials.add(ColorMaterial::from(Color::srgb(1.0, 0.8, 0.667)));
    parent.spawn((
        PlayerHead,
        Mesh2d(head_mesh),
        MeshMaterial2d(head_material),
        Transform::from_translation(Vec3::new(0.0, HEAD_Y, 0.6)),
    ));
    parent.spawn((
        PlayerLaptop,
        Sprite::from_color(Color::srgb(0.2, 0.2, 0.2), Vec2::new(LAPTOP_W, LAPTOP_H)),
        Transform::from_translation(Vec3::new(0.0, LAPTOP_Y, 0.55)),
    ));
    parent.spawn((
        PlayerLaptopScreen,
        Sprite::from_color(
            Color::srgb(0.0, 1.0, 0.0),
            Vec2::new(
                LAPTOP_W - LAPTOP_SCREEN_PAD * 2.0,
                LAPTOP_H - LAPTOP_SCREEN_PAD * 2.0,
            ),
        ),
        Transform::from_translation(Vec3::new(0.0, LAPTOP_Y, 0.56)),
    ));
}

//## Billy

/// Marker: Billy's circular head.
#[derive(Component)]
pub struct BillyHead;

/// Spawn Billy's head as a child of the existing `BillyMarker` body. Billy has
/// no source art in IDApixiTIK (there is no guard/NPC anywhere in that
/// prototype); this extends the same layered-shape language the player just
/// got, rather than porting anything literal.
pub fn spawn_billy_children(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
) {
    let head_mesh = meshes.add(Circle::new(HEAD_RADIUS + 1.0));
    let head_material = materials.add(ColorMaterial::from(Color::srgb(1.0, 0.8, 0.667)));
    parent.spawn((
        BillyHead,
        Mesh2d(head_mesh),
        MeshMaterial2d(head_material),
        Transform::from_translation(Vec3::new(0.0, HEAD_Y + 4.0, 0.6)),
    ));
}

//## Devices

use idaptik_core::device::DeviceKind;

/// The seven original-taxonomy device sprites, ported from `WorldScreen.res`'s
/// `WorldDeviceIcon`, plus three originally-designed fallbacks (`SmartDoor`,
/// `Light`, `Substation`) for kinds that appear on the grounded network graph
/// but have no source art, and a plain box for every other kind (the UMS
/// parity/physical-function kinds this game added after the port), so the
/// caller never has to special-case an unknown one.
pub fn spawn_device_icon(
    commands: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    kind: DeviceKind,
) {
    match kind {
        DeviceKind::Laptop | DeviceKind::Desktop => {
            // Desk plus a screen-and-base laptop, transliterated from
            // WorldScreen.res's `Laptop` arm.
            commands.spawn((
                Sprite::from_color(Color::srgb(0.29, 0.22, 0.16), Vec2::new(70.0, 20.0)),
                Transform::from_translation(Vec3::new(0.0, -10.0, 0.0)),
            ));
            commands.spawn((
                Sprite::from_color(Color::srgb(0.2, 0.2, 0.2), Vec2::new(50.0, 35.0)),
                Transform::from_translation(Vec3::new(0.0, 17.5, 0.1)),
            ));
            commands.spawn((
                Sprite::from_color(Color::srgb(0.0, 0.6, 0.9), Vec2::new(44.0, 29.0)),
                Transform::from_translation(Vec3::new(0.0, 17.5, 0.2)),
            ));
        }
        DeviceKind::Server => {
            // A rack of six units, each with a green LED.
            commands.spawn((
                Sprite::from_color(Color::srgb(0.2, 0.2, 0.2), Vec2::new(50.0, 120.0)),
                Transform::from_translation(Vec3::new(0.0, 60.0, 0.0)),
            ));
            for i in 0..6 {
                let y = 10.0 + i as f32 * 18.0;
                commands.spawn((
                    Sprite::from_color(Color::srgb(0.0, 0.6, 0.9), Vec2::new(44.0, 16.0)),
                    Transform::from_translation(Vec3::new(0.0, y, 0.1)),
                ));
                let led_mesh = meshes.add(Circle::new(3.0));
                let led_material = materials.add(ColorMaterial::from(Color::srgb(0.0, 1.0, 0.0)));
                commands.spawn((
                    Mesh2d(led_mesh),
                    MeshMaterial2d(led_material),
                    Transform::from_translation(Vec3::new(17.0, y + 8.0, 0.2)),
                ));
            }
        }
        DeviceKind::Router | DeviceKind::AccessPoint | DeviceKind::Switch => {
            // Shelf, box, two antennas, four LEDs.
            commands.spawn((
                Sprite::from_color(Color::srgb(0.29, 0.22, 0.16), Vec2::new(60.0, 8.0)),
                Transform::from_translation(Vec3::new(0.0, -36.0, 0.0)),
            ));
            commands.spawn((
                Sprite::from_color(Color::srgb(0.0, 0.6, 0.9), Vec2::new(50.0, 20.0)),
                Transform::from_translation(Vec3::new(0.0, -20.0, 0.1)),
            ));
            for x in [-16.0, 16.0] {
                commands.spawn((
                    Sprite::from_color(Color::srgb(0.2, 0.2, 0.2), Vec2::new(4.0, 25.0)),
                    Transform::from_translation(Vec3::new(x, 2.5, 0.1)),
                ));
            }
            for i in 0..4 {
                let x = -15.0 + i as f32 * 10.0;
                let led_mesh = meshes.add(Circle::new(3.0));
                let led_material = materials.add(ColorMaterial::from(Color::srgb(0.0, 1.0, 0.0)));
                commands.spawn((
                    Mesh2d(led_mesh),
                    MeshMaterial2d(led_material),
                    Transform::from_translation(Vec3::new(x, -10.0, 0.2)),
                ));
            }
        }
        DeviceKind::IotCamera | DeviceKind::Camera => {
            // Pole, mount, lensed body -- reused for both the network-graph
            // IotCamera and the physical-function Camera actuator, since both
            // read visually as "a camera".
            commands.spawn((
                Sprite::from_color(Color::srgb(0.4, 0.4, 0.4), Vec2::new(8.0, 100.0)),
                Transform::from_translation(Vec3::new(0.0, -50.0, 0.0)),
            ));
            commands.spawn((
                Sprite::from_color(Color::srgb(0.27, 0.27, 0.27), Vec2::new(30.0, 10.0)),
                Transform::from_translation(Vec3::new(0.0, -5.0, 0.1)),
            ));
            let body_mesh = meshes.add(Circle::new(18.0));
            let body_material = materials.add(ColorMaterial::from(Color::srgb(0.0, 0.6, 0.9)));
            commands.spawn((
                Mesh2d(body_mesh),
                MeshMaterial2d(body_material),
                Transform::from_translation(Vec3::new(0.0, 20.0, 0.2)),
            ));
            let lens_mesh = meshes.add(Circle::new(10.0));
            let lens_material = materials.add(ColorMaterial::from(Color::srgb(0.13, 0.13, 0.13)));
            commands.spawn((
                Mesh2d(lens_mesh),
                MeshMaterial2d(lens_material),
                Transform::from_translation(Vec3::new(0.0, 20.0, 0.3)),
            ));
        }
        DeviceKind::Terminal => {
            // Desk, CRT monitor, terminal-green text, keyboard.
            commands.spawn((
                Sprite::from_color(Color::srgb(0.29, 0.22, 0.16), Vec2::new(80.0, 30.0)),
                Transform::from_translation(Vec3::new(0.0, -15.0, 0.0)),
            ));
            commands.spawn((
                Sprite::from_color(Color::srgb(0.2, 0.2, 0.2), Vec2::new(60.0, 50.0)),
                Transform::from_translation(Vec3::new(0.0, 25.0, 0.1)),
            ));
            commands.spawn((
                Sprite::from_color(Color::BLACK, Vec2::new(54.0, 44.0)),
                Transform::from_translation(Vec3::new(0.0, 25.0, 0.2)),
            ));
            commands.spawn((
                Text2d::new(">_"),
                TextFont::from_font_size(16.0),
                TextColor(Color::srgb(0.0, 1.0, 0.0)),
                Transform::from_translation(Vec3::new(0.0, 25.0, 0.3)),
            ));
            commands.spawn((
                Sprite::from_color(Color::srgb(0.13, 0.13, 0.13), Vec2::new(50.0, 8.0)),
                Transform::from_translation(Vec3::new(0.0, -2.0, 0.1)),
            ));
        }
        DeviceKind::PowerStation | DeviceKind::Substation => {
            // Industrial cabinet, ventilation grille, power meter, warning
            // stripe, status light. Substation has no source art of its own;
            // it is conceptually a power station, so it reuses this look.
            commands.spawn((
                Sprite::from_color(Color::srgb(0.16, 0.16, 0.16), Vec2::new(80.0, 150.0)),
                Transform::from_translation(Vec3::new(0.0, 75.0, 0.0)),
            ));
            for i in 0..6 {
                let y = 10.0 + i as f32 * 12.0;
                commands.spawn((
                    Sprite::from_color(Color::srgb(0.1, 0.1, 0.1), Vec2::new(60.0, 8.0)),
                    Transform::from_translation(Vec3::new(0.0, y, 0.1)),
                ));
            }
            commands.spawn((
                Sprite::from_color(Color::srgb(0.95, 0.8, 0.0), Vec2::new(80.0, 20.0)),
                Transform::from_translation(Vec3::new(0.0, -10.0, 0.1)),
            ));
            let light_mesh = meshes.add(Circle::new(8.0));
            let light_material = materials.add(ColorMaterial::from(Color::srgb(0.0, 1.0, 0.0)));
            commands.spawn((
                Mesh2d(light_mesh),
                MeshMaterial2d(light_material),
                Transform::from_translation(Vec3::new(0.0, 140.0, 0.2)),
            ));
        }
        DeviceKind::Ups => {
            // Smaller box with an LCD display and status LEDs.
            commands.spawn((
                Sprite::from_color(Color::srgb(0.24, 0.24, 0.24), Vec2::new(60.0, 80.0)),
                Transform::from_translation(Vec3::new(0.0, 40.0, 0.0)),
            ));
            commands.spawn((
                Sprite::from_color(Color::srgb(0.1, 0.23, 0.1), Vec2::new(40.0, 20.0)),
                Transform::from_translation(Vec3::new(0.0, 65.0, 0.1)),
            ));
            for x in [-10.0, 0.0, 10.0] {
                let led_mesh = meshes.add(Circle::new(4.0));
                let led_material = materials.add(ColorMaterial::from(Color::srgb(0.0, 1.0, 0.0)));
                commands.spawn((
                    Mesh2d(led_mesh),
                    MeshMaterial2d(led_material),
                    Transform::from_translation(Vec3::new(x, 40.0, 0.1)),
                ));
            }
        }
        DeviceKind::SmartDoor => {
            // Reuses the existing scene door-slab look (a plain slab); Net
            // View draws it with a neutral colour since there is no live
            // door-state to key off here, unlike `scene::sync_doors`.
            commands.spawn((
                Sprite::from_color(Color::srgb(0.5, 0.5, 0.55), Vec2::new(12.0, 96.0)),
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            ));
        }
        DeviceKind::Light => {
            // A small fixture: a circle with a downward cone.
            let mesh = meshes.add(Circle::new(6.0));
            let material = materials.add(ColorMaterial::from(Color::srgb(0.95, 0.9, 0.6)));
            commands.spawn((
                Mesh2d(mesh),
                MeshMaterial2d(material),
                Transform::from_translation(Vec3::new(0.0, 10.0, 0.0)),
            ));
            commands.spawn((
                Sprite::from_color(Color::srgba(0.95, 0.9, 0.6, 0.3), Vec2::new(20.0, 14.0)),
                Transform::from_translation(Vec3::new(0.0, 0.0, -0.1)),
            ));
        }
        // Everything else (Firewall, Lock, Elevator, Sensor, and the UMS
        // parity kinds PatchPanel/FibreHub/PhoneSystem/PowerSupply) has no
        // source art and no bespoke design of its own yet: a plain neutral
        // box, so the caller never has to match on kind itself.
        _ => {
            commands.spawn((
                Sprite::from_color(Color::srgb(0.4, 0.4, 0.42), Vec2::new(40.0, 40.0)),
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            ));
        }
    }
}

//## Net View icons

use idaptik_core::device::SecurityLevel;

/// The flat coloured-square-with-emblem style Net View's node grid uses,
/// ported from IDApixiTIK's `NetworkDesktop.res` -- the actual "Network View
/// (Debug)" screen behind `WorldScreen.res`'s debug button. This is a
/// different, much simpler style than `spawn_device_icon`'s physical-world
/// props above, which belong to placed world objects, not this abstract
/// graph view.
const NET_ICON_SIZE: f32 = 52.0;
const NET_ICON_BORDER: f32 = 4.0;

/// The square's fill colour, ported from `DeviceTypes.res`'s `getDeviceColor`
/// (the seven original-taxonomy kinds) plus `Firewall`'s own dark red (the
/// source detects "firewall" by an IP heuristic on a plain Router; this
/// taxonomy already has a distinct `Firewall` kind, so it gets the colour
/// directly) and originally-designed neutral fills for every kind with no
/// source art.
fn net_icon_colour(kind: DeviceKind) -> Color {
    match kind {
        DeviceKind::Laptop | DeviceKind::Desktop => Color::srgb_u8(0x21, 0x96, 0xf3),
        DeviceKind::Router | DeviceKind::AccessPoint | DeviceKind::Switch => {
            Color::srgb_u8(0xff, 0x98, 0x00)
        }
        DeviceKind::Firewall => Color::srgb_u8(0xd3, 0x2f, 0x2f),
        DeviceKind::Server => Color::srgb_u8(0x9c, 0x27, 0xb0),
        DeviceKind::IotCamera | DeviceKind::Camera => Color::srgb_u8(0xf4, 0x43, 0x36),
        DeviceKind::Terminal => Color::srgb_u8(0x4c, 0xaf, 0x50),
        DeviceKind::PowerStation | DeviceKind::Substation => Color::srgb_u8(0xff, 0xeb, 0x3b),
        DeviceKind::Ups => Color::srgb_u8(0x79, 0x55, 0x48),
        DeviceKind::SmartDoor => Color::srgb_u8(0x75, 0x75, 0x75),
        DeviceKind::Light => Color::srgb_u8(0xff, 0xf5, 0x9d),
        _ => Color::srgb_u8(0x61, 0x61, 0x61),
    }
}

/// The security-level dot colour, ported verbatim from `getSecurityColor`.
fn net_icon_security_colour(level: SecurityLevel) -> Color {
    match level {
        SecurityLevel::Open => Color::srgb(0.0, 1.0, 0.0),
        SecurityLevel::Weak => Color::srgb(1.0, 1.0, 0.0),
        SecurityLevel::Medium => Color::srgb(1.0, 0.596, 0.0),
        SecurityLevel::Strong => Color::srgb(1.0, 0.0, 0.0),
    }
}

/// The white-silhouette emblem drawn over the coloured square, ported from
/// `createDeviceGraphic` (the seven original kinds) plus original, simple
/// glyphs for `Firewall`, `SmartDoor` and `Light`; every other kind gets none.
fn spawn_net_icon_emblem(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    kind: DeviceKind,
) {
    let white = Color::WHITE;
    match kind {
        DeviceKind::Laptop | DeviceKind::Desktop => {
            parent.spawn((
                Sprite::from_color(white, Vec2::new(20.0, 12.0)),
                Transform::from_translation(Vec3::new(0.0, 3.0, 0.1)),
            ));
            parent.spawn((
                Sprite::from_color(white, Vec2::new(10.0, 1.5)),
                Transform::from_translation(Vec3::new(0.0, -5.0, 0.1)),
            ));
        }
        DeviceKind::Router | DeviceKind::AccessPoint | DeviceKind::Switch => {
            let mesh = meshes.add(Circle::new(7.0));
            let material = materials.add(ColorMaterial::from(white));
            parent.spawn((
                Mesh2d(mesh),
                MeshMaterial2d(material),
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
            ));
            parent.spawn((
                Sprite::from_color(white, Vec2::new(2.0, 8.0)),
                Transform::from_translation(Vec3::new(0.0, 8.0, 0.1)),
            ));
        }
        DeviceKind::Firewall => {
            for (x, y, w) in [
                (0.0, 8.0, 15.0),
                (-4.0, 2.0, 6.0),
                (4.5, 2.0, 8.0),
                (0.0, -4.0, 15.0),
            ] {
                parent.spawn((
                    Sprite::from_color(white, Vec2::new(w, 4.0)),
                    Transform::from_translation(Vec3::new(x, y, 0.1)),
                ));
            }
        }
        DeviceKind::Server => {
            for y in [8.0, 0.0, -8.0] {
                parent.spawn((
                    Sprite::from_color(white, Vec2::new(20.0, 4.0)),
                    Transform::from_translation(Vec3::new(0.0, y, 0.1)),
                ));
            }
        }
        DeviceKind::IotCamera | DeviceKind::Camera => {
            let mesh = meshes.add(Circle::new(6.0));
            let material = materials.add(ColorMaterial::from(white));
            parent.spawn((
                Mesh2d(mesh),
                MeshMaterial2d(material),
                Transform::from_translation(Vec3::new(0.0, 3.0, 0.1)),
            ));
            parent.spawn((
                Sprite::from_color(white, Vec2::new(5.0, 4.0)),
                Transform::from_translation(Vec3::new(0.0, -4.0, 0.1)),
            ));
        }
        DeviceKind::Terminal => {
            parent.spawn((
                Sprite::from_color(Color::BLACK, Vec2::new(25.0, 15.0)),
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
            ));
            parent.spawn((
                Text2d::new(">_"),
                TextFont::from_font_size(10.0),
                TextColor(Color::srgb(0.0, 1.0, 0.0)),
                Transform::from_translation(Vec3::new(-6.0, 0.0, 0.2)),
            ));
        }
        DeviceKind::PowerStation | DeviceKind::Substation => {
            parent.spawn((
                Sprite::from_color(white, Vec2::new(15.0, 17.5)),
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
            ));
            for y in [4.0, -1.0, -6.0] {
                parent.spawn((
                    Sprite::from_color(Color::srgb_u8(0xff, 0xeb, 0x3b), Vec2::new(10.0, 2.5)),
                    Transform::from_translation(Vec3::new(0.0, y, 0.2)),
                ));
            }
        }
        DeviceKind::Ups => {
            parent.spawn((
                Sprite::from_color(white, Vec2::new(17.5, 12.5)),
                Transform::from_translation(Vec3::new(-1.0, 0.0, 0.1)),
            ));
            for x in [-4.0, 4.0] {
                parent.spawn((
                    Sprite::from_color(Color::srgb(0.0, 1.0, 0.0), Vec2::new(5.0, 7.5)),
                    Transform::from_translation(Vec3::new(x, 0.0, 0.2)),
                ));
            }
        }
        DeviceKind::SmartDoor => {
            parent.spawn((
                Sprite::from_color(white, Vec2::new(10.0, 22.0)),
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
            ));
            let mesh = meshes.add(Circle::new(1.5));
            let material = materials.add(ColorMaterial::from(Color::BLACK));
            parent.spawn((
                Mesh2d(mesh),
                MeshMaterial2d(material),
                Transform::from_translation(Vec3::new(3.0, 0.0, 0.2)),
            ));
        }
        DeviceKind::Light => {
            let mesh = meshes.add(Circle::new(6.0));
            let material = materials.add(ColorMaterial::from(white));
            parent.spawn((
                Mesh2d(mesh),
                MeshMaterial2d(material),
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
            ));
        }
        _ => {}
    }
}

/// Spawn a Net View node icon: a bordered coloured square, its emblem, and a
/// security-level dot. Callers add the name label separately, matching the
/// existing pattern in `net_view.rs`'s `setup_net_view`.
pub fn spawn_network_icon(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    kind: DeviceKind,
    security: SecurityLevel,
) {
    let full = NET_ICON_SIZE + NET_ICON_BORDER;
    parent.spawn((
        Sprite::from_color(Color::BLACK, Vec2::splat(full)),
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
    ));
    parent
        .spawn((
            Sprite::from_color(net_icon_colour(kind), Vec2::splat(NET_ICON_SIZE)),
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.05)),
            Visibility::Inherited,
        ))
        .with_children(|square| {
            spawn_net_icon_emblem(square, meshes, materials, kind);
        });
    let dot_mesh = meshes.add(Circle::new(4.0));
    let dot_material = materials.add(ColorMaterial::from(net_icon_security_colour(security)));
    parent.spawn((
        Mesh2d(dot_mesh),
        MeshMaterial2d(dot_material),
        Transform::from_translation(Vec3::new(
            NET_ICON_SIZE * 0.5 - 4.0,
            NET_ICON_SIZE * 0.5 - 4.0,
            0.2,
        )),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crouching_hides_the_laptop_and_tints_the_body_purple() {
        let (color, laptop_shows) = player_colours(true, false);
        assert!(
            !laptop_shows,
            "PlayerGraphics.res hides the laptop while crouching"
        );
        assert_eq!(color, Color::srgb(0.267, 0.333, 0.6));
    }

    #[test]
    fn sprinting_darkens_the_body_and_keeps_the_laptop() {
        let (color, laptop_shows) = player_colours(false, true);
        assert!(laptop_shows);
        assert_eq!(color, Color::srgb(0.133, 0.267, 0.533));
    }

    #[test]
    fn idle_is_the_base_body_colour_with_the_laptop_showing() {
        let (color, laptop_shows) = player_colours(false, false);
        assert!(laptop_shows);
        assert_eq!(color, Color::srgb(0.2, 0.4, 0.8));
    }
}

#[cfg(test)]
mod device_icon_tests {
    use super::*;
    use idaptik_core::device::DeviceKind;

    /// Every `DeviceKind` must spawn something -- this is the whole contract
    /// Net View (Part C) leans on: it never has to match on kind itself.
    #[test]
    fn every_device_kind_spawns_without_panicking() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<ColorMaterial>>();
        let kinds = [
            DeviceKind::Laptop,
            DeviceKind::Router,
            DeviceKind::Server,
            DeviceKind::IotCamera,
            DeviceKind::Terminal,
            DeviceKind::PowerStation,
            DeviceKind::Ups,
            DeviceKind::Firewall,
            DeviceKind::SmartDoor,
            DeviceKind::Camera,
            DeviceKind::Lock,
            DeviceKind::Elevator,
            DeviceKind::Light,
            DeviceKind::Sensor,
            DeviceKind::Substation,
            DeviceKind::PatchPanel,
            DeviceKind::FibreHub,
            DeviceKind::PhoneSystem,
            DeviceKind::AccessPoint,
            DeviceKind::Switch,
            DeviceKind::Desktop,
            DeviceKind::PowerSupply,
        ];
        for kind in kinds {
            app.world_mut()
                .resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
                    world.resource_scope(|world, mut materials: Mut<Assets<ColorMaterial>>| {
                        let mut commands = world.commands();
                        let root = commands.spawn_empty().id();
                        commands.entity(root).with_children(|parent| {
                            spawn_device_icon(parent, &mut meshes, &mut materials, kind);
                        });
                    });
                });
        }
        app.update();
    }
}
