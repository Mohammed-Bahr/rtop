use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Sparkline};
use ratatui::Frame;

use crate::app::App;
use crate::system::processes::ProcessInfo;
use crate::utils::formatting::{format_bytes, format_duration};
use crate::ui::theme::{theme, Theme};

/// Render the process details overlay for the currently selected process.
pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let Some(proc) = details_target(app) else { return };
    let t = theme(&app.config.theme);

    let popup = centered_rect(70, 80, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Process Details — {} ({}) ", proc.name, proc.pid))
        .border_style(Style::default().fg(t.accent))
        .style(Style::default().bg(ratatui::style::Color::Reset));
    let inner = block.inner(popup);
    f.render_widget(Clear, popup);
    f.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // fields
            Constraint::Min(3),    // cpu history
            Constraint::Length(1), // gap
            Constraint::Min(3),    // mem history
            Constraint::Length(1), // hint
        ])
        .split(inner);

    let fields = vec![
        field("PID", proc.pid.to_string(), t),
        field("Parent PID", opt_str(proc.parent), t),
        field("Name", proc.name.clone(), t),
        field("User", empty_dash(&proc.user), t),
        field("State", proc.state.clone(), t),
        field("CPU", format!("{:.1}%", proc.cpu), t),
        field(
            "Memory",
            format!("{} ({:.1}%)", format_bytes(proc.mem_bytes), proc.mem_percent),
            t,
        ),
        field("Virtual", format_bytes(proc.virt_bytes), t),
        field("Runtime", format_duration(proc.runtime_secs), t),
        field(
            "Started",
            chrono_fmt(proc.start_epoch),
            t,
        ),
        field("Command", if proc.command.is_empty() { "-".into() } else { proc.command.clone() }, t),
    ];

    f.render_widget(
        Paragraph::new(fields)
            .wrap(ratatui::widgets::Wrap { trim: false }),
        chunks[0],
    );

    // History sparklines (only available once this PID has been observed).
    match app.proc_history(proc.pid) {
        Some((cpu_hist, mem_hist)) => {
            f.render_widget(
                Sparkline::default()
                    .data(&cpu_hist.sparkline(100.0))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" CPU history (%) ")
                            .border_style(Style::default().fg(t.accent)),
                    )
                    .style(Style::default().fg(t.accent)),
                chunks[1],
            );
            let max_mem = mem_hist.iter().fold(0.0f64, |a, b| a.max(b)).max(1.0);
            f.render_widget(
                Sparkline::default()
                    .data(&mem_hist.sparkline(max_mem))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" Memory history ")
                            .border_style(Style::default().fg(t.accent)),
                    )
                    .style(Style::default().fg(t.warn)),
                chunks[3],
            );
        }
        None => {
            f.render_widget(
                Paragraph::new("Collecting history…").block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" History ")
                        .border_style(Style::default().fg(t.dim)),
                ),
                chunks[1],
            );
        }
    }

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "Esc close",
            Style::default().fg(t.dim),
        )))
        ,
        chunks[4],
    );
}

/// Which process the overlay shows: tree selection on the tree screen,
/// table selection elsewhere.
pub fn details_target(app: &App) -> Option<&ProcessInfo> {
    if app.screen == crate::app::Screen::Tree {
        app.tree_selected()
    } else {
        app.selected_process()
    }
}

fn field(label: &str, value: String, t: Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label:<12}"),
            Style::default().fg(t.text).add_modifier(Modifier::BOLD),
        ),
        Span::raw(value),
    ])
}

/// Format a Unix timestamp as local "YYYY-MM-DD HH:MM:SS" without adding a
/// dependency: days-from-epoch civil date algorithm.
fn chrono_fmt(epoch: u64) -> String {
    if epoch == 0 {
        return "-".into();
    }
    let days = epoch / 86_400;
    let rem = epoch % 86_400;
    let (h, m, sec) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // Howard Hinnant's civil_from_days algorithm.
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{m:02}:{sec:02}")
}

fn opt_str(v: Option<u32>) -> String {
    v.map(|p| p.to_string()).unwrap_or_else(|| "-".into())
}

fn empty_dash(s: &str) -> String {
    if s.is_empty() { "-".into() } else { s.into() }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let vx = (r.width as f32 * percent_x as f32 / 100.0).round() as u16;
    let vy = (r.height as f32 * percent_y as f32 / 100.0).round() as u16;
    Rect {
        x: r.x + (r.width.saturating_sub(vx)) / 2,
        y: r.y + (r.height.saturating_sub(vy)) / 2,
        width: vx.min(r.width),
        height: vy.min(r.height),
    }
}
