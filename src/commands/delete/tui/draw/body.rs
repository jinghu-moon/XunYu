use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, ListState, Paragraph},
};

use super::super::super::DeleteRecord;
use super::super::super::deleter::Outcome;
use super::super::super::tree::FileTree;
use super::super::types::AppState;
use super::panels::{draw_info_panel, draw_tree_panel};

pub(super) fn draw_body(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    tree: Option<&FileTree>,
    list_state: &mut ListState,
) {
    let border = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
        .border_style(Style::default().fg(Color::DarkGray));

    match state {
        AppState::Loading => {
            let inner = border.inner(area);
            f.render_widget(border, area);
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    " scanning, please wait...",
                    Style::default().fg(Color::DarkGray),
                ))),
                inner,
            );
        }
        AppState::Deleting { .. } => {
            let inner = border.inner(area);
            f.render_widget(border, area);
            f.render_widget(
                Paragraph::new(vec![
                    Line::raw(""),
                    Line::from(Span::styled(
                        " deleting...",
                        Style::default().fg(Color::Yellow),
                    )),
                ]),
                inner,
            );
        }
        AppState::Done(results) => {
            let inner = border.inner(area);
            f.render_widget(border, area);
            draw_done(f, inner, results);
        }
        _ => {
            if let Some(t) = tree {
                let cols = ratatui::layout::Layout::default()
                    .direction(ratatui::layout::Direction::Horizontal)
                    .constraints([
                        ratatui::layout::Constraint::Percentage(65),
                        ratatui::layout::Constraint::Percentage(35),
                    ])
                    .split(area);
                draw_tree_panel(f, cols[0], t, list_state);
                draw_info_panel(f, cols[1], t);
            } else {
                f.render_widget(border, area);
            }
        }
    }
}

fn draw_done(f: &mut Frame, area: Rect, results: &[DeleteRecord]) {
    let ok = results.iter().filter(|r| r.outcome.is_success()).count();
    let fail = results.iter().filter(|r| r.outcome.is_error()).count();
    let max = area.height.saturating_sub(5) as usize;

    let mut lines = vec![
        Line::raw(""),
        Line::from(Span::styled(
            format!(" done  ok: {}  fail: {}", ok, fail),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
    ];

    for r in results.iter().take(max) {
        let (color, label) = match &r.outcome {
            Outcome::Error(c) => (Color::Red, format!("FAIL ({})", c)),
            o => (Color::Green, format!("OK {}", o.label())),
        };
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                r.path.display().to_string(),
                Style::default().fg(Color::White),
            ),
            Span::styled(format!("  {}", label), Style::default().fg(color)),
        ]));
    }
    if results.len() > max {
        lines.push(Line::from(Span::styled(
            format!("  ... {} more", results.len() - max),
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        "  Enter / q to exit",
        Style::default().fg(Color::DarkGray),
    )));

    f.render_widget(Paragraph::new(lines), area);
}
