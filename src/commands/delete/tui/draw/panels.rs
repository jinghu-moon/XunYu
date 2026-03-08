use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use super::super::super::tree::{CheckState, FileTree, NodeKind, TreeNode};
use super::super::util::{calc_scroll, fmt_size};

pub(super) fn draw_tree_panel(
    f: &mut Frame,
    area: Rect,
    tree: &FileTree,
    list_state: &mut ListState,
) {
    let filter_title = if tree.filter_active && !tree.filter.is_empty() {
        format!("tree  [/{}]", tree.filter)
    } else {
        "tree".into()
    };
    let ft_color = if tree.filter_active {
        Color::Yellow
    } else {
        Color::DarkGray
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(filter_title, Style::default().fg(ft_color)));

    let inner_h = area.height.saturating_sub(2) as usize;
    let visible = tree.visible_nodes();
    let cursor = tree.cursor;
    let scroll = calc_scroll(cursor, inner_h, visible.len());

    list_state.select(Some(cursor.saturating_sub(scroll)));

    let items: Vec<ListItem> = visible
        .iter()
        .skip(scroll)
        .take(inner_h)
        .enumerate()
        .map(|(i, &id)| make_item(&tree.nodes[id], scroll + i == cursor, tree))
        .collect();

    f.render_stateful_widget(List::new(items).block(block), area, list_state);
}

fn make_item(node: &TreeNode, focused: bool, tree: &FileTree) -> ListItem<'static> {
    let indent = "  ".repeat(node.depth.saturating_sub(1));
    let bg = if focused {
        Color::Rgb(20, 40, 70)
    } else {
        Color::Reset
    };

    let line = match node.kind {
        NodeKind::Dir => {
            let (name_fg, bold) = if focused {
                (Color::White, Modifier::BOLD)
            } else if node.target_count > 0 {
                (Color::Yellow, Modifier::empty())
            } else {
                (Color::Blue, Modifier::empty())
            };
            let (check_sym, check_fg) = match node.check {
                CheckState::Checked => ("[x]", Color::Green),
                CheckState::Indeterminate => ("[-]", Color::Yellow),
                CheckState::Unchecked => ("[ ]", Color::DarkGray),
            };
            let n_checked = count_checked_in(node.id, tree);
            let badge = if node.target_count > 0 {
                format!(" {} {}/{}", check_sym, n_checked, node.target_count)
            } else {
                String::new()
            };
            let arrow = if node.expanded { "v" } else { ">" };

            Line::from(vec![
                Span::raw(indent),
                Span::styled(format!("{} ", arrow), Style::default().fg(Color::DarkGray)),
                Span::raw("[D] "),
                Span::styled(
                    node.name.clone(),
                    Style::default().fg(name_fg).add_modifier(bold),
                ),
                Span::styled("/", Style::default().fg(Color::DarkGray)),
                Span::styled(badge, Style::default().fg(check_fg)),
            ])
        }
        NodeKind::ExcludedDir => Line::from(vec![
            Span::raw(indent),
            Span::styled("  [X] ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}/", node.name),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM),
            ),
            Span::styled(
                " (excluded)",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM),
            ),
        ]),
        NodeKind::TargetFile => {
            let checked = node.check == CheckState::Checked;
            let (sym, sym_fg) = if checked {
                ("[x]", Color::Green)
            } else {
                ("[ ]", Color::Yellow)
            };
            let (name_fg, name_mod) = if focused {
                (Color::LightRed, Modifier::BOLD)
            } else if checked {
                (Color::LightGreen, Modifier::BOLD)
            } else {
                (Color::LightYellow, Modifier::empty())
            };
            let size_s = node
                .size
                .map(|s| format!(" ({})", fmt_size(s)))
                .unwrap_or_default();

            Line::from(vec![
                Span::raw(indent),
                Span::styled(format!("  {} ", sym), Style::default().fg(sym_fg)),
                Span::raw("[T] "),
                Span::styled(
                    node.name.clone(),
                    Style::default().fg(name_fg).add_modifier(name_mod),
                ),
                Span::styled(size_s, Style::default().fg(Color::DarkGray)),
            ])
        }
        NodeKind::File => Line::from(vec![
            Span::raw(indent),
            Span::styled("  [F] ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                node.name.clone(),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM),
            ),
        ]),
    };

    ListItem::new(line).style(Style::default().bg(bg))
}

