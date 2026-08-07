//! Bevy input feed for the lockstep protocol.
//!
//! This module provides a Bevy-compatible implementation of the InputFeed
//! trait from idaptik-net, capturing keyboard input and converting it to
//! Command streams for the delay-lockstep protocol.

use bevy::prelude::*;
use idaptik_core::scenario::command::{Buttons, Command, Button};
use idaptik_net::lockstep::InputFeed;
use std::collections::BTreeMap;

/// Bevy-compatible input feed for lockstep.
///
/// This struct captures keyboard input from Bevy and provides it to the
/// lockstep core via the InputFeed trait.
#[derive(Resource, Default)]
pub struct BevyInputFeed {
    /// Commands queued for each tick
    pending: BTreeMap<u64, Vec<Command>>,
    /// Current tick being sampled
    current_tick: u64,
    /// Held buttons state (persistent across ticks)
    held: Buttons,
}

impl BevyInputFeed {
    /// Create a new Bevy input feed
    pub fn new() -> Self {
        Self {
            pending: BTreeMap::new(),
            current_tick: 0,
            held: Buttons::default(),
        }
    }

    /// Queue commands for the current tick
    pub fn queue_commands(&mut self, commands: Vec<Command>) {
        self.pending.entry(self.current_tick).or_default().extend(commands);
    }

    /// Advance to the next tick
    pub fn advance_tick(&mut self) {
        self.current_tick += 1;
    }

    /// Update held buttons from keyboard state
    pub fn update_held(&mut self, keyboard: &Res<ButtonInput<KeyCode>>) {
        // Update held buttons based on current keyboard state
        // This is called each frame to update the button state
        let mut new_held = Buttons::default();
        
        // Check each button's key
        if keyboard.pressed(KeyCode::ArrowLeft) || keyboard.pressed(KeyCode::KeyA) {
            new_held.set(Button::Left, true);
        }
        if keyboard.pressed(KeyCode::ArrowRight) || keyboard.pressed(KeyCode::KeyD) {
            new_held.set(Button::Right, true);
        }
        if keyboard.pressed(KeyCode::ArrowDown) || keyboard.pressed(KeyCode::KeyS) {
            new_held.set(Button::Crouch, true);
        }
        if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
            new_held.set(Button::Sprint, true);
        }
        
        self.held = new_held;
    }
}

impl InputFeed for BevyInputFeed {
    fn commands_for(&mut self, at: u64) -> Vec<Command> {
        // Return commands for the specified tick and remove them from the queue
        self.pending.remove(&at).unwrap_or_default()
    }
}

/// System to capture keyboard input for lockstep
///
/// This system reads keyboard events and converts them to commands,
/// then queues them in the BevyInputFeed for the lockstep core to consume.
pub fn capture_lockstep_input(
    mut feed: ResMut<BevyInputFeed>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    let mut commands = Vec::new();

    // Update held button state
    feed.update_held(&keyboard);

    // Check for just-pressed keys (edge commands)
    for key in keyboard.get_just_pressed() {
        if let Some(cmd) = key_to_edge_command(*key) {
            commands.push(cmd);
        }
    }

    // Check for just-released keys (button release)
    for key in keyboard.get_just_released() {
        if let Some(button) = key_to_button(*key) {
            commands.push(Command::SetButton {
                button,
                down: false,
            });
        }
    }

    // Queue commands for current tick
    if !commands.is_empty() {
        feed.queue_commands(commands);
    }
}

/// Convert a key to an edge command (pressed once)
fn key_to_edge_command(key: KeyCode) -> Option<Command> {
    use idaptik_core::scenario::ActionKind;
    match key {
        KeyCode::Space => Some(Command::Jump),
        KeyCode::KeyE => Some(Command::Interact),
        KeyCode::KeyQ => Some(Command::ThrowUsb),
        KeyCode::Digit1 => Some(Command::Uplink {
            kind: ActionKind::Camera,
        }),
        KeyCode::Digit2 => Some(Command::Uplink {
            kind: ActionKind::Door,
        }),
        KeyCode::Digit3 => Some(Command::Uplink {
            kind: ActionKind::Vacuum,
        }),
        KeyCode::Digit4 => Some(Command::Uplink {
            kind: ActionKind::Lights,
        }),
        KeyCode::Escape => Some(Command::Pause { on: true }),
        KeyCode::KeyR => Some(Command::Restart),
        _ => None,
    }
}

/// Convert a key to a button for held state
fn key_to_button(key: KeyCode) -> Option<Button> {
    match key {
        KeyCode::ArrowLeft | KeyCode::KeyA => Some(Button::Left),
        KeyCode::ArrowRight | KeyCode::KeyD => Some(Button::Right),
        KeyCode::ArrowDown | KeyCode::KeyS => Some(Button::Crouch),
        KeyCode::ShiftLeft | KeyCode::ShiftRight => Some(Button::Sprint),
        _ => None,
    }
}

/// System to advance the input feed tick counter
///
/// Called at the start of each lockstep tick to move to the next tick.
pub fn advance_input_feed_tick(mut feed: ResMut<BevyInputFeed>) {
    feed.advance_tick();
}
