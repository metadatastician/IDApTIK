//! Fyrox frontend for IDApTIK (evaluation — see ADR-0003).
//!
//! Currently a headless bring-up that loads the shared core and links the Fyrox
//! engine. Standing up the Fyrox `Executor` + `Plugin` is the next step (sketched
//! below); like the Bevy frontend it must stay thin — gameplay truth lives in
//! [`idaptik_core`], not here.

use fyrox::core::algebra::Vector2;
use idaptik_core::demo_network;

fn main() {
    let net = demo_network();

    // A trivial use of a stable Fyrox type so the engine dependency is real and
    // linked while the full app is scaffolded.
    let origin = Vector2::new(0.0_f32, 0.0_f32);
    println!(
        "IDApTIK/fyrox: loaded network with {} devices (origin {origin:?})",
        net.len()
    );

    // TODO(ADR-0003): stand up the Fyrox Executor + Plugin, e.g.
    //   let mut executor = fyrox::engine::executor::Executor::new();
    //   executor.add_plugin(Game::default());
    //   executor.run();
}
