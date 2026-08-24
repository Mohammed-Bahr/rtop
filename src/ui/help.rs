use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::ui::theme::{theme, Theme};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let t = theme(&app.config.theme);
    let lines = vec![
        title("Keyboard Shortcuts", t),
        blank(),
        row("Tab / ←→", "Switch views", t),
        row("↑ / k  ↓ / j", "Move selection", t),
        row("PgUp/PgDn, g/G", "Jump (page/home/end)", t),
        row("Enter", "Process details", t),
        row("Esc", "Close overlay / clear search", t),
        row("/", "Search processes (by name, PID, user)", t),
        row("s", "Cycle sort column", t),
        row("S", "Reverse sort direction", t),
        row("r", "Refresh now", t),
        row("t", "Send SIGTERM (confirm)", t),
        row("x", "Send SIGKILL (confirm)", t),
        row("p", "Send SIGSTOP", t),
        row("c", "Send SIGCONT", t),
        row("e", "Tree: expand/collapse node", t),
        row("Space", "Freeze / unfreeze live updates", t),
        row("?", "Toggle this help", t),
        row("q", "Quit", t),
        blank(),
        Line::from(Span::styled(
            "While frozen the displayed data is static but navigation, search and views stay interactive.",
            Style::default().fg(t.dim),
        )),
    ];

    let width = 62.min(area.width.saturating_sub(4));
    let height = (lines.len() as u16 + 2).min(area.height.saturating_sub(2));
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    };

    f.render_widget(Clear, popup);
    f.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Help ")
                    .border_style(Style::default().fg(t.accent)),
            )
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn row(keys: &str, desc: &str, t: Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {keys:<18}"),
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ),
        Span::raw(desc.to_string()),
    ])
}

fn title(text: &str, t: Theme) -> Line<'static> {
    Line::from(Span::styled(
        text.to_string(),
        Style::default().fg(t.accent).add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    ))
}

fn blank() -> Line<'static> {
    Line::from("")
}
