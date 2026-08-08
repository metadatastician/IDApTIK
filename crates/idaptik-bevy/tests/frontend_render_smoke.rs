//! Regression: graphical netplay must install the actual scene, not merely a
//! network-status overlay in an otherwise empty Bevy window.

use bevy::prelude::*;
use idaptik_bevy::FrontendRenderPlugin;
use idaptik_bevy::driver::SimDriverPlugin;
use idaptik_bevy::hud::StatusText;
use idaptik_bevy::scene::PlayerMarker;
use idaptik_core::RunConfig;

#[test]
fn shared_renderer_spawns_camera_player_and_hud() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<ColorMaterial>>()
        .add_plugins(SimDriverPlugin {
            cfg: RunConfig::standard(),
            seed: 123456,
        })
        .add_plugins(FrontendRenderPlugin);

    app.world_mut().run_schedule(Startup);

    let world = app.world_mut();
    let cameras = world.query::<&Camera2d>().iter(world).count();
    let players = world.query::<&PlayerMarker>().iter(world).count();
    let status_lines = world.query::<&StatusText>().iter(world).count();

    assert_eq!(cameras, 1);
    assert_eq!(players, 1);
    assert_eq!(status_lines, 1);
}
