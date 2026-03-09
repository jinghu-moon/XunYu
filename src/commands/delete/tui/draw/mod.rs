use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::ListState,
};

use super::super::tree::FileTree;
use super::types::AppState;

mod body;
mod header;
mod panels;
mod popup;
mod status;

#[allow(clippy::too_many_arguments)]
pub(super) fn draw(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    tree: Option<&FileTree>,
    list_state: &mut ListState,
    title: &str,
    spinner: &str,
    dry_run: bool,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(4),
            Constraint::Length(3),
        ])
        .split(area);

    header::draw_header(f, layout[0], title, state, tree, spinner, dry_run);
    body::draw_body(f, layout[1], state, tree, list_state);
    status::draw_statusbar(f, layout[2], state, tree);

    if let AppState::ConfirmDelete = state
        && let Some(t) = tree
    {
        popup::draw_confirm_popup(f, area, t);
    }
}
