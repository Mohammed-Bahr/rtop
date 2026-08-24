use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph, Sparkline};
use ratatui::Frame;

use crate::app::App;
use crate::ui::theme::{theme, Theme};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let t = theme(&app.config.theme);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),  // total gauge
            Constraint::Length(5), // history sparkline
            Constraint::Min(0),    // per-core grid + info
        ])
        .split(area);

    draw_total(f, app, t, chunks[0]);
    draw_history(f, app, t, chunks[1]);
    draw_cores(f, app, t, chunks[2]);
}

fn usage_color(percent: f32, t: Theme) -> ratatui::style::Color {
    if percent > 90.0 {
        t.error
    } else if percent > 70.0 {
        t.warn
    } else {
        t.ok
    }
}

fn draw_total(f: &mut Frame, app: &App, t: Theme, area: Rect) {
    let pct = app.snapshot.as_ref().map(|s| s.cpu_total).unwrap_or(0.0);
    let label = match app.snapshot.as_ref().and_then(|s| s.temp_celsius) {
        Some(temp) => format!(" Total {pct:.1}%   {temp:.0}°C "),
        None => format!(" Total {pct:.1}% "),
    };
    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" CPU ")
                .border_style(Style::default().fg(t.accent)),
        )
        .gauge_style(Style::default().fg(usage_color(pct, t)))
        .percent(pct.clamp(0.0, 100.0) as u16)
        .label(label);
    f.render_widget(gauge, area);
}

fn draw_history(f: &mut Frame, app: &App, t: Theme, area: Rect) {
    let data = app.history_cpu_total.sparkline(100.0);
    let title = match app.history_cpu_total.last() {
        Some(v) => format!(" History (%) — now {v:.0} "),
        None => " History (%) ".to_string(),
    };
    let spark = Sparkline::default()
        .data(&data)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(t.accent)),
        )
        .style(Style::default().fg(t.accent));
    f.render_widget(spark, area);
}

fn draw_cores(f: &mut Frame, app: &App, t: Theme, area: Rect) {
    let Some(snap) = app.snapshot.as_ref() else { return };
    let cores: Vec<crate::system::CpuInfo> = snap.cpus.iter().skip(1).cloned().collect();
    let info_block = Block::default()
        .borders(Borders::ALL)
        .title(" Cores ")
        .border_style(Style::default().fg(t.accent));
    let inner = info_block.inner(area);
    f.render_widget(info_block, area);
    if inner.width < 20 || inner.height < 1 || cores.is_empty() {
        return;
    }

    // Two-column layout of core bars.
    let half = (cores.len() + 1) / 2;
    let left_cols = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(1); half])
        .split(inner);
    let right_inner = Rect {
        x: inner.x + inner.width / 2,
        width: inner.width - inner.width / 2,
        ..inner
    };
    let right_half = cores.len() - half;
    let right_cols = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(1); right_half.max(1)])
        .split(right_inner);

    for (i, core) in cores.iter().enumerate() {
        let is_left = i < half;
        let rows = if is_left { &left_cols } else { &right_cols };
        let row_idx = if is_left { i } else { i - half };
        if row_idx >= rows.len() {
            break;
        }
        let bar_width = (rows[row_idx].width.saturating_sub(14)).clamp(5, 30) as usize;
        let filled = ((core.usage / 100.0) * bar_width as f32).round() as usize;
        let freq = core
            .freq_mhz
            .map(|mhz| format!(" {:>4}MHz", mhz))
            .unwrap_or_default();
        let line = Line::from(vec![
            Span::styled(format!("{:>7} ", core.name), Style::default().fg(t.text)),
            Span::styled("█".repeat(filled), Style::default().fg(usage_color(core.usage, t))),
            Span::styled("░".repeat(bar_width - filled), Style::default().fg(t.dim)),
            Span::styled(format!(" {:>5.1}%{}", core.usage, freq), Style::default().fg(t.dim)),
        ]);
        f.render_widget(Paragraph::new(line), rows[row_idx]);
    }

}
