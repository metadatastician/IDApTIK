//! Bevy frontend for IDApTIK (evaluation — see ADR-0003).
//!
//! Deliberately thin: it owns rendering/input only and treats [`idaptik_core`]
//! as the source of gameplay truth. The frontend speaks the exact same
//! [`idaptik_core::scenario::Command`] / [`idaptik_core::scenario::Event`] wire
//! API the TUI uses: keyboard input becomes `Command`s, a fixed 60 Hz step
//! folds them into a `TickInput` and calls `GhostLobbySim::tick`, and every
//! visual is a pure view over the returned state and events. No game logic
//! lives here.
//!
//! * [`driver`] — the sim resource and the fixed-rate tick. Render-free, so a
//!   headless test `App` can drive it without a window (see `tests/parity.rs`).
//! * [`keymap`] — Bevy keyboard events → `Command`s, mirroring the canonical
//!   bindings in `idaptik-tui/src/keymap.rs`.
//! * [`scene`] — the side-on 2.5D cross-section: rooms as a row of flat quads
//!   (laid out data-driven from the room definitions so a second floor row
//!   slots in above), door slabs, camera view cones, the player and Billy.
//! * [`hud`] — status line, meter bars, event log tail and the result overlay.

pub mod driver;
pub mod hud;
pub mod keymap;
pub mod scene;

use bevy::prelude::*;

/// Everything the windowed frontend adds on top of [`driver::SimDriverPlugin`]:
/// input decoding, the scene, and the HUD.
pub struct FrontendPlugin;

impl Plugin for FrontendPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (scene::setup_scene, hud::setup_hud))
            .add_systems(
                // Decode the keyboard right before the fixed main loop so a
                // press lands on this frame's tick, not the next frame's.
                RunFixedMainLoop,
                keymap::keyboard_input.in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
            )
            .add_systems(
                Update,
                (
                    scene::sync_doors,
                    scene::sync_player,
                    scene::sync_billy,
                    scene::sync_props,
                    scene::draw_camera_cones,
                    hud::update_meters,
                    hud::update_status_text,
                    hud::update_log_text,
                    hud::update_result_text,
                ),
            );
    }
}
