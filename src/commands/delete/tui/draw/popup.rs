use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use super::super::super::tree::FileTree;
use super::super::util::centered_rect;

pub(super) fn draw_confirm_popup(f: &mut Frame, area: Rect, tree: &FileTree) {
    let (_, checked) = tree.stats();
    let paths = tree.checked_paths();
    let popup = centered_rect(56, 12, area);
    f.render_widget(Clear, popup);

    let mut lines = vec![Line::raw("")];
    for p in paths.iter().take(4) {
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        lines.push(Line::from(vec![
            Span::styled("  [T] ", Style::default()),
            Span::styled(name.to_string(), Style::default().fg(Color::LightRed)),
        ]));
    }
    if paths.len() > 4 {
        lines.push(Line::from(Span::styled(
            format!("  ... and {} more", paths.len() - 4),
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled("  total ", Style::default().fg(Color::Yellow)),
        Span::styled(
            checked.to_string(),
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" files will be deleted", Style::default().fg(Color::Yellow)),
    ]));
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled("    [", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "Y",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("] confirm    [", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "N",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::styled("] cancel", Style::default().fg(Color::DarkGray)),
    ]));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title(Span::styled(
            "confirm delete",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));
    f.render_widget(Paragraph::new(lines).block(block), popup);
}
