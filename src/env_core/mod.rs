pub mod annotations;
pub mod audit;
pub mod batch;
pub mod config;
pub mod dep_graph;
pub mod diff;
pub mod doctor;
pub mod events;
pub mod io;
pub mod lock;
pub mod notifier;
pub mod profile;
pub mod registry;
pub mod schema;
pub mod snapshot;
pub mod template;
pub mod types;
pub mod uac;
pub mod var_type;
pub mod watch;

mod manager;
mod ops_io;
mod ops_profile;
mod ops_read;
mod ops_run;
mod ops_schema;
mod ops_snapshot;
mod ops_write;
mod write_guard;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};

use config::{
    EnvCoreConfig, config_file_path, get_config_value, load_env_config, reset_env_config,
    save_env_config, set_config_value,
};
use types::{
    AnnotationEntry, BatchResult, DoctorFixResult, DoctorReport, EnvAuditEntry, EnvDiff, EnvError,
    EnvEvent, EnvEventType, EnvProfileMeta, EnvResult, EnvSchema, EnvScope, EnvStatusSummary,
    EnvVar, EnvWatchEvent, ExportFormat, ImportApplyResult, ImportStrategy, LiveExportFormat,
    RunCommandResult, SchemaRule, ShellExportFormat, SnapshotMeta, TemplateExpandResult,
    TemplateValidationReport, ValidationReport,
};

pub type EventCallback = Arc<dyn Fn(EnvEvent) + Send + Sync>;

pub use manager::EnvManager;
pub use types::*;