fn count_checked_in(dir_id: usize, tree: &FileTree) -> usize {
    tree.nodes[dir_id]
        .children
        .iter()
        .map(|&cid| match tree.nodes[cid].kind {
            NodeKind::TargetFile => {
                if tree.nodes[cid].check == CheckState::Checked {
                    1
                } else {
                    0
                }
            }
            NodeKind::Dir => count_checked_in(cid, tree),
            _ => 0,
        })
        .sum()
}

pub(super) fn draw_info_panel(f: &mut Frame, area: Rect, tree: &FileTree) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled("info", Style::default().fg(Color::DarkGray)));

    let (total, checked) = tree.stats();
    let mut lines: Vec<Line> = Vec::new();

    if let Some(id) = tree.cursor_node_id() {
        let node = &tree.nodes[id];
        lines.push(Line::from(vec![
            Span::styled("name ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                node.name.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        let path_s = node.path.to_string_lossy().to_string();
        let max_w = area.width.saturating_sub(7) as usize;
        let disp = if path_s.len() > max_w && max_w > 4 {
            format!("...{}", &path_s[path_s.len().saturating_sub(max_w - 3)..])
        } else {
            path_s
        };
        lines.push(Line::from(vec![
            Span::styled("path ", Style::default().fg(Color::DarkGray)),
            Span::styled(disp, Style::default().fg(Color::DarkGray)),
        ]));
        lines.push(Line::raw(""));

        match node.kind {
            NodeKind::TargetFile => {
                lines.push(Line::from(Span::styled(
                    "target file",
                    Style::default()
                        .fg(Color::LightRed)
                        .add_modifier(Modifier::BOLD),
                )));
                if let Some(sz) = node.size {
                    lines.push(Line::from(vec![
                        Span::styled("size ", Style::default().fg(Color::DarkGray)),
                        Span::styled(fmt_size(sz), Style::default().fg(Color::White)),
                    ]));
                }
                lines.push(Line::raw(""));
                let (sym, fg, label) = if node.check == CheckState::Checked {
                    ("[x]", Color::Green, "selected")
                } else {
                    ("[ ]", Color::DarkGray, "not selected")
                };
                lines.push(Line::from(vec![
                    Span::raw(format!("{} ", sym)),
                    Span::styled(label, Style::default().fg(fg)),
                ]));
            }
            NodeKind::Dir => {
                lines.push(Line::from(Span::styled(
                    "directory",
                    Style::default().fg(Color::Blue),
                )));
                lines.push(Line::from(vec![
                    Span::styled("targets ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        node.target_count.to_string(),
                        Style::default().fg(if node.target_count > 0 {
                            Color::Yellow
                        } else {
                            Color::DarkGray
                        }),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("children ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        node.children.len().to_string(),
                        Style::default().fg(Color::White),
                    ),
                ]));
            }
            NodeKind::ExcludedDir => {
                lines.push(Line::from(Span::styled(
                    "excluded directory",
                    Style::default().fg(Color::DarkGray),
                )));
                lines.push(Line::from(Span::styled(
                    "(not scanned)",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::DIM),
                )));
            }
            NodeKind::File => {
                lines.push(Line::from(Span::styled(
                    "regular file",
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled("selected ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}/{}", checked, total),
            Style::default()
                .fg(if checked > 0 {
                    Color::Cyan
                } else {
                    Color::DarkGray
                })
                .add_modifier(if checked > 0 {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ),
    ]));

    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        "keys",
        Style::default().fg(Color::DarkGray),
    )));
    for (k, v) in [
        ("Space", "toggle"),
        ("a", "select all"),
        ("A", "clear all"),
        ("Right/l", "expand"),
        ("Left/h", "collapse"),
        ("E", "expand all"),
        ("C", "collapse all"),
        ("/", "filter"),
        ("d/Del", "delete"),
        ("q", "quit"),
    ] {
        lines.push(Line::from(vec![
            Span::styled(format!("{:<8}", k), Style::default().fg(Color::Cyan)),
            Span::styled(v, Style::default().fg(Color::DarkGray)),
        ]));
    }

    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}
