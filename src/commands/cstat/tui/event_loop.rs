use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::output::{CliError, CliResult};

use super::app_state::App;
use super::keymap;

impl App {
    pub(super) fn run(
        &mut self,
        term: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> CliResult {
        loop {
            term.draw(|f| self.render(f))
                .map_err(|e| CliError::new(1, format!("TUI draw: {}", e)))?;

            let ev = event::read().map_err(|e| CliError::new(1, format!("TUI event: {}", e)))?;
            if let Event::Key(key) = ev {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                if self.show_confirm.is_some() {
                    keymap::handle_confirm_key(self, key.code);
                    continue;
                }

                if keymap::handle_main_key(self, key.code) {
                    break;
                }
            }
        }
        Ok(())
    }
}
