use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph, Sparkline};
use ratatui::Frame;

use crate::app::App;
use crate::utils::formatting::format_bytes;
use crate::ui::theme::{theme, Theme};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let t = theme(&app.config.theme);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),  // RAM gauge
            Constraint::Length(5),  // RAM history
            Constraint::Length(4),  // swap gauge
            Constraint::Length(5),  // swap history
            Constraint::Min(0),     // numbers
        ])
        .split(area);

    let Some(snap) = app.snapshot.as_ref() else { return };
    let mem = &snap.mem;

    // RAM
    let ram_pct = mem.used_percent();
    f.render_widget(
        Gauge::default()
            .block(mem_block(" Memory (RAM) ", t))
            .gauge_style(Style::default().fg(bar_color(ram_pct, t)))
            .percent(ram_pct.clamp(0.0, 100.0) as u16)
            .label(format!(
                " {} / {} ({ram_pct:.1}%) ",
                format_bytes(mem.used),
                format_bytes(mem.total)
            )),
        chunks[0],
    );
    f.render_widget(
        Sparkline::default()
            .data(&app.history_ram.sparkline(100.0))
            .block(mem_block(" RAM history (%) ", t))
            .style(Style::default().fg(t.accent)),
        chunks[1],
    );

    // Swap
    if mem.swap_total > 0 {
        let swap_pct = mem.swap_percent();
        f.render_widget(
            Gauge::default()
                .block(mem_block(" Swap ", t))
                .gauge_style(Style::default().fg(bar_color(swap_pct, t)))
                .percent(swap_pct.clamp(0.0, 100.0) as u16)
                .label(format!(
                    " {} / {} ({swap_pct:.1}%) ",
                    format_bytes(mem.swap_used),
                    format_bytes(mem.swap_total)
                )),
            chunks[2],
        );
        f.render_widget(
            Sparkline::default()
                .data(&app.history_swap.sparkline(100.0))
                .block(mem_block(" Swap history (%) ", t))
                .style(Style::default().fg(t.warn)),
            chunks[3],
        );
    }

    // Numbers
    let lines = vec![
        Line::from(vec![
            Span::styled("Total:     ", Style::default().fg(t.text).add_modifier(Modifier::BOLD)),
            Span::raw(format_bytes(mem.total)),
        ]),
        Line::from(vec![
            Span::styled("Used:      ", Style::default().fg(t.text).add_modifier(Modifier::BOLD)),
            Span::raw(format_bytes(mem.used)),
        ]),
        Line::from(vec![
            Span::styled("Available: ", Style::default().fg(t.text).add_modifier(Modifier::BOLD)),
            Span::raw(format_bytes(mem.available)),
        ]),
        Line::from(vec![
            Span::styled("Free:      ", Style::default().fg(t.text).add_modifier(Modifier::BOLD)),
            Span::raw(format_bytes(mem.total.saturating_sub(mem.used))),
        ]),
        Line::from(vec![
            Span::styled("Swap used: ", Style::default().fg(t.text).add_modifier(Modifier::BOLD)),
            Span::raw(format!(
                "{} / {}",
                format_bytes(mem.swap_used),
                format_bytes(mem.swap_total)
            )),
        ]),
    ];
    f.render_widget(
        Paragraph::new(lines).block(mem_block(" Details ", t)),
        chunks[4],
    );
}

fn mem_block(title: &str, t: Theme) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(title.to_string())
        .border_style(Style::default().fg(t.accent))
}

fn bar_color(percent: f32, t: Theme) -> ratatui::style::Color {
    if percent > 90.0 {
        t.error
    } else if percent > 75.0 {
        t.warn
    } else {
        t.ok
    }
}

