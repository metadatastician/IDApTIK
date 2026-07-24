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
    // The footer takes the rows its bindings actually need at this width, which is
    // more of them the narrower the terminal is. The log yields them, because it is
    // the one pane that can: it already tails to whatever height it is given.
    let footer = footer_lines(hint, area.width as usize);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),                   // HUD
            Constraint::Length(3),                   // room strip
            Constraint::Length(6),                   // meters
            Constraint::Length(6),                   // belief + objectives
            Constraint::Min(4),                      // log
            Constraint::Length(footer.len() as u16), // key hints
        ])
        .split(area);

    draw_hud(frame, rows[0], sim);
    draw_strip(frame, rows[1], sim);
    draw_meters(frame, rows[2], sim);
    draw_belief_and_objectives(frame, rows[3], sim);
    draw_log(frame, rows[4], log);
    draw_footer(frame, rows[5], footer);

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
///
/// The panel is one row tall, so a line too long for it cannot wrap: it truncates
/// from the right, which is exactly where the reachable count sits. The readout
/// therefore spends its width on the numbers and gives up the prose, taking the
/// widest of three forms that fits. The terse form fits any panel worth drawing.
fn draw_uplink(frame: &mut Frame, area: Rect, sim: &GhostLobbySim) {
    let hacker = &sim.state().agents.hacker;
    let kind = format!("{:?}", hacker.vantage().kind);
    let hops = hacker.hops();
    let reach = hacker.reachable_count(sim.graph());
    let wide = format!("vantage {kind}     pivot depth {hops}     nodes reachable {reach}");
    let middling = format!("vantage {kind} | depth {hops} | reachable {reach}");
    let terse = format!("{kind} d{hops} r{reach}");
    // The bordered block spends two columns on its own frame.
    let inner = area.width.saturating_sub(2) as usize;
    let fits = |form: &String| form.chars().count() <= inner;
    // The widest form that fits, falling back to the terse one, which fits any
    // panel worth drawing.
    let text = [wide, middling].into_iter().find(fits).unwrap_or(terse);
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

/// The column an audience's bindings begin at, leaving room for its label.
const FOOTER_LABEL_WIDTH: usize = 9;

/// One audience's bindings, packed into as few rows of `width` as they need.
/// The break falls only between whole bindings, never inside one, and a
/// continuation row lines up under the label column.
fn footer_rows(label: &str, bindings: &[&str], width: usize) -> Vec<String> {
    let budget = width.saturating_sub(FOOTER_LABEL_WIDTH).max(1);
    let mut packed: Vec<String> = Vec::new();
    let mut row = String::new();
    for binding in bindings {
        let candidate = if row.is_empty() {
            (*binding).to_owned()
        } else {
            format!("{row} | {binding}")
        };
        if !row.is_empty() && candidate.chars().count() > budget {
            packed.push(std::mem::take(&mut row));
            row = (*binding).to_owned();
        } else {
            row = candidate;
        }
    }
    packed.push(row);
    packed
        .into_iter()
        .enumerate()
        .map(|(index, row)| {
            let head = if index == 0 {
                format!("{label:<w$}", w = FOOTER_LABEL_WIDTH)
            } else {
                " ".repeat(FOOTER_LABEL_WIDTH)
            };
            format!("{head}{row}")
        })
        .collect()
}

/// `text` greedily wrapped to `width` columns, breaking only at spaces.
fn wrap_words(text: &str, width: usize) -> Vec<String> {
    let mut rows: Vec<String> = Vec::new();
    let mut row = String::new();
    for word in text.split_whitespace() {
        let candidate = if row.is_empty() {
            word.to_owned()
        } else {
            format!("{row} {word}")
        };
        if !row.is_empty() && candidate.chars().count() > width {
            rows.push(std::mem::take(&mut row));
            row = word.to_owned();
        } else {
            row = candidate;
        }
    }
    if !row.is_empty() {
        rows.push(row);
    }
    rows
}

/// The key hints, one audience at a time: the session, the uplink, the body, and
/// last the hint.
///
/// The bindings are packed here rather than left to `Wrap`, because the footer
/// used to be a fixed four rows and a `Paragraph` drops what will not fit without
/// a word of complaint. At 80 columns each audience still takes the single row it
/// always did; at 50, an ordinary tmux split, BODY and UPLINK each need two, and
/// the fixed budget silently ate the SESSION line whole -- `Esc quit` among it.
/// Pre-wrapped, the row count is known, so the footer can ask the layout for the
/// height it actually needs.
///
/// The session leads for the same reason: should a terminal ever be too short to
/// honour that height, the rows lost are the last drawn, and the way out of the
/// game is not a thing to lose. The hint trails for the converse reason, and
/// takes a row of its own because appended it used to shove the bindings off the
/// right edge the moment a player asked for help.
fn footer_lines(hint: Option<&str>, width: usize) -> Vec<Line<'static>> {
    let width = width.max(1);
    let mut rows = footer_rows(
        "SESSION",
        &["Tab pause", "R restart", "H hint", "Esc quit"],
        width,
    );
    rows.extend(footer_rows(
        "UPLINK",
        &["1-4 actions", "p bridge", "P isp", "g grid", "x back"],
        width,
    ));
    rows.extend(footer_rows(
        "BODY",
        &[
            "A/D move",
            "Shift sprint",
            "W jump",
            "S hide",
            "E interact",
            "Q throw",
        ],
        width,
    ));
    rows.extend(wrap_words(hint.unwrap_or_default(), width));
    rows.into_iter().map(Line::from).collect()
}

fn draw_footer(frame: &mut Frame, area: Rect, lines: Vec<Line<'static>>) {
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(Color::DarkGray)),
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

    /// The reachable count the uplink panel is showing, in whichever of the
    /// readout's forms the panel was wide enough for.
    fn reachable_shown(screen: &str) -> usize {
        let line = screen
            .lines()
            .find(|l| l.contains("reachable"))
            .expect("the uplink panel names its reachable count");
        line.split("reachable")
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
        // Two regressions, one test. The pivot keys pushed the old single-line
        // footer past 150 columns and a `Paragraph` truncates silently; splitting
        // it per audience only moved the threshold, because four fixed rows cannot
        // hold three audiences once two of them wrap. 50 columns is an ordinary
        // tmux split, and `Esc quit` is the canary: it was the first thing lost.
        for width in [80, 50] {
            let s = screen_at(&sim(), width, None);
            for hint in [
                "p bridge",
                "P isp",
                "g grid",
                "x back",
                "Tab pause",
                "R restart",
                "H hint",
                "Esc quit",
                "A/D move",
                "Q throw",
            ] {
                assert!(
                    s.contains(hint),
                    "{hint} was cut off at {width} columns: {s}"
                );
            }
        }
    }

    #[test]
    fn the_uplink_readout_keeps_its_count_in_a_narrow_terminal() {
        // The reachable count sits at the right end of a line that cannot wrap, so
        // it is the first thing a narrow panel truncates -- and it is the one number
        // the panel exists to answer. It must survive, in some form, at any width a
        // player might reasonably use.
        let mut sim = sim();
        immediate(
            &mut sim,
            vec![Command::Pivot {
                target: PivotTarget::Bridge,
            }],
        );
        let wide = reachable_shown(&screen(&sim));
        for width in [80, 60, 50, 40] {
            let s = screen_at(&sim, width, None);
            assert_eq!(
                reachable_shown(&s),
                wide,
                "the reachable count was lost at {width} columns: {s}"
            );
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
