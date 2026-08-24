use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Row, Table};
use ratatui::Frame;

use crate::app::App;
use crate::utils::formatting::{format_bytes, format_rate};
use crate::ui::theme::theme;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let t = theme(&app.config.theme);
    let Some(snap) = app.snapshot.as_ref() else { return };

    let rows = snap.net.iter().map(|n| {
        Row::new(vec![
            Cell::from(n.interface.clone()),
            Cell::from(format_rate(n.rx_bps)).style(Style::default().fg(t.ok)),
            Cell::from(format_rate(n.tx_bps)).style(Style::default().fg(t.warn)),
            Cell::from(format_bytes(n.rx_total)),
            Cell::from(format_bytes(n.tx_total)),
        ])
    });

    let table = Table::new(rows)
        .header(
            Row::new(vec!["Interface", "RX/s", "TX/s", "RX total", "TX total"])
                .style(Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
        )
        .widths(&[
            Constraint::Percentage(25),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(14),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Network ")
                .border_style(Style::default().fg(t.accent)),
        );
    f.render_widget(table, area);
}
