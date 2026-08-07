//! Persistent input state: a held-button set carried across frames plus queued
//! edge/immediate commands, folded into one [`TickInput`] per simulation tick.

use crate::keymap::Intent;
use idaptik_core::scenario::command::{Button, Buttons, Command, Edge, TickInput};

const HELD_BUTTONS: [Button; 5] = [
    Button::Left,
    Button::Right,
    Button::Crouch,
    Button::Sprint,
    Button::Interact,
];

/// Non-enhanced terminals do not report key releases. Keep a held key alive
/// across a short gap between OS key-repeat events, then synthesize release.
const FALLBACK_HOLD_TICKS: u8 = 24;

/// Accumulated frontend input between simulation ticks.
#[derive(Default)]
pub struct InputState {
    held: Buttons,
    keyboard_enhanced: bool,
    fallback_ticks: [u8; HELD_BUTTONS.len()],
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
        Self::with_keyboard_enhancement(true)
    }

    /// Fresh input state for a terminal with or without release events.
    pub fn with_keyboard_enhancement(keyboard_enhanced: bool) -> Self {
        Self {
            keyboard_enhanced,
            ..Self::default()
        }
    }

    /// Apply decoded intents, updating the held set and queuing commands.
    pub fn apply(&mut self, intents: Vec<Intent>) {
        for intent in intents {
            match intent {
                Intent::Hold(button, down) => self.set_held(button, down),
                Intent::Edge(cmd) => match cmd {
                    Command::Jump => self.edges.push(Edge::JumpUp),
                    Command::Interact => self.edges.push(Edge::InteractPress),
                    Command::ThrowUsb => self.edges.push(Edge::Throw),
                    // A pivot lands where an uplink lands: before the systems, at
                    // the pre-increment `t`, since every one of them reads the
                    // vantage it moves.
                    Command::Uplink { .. } | Command::Pivot { .. } | Command::Unpivot => {
                        self.immediates.push(cmd)
                    }
                    // Deliberately non-exhaustive: SetButton/ForceCrisis/ForceExtract/
                    // ForceFail/Pause/Restart are handled elsewhere in `apply`, and
                    // NetSsh/NetHack are Net View's Bevy-only click commands, which
                    // this TUI never constructs -- safe to drop here today, but see
                    // `idaptik-net::envelope::seat_for` for the exhaustive-match
                    // alternative if that ever needs to change.
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
        self.decay_fallback_holds();
        let input = TickInput {
            buttons: self.held,
            edges: std::mem::take(&mut self.edges),
            immediates: std::mem::take(&mut self.immediates),
        };
        input
    }

    fn set_held(&mut self, button: Button, down: bool) {
        self.held.set(button, down);
        if !self.keyboard_enhanced {
            self.fallback_ticks[button_index(button)] = if down { FALLBACK_HOLD_TICKS } else { 0 };
        }
    }

    fn decay_fallback_holds(&mut self) {
        if self.keyboard_enhanced {
            return;
        }
        for (idx, button) in HELD_BUTTONS.into_iter().enumerate() {
            if self.fallback_ticks[idx] == 0 {
                continue;
            }
            self.fallback_ticks[idx] -= 1;
            if self.fallback_ticks[idx] == 0 {
                self.held.set(button, false);
            }
        }
    }
}

fn button_index(button: Button) -> usize {
    match button {
        Button::Left => 0,
        Button::Right => 1,
        Button::Crouch => 2,
        Button::Sprint => 3,
        Button::Interact => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(state: &mut InputState, button: Button) -> bool {
        state.sample().buttons.has(button)
    }

    #[test]
    fn enhanced_terminals_hold_until_release() {
        let mut input = InputState::with_keyboard_enhancement(true);

        input.apply(vec![Intent::Hold(Button::Left, true)]);
        for _ in 0..(FALLBACK_HOLD_TICKS as usize + 2) {
            assert!(held(&mut input, Button::Left));
        }

        input.apply(vec![Intent::Hold(Button::Left, false)]);
        assert!(!held(&mut input, Button::Left));
    }

    #[test]
    fn non_enhanced_terminals_release_stale_held_keys() {
        let mut input = InputState::with_keyboard_enhancement(false);

        input.apply(vec![Intent::Hold(Button::Left, true)]);
        for _ in 0..(FALLBACK_HOLD_TICKS - 1) {
            assert!(held(&mut input, Button::Left));
        }
        assert!(!held(&mut input, Button::Left));
    }

    #[test]
    fn non_enhanced_repeat_refreshes_the_fallback_hold() {
        let mut input = InputState::with_keyboard_enhancement(false);

        input.apply(vec![Intent::Hold(Button::Right, true)]);
        for _ in 0..(FALLBACK_HOLD_TICKS - 1) {
            assert!(held(&mut input, Button::Right));
        }

        input.apply(vec![Intent::Hold(Button::Right, true)]);
        for _ in 0..(FALLBACK_HOLD_TICKS - 1) {
            assert!(held(&mut input, Button::Right));
        }
        assert!(!held(&mut input, Button::Right));
    }

    #[test]
    fn non_enhanced_opposing_keys_do_not_latch_into_paralysis() {
        let mut input = InputState::with_keyboard_enhancement(false);

        input.apply(vec![Intent::Hold(Button::Left, true)]);
        assert!(held(&mut input, Button::Left));
        for _ in 0..(FALLBACK_HOLD_TICKS - 1) {
            input.sample();
        }

        input.apply(vec![Intent::Hold(Button::Right, true)]);
        let sample = input.sample();
        assert!(!sample.buttons.has(Button::Left));
        assert!(sample.buttons.has(Button::Right));
    }

    #[test]
    fn non_enhanced_release_still_clears_immediately_if_reported() {
        let mut input = InputState::with_keyboard_enhancement(false);

        input.apply(vec![Intent::Hold(Button::Crouch, true)]);
        assert!(held(&mut input, Button::Crouch));
        input.apply(vec![Intent::Hold(Button::Crouch, false)]);
        assert!(!held(&mut input, Button::Crouch));
    }
}
