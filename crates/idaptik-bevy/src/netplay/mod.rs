//! Multiplayer netplay module for idaptik-bevy.
//!
//! This module provides the Bevy integration for the delay-lockstep multiplayer
//! protocol from idaptik-net, allowing the GUI frontend to participate in
//! multiplayer sessions over Phoenix Channels.

pub mod input;
pub mod plugin;
pub mod connection;
pub mod ui;

pub use input::BevyInputFeed;
pub use plugin::{NetplayConfig, NetplayMode, NetplayPlugin, NetplayState, ConnectionStatusUi, ConnectionStatusText};
pub use connection::{NetworkMessage, NetplayStatus, IncomingMessage};
pub use ui::NetplayAppState;
