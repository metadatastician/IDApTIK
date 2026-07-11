//! Persistent input state: a held-button set carried across frames plus queued
//! edge/immediate commands, folded into one [`TickInput`] per simulation tick.

use crate::keymap::Intent;
use idaptik_core::scenario::command::{Buttons, Command, Edge, TickInput};

/// Accumulated frontend input between simulation ticks.
#[derive(Default)]
pub struct InputState {
    held: Buttons,
    edges: Vec<Edge>,
    immediates: Vec<Command>,
    paused: bool,
    /// Frontend-only: the user asked for a hint this frame.
    pub hint: bool,
    /// Frontend-only: the user asked to quit.
    pub quit: bool,
}

impl InputState {
    /// Fresh input state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply decoded intents, updating the held set and queuing commands.
    pub fn apply(&mut self, intents: Vec<Intent>) {
        for intent in intents {
            match intent {
                Intent::Hold(button, down) => self.held.set(button, down),
                Intent::Edge(cmd) => match cmd {
                    Command::Jump => self.edges.push(Edge::JumpUp),
                    Command::Interact => self.edges.push(Edge::InteractPress),
                    Command::ThrowUsb => self.edges.push(Edge::Throw),
                    Command::Uplink { .. } => self.immediates.push(cmd),
                    _ => {}
                },
                Intent::TogglePause => {
                    self.paused = !self.paused;
                    self.immediates.push(Command::Pause { on: self.paused });
                }
                Intent::Restart => {
                    self.paused = false;
                    self.immediates.push(Command::Restart);
                }
                Intent::Hint => self.hint = true,
                Intent::Quit => self.quit = true,
                Intent::Ignore => {}
            }
        }
    }

    /// Sample one tick's input, draining queued edges/immediates (held persists).
    pub fn sample(&mut self) -> TickInput {
        TickInput {
            buttons: self.held,
            edges: std::mem::take(&mut self.edges),
            immediates: std::mem::take(&mut self.immediates),
        }
    }
}
