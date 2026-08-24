use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::{App, ConfirmDialog};
use crate::ui::theme::theme;

pub fn draw(f: &mut Frame, app: &App, dialog: &ConfirmDialog, area: Rect) {
    let t = theme(&app.config.theme);

    let width = 52.min(area.width.saturating_sub(4));
    let height = (dialog.lines.len() as u16 + 2).min(area.height.saturating_sub(2));
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    };

    let destructive = dialog.signal.requires_confirmation();
    let border_color = if destructive { t.error } else { t.accent };

    f.render_widget(Clear, popup);
    f.render_widget(
        Paragraph::new(dialog.lines.iter().cloned().map(Line::from).collect::<Vec<_>>())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" {} ", dialog.title))
                    .border_style(Style::default().fg(border_color)),
            )
            .style(Style::default().add_modifier(if destructive {
                Modifier::BOLD
            } else {
                Modifier::empty()
            })),
        popup,
    );
}
