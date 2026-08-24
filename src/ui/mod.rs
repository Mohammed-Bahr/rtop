pub mod cpu_view;
pub mod dialogs;
pub mod disk_view;
pub mod footer;
pub mod header;
pub mod help;
pub mod memory_view;
pub mod network_view;
pub mod process_details;
pub mod process_table;
pub mod theme;
pub mod tree_view;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

use crate::app::{App, Screen};

/// Render one frame. The UI layer only reads app state (plus the table
/// state needed by ratatui's stateful widget).
pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(0),    // main area
            Constraint::Length(3), // footer
        ])
        .split(f.size());

    header::draw(f, app, chunks[0]);

    match app.screen {
        Screen::Processes => process_table::draw(f, app, chunks[1]),
        Screen::Cpu => cpu_view::draw(f, app, chunks[1]),
        Screen::Memory => memory_view::draw(f, app, chunks[1]),
        Screen::Disk => disk_view::draw(f, app, chunks[1]),
        Screen::Network => network_view::draw(f, app, chunks[1]),
        Screen::Tree => tree_view::draw(f, app, chunks[1]),
    }

    footer::draw(f, app, chunks[2]);

    if app.details_open {
        process_details::draw(f, app, chunks[1]);
    }
    if app.help_open {
        help::draw(f, app, chunks[1]);
    }
    if let Some(dialog) = app.dialog.clone() {
        dialogs::draw(f, app, &dialog, f.size());
    }
}
