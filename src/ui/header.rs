use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::ui::theme::{theme, Theme};

/// Render the top summary bar: host info on the left, freeze indicator
/// pinned to the right edge.
pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let t = theme(&app.config.theme);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.accent));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let cols = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: inner.height,
    };
    let (left_area, right_area) = if inner.width > 30 {
        (
            Rect { width: inner.width - 12, ..cols },
            Rect {
                x: inner.x + inner.width - 12,
                y: inner.y,
                width: 12,
                height: inner.height,
            },
        )
    } else {
        (inner, Rect::default())
    };

    let left = Paragraph::new(header_lines(app, t));
    f.render_widget(left, left_area);
    f.render_widget(freeze_indicator(app, t), right_area);
}

fn header_lines(app: &App, t: Theme) -> Vec<Line<'static>> {
    let Some(snap) = app.snapshot.as_ref() else {
        return vec![Line::from("Collecting system data…")];
    };
    vec![
        Line::from(vec![
            Span::styled(
                format!("{} ", snap.hostname),
                Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                if snap.kernel.is_empty() {
                    snap.os_name.clone()
                } else {
                    format!("{} ({})", snap.os_name, snap.kernel)
                },
                Style::default().fg(t.dim),
            ),
        ]),
        Line::from(vec![
            Span::styled("CPU", Style::default().fg(t.text)),
            Span::raw(format!(" {:>5.1}%  ", snap.cpu_total)),
            Span::styled("RAM", Style::default().fg(t.text)),
            Span::raw(format!(
                " {:>5.1}%  ",
                snap.mem.used_percent()
            )),
            Span::styled(
                format!(
                    "Load {:.2} {:.2} {:.2}  Up {}",
                    snap.load[0],
                    snap.load[1],
                    snap.load[2],
                    crate::utils::formatting::format_duration(snap.uptime_secs)
                ),
                Style::default().fg(t.dim),
            ),
        ]),
    ]
}

fn freeze_indicator(app: &App, t: Theme) -> Paragraph<'static> {
    if app.frozen {
        Paragraph::new(Line::from(Span::styled(
            "❚❚ FROZEN",
            Style::default()
                .fg(t.frozen)
                .add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Right)
    } else {
        Paragraph::new(Line::from(Span::styled(
            "● LIVE",
            Style::default().fg(t.ok).add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Right)
    }
}
