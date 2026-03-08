use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use super::super::super::tree::FileTree;
use super::super::types::AppState;

pub(super) fn draw_header(
    f: &mut Frame,
    area: Rect,
    title: &str,
    state: &AppState,
    tree: Option<&FileTree>,
    spinner: &str,
    dry_run: bool,
) {
    let (total, checked) = tree.map(|t| t.stats()).unwrap_or((0, 0));

    let mut spans: Vec<Span> = vec![
        Span::styled(
            " xun delete ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            title.to_string(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
    ];

    if dry_run {
        spans.push(Span::styled(
            "[DRY RUN] ",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));
    }

    spans.push(match state {
        AppState::Loading => Span::styled(
            format!("{} scanning...", spinner),
            Style::default().fg(Color::Yellow),
        ),
        AppState::Filtering => Span::styled(
            "filter mode".to_string(),
            Style::default().fg(Color::Yellow),
        ),
        AppState::ConfirmDelete => Span::styled(
            "confirm delete".to_string(),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        AppState::Deleting { .. } => Span::styled(
            "deleting...".to_string(),
            Style::default().fg(Color::Yellow),
        ),
        AppState::Done(_) => Span::styled(
            "done".to_string(),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        _ => {
            let (color, bold) = if checked > 0 {
                (Color::Cyan, Modifier::BOLD)
            } else {
                (Color::DarkGray, Modifier::empty())
            };
            Span::styled(
                format!("selected {}/{}", checked, total),
                Style::default().fg(color).add_modifier(bold),
            )
        }
    });

    let para = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(para, area);
}
