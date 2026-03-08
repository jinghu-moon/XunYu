mod app_state;
mod event_loop;
mod keymap;
mod render;

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::cstat::report::Report;
use crate::output::{CliError, CliResult};

use self::app_state::App;

pub(crate) fn run_cstat_tui(report: Report, scan_path: &str) -> CliResult {
    enable_raw_mode().map_err(|e| CliError::new(1, format!("TUI init: {}", e)))?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)
        .map_err(|e| CliError::new(1, format!("TUI init: {}", e)))?;
    let backend = CrosstermBackend::new(stdout);
    let mut term =
        Terminal::new(backend).map_err(|e| CliError::new(1, format!("TUI init: {}", e)))?;

    let result = App::new(report, scan_path.to_owned()).run(&mut term);

    let _ = disable_raw_mode();
    let _ = execute!(term.backend_mut(), LeaveAlternateScreen);
    let _ = term.show_cursor();

    result
}
