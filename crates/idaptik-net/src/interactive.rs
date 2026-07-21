//! The interactive terminal frontend for a live netplay seat.
//!
//! Reuses `idaptik-tui`'s whole face — [`idaptik_tui::render`] for drawing,
//! [`idaptik_tui::keymap`] + [`idaptik_tui::input`] for key handling — so a
//! networked seat plays exactly like the single-player TUI. The one genuinely
//! new piece is the seam between them: a sampled [`TickInput`] must become
//! wire [`Command`]s (ADR-0005's typed stream), which means held buttons turn
//! into `SetButton` *diffs* against what this seat last sent, edges turn back
//! into their press commands, and everything is filtered down to what this
//! seat's role may say (the relay would reject the rest anyway; filtering
//! here keeps foreign keys inert instead of fatal).

use crate::envelope::{Role, Seat, seat_for};
use crate::live::{LiveFrontend, LiveStatus};
use crate::lockstep::InputFeed;
use crate::seat::ALL_BUTTONS;
use crossterm::event::{
    self, Event as CtEvent, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    supports_keyboard_enhancement,
};
use idaptik_core::scenario::command::{Buttons, Command, Edge};
use idaptik_core::scenario::event::{Event as SimEvent, LogLine};
use idaptik_core::scenario::{GhostLobbySim, log_view};
use idaptik_tui::input::InputState;
use idaptik_tui::keymap::map_key;
use idaptik_tui::render;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;
use std::time::Duration;

/// A live seat's terminal: raw-mode ratatui over stdout, torn down on drop so
/// every exit path (including errors) restores the terminal.
pub struct TerminalFrontend {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    enhanced: bool,
    input: InputState,
    role: Role,
    log: Vec<LogLine>,
    /// The held set this seat last put on the wire — the diff base for
    /// `SetButton`.
    last_sent: Buttons,
    status_line: Option<String>,
}

impl TerminalFrontend {
    pub fn new(role: Role) -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let enhanced = supports_keyboard_enhancement().unwrap_or(false);
        if enhanced {
            let _ = execute!(
                stdout,
                PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
            );
        }
        let terminal = Terminal::new(CrosstermBackend::new(stdout))?;
        Ok(Self {
            terminal,
            enhanced,
            input: InputState::new(),
            role,
            log: Vec::new(),
            last_sent: Buttons::default(),
            status_line: None,
        })
    }

    /// Drain every pending key event into the input state, non-blocking.
    /// Called from both the sample path and the frame path so quitting works
    /// even while blocked on the peer.
    fn poll_keys(&mut self) {
        loop {
            match event::poll(Duration::ZERO) {
                Ok(true) => {
                    if let Ok(CtEvent::Key(key)) = event::read() {
                        self.input.apply(map_key(key));
                    }
                }
                Ok(false) => break,
                Err(_) => {
                    self.input.quit = true;
                    break;
                }
            }
        }
    }
}

impl Drop for TerminalFrontend {
    fn drop(&mut self) {
        if self.enhanced {
            let _ = execute!(self.terminal.backend_mut(), PopKeyboardEnhancementFlags);
        }
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

impl InputFeed for TerminalFrontend {
    fn commands_for(&mut self, _at: u64) -> Vec<Command> {
        self.poll_keys();
        let ti = self.input.sample();

        let mut out = Vec::new();
        for b in ALL_BUTTONS {
            if self.last_sent.has(b) != ti.buttons.has(b) {
                out.push(Command::SetButton {
                    button: b,
                    down: ti.buttons.has(b),
                });
            }
        }
        for e in ti.edges {
            out.push(match e {
                Edge::JumpUp => Command::Jump,
                Edge::InteractPress => Command::Interact,
                Edge::Throw => Command::ThrowUsb,
                Edge::Uplink(kind) => Command::Uplink { kind },
            });
        }
        out.extend(ti.immediates);

        let mine = match self.role {
            Role::Infiltrator => Seat::Infiltrator,
            Role::Hacker => Seat::Hacker,
        };
        out.retain(|cmd| matches!(seat_for(cmd), Seat::Either) || seat_for(cmd) == mine);

        // Advance the diff base only past what survived the filter (a
        // hacker's movement keys must stay inert, not accumulate).
        if self.role == Role::Infiltrator {
            self.last_sent = ti.buttons;
        }
        out
    }
}

impl LiveFrontend for TerminalFrontend {
    fn frame(&mut self, sim: &GhostLobbySim, fresh: &[SimEvent]) -> bool {
        self.poll_keys();
        if self.input.quit {
            return false;
        }
        let tick = sim.current_tick();
        let t = sim.time();
        for e in fresh {
            if matches!(e, SimEvent::Restarted { .. }) {
                self.log.clear();
            }
            if let Some(line) = log_view(e, tick, t) {
                self.log.push(line);
            }
        }
        if self.log.len() > 500 {
            let drop = self.log.len() - 500;
            self.log.drain(0..drop);
        }
        let log = &self.log;
        let hint = self.status_line.as_deref();
        self.terminal
            .draw(|f| render::draw(f, sim, log, hint))
            .is_ok()
    }

    fn status(&mut self, status: LiveStatus) {
        self.status_line = match status {
            LiveStatus::WaitingForPeer => Some("waiting for the other seat to join…".to_owned()),
            LiveStatus::Live => None,
            LiveStatus::PeerLost => {
                Some("the other seat was lost — holding the run for a rejoin…".to_owned())
            }
            LiveStatus::Resyncing => Some("the other seat is back — resyncing…".to_owned()),
        };
    }
}
