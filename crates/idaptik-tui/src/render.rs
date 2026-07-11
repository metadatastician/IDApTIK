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
            Constraint::Length(1), // key hints
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
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25); 4])
        .split(area);
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

fn draw_footer(frame: &mut Frame, area: Rect, hint: Option<&str>) {
    let text = match hint {
        Some(h) => format!("A/D move · Shift sprint · W jump · S hide · E interact · Q throw · 1-4 uplink · P pause · R restart · H hint · Esc quit   —   {h}"),
        None => "A/D move · Shift sprint · W jump · S hide · E interact · Q throw · 1-4 uplink · P pause · R restart · H hint · Esc quit".to_owned(),
    };
    frame.render_widget(
        Paragraph::new(text).style(Style::default().fg(Color::DarkGray)),
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
