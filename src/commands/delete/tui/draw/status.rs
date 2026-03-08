use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use super::super::super::tree::FileTree;
use super::super::types::AppState;

pub(super) fn draw_statusbar(f: &mut Frame, area: Rect, state: &AppState, tree: Option<&FileTree>) {
    let text = match state {
        AppState::Loading => "Esc to cancel".into(),
        AppState::Filtering => {
            let q = tree
                .and_then(|t| if t.filter.is_empty() { None } else { Some(t.filter.as_str()) })
                .unwrap_or("");
            format!("filter: /{}  Enter=apply  Esc=clear", q)
        }
        AppState::ConfirmDelete => "Y to confirm, N/Esc to cancel".into(),
        AppState::Done(_) => "Enter/q to exit".into(),
        AppState::Deleting { .. } => "deleting...".into(),
        _ => "Up/Down navigate  Space toggle  Right expand  Left collapse  d delete  / filter  q quit"
            .into(),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let para = Paragraph::new(Line::from(Span::styled(
        text,
        Style::default().fg(Color::DarkGray),
    )))
    .block(block);
    f.render_widget(para, area);
}
