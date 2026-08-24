use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::utils::formatting::truncate;
use crate::ui::theme::theme;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let t = theme(&app.config.theme);
    let nodes = app.tree_nodes();
    if nodes.is_empty() {
        f.render_widget(
            Paragraph::new("No process data").block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Process Tree ")
                    .border_style(Style::default().fg(t.accent)),
            ),
            area,
        );
        return;
    }

    // Visible window around the cursor.
    let inner_h = area.height.saturating_sub(2) as usize;
    let start = app
        .tree_cursor
        .saturating_sub(inner_h / 2)
        .min(nodes.len().saturating_sub(inner_h.min(nodes.len())));
    let end = (start + inner_h).min(nodes.len());

    let lines: Vec<Line> = nodes[start..end]
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let idx = start + i;
            let proc = &app.snapshot.as_ref().unwrap().processes[&node.pid];
            let marker = if !node.has_children {
                "  "
            } else if node.collapsed {
                "[+]"
            } else {
                "[-]"
            };
            let indent = "  ".repeat(node.depth);
            let selected = idx == app.tree_cursor;
            let style = Style::default().fg(if selected { t.accent } else { t.text });
            Line::from(vec![
                Span::styled(if selected { "> " } else { "  " }, style),
                Span::styled(indent, style),
                Span::styled(marker.to_string(), Style::default().fg(t.dim)),
                Span::styled(
                    format!("{} ", truncate(&proc.name, 40)),
                    style.add_modifier(if selected { Modifier::BOLD } else { Modifier::empty() }),
                ),
                Span::styled(
                    format!("({})", proc.pid),
                    Style::default().fg(t.dim),
                ),
            ])
        })
        .collect();

    let title = format!(
        " Process Tree — ↑↓ navigate, e expand/collapse, Enter details ({} processes) ",
        nodes.len()
    );
    f.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(t.accent)),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
}
