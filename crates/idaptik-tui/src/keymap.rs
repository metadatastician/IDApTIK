//! crossterm key events → high-level [`Intent`]s using the HTML key bindings.
//!
//! Held movement (A/D/S/Shift and E) is tracked via `Press`/`Release` when the
//! terminal reports them (keyboard-enhancement flags are requested at startup);
//! otherwise it degrades to a held-until-next-frame model driven by key repeat.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use idaptik_core::scenario::ActionKind;
use idaptik_core::scenario::command::{Button, Command, PivotTarget};

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
        // The pivots. Case is significant here and nowhere else on this keyboard:
        // `p` and `P` are the floor's two lines, and they are the one pair of verbs
        // a player must not confuse, so they sit on the same finger. `g` is the
        // upstream line's second hop, without which the substation is unreachable
        // and the power-line strategy is unplayable.
        KeyCode::Char('p') => pivot(is_press, PivotTarget::Bridge),
        KeyCode::Char('P') => pivot(is_press, PivotTarget::IspOps),
        KeyCode::Char('g') | KeyCode::Char('G') => pivot(is_press, PivotTarget::GridJump),
        KeyCode::Char('x') | KeyCode::Char('X') if is_press => {
            vec![Intent::Edge(Command::Unpivot)]
        }
        // Pause moved off `p` to make room for the pivots; `p` is the verb a
        // player reaches for a hundred times a run, pause perhaps twice.
        KeyCode::Tab if is_press => vec![Intent::TogglePause],
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

fn pivot(is_press: bool, target: PivotTarget) -> Vec<Intent> {
    if is_press {
        vec![Intent::Edge(Command::Pivot { target })]
    } else {
        vec![Intent::Ignore]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: KeyCode) -> Vec<Intent> {
        map_key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn the_pivot_keys_map_to_the_footholds_they_name() {
        // Both lines, and both depths of the upstream one. Binding `P` without `g`
        // would ship the substation strategy unreachable, so all three are pinned.
        assert_eq!(
            press(KeyCode::Char('p')),
            vec![Intent::Edge(Command::Pivot {
                target: PivotTarget::Bridge
            })]
        );
        assert_eq!(
            map_key(KeyEvent::new(KeyCode::Char('P'), KeyModifiers::SHIFT)),
            vec![Intent::Edge(Command::Pivot {
                target: PivotTarget::IspOps
            })],
            "shifted P must reach the ISP, not fall through to the sprint arm"
        );
        assert_eq!(
            press(KeyCode::Char('g')),
            vec![Intent::Edge(Command::Pivot {
                target: PivotTarget::GridJump
            })]
        );
        assert_eq!(
            press(KeyCode::Char('x')),
            vec![Intent::Edge(Command::Unpivot)]
        );
    }

    #[test]
    fn pause_answers_to_tab_now_that_p_pivots() {
        assert_eq!(press(KeyCode::Tab), vec![Intent::TogglePause]);
        assert!(
            !press(KeyCode::Char('p')).contains(&Intent::TogglePause),
            "p must no longer pause, or the pivot is unreachable"
        );
    }
}
