//! Ratatui side-view rendering of the Ghost Lobby runtime.

use idaptik_core::scenario::GhostLobbySim;
use idaptik_core::scenario::common::{Channel, Severity};
use idaptik_core::scenario::event::LogLine;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap};

const STRIP_WIDTH: usize = 72;

/// Draw the whole frame.
pub fn draw(frame: &mut Frame, sim: &GhostLobbySim, log: &[LogLine], hint: Option<&str>) {
    let area = frame.area();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // HUD
            Constraint::Length(3), // room strip
            Constraint::Length(6), // meters
            Constraint::Length(6), // belief + objectives
            Constraint::Min(4),    // log
            Constraint::Length(4), // key hints: body / uplink / session / hint
        ])
        .split(area);

    draw_hud(frame, rows[0], sim);
    draw_strip(frame, rows[1], sim);
    draw_meters(frame, rows[2], sim);
    draw_belief_and_objectives(frame, rows[3], sim);
    draw_log(frame, rows[4], log);
    draw_footer(frame, rows[5], hint);

    if sim.is_ended() {
        draw_result_overlay(frame, area, sim);
    }
}

fn draw_hud(frame: &mut Frame, area: Rect, sim: &GhostLobbySim) {
    let s = sim.state();
    let secs = s.t as u64;
    let text = format!(
        "Envelope 001 — Ghost Lobby   phase {:?}   {:02}:{:02}   Billy: {:?}   seed {:08X}",
        s.phase,
        secs / 60,
        secs % 60,
        s.billy.mode,
        sim.seed(),
    );
    let p = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("STATUS"));
    frame.render_widget(p, area);
}

fn draw_strip(frame: &mut Frame, area: Rect, sim: &GhostLobbySim) {
    let def = sim.definition();
    let s = sim.state();
    let width = def.world.width.max(1.0);
    let col = |x: f64| -> usize {
        ((x / width) * STRIP_WIDTH as f64).clamp(0.0, (STRIP_WIDTH - 1) as f64) as usize
    };
    let mut lane = [b'.'; STRIP_WIDTH];
    // Doors.
    for door in &s.doors {
        let c = col(door.x);
        lane[c] = if door.open > 0.0 { b'|' } else { b'#' };
    }
    // Props.
    if !s.chute.revealed {
        lane[col(s.chute.x)] = b'?';
    } else {
        lane[col(s.chute.x)] = b'v';
    }
    if s.vacuum.active {
        lane[col(s.vacuum.x)] = b'o';
    }
    // Billy and player last so they win.
    if s.billy.mode != idaptik_core::scenario::common::BillyMode::Offsite {
        lane[col(s.billy.x)] = b'B';
    }
    lane[col(s.player.x)] = b'P';
    let strip: String = lane.iter().map(|&b| b as char).collect();
    let p = Paragraph::new(strip).block(
        Block::default()
            .borders(Borders::ALL)
            .title("KITCHEN | HALL | OFFICE | LAUNDRY | EXIT"),
    );
    frame.render_widget(p, area);
}

fn draw_meters(frame: &mut Frame, area: Rect, sim: &GhostLobbySim) {
    let s = sim.state();
    // The row keeps the height it always had: the gauges give up their spare
    // three lines to the uplink readout beneath them rather than the log giving
    // up any of its own.
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3)])
        .split(area);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20); 5])
        .split(rows[0]);
    gauge(frame, cols[0], "support", s.support, Color::Green);
    gauge(
        frame,
        cols[1],
        "bandwidth",
        s.bandwidth / 100.0,
        Color::Cyan,
    );
    gauge(frame, cols[2], "alert", s.alert / 100.0, Color::Red);
    let iso = (s.isolation / sim_support_limit(sim)).clamp(0.0, 1.0);
    gauge(frame, cols[3], "isolation", iso, Color::Magenta);
    // Already a fraction of its own threshold, which is a per-difficulty knob:
    // dividing it by anything here would be dividing it twice.
    gauge(
        frame,
        cols[4],
        "trace",
        f64::from(s.agents.hacker.trace_fraction()),
        Color::Yellow,
    );
    draw_uplink(frame, rows[1], sim);
}

/// Where the hacker is playing from, how deep they have reached, and how much of
/// the floor answers them there. Cold from the van that last number is what says
/// why every uplink is being refused, so it is the readout the pivot keys exist
/// to move.
fn draw_uplink(frame: &mut Frame, area: Rect, sim: &GhostLobbySim) {
    let hacker = &sim.state().agents.hacker;
    let text = format!(
        "vantage {:?}     pivot depth {}     nodes reachable {}",
        hacker.vantage().kind,
        hacker.hops(),
        hacker.reachable(sim.graph()).len(),
    );
    frame.render_widget(
        Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .title("HACKER UPLINK"),
        ),
        area,
    );
}

