use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use dialoguer::{Confirm, Input, Select};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph};

use crate::env_core::types::{
    DoctorReport, EnvAuditEntry, EnvProfileMeta, EnvScope, EnvVar, ExportFormat, ImportStrategy,
    SnapshotMeta,
};
use crate::env_core::{EnvManager, doctor, uac};
use crate::output::{CliError, CliResult};

mod app_state;
mod event_loop;
mod keymap;
mod prompts;
mod render;

use app_state::*;
pub(crate) use event_loop::run_env_tui;
use keymap::*;
use prompts::*;
use render::*;

static FORCE_FULL_REDRAW: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Panel {
    Vars,
    Path,
    Snapshots,
    Profiles,
    History,
    Doctor,
    Io,
}

impl Panel {
    fn next(self) -> Self {
        match self {
            Self::Vars => Self::Path,
            Self::Path => Self::Snapshots,
            Self::Snapshots => Self::Profiles,
            Self::Profiles => Self::History,
            Self::History => Self::Doctor,
            Self::Doctor => Self::Io,
            Self::Io => Self::Vars,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Vars => "Vars",
            Self::Path => "PATH",
            Self::Snapshots => "Snapshots",
            Self::Profiles => "Profiles",
            Self::History => "History",
            Self::Doctor => "Doctor",
            Self::Io => "Import/Export",
        }
    }
}
