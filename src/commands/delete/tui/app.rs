use std::collections::HashSet;
use std::io;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{self},
};
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend, widgets::ListState};
use regex::Regex;

use super::super::tree::{FileTree, NodeKind};
use super::super::{DeleteOptions, DeleteRecord};
use super::draw::draw;
use super::types::AppState;

pub(crate) fn run(
    root: PathBuf,
    target_names: &HashSet<String>,
    match_all: bool,
    exclude_dirs: &HashSet<String>,
    patterns: &[Regex],
    opts: &DeleteOptions,
) -> crate::output::CliResult<Vec<DeleteRecord>> {
    let res = run_inner(root, target_names, match_all, exclude_dirs, patterns, opts)
        .map_err(|e| crate::output::CliError::new(1, format!("TUI error: {e}")))?;
    Ok(res)
}

fn run_inner(
    root: PathBuf,
    target_names: &HashSet<String>,
    match_all: bool,
    exclude_dirs: &HashSet<String>,
    patterns: &[Regex],
    opts: &DeleteOptions,
) -> io::Result<Vec<DeleteRecord>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let results = app_loop(
        &mut term,
        root,
        target_names,
        match_all,
        exclude_dirs,
        patterns,
        opts,
    );

    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen)?;
    term.show_cursor()?;

    Ok(results.unwrap_or_default())
}

fn app_loop(
    term: &mut Terminal<CrosstermBackend<io::Stdout>>,
    root: PathBuf,
    target_names: &HashSet<String>,
    match_all: bool,
    exclude_dirs: &HashSet<String>,
    patterns: &[Regex],
    opts: &DeleteOptions,
) -> io::Result<Vec<DeleteRecord>> {
    let cancel_scan = Arc::new(AtomicBool::new(false));
    let (scan_tx, scan_rx) = mpsc::channel::<FileTree>();
    {
        let root_c = root.clone();
        let tgt_c = target_names.clone();
        let excl_c = exclude_dirs.clone();
        let pat_c: Vec<Regex> = patterns.to_vec();
        let cancel_c = cancel_scan.clone();
        std::thread::spawn(move || {
            let _ = scan_tx.send(FileTree::build(
                &root_c, &tgt_c, match_all, &excl_c, &pat_c, &cancel_c,
            ));
        });
    }

    let mut state: AppState = AppState::Loading;
    let mut tree: Option<FileTree> = None;
    let mut list_state: ListState = ListState::default();
    let mut spinner_i: usize = 0;
    let spinner: &[&str] = &["-", "\\", "|", "/"];
    let tick: Duration = Duration::from_millis(80);
    let mut last_tick: Instant = Instant::now();
    let title: String = root.to_string_lossy().to_string();

    loop {
        if matches!(state, AppState::Loading) {
            if let Ok(t) = scan_rx.try_recv() {
                tree = Some(t);
                state = AppState::Browsing;
            }
        }

        if let AppState::Deleting { rx } = &state {
            if let Ok(results) = rx.try_recv() {
                state = AppState::Done(results);
            }
        }

        let sp = spinner[spinner_i % spinner.len()];
        term.draw(|f| {
            let area = f.area();
            draw(
                f,
                area,
                &state,
                tree.as_ref(),
                &mut list_state,
                &title,
                sp,
                opts.dry_run,
            );
        })?;

        let timeout = tick
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::ZERO);
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match &mut state {
                    AppState::Loading => {
                        if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
                            cancel_scan.store(true, Ordering::Relaxed);
                            return Ok(vec![]);
                        }
                    }
                    AppState::Browsing => {
                        if let Some(t) = tree.as_mut() {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => return Ok(vec![]),

                                KeyCode::Up | KeyCode::Char('k') => t.move_up(),
                                KeyCode::Down | KeyCode::Char('j') => t.move_down(),
                                KeyCode::PageUp => t.move_page_up(20),
                                KeyCode::PageDown => t.move_page_down(20),
                                KeyCode::Home | KeyCode::Char('g') => t.jump_top(),
                                KeyCode::End | KeyCode::Char('G') => t.jump_bottom(),

                                KeyCode::Right | KeyCode::Char('l') => t.expand_cursor(),
                                KeyCode::Left | KeyCode::Char('h') => t.collapse_cursor(),
                                KeyCode::Char('E') => t.expand_all(),
                                KeyCode::Char('C') => t.collapse_all(),

                                KeyCode::Enter => {
                                    if let Some(id) = t.cursor_node_id() {
                                        if t.nodes[id].kind == NodeKind::Dir {
                                            t.toggle_expand_cursor();
                                        } else {
                                            t.toggle_check_cursor();
                                        }
                                    }
                                }

                                KeyCode::Char(' ') => t.toggle_check_cursor(),
                                KeyCode::Char('a') => t.check_all_targets(),
                                KeyCode::Char('A') => t.uncheck_all(),

                                KeyCode::Char('/') => {
                                    if let Some(t) = tree.as_mut() {
                                        t.filter.clear();
                                        t.filter_active = false;
                                    }
                                    state = AppState::Filtering;
                                }

                                KeyCode::Char('d') | KeyCode::Delete => {
                                    if !t.checked_paths().is_empty() {
                                        state = AppState::ConfirmDelete;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    AppState::Filtering => {
                        if let Some(t) = tree.as_mut() {
                            match key.code {
                                KeyCode::Esc => {
                                    t.filter.clear();
                                    t.filter_active = false;
                                    state = AppState::Browsing;
                                }
                                KeyCode::Enter => {
                                    t.filter_active = !t.filter.is_empty();
                                    state = AppState::Browsing;
                                }
                                KeyCode::Backspace => {
                                    t.filter.pop();
                                }
                                KeyCode::Char(c)
                                    if !key
                                        .modifiers
                                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                                {
                                    t.filter.push(c);
                                    t.filter_active = true;
                                }
                                _ => {}
                            }
                        }
                    }
                    AppState::ConfirmDelete => match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                            if let Some(t) = &tree {
                                let paths = t.checked_paths();
                                let opts_c = opts.clone();
                                let (dtx, drx) = mpsc::channel::<Vec<DeleteRecord>>();

                                std::thread::spawn(move || {
                                    let res =
                                        super::super::pipeline::delete_paths(paths, &opts_c, None);
                                    let _ = dtx.send(res);
                                });

                                state = AppState::Deleting { rx: drx };
                            }
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            state = AppState::Browsing;
                        }
                        _ => {}
                    },
                    AppState::Deleting { .. } => {}
                    AppState::Done(results) => {
                        if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter) {
                            return Ok(results.clone());
                        }
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick {
            spinner_i = spinner_i.wrapping_add(1);
            last_tick = Instant::now();
        }
    }
}