fn sim_support_limit(sim: &GhostLobbySim) -> f64 {
    let cfg = sim.config();
    sim.definition()
        .difficulty
        .get(&cfg.difficulty)
        .map(|p| p.support_limit)
        .unwrap_or(1.0)
        .max(0.001)
}

fn gauge(frame: &mut Frame, area: Rect, title: &str, ratio: f64, color: Color) {
    let g = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title.to_string()),
        )
        .gauge_style(Style::default().fg(color))
        .ratio(ratio.clamp(0.0, 1.0));
    frame.render_widget(g, area);
}

fn draw_belief_and_objectives(frame: &mut Frame, area: Rect, sim: &GhostLobbySim) {
    let s = sim.state();
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let belief = vec![
        Line::from(format!(
            "note interest   {:6.1}   (belief @ 48)",
            s.billy.note_interest
        )),
        Line::from(format!("usb  interest   {:6.1}", s.billy.usb_interest)),
        Line::from(format!("player interest {:6.1}", s.billy.player_interest)),
        Line::from(format!("belief: {:?}", s.billy.belief)),
    ];
    frame.render_widget(
        Paragraph::new(belief).block(Block::default().borders(Borders::ALL).title("BILLY BELIEF")),
        cols[0],
    );

    let (note, misdirect, exit) = s.objective_ledger;
    let obj = vec![
        Line::from(format!("1  contact note   {note:?}")),
        Line::from(format!("2  misdirect USB  {misdirect:?}")),
        Line::from(format!("3  extraction     {exit:?}")),
    ];
    frame.render_widget(
        Paragraph::new(obj).block(Block::default().borders(Borders::ALL).title("OBJECTIVES")),
        cols[1],
    );
}

fn severity_color(sev: Severity) -> Color {
    match sev {
        Severity::Info => Color::Gray,
        Severity::Good => Color::Green,
        Severity::Warn => Color::Yellow,
        Severity::Bad => Color::Red,
        Severity::Billy => Color::Magenta,
        Severity::Hacker => Color::Cyan,
    }
}

fn draw_log(frame: &mut Frame, area: Rect, log: &[LogLine]) {
    let visible = (area.height.saturating_sub(2) as usize).max(1);
    let filtered: Vec<&LogLine> = log
        .iter()
        .filter(|l| matches!(l.channel, Channel::Log | Channel::Telemetry))
        .collect();
    let start = filtered.len().saturating_sub(visible);
    let items: Vec<ListItem> = filtered[start..]
        .iter()
        .map(|l| {
            let secs = l.t as u64;
            let stamp = format!("{:02}:{:02} ", secs / 60, secs % 60);
            ListItem::new(Line::from(vec![
                Span::styled(stamp, Style::default().fg(Color::DarkGray)),
                Span::styled(
                    l.text.clone(),
                    Style::default().fg(severity_color(l.severity)),
                ),
            ]))
        })
        .collect();
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("EVENT LOG"));
    frame.render_widget(list, area);
}

/// The key hints, one line per audience: the body, the uplink, the session.
///
/// One line per audience rather than one line for all of them, because the pivot
/// verbs pushed the old single line past 150 columns and a `Paragraph` truncates
/// at its width without a word of complaint: on an 80-column terminal every key
/// from the pivots rightwards simply vanished. Split this way each line clears 80
/// on its own, so nothing is cut and no binding is broken across a wrap. The hint
/// takes a line of its own for the same reason: appended, it used to shove the
/// bindings off the right edge the moment a player asked for help.
fn draw_footer(frame: &mut Frame, area: Rect, hint: Option<&str>) {
    let lines = vec![
        Line::from("BODY     A/D move | Shift sprint | W jump | S hide | E interact | Q throw"),
        Line::from("UPLINK   1-4 actions | p bridge | P isp | g grid | x back"),
        Line::from("SESSION  Tab pause | R restart | H hint | Esc quit"),
        Line::from(hint.unwrap_or_default().to_owned()),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::DarkGray)),
        area,
    );
}

