//! Multiplayer netplay module for idaptik-bevy.
//!
//! This module provides the Bevy integration for the delay-lockstep multiplayer
//! protocol from idaptik-net, allowing the GUI frontend to participate in
//! multiplayer sessions over Phoenix Channels.

pub mod connection;
pub mod input;
pub mod plugin;
pub mod ui;

pub use connection::{IncomingMessage, NetplayStatus, NetworkMessage};
pub use input::BevyInputFeed;
pub use plugin::{
    ConnectionStatusText, ConnectionStatusUi, NetplayConfig, NetplayMode, NetplayPlugin,
    NetplayState,
};
pub use ui::NetplayAppState;
