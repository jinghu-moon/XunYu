use crossterm::event::KeyCode;

use super::app_state::{App, Tab};

pub(super) fn handle_confirm_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Some(path) = app.show_confirm.take() {
                match std::fs::remove_file(&path) {
                    Ok(()) => {
                        app.remove_from_issues(&path);
                        app.message = Some(("Deleted.".into(), false));
                    }
                    Err(e) => {
                        app.message = Some((format!("Error: {}", e), true));
                    }
                }
            }
        }
        _ => {
            app.show_confirm = None;
            app.message = Some(("Cancelled.".into(), false));
        }
    }
}

pub(super) fn handle_main_key(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => true,
        KeyCode::Tab => {
            app.tab = match app.tab {
                Tab::Stats => Tab::Issues,
                Tab::Issues => Tab::Stats,
            };
            app.message = None;
            false
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_cursor(-1);
            false
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.move_cursor(1);
            false
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            app.delete_selected();
            false
        }
        KeyCode::Char('e') | KeyCode::Char('E') => {
            app.export_json();
            false
        }
        _ => false,
    }
}