fn draw_result_overlay(frame: &mut Frame, area: Rect, sim: &GhostLobbySim) {
    let Some(d) = sim.debrief() else {
        return;
    };
    let w = area.width.saturating_sub(8).min(80);
    let h = area.height.saturating_sub(4).min(16);
    let rect = Rect {
        x: area.x + (area.width.saturating_sub(w)) / 2,
        y: area.y + (area.height.saturating_sub(h)) / 2,
        width: w,
        height: h,
    };
    let mut lines = vec![
        Line::from(Span::styled(
            format!("  GRADE {:?}   SCORE {}", d.grade, d.score),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(d.title.clone()),
        Line::from(""),
        Line::from(d.summary.clone()),
        Line::from(""),
    ];
    for tag in &d.tags {
        lines.push(Line::from(format!("• {}", tag.text)));
    }
    let block = Block::default().borders(Borders::ALL).title(if d.success {
        "EXTRACTED"
    } else {
        "FAILED"
    });
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: true }).block(block),
        rect,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use idaptik_core::RunConfig;
    use idaptik_core::scenario::command::{Command, PivotTarget, TickInput};
    use idaptik_core::scenario::ghost_lobby;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    /// The whole frame, rendered to text at `width` columns.
    fn screen_at(sim: &GhostLobbySim, width: u16, hint: Option<&str>) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, 40)).expect("a test backend");
        terminal
            .draw(|f| draw(f, sim, &[], hint))
            .expect("the frame draws");
        terminal.backend().to_string()
    }

    /// The whole frame, rendered to text at a comfortable terminal size.
    fn screen(sim: &GhostLobbySim) -> String {
        screen_at(sim, 130, None)
    }

    fn sim() -> GhostLobbySim {
        let mut sim = GhostLobbySim::new(ghost_lobby(), RunConfig::standard(), 123456)
            .expect("the definition is valid");
        let _ = sim.drain_events();
        sim
    }

    /// Fire `cmds` as immediates on one tick.
    fn immediate(sim: &mut GhostLobbySim, cmds: Vec<Command>) {
        sim.tick(&TickInput {
            immediates: cmds,
            ..Default::default()
        });
    }

    /// The reachable count the uplink panel is showing.
    fn reachable_shown(screen: &str) -> usize {
        let line = screen
            .lines()
            .find(|l| l.contains("nodes reachable"))
            .expect("the uplink panel names its reachable count");
        line.split("nodes reachable")
            .nth(1)
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|n| n.parse().ok())
            .expect("and that count is a number")
    }

    #[test]
    fn the_panel_names_every_series_it_draws() {
        // The gauges are titled, not lettered: a bar labelled "T" is not a readout.
        let s = screen(&sim());
        for series in ["support", "bandwidth", "alert", "isolation", "trace"] {
            assert!(s.contains(series), "the panel must label {series}: {s}");
        }
        assert!(s.contains("HACKER UPLINK"));
    }

    #[test]
    fn the_uplink_panel_follows_the_hacker_in_and_back_out() {
        // The readout is the only thing on screen that explains why an uplink was
        // refused, so it must actually move when the hacker does.
        let mut sim = sim();
        let cold = screen(&sim);
        assert!(cold.contains("pivot depth 0"), "{cold}");
        let cold_reach = reachable_shown(&cold);

        immediate(
            &mut sim,
            vec![Command::Pivot {
                target: PivotTarget::Bridge,
            }],
        );
        let deep = screen(&sim);
        assert!(deep.contains("pivot depth 1"), "{deep}");
        assert!(
            reachable_shown(&deep) > cold_reach,
            "pivoting must open the floor: {cold_reach} -> {}",
            reachable_shown(&deep)
        );

        immediate(&mut sim, vec![Command::Unpivot]);
        let home = screen(&sim);
        assert!(home.contains("pivot depth 0"), "{home}");
        assert_eq!(
            reachable_shown(&home),
            cold_reach,
            "backing out must hand back exactly the reach the pivot bought"
        );
    }

    #[test]
    fn the_footer_teaches_the_pivot_keys() {
        // Every one of them: a player who is never told `g` exists cannot walk the
        // upstream line, however well the sim models it.
        let s = screen(&sim());
        for hint in ["p bridge", "P isp", "g grid", "x back", "Tab pause"] {
            assert!(s.contains(hint), "the footer must teach {hint}: {s}");
        }
    }

    #[test]
    fn the_footer_survives_a_narrow_terminal() {
        // The regression this guards is the one the pivot keys caused: they pushed
        // the old single-line footer past 150 columns, and a `Paragraph` truncates
        // silently. Wrapped, every key must still be legible at 80 columns -- and
        // `Esc quit`, the rightmost of them, is the canary.
        let s = screen_at(&sim(), 80, None);
        for hint in [
            "p bridge",
            "P isp",
            "g grid",
            "x back",
            "Tab pause",
            "R restart",
            "Esc quit",
        ] {
            assert!(s.contains(hint), "{hint} was cut off at 80 columns: {s}");
        }
    }

    #[test]
    fn a_hint_does_not_evict_the_key_bindings() {
        // The hint used to be glued onto the end of the same line it now sits
        // beneath, which pushed the bindings off the right edge the moment a
        // player asked for help.
        let s = screen_at(
            &sim(),
            80,
            Some("The real lead is the boring kitchen note."),
        );
        assert!(s.contains("boring kitchen note"), "the hint shows: {s}");
        assert!(s.contains("p bridge"), "and the bindings survive it: {s}");
    }
}
