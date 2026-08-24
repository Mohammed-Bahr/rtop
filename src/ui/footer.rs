use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::action::Mode;
use crate::app::App;
use crate::ui::theme::theme;

/// Render the bottom bar: search input, transient status message, or the
/// default keybinding hints.
pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let t = theme(&app.config.theme);
    let (line, style) = if app.mode == Mode::Searching {
        (
            Line::from(vec![
                Span::styled("/", Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
                Span::raw(app.search_query.clone()),
                Span::styled("█", Style::default().fg(t.accent)),
            ]),
            Style::default(),
        )
    } else if let Some(msg) = app.status_text() {
        let is_error = msg.contains("permission denied")
            || msg.contains("failed")
            || msg.contains("no longer exists");
        (Line::from(msg.to_string()), Style::default().fg(if is_error { t.error } else { t.ok }))
    } else {
        (hints_line(), Style::default().fg(t.dim))
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.accent));
    f.render_widget(
        Paragraph::new(line).style(style).block(block),
        area,
    );

    // Screen tab strip rendered into the footer border title area.
    let tabs: Vec<Span> = crate::app::Screen::ALL
        .iter()
        .flat_map(|s| {
            let active = *s == app.screen;
            vec![
                Span::styled(
                    format!(" {} ", s.label()),
                    if active {
                        Style::default()
                            .fg(t.accent)
                            .add_modifier(Modifier::BOLD | Modifier::REVERSED)
                    } else {
                        Style::default().fg(t.dim)
                    },
                ),
            ]
        })
        .collect();
    f.render_widget(
        Paragraph::new(Line::from(tabs)).alignment(Alignment::Center),
        Rect {
            y: area.y.saturating_sub(0),
            x: area.x,
            width: area.width,
            height: 1.min(area.height),
        },
    );
}

fn hints_line() -> Line<'static> {
    Line::from(
        "Tab views  ↑↓ select  Enter details  / search  s sort  S reverse  t/x/p/c signals  Space freeze  ? help  q quit",
    )
}
