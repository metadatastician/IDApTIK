//! Bevy frontend for IDApTIK (evaluation — see ADR-0003).
//!
//! Deliberately thin: it owns rendering/input only and treats
//! [`idaptik_core`] as the source of gameplay truth. No game logic here.

use bevy::prelude::*;
use idaptik_core::{Network, demo_network};

/// The authoritative network, wrapped so Bevy can hold it as a resource.
#[derive(Resource)]
struct GameNetwork(Network);

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(GameNetwork(demo_network()))
        .add_systems(Startup, report_network)
        .run()
}

/// Placeholder bring-up: prove the core is wired in by logging the loaded
/// network. Real scenes/entities replace this as the Envelope milestone fills in.
fn report_network(net: Res<GameNetwork>) {
    info!(
        "IDApTIK/bevy: loaded network with {} devices",
        net.0.len()
    );
    for device in net.0.devices() {
        info!("  {} [{}] {:?}", device.name, device.ip, device.kind);
    }
}
