//! crossterm key events → high-level [`Intent`]s using the HTML key bindings.
//!
//! Held movement (A/D/S/Shift and E) is tracked via `Press`/`Release` when the
//! terminal reports them (keyboard-enhancement flags are requested at startup);
//! otherwise it degrades to a held-until-next-frame model driven by key repeat.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use idaptik_core::scenario::ActionKind;
use idaptik_core::scenario::command::{Button, Command};

/// A decoded high-level input intent.
#[derive(Debug, Clone, PartialEq)]
pub enum Intent {
    /// Press or release a held movement button.
    Hold(Button, bool),
    /// Fire an edge/uplink command this frame.
    Edge(Command),
    /// Toggle pause.
    TogglePause,
    /// Restart from the same seed.
    Restart,
    /// Show a frontend hint.
    Hint,
    /// Quit the frontend.
    Quit,
    /// No mapping.
    Ignore,
}

/// Map a crossterm key event to zero or more intents.
pub fn map_key(ev: KeyEvent) -> Vec<Intent> {
    let down = matches!(ev.kind, KeyEventKind::Press | KeyEventKind::Repeat);
    let is_press = ev.kind == KeyEventKind::Press;

    // Ctrl-C always quits.
    if ev.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(ev.code, KeyCode::Char('c') | KeyCode::Char('C'))
    {
        return vec![Intent::Quit];
    }

    match ev.code {
        KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Left => {
            vec![Intent::Hold(Button::Left, down)]
        }
        KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Right => {
            vec![Intent::Hold(Button::Right, down)]
        }
        KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Down => {
            vec![Intent::Hold(Button::Crouch, down)]
        }
        KeyCode::Char('e') | KeyCode::Char('E') => {
            // Interact is BOTH a held button and an edge press.
            let mut out = vec![Intent::Hold(Button::Interact, down)];
            if is_press {
                out.push(Intent::Edge(Command::Interact));
            }
            out
        }
        KeyCode::Char('w') | KeyCode::Char('W') | KeyCode::Up | KeyCode::Char(' ') => {
            if is_press {
                vec![Intent::Edge(Command::Jump)]
            } else {
                vec![Intent::Ignore]
            }
        }
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            if is_press {
                vec![Intent::Edge(Command::ThrowUsb)]
            } else {
                vec![Intent::Ignore]
            }
        }
        KeyCode::Char('1') => uplink(is_press, ActionKind::Camera),
        KeyCode::Char('2') => uplink(is_press, ActionKind::Door),
        KeyCode::Char('3') => uplink(is_press, ActionKind::Vacuum),
        KeyCode::Char('4') => uplink(is_press, ActionKind::Lights),
        KeyCode::Char('p') | KeyCode::Char('P') if is_press => vec![Intent::TogglePause],
        KeyCode::Char('r') | KeyCode::Char('R') if is_press => vec![Intent::Restart],
        KeyCode::Char('h') | KeyCode::Char('H') if is_press => vec![Intent::Hint],
        KeyCode::Esc if is_press => vec![Intent::Quit],
        // Shift is a modifier for sprint: treat its own key events as sprint.
        _ if ev.modifiers.contains(KeyModifiers::SHIFT) => {
            vec![Intent::Hold(Button::Sprint, down)]
        }
        _ => vec![Intent::Ignore],
    }
}

fn uplink(is_press: bool, kind: ActionKind) -> Vec<Intent> {
    if is_press {
        vec![Intent::Edge(Command::Uplink { kind })]
    } else {
        vec![Intent::Ignore]
    }
}
