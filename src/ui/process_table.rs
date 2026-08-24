use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Row, Table};
use ratatui::Frame;

use crate::app::App;
use crate::system::processes::SortKey;
use crate::ui::theme::theme;

/// Column spec: config key + header label + width constraint.
fn column_spec(key: &str) -> Option<(&'static str, Constraint)> {
    Some(match key {
        "pid" => ("PID", Constraint::Length(8)),
        "name" => ("Name", Constraint::Percentage(35)),
        "cpu" => ("CPU%", Constraint::Length(8)),
        "mem" => ("Mem", Constraint::Length(10)),
        "mem_percent" => ("MEM%", Constraint::Length(7)),
        "user" => ("User", Constraint::Length(12)),
        "state" => ("State", Constraint::Length(9)),
        "virt" => ("Virt", Constraint::Length(10)),
        "time" => ("Time", Constraint::Length(9)),
        _ => return None,
    })
}

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let t = theme(&app.config.theme);

    let specs: Vec<(&str, Constraint)> = app
        .config
        .columns
        .iter()
        .filter_map(|c| column_spec(c))
        .collect();

    // Header row with an arrow on the active sort column.
    let header_cells = specs.iter().map(|(label, _)| {
        let sorted_by_key = sort_column_label(app.sort_key);
        let text = if *label == sorted_by_key {
            format!(
                "{}{}",
                label,
                if app.sort_descending { " ▼" } else { " ▲" }
            )
        } else {
            (*label).to_string()
        };
        Cell::from(text).style(
            Style::default()
                .fg(t.accent)
                .add_modifier(Modifier::BOLD),
        )
    });

    let rows = app.display_rows.iter().map(|p| {
        let cells = specs.iter().map(|(label, _)| {
            let text = match *label {
                "PID" => p.pid.to_string(),
                "Name" => p.name.clone(),
                "CPU%" => format!("{:.1}", p.cpu),
                "Mem" => crate::utils::formatting::format_bytes(p.mem_bytes),
                "MEM%" => format!("{:.1}", p.mem_percent),
                "User" => if p.user.is_empty() { "-".into() } else { p.user.clone() },
                "State" => p.state.clone(),
                "Virt" => crate::utils::formatting::format_bytes(p.virt_bytes),
                "Time" => crate::utils::formatting::format_duration(p.runtime_secs),
                _ => String::new(),
            };
            Cell::from(text)
        });
        Row::new(cells)
    });

    let title = format!(
        " Processes [{}] — {} shown / {} total ",
        sort_title(app),
        app.display_rows.len(),
        snap_count(app)
    );

    let widths: Vec<Constraint> = specs.iter().map(|(_, c)| *c).collect();
    let table = Table::new(rows)
        .header(Row::new(header_cells).bottom_margin(0))
        .widths(&widths)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .bg(t.highlight_bg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(table, area, &mut app.table_state);
}

fn sort_title(app: &App) -> String {
    format!(
        "{}{}",
        app.sort_key.label(),
        if app.sort_descending { " ↓" } else { " ↑" }
    )
}

fn snap_count(app: &App) -> usize {
    app.snapshot.as_ref().map(|s| s.process_count()).unwrap_or(0)
}

/// Map a SortKey to the table column label it controls.
fn sort_column_label(key: SortKey) -> &'static str {
    match key {
        SortKey::Pid => "PID",
        SortKey::Name => "Name",
        SortKey::Cpu => "CPU%",
        SortKey::Memory => "Mem",
        SortKey::Runtime => "Time",
    }
}
