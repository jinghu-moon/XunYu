//! Find CLI 定义（clap derive）
//!
//! 新架构的 find 命令定义，替代 argh 版本。

use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::xun_core::table_row::TableRow;
use crate::xun_core::value::{ColumnDef, Value, ValueKind};

/// Find files and directories by pattern and metadata.
#[derive(Parser, Debug, Clone)]
#[command(
    after_help = "EXAMPLES:\n    \
        xun find -i \"*.rs\"                # find all .rs files\n    \
        xun find -i \"*.rs\" -e \"target\"   # exclude target dir\n    \
        xun find --size +1M               # files larger than 1MB\n    \
        xun find --modified 7d            # modified in last 7 days\n    \
        xun find -i \"*.log\" --delete     # find and delete logs"
)]
pub struct FindCmd {
    /// base directories (default: cwd)
    pub paths: Vec<String>,

    /// include glob pattern (repeatable or comma separated)
    #[arg(short = 'i', long)]
    pub include: Vec<String>,

    /// exclude glob pattern (repeatable or comma separated)
    #[arg(short = 'e', long)]
    pub exclude: Vec<String>,

    /// include regex pattern (repeatable)
    #[arg(long)]
    pub regex_include: Vec<String>,

    /// exclude regex pattern (repeatable)
    #[arg(long)]
    pub regex_exclude: Vec<String>,

    /// include extensions (comma separated, repeatable)
    #[arg(long)]
    pub extension: Vec<String>,

    /// exclude extensions (comma separated, repeatable)
    #[arg(long)]
    pub not_extension: Vec<String>,

    /// include names (comma separated, repeatable)
    #[arg(long)]
    pub name: Vec<String>,

    /// load rules from file (glob, default exclude)
    #[arg(short = 'F', long)]
    pub filter_file: Option<String>,

    /// size filter (repeatable)
    #[arg(short = 's', long)]
    pub size: Vec<String>,

    /// fuzzy size filter
    #[arg(long)]
    pub fuzzy_size: Option<String>,

    /// mtime filter (repeatable)
    #[arg(long)]
    pub mtime: Vec<String>,

    /// ctime filter (repeatable)
    #[arg(long)]
    pub ctime: Vec<String>,

    /// atime filter (repeatable)
    #[arg(long)]
    pub atime: Vec<String>,

    /// depth filter
    #[arg(short = 'd', long)]
    pub depth: Option<String>,

    /// attribute filter (e.g. +h,-r)
    #[arg(long)]
    pub attribute: Option<String>,

    /// only empty files
    #[arg(long)]
    pub empty_files: bool,

    /// exclude empty files
    #[arg(long)]
    pub not_empty_files: bool,

    /// only empty directories
    #[arg(long)]
    pub empty_dirs: bool,

    /// exclude empty directories
    #[arg(long)]
    pub not_empty_dirs: bool,

    /// case sensitive matching
    #[arg(long)]
    pub case: bool,

    /// count only
    #[arg(short = 'c', long)]
    pub count: bool,

    /// dry run (no filesystem scan)
    #[arg(long)]
    pub dry_run: bool,

    /// test path for dry run
    #[arg(long)]
    pub test_path: Option<String>,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}

// ============================================================
// FindResult — 查找结果输出类型
// ============================================================

/// 查找结果条目。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindResult {
    /// 文件路径
    pub path: String,
    /// 文件类型（file/dir/link）
    pub kind: String,
    /// 文件大小（字节）
    pub size: u64,
}

impl FindResult {
    pub fn new(path: impl Into<String>, kind: impl Into<String>, size: u64) -> Self {
        Self {
            path: path.into(),
            kind: kind.into(),
            size,
        }
    }
}

impl TableRow for FindResult {
    fn columns() -> Vec<ColumnDef> {
        vec![
            ColumnDef::new("path", ValueKind::String),
            ColumnDef::new("kind", ValueKind::String),
            ColumnDef::new("size", ValueKind::Filesize),
        ]
    }

    fn cells(&self) -> Vec<Value> {
        vec![
            Value::String(self.path.clone()),
            Value::String(self.kind.clone()),
            Value::Filesize(self.size),
        ]
    }
}

// ============================================================
// CommandSpec 实现
// ============================================================

use crate::xun_core::command::CommandSpec;
use crate::xun_core::context::CmdContext;
use crate::xun_core::error::XunError;

/// find 命令。
pub struct FindCmdSpec {
    pub args: FindCmd,
}

impl CommandSpec for FindCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        crate::find::cmd_find(self.args.clone())?;
        Ok(Value::Null)
    }
}
