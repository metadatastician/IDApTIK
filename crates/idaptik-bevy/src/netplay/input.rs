//! Bevy input feed for the lockstep protocol.
//!
//! This module provides a Bevy-compatible implementation of the InputFeed
//! trait from idaptik-net, capturing keyboard input and converting it to
//! Command streams for the delay-lockstep protocol.

use bevy::prelude::*;
use idaptik_core::scenario::command::Command;
use idaptik_net::lockstep::InputFeed;
use std::collections::BTreeMap;

use crate::driver::CommandQueue;

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
    /// The canonical Bevy decoder's held/pause state and pending commands.
    command_queue: CommandQueue,
}

impl BevyInputFeed {
    /// Create a new Bevy input feed
    pub fn new() -> Self {
        Self {
            pending: BTreeMap::new(),
            current_tick: 0,
            command_queue: CommandQueue::default(),
        }
    }

    /// Queue commands for the current tick
    pub fn queue_commands(&mut self, commands: Vec<Command>) {
        self.pending
            .entry(self.current_tick)
            .or_default()
            .extend(commands);
    }

    /// Advance to the next tick
    pub fn advance_tick(&mut self) {
        self.current_tick += 1;
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
    mut exit: MessageWriter<AppExit>,
) {
    // Reuse local Bevy's canonical decoder so multiplayer cannot silently
    // lose movement presses, interaction holds, pivots, pause or restart.
    let quit = crate::keymap::decode(&keyboard, &mut feed.command_queue);
    let commands = std::mem::take(&mut feed.command_queue.pending);

    // Queue commands for current tick
    if !commands.is_empty() {
        feed.queue_commands(commands);
    }
    if quit {
        exit.write(AppExit::Success);
    }
}

/// System to advance the input feed tick counter
///
/// Called at the start of each lockstep tick to move to the next tick.
pub fn advance_input_feed_tick(mut feed: ResMut<BevyInputFeed>) {
    feed.advance_tick();
}

#[cfg(test)]
mod tests {
    use super::*;
    use idaptik_core::scenario::command::{Button, Command};

    #[test]
    fn movement_press_and_release_enter_the_lockstep_feed() {
        let mut feed = BevyInputFeed::new();
        let mut keys = ButtonInput::<KeyCode>::default();

        keys.press(KeyCode::KeyA);
        let quit = crate::keymap::decode(&keys, &mut feed.command_queue);
        assert!(!quit);
        let pressed = std::mem::take(&mut feed.command_queue.pending);
        feed.queue_commands(pressed);
        assert_eq!(
            feed.commands_for(0),
            vec![Command::SetButton {
                button: Button::Left,
                down: true,
            }]
        );

        keys.clear();
        keys.release(KeyCode::KeyA);
        crate::keymap::decode(&keys, &mut feed.command_queue);
        let released = std::mem::take(&mut feed.command_queue.pending);
        feed.queue_commands(released);
        assert_eq!(
            feed.commands_for(0),
            vec![Command::SetButton {
                button: Button::Left,
                down: false,
            }]
        );
    }
}
