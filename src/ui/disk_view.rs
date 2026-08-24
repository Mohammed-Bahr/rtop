use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::Frame;

use crate::app::App;
use crate::utils::formatting::{format_bytes, format_rate};
use crate::ui::theme::theme;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let t = theme(&app.config.theme);

    // Aggregate I/O throughput header.
    let io_line = match app.snapshot.as_ref().and_then(|s| s.disk_io) {
        Some(io) => Line::from(vec![
            Span::styled(" Read ", Style::default().fg(t.text).add_modifier(Modifier::BOLD)),
            Span::styled(format_rate(io.read_bps), Style::default().fg(t.ok)),
            Span::styled("   Write ", Style::default().fg(t.text).add_modifier(Modifier::BOLD)),
            Span::styled(format_rate(io.write_bps), Style::default().fg(t.warn)),
        ]),
        None => Line::from(Span::styled(
            " I/O throughput unavailable (requires /proc/diskstats)",
            Style::default().fg(t.dim),
        )),
    };

    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);
    f.render_widget(Paragraph::new(io_line), chunks[0]);

    let Some(snap) = app.snapshot.as_ref() else { return };
    let rows = snap.disks.iter().map(|d| {
        Row::new(vec![
            Cell::from(d.name.clone()),
            Cell::from(d.mount_point.clone()),
            Cell::from(d.file_system.clone()),
            Cell::from(format_bytes(d.total)),
            Cell::from(format_bytes(d.used())),
            Cell::from(format_bytes(d.available)),
            Cell::from(format!("{:.1}%", d.used_percent())),
            Cell::from(if d.removable { "removable" } else { "" }),
        ])
        .style(Style::default().fg(if d.used_percent() > 90.0 {
            t.error
        } else {
            t.text
        }))
    });

    let table = Table::new(rows)
        .header(
            Row::new(vec![
                "Device",
                "Mount",
                "FS",
                "Total",
                "Used",
                "Free",
                "Used%",
                "",
            ])
            .style(Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
        )
        .widths(&[
            Constraint::Percentage(15),
            Constraint::Percentage(20),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(10),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Filesystems ")
                .border_style(Style::default().fg(t.accent)),
        );
    f.render_widget(table, chunks[1]);
}

