//! Bookmark CLI 定义（clap derive）
//!
//! 新架构的 bookmark 命令定义，替代 argh 版本。
//! 共 27 个子命令，其中 Z/Zi/O/Oi/Open 共享 FuzzyArgs。

use clap::{Args, Parser, Subcommand};

use super::table_row::TableRow;
use super::value::{ColumnDef, Value, ValueKind};

// ── Bookmark 主命令 ──────────────────────────────────────────────

/// Bookmark management and navigation.
#[derive(Parser, Debug, Clone)]
#[command(name = "bookmark", about = "Bookmark management and navigation")]
pub struct BookmarkCmd {
    #[command(subcommand)]
    pub sub: BookmarkSubCommand,
}

/// Bookmark 子命令枚举（27 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum BookmarkSubCommand {
    /// Jump to a bookmark (fuzzy match)
    Z(BookmarkZArgs),
    /// Jump to a bookmark with interactive selection
    Zi(BookmarkZiArgs),
    /// Open in Explorer (fuzzy match)
    O(BookmarkOArgs),
    /// Open a bookmark with interactive selection
    Oi(BookmarkOiArgs),
    /// Open in file manager
    Open(BookmarkOpenArgs),
    /// Save current directory as bookmark
    Save(BookmarkSaveArgs),
    /// Save current directory or specific path as bookmark
    Set(BookmarkSetArgs),
    /// Delete a bookmark
    #[command(name = "rm", alias = "delete")]
    Rm(BookmarkDeleteArgs),
    /// Tag management
    Tag(BookmarkTagCmd),
    /// Pin a bookmark
    Pin(BookmarkPinArgs),
    /// Remove pin from a bookmark
    Unpin(BookmarkUnpinArgs),
    /// Undo previous bookmark mutations
    Undo(BookmarkUndoArgs),
    /// Redo previously undone bookmark mutations
    Redo(BookmarkRedoArgs),
    /// Rename a bookmark
    Rename(BookmarkRenameArgs),
    /// List all bookmarks
    List(BookmarkListArgs),
    /// Show recent bookmarks
    Recent(BookmarkRecentArgs),
    /// Show statistics
    Stats(BookmarkStatsArgs),
    /// Check bookmark health
    Check(BookmarkCheckArgs),
    /// Clean up dead links
    Gc(BookmarkGcArgs),
    /// Deduplicate bookmarks
    Dedup(BookmarkDedupArgs),
    /// Export bookmarks
    Export(BookmarkExportArgs),
    /// Import bookmarks
    Import(BookmarkImportArgs),
    /// Generate bookmark shell integration
    Init(BookmarkInitArgs),
    /// Record a visited directory for auto-learn
    Learn(BookmarkLearnArgs),
    /// Update frecency (touch)
    Touch(BookmarkTouchArgs),
    /// List all keys (for tab completion)
    Keys(BookmarkKeysArgs),
    /// All bookmarks (machine output)
    All(BookmarkAllArgs),
}

// ── 共享参数：FuzzyArgs（Z/Zi/O/Oi/Open 复用） ──────────────────

/// 模糊匹配子命令的公共参数。
///
/// Z、Zi、O、Oi、Open 五个子命令参数完全一致，抽取共享结构体。
#[derive(Args, Debug, Clone)]
pub struct FuzzyArgs {
    /// fuzzy pattern
    pub patterns: Vec<String>,

    /// filter by tag
    #[arg(short = 't', long)]
    pub tag: Option<String>,

    /// list matches instead of executing
    #[arg(short = 'l', long)]
    pub list: bool,

    /// show factor scores
    #[arg(short = 's', long)]
    pub score: bool,

    /// explain top-1 result
    #[arg(long)]
    pub why: bool,

    /// preview only; do not execute
    #[arg(long)]
    pub preview: bool,

    /// limit listed results
    #[arg(short = 'n', long)]
    pub limit: Option<usize>,

    /// output json
    #[arg(long)]
    pub json: bool,

    /// output tsv
    #[arg(long)]
    pub tsv: bool,

    /// use global scope
    #[arg(short = 'g', long)]
    pub global: bool,

    /// prefer child scope
    #[arg(short = 'c', long)]
    pub child: bool,

    /// restrict to base dir
    #[arg(long)]
    pub base: Option<String>,

    /// workspace scope
    #[arg(short = 'w', long)]
    pub workspace: Option<String>,

    /// use config preset
    #[arg(long)]
    pub preset: Option<String>,
}

// ── Z / Zi / O / Oi / Open ──────────────────────────────────────

/// Jump to a bookmark (fuzzy match).
#[derive(Parser, Debug, Clone)]
pub struct BookmarkZArgs {
    #[command(flatten)]
    pub fuzzy: FuzzyArgs,
}

/// Jump to a bookmark with interactive selection.
#[derive(Parser, Debug, Clone)]
pub struct BookmarkZiArgs {
    #[command(flatten)]
    pub fuzzy: FuzzyArgs,
}

/// Open in Explorer (fuzzy match).
#[derive(Parser, Debug, Clone)]
pub struct BookmarkOArgs {
    #[command(flatten)]
    pub fuzzy: FuzzyArgs,
}

/// Open a bookmark with interactive selection.
#[derive(Parser, Debug, Clone)]
pub struct BookmarkOiArgs {
    #[command(flatten)]
    pub fuzzy: FuzzyArgs,
}

/// Open in file manager.
#[derive(Parser, Debug, Clone)]
pub struct BookmarkOpenArgs {
    #[command(flatten)]
    pub fuzzy: FuzzyArgs,
}

// ── Save / Set ──────────────────────────────────────────────────

/// Save current directory as bookmark.
#[derive(Parser, Debug, Clone)]
pub struct BookmarkSaveArgs {
    /// bookmark name (optional, defaults to current dir name)
    pub name: Option<String>,

    /// tags (comma separated)
    #[arg(short = 't', long)]
    pub tag: Option<String>,

    /// description
    #[arg(long)]
    pub desc: Option<String>,

    /// workspace label
    #[arg(short = 'w', long)]
    pub workspace: Option<String>,
}

/// Save current directory or specific path as bookmark.
#[derive(Parser, Debug, Clone)]
pub struct BookmarkSetArgs {
    /// bookmark name
    pub name: String,

    /// path (optional, defaults to current dir)
    pub path: Option<String>,

    /// tags (comma separated)
    #[arg(short = 't', long)]
    pub tag: Option<String>,

    /// description
    #[arg(long)]
    pub desc: Option<String>,

    /// workspace label
    #[arg(short = 'w', long)]
    pub workspace: Option<String>,
}

// ── Delete / Pin / Unpin / Touch ─────────────────────────────────

/// Delete a bookmark.
#[derive(Parser, Debug, Clone)]
pub struct BookmarkDeleteArgs {
    /// bookmark name
    pub name: String,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Pin a bookmark.
#[derive(Parser, Debug, Clone)]
pub struct BookmarkPinArgs {
    /// bookmark name
    pub name: String,
}

/// Remove pin from a bookmark.
#[derive(Parser, Debug, Clone)]
pub struct BookmarkUnpinArgs {
    /// bookmark name
    pub name: String,
}

/// Update frecency (touch).
#[derive(Parser, Debug, Clone)]
pub struct BookmarkTouchArgs {
    /// bookmark name
    pub name: String,
}

// ── Undo / Redo ─────────────────────────────────────────────────

/// Undo previous bookmark mutations.
#[derive(Parser, Debug, Clone)]
pub struct BookmarkUndoArgs {
    /// number of undo steps
    #[arg(short = 'n', long, default_value_t = 1)]
    pub steps: usize,
}

/// Redo previously undone bookmark mutations.
#[derive(Parser, Debug, Clone)]
pub struct BookmarkRedoArgs {
    /// number of redo steps
    #[arg(short = 'n', long, default_value_t = 1)]
    pub steps: usize,
}

// ── Rename ──────────────────────────────────────────────────────

/// Rename a bookmark.
#[derive(Parser, Debug, Clone)]
pub struct BookmarkRenameArgs {
    /// old name
    pub old: String,

    /// new name
    pub new: String,
}

// ── List / Recent / Stats / Check ───────────────────────────────

/// List all bookmarks.
#[derive(Parser, Debug, Clone)]
pub struct BookmarkListArgs {
    /// filter by tag
    #[arg(short = 't', long)]
    pub tag: Option<String>,

    /// sort by: name | last | visits
    #[arg(short = 's', long, default_value = "name")]
    pub sort: String,

    /// limit results
    #[arg(short = 'n', long)]
    pub limit: Option<usize>,

    /// offset results
    #[arg(long)]
    pub offset: Option<usize>,

    /// reverse sort order
    #[arg(long)]
    pub reverse: bool,

    /// output as TSV (Fast Path)
    #[arg(long)]
    pub tsv: bool,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}

/// Show recent bookmarks.
#[derive(Parser, Debug, Clone)]
pub struct BookmarkRecentArgs {
    /// limit results
    #[arg(short = 'n', long, default_value_t = 10)]
    pub limit: usize,

    /// filter by tag
    #[arg(short = 't', long)]
    pub tag: Option<String>,

    /// filter by workspace
    #[arg(short = 'w', long)]
    pub workspace: Option<String>,

    /// only include records since duration (e.g. 7d, 24h, 30m)
    #[arg(long)]
    pub since: Option<String>,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}

/// Show statistics.
#[derive(Parser, Debug, Clone)]
pub struct BookmarkStatsArgs {
    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,

    /// show usage insights and suggestions
    #[arg(long)]
    pub insights: bool,
}

/// Check bookmark health (missing paths, duplicates, stale).
#[derive(Parser, Debug, Clone)]
pub struct BookmarkCheckArgs {
    /// stale threshold in days
    #[arg(short = 'd', long, default_value_t = 90)]
    pub days: u64,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}

// ── Gc / Dedup / Export / Import ─────────────────────────────────

/// Clean up dead links.
#[derive(Parser, Debug, Clone)]
pub struct BookmarkGcArgs {
    /// delete all dead links without confirmation
    #[arg(long)]
    pub purge: bool,

    /// preview only; do not delete
    #[arg(long)]
    pub dry_run: bool,

    /// only clean learned/imported records
    #[arg(long)]
    pub learned: bool,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,
}

/// Deduplicate bookmarks.
#[derive(Parser, Debug, Clone)]
pub struct BookmarkDedupArgs {
    /// mode: path | name
    #[arg(short = 'm', long, default_value = "path")]
    pub mode: String,

    /// output format: auto|table|tsv|json
    #[arg(short = 'f', long, default_value = "auto")]
    pub format: String,

    /// skip confirmation (interactive mode only)
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// Export bookmarks.
#[derive(Parser, Debug, Clone)]
pub struct BookmarkExportArgs {
    /// format: json | tsv
    #[arg(short = 'f', long, default_value = "json")]
    pub format: String,

    /// output file (optional)
    #[arg(short = 'o', long)]
    pub out: Option<String>,
}

/// Import bookmarks.
#[derive(Parser, Debug, Clone)]
pub struct BookmarkImportArgs {
    /// format: json | tsv
    #[arg(short = 'f', long, default_value = "json")]
    pub format: String,

    /// import source: autojump | zoxide | z | fasd | history
    #[arg(long)]
    pub from: Option<String>,

    /// input file (optional, default stdin)
    #[arg(short = 'i', long)]
    pub input: Option<String>,

    /// mode: merge | overwrite
    #[arg(short = 'm', long, default_value = "merge")]
    pub mode: String,

    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}

// ── Init / Learn / Keys / All ───────────────────────────────────

/// Generate bookmark shell integration.
#[derive(Parser, Debug, Clone)]
pub struct BookmarkInitArgs {
    /// shell type: powershell | bash | zsh | fish
    pub shell: String,

    /// custom command prefix (e.g. j -> j/ji/jo/joi)
    #[arg(long)]
    pub cmd: Option<String>,
}

/// Record a visited directory for auto-learn.
#[derive(Parser, Debug, Clone)]
pub struct BookmarkLearnArgs {
    /// path to learn
    #[arg(long)]
    pub path: String,
}

/// List all keys (for tab completion).
#[derive(Parser, Debug, Clone)]
pub struct BookmarkKeysArgs {}

/// All bookmarks (machine output).
#[derive(Parser, Debug, Clone)]
pub struct BookmarkAllArgs {
    /// filter by tag
    pub tag: Option<String>,
}

// ── Tag 子命令组 ─────────────────────────────────────────────────

/// Tag management.
#[derive(Parser, Debug, Clone)]
pub struct BookmarkTagCmd {
    #[command(subcommand)]
    pub sub: TagSubCommand,
}

/// Tag 子命令枚举（5 个变体）。
#[derive(Subcommand, Debug, Clone)]
pub enum TagSubCommand {
    /// Add tags to a bookmark
    Add(TagAddArgs),
    /// Add tags to multiple bookmarks
    AddBatch(TagAddBatchArgs),
    /// Remove tags from a bookmark
    Remove(TagRemoveArgs),
    /// List all tags and counts
    List(TagListArgs),
    /// Rename a tag across all bookmarks
    Rename(TagRenameArgs),
}

/// Add tags to a bookmark.
#[derive(Parser, Debug, Clone)]
pub struct TagAddArgs {
    /// bookmark name
    pub name: String,

    /// tags (comma separated)
    pub tags: String,
}

/// Add tags to multiple bookmarks.
#[derive(Parser, Debug, Clone)]
pub struct TagAddBatchArgs {
    /// tags (comma separated)
    pub tags: String,

    /// bookmark names
    pub names: Vec<String>,
}

/// Remove tags from a bookmark.
#[derive(Parser, Debug, Clone)]
pub struct TagRemoveArgs {
    /// bookmark name
    pub name: String,

    /// tags (comma separated)
    pub tags: String,
}

/// List all tags and counts.
#[derive(Parser, Debug, Clone)]
pub struct TagListArgs {}

/// Rename a tag across all bookmarks.
#[derive(Parser, Debug, Clone)]
pub struct TagRenameArgs {
    /// old tag
    pub old: String,

    /// new tag
    pub new: String,
}

// ── BookmarkEntry 输出类型 ───────────────────────────────────────

/// 书签条目，用于表格/JSON 输出。
#[derive(Debug, Clone)]
pub struct BookmarkEntry {
    pub name: String,
    pub path: String,
    pub tags: String,
    pub visits: u64,
    pub last_used: String,
    pub pinned: bool,
}

impl BookmarkEntry {
    pub fn new(
        name: impl Into<String>,
        path: impl Into<String>,
        tags: impl Into<String>,
        visits: u64,
        last_used: impl Into<String>,
        pinned: bool,
    ) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            tags: tags.into(),
            visits,
            last_used: last_used.into(),
            pinned,
        }
    }
}

impl TableRow for BookmarkEntry {
    fn columns() -> Vec<ColumnDef> {
        vec![
            ColumnDef::new("name", ValueKind::String),
            ColumnDef::new("path", ValueKind::Path),
            ColumnDef::new("tags", ValueKind::String),
            ColumnDef::new("visits", ValueKind::Int),
            ColumnDef::new("last_used", ValueKind::Date),
            ColumnDef::new("pinned", ValueKind::Bool),
        ]
    }

    fn cells(&self) -> Vec<Value> {
        vec![
            Value::String(self.name.clone()),
            Value::String(self.path.clone()),
            Value::String(self.tags.clone()),
            Value::Int(self.visits as i64),
            Value::String(self.last_used.clone()),
            Value::Bool(self.pinned),
        ]
    }
}

// ============================================================
// CommandSpec 实现
// ============================================================

use super::command::CommandSpec;
use super::context::CmdContext;
use super::error::XunError;
use super::services::bookmark as bookmark_svc;

// ── 导航命令（Z / Zi / O / Oi / Open）────────────────────────

/// bookmark z — 跳转到书签（模糊匹配）。
pub struct BookmarkZCmd {
    pub args: BookmarkZArgs,
}

impl CommandSpec for BookmarkZCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let f = &self.args.fuzzy;
        bookmark_svc::z_bookmark(
            &f.patterns,
            f.tag.as_deref(),
            f.list,
            f.score,
            f.why,
            f.preview,
            f.limit,
            f.global,
            f.child,
            f.base.as_deref(),
            f.workspace.as_deref(),
            f.preset.as_deref(),
        )
    }
}

/// bookmark zi — 交互式选择书签。
pub struct BookmarkZiCmd {
    pub args: BookmarkZiArgs,
}

impl CommandSpec for BookmarkZiCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let f = &self.args.fuzzy;
        bookmark_svc::zi_bookmark(&f.patterns, f.tag.as_deref(), f.global)
    }
}

/// bookmark o — 在 Explorer 中打开书签。
pub struct BookmarkOCmd {
    pub args: BookmarkOArgs,
}

impl CommandSpec for BookmarkOCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let f = &self.args.fuzzy;
        bookmark_svc::o_bookmark(&f.patterns, f.tag.as_deref(), f.global)
    }
}

/// bookmark oi — 交互式选择后在 Explorer 打开。
pub struct BookmarkOiCmd {
    pub args: BookmarkOiArgs,
}

impl CommandSpec for BookmarkOiCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let f = &self.args.fuzzy;
        bookmark_svc::oi_bookmark(&f.patterns, f.tag.as_deref(), f.global)
    }
}

/// bookmark open — 在文件管理器中打开书签。
pub struct BookmarkOpenCmd {
    pub args: BookmarkOpenArgs,
}

impl CommandSpec for BookmarkOpenCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let f = &self.args.fuzzy;
        bookmark_svc::open_bookmark(&f.patterns, f.tag.as_deref(), f.global)
    }
}

// ── CRUD 命令（Save / Set / Rename / Delete）──────────────────

/// bookmark save — 保存当前目录为书签。
pub struct BookmarkSaveCmd {
    pub args: BookmarkSaveArgs,
}

impl CommandSpec for BookmarkSaveCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        bookmark_svc::save_bookmark(
            self.args.name.as_deref(),
            self.args.tag.as_deref(),
            self.args.desc.as_deref(),
            self.args.workspace.as_deref(),
        )
    }
}

/// bookmark set — 设置书签（指定路径）。
pub struct BookmarkSetCmd {
    pub args: BookmarkSetArgs,
}

impl CommandSpec for BookmarkSetCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.name.is_empty() {
            return Err(XunError::user("bookmark name is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        bookmark_svc::set_bookmark(
            &self.args.name,
            self.args.path.as_deref(),
            self.args.tag.as_deref(),
            self.args.desc.as_deref(),
            self.args.workspace.as_deref(),
        )
    }
}

/// bookmark rm — 删除书签。
pub struct BookmarkRmCmd {
    pub args: BookmarkDeleteArgs,
}

impl CommandSpec for BookmarkRmCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.name.is_empty() {
            return Err(XunError::user("bookmark name is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        // 使用 Operation trait 实现（支持 preview/rollback）
        let op = bookmark_svc::BookmarkDeleteOp::new(&self.args.name);
        use super::operation::Operation;
        op.execute(_ctx)?;
        Ok(Value::Null)
    }
}

/// bookmark rename — 重命名书签。
pub struct BookmarkRenameCmd {
    pub args: BookmarkRenameArgs,
}

impl CommandSpec for BookmarkRenameCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.old.is_empty() || self.args.new.is_empty() {
            return Err(XunError::user("both old and new names are required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        bookmark_svc::rename_bookmark(&self.args.old, &self.args.new)?;
        Ok(Value::Null)
    }
}

// ── Pin / Unpin / Touch ──────────────────────────────────────

/// bookmark pin — 固定书签。
pub struct BookmarkPinCmd {
    pub args: BookmarkPinArgs,
}

impl CommandSpec for BookmarkPinCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.name.is_empty() {
            return Err(XunError::user("bookmark name is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        bookmark_svc::pin_bookmark(&self.args.name)?;
        Ok(Value::Null)
    }
}

/// bookmark unpin — 取消固定书签。
pub struct BookmarkUnpinCmd {
    pub args: BookmarkUnpinArgs,
}

impl CommandSpec for BookmarkUnpinCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.name.is_empty() {
            return Err(XunError::user("bookmark name is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        bookmark_svc::unpin_bookmark(&self.args.name)?;
        Ok(Value::Null)
    }
}

/// bookmark touch — 更新书签访问计数。
pub struct BookmarkTouchCmd {
    pub args: BookmarkTouchArgs,
}

impl CommandSpec for BookmarkTouchCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.name.is_empty() {
            return Err(XunError::user("bookmark name is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        bookmark_svc::touch_bookmark(&self.args.name)?;
        Ok(Value::Null)
    }
}

// ── Undo / Redo ──────────────────────────────────────────────

/// bookmark undo — 撤销书签操作。
pub struct BookmarkUndoCmd {
    pub args: BookmarkUndoArgs,
}

impl CommandSpec for BookmarkUndoCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let op = bookmark_svc::BookmarkUndoOp::new(self.args.steps);
        use super::operation::Operation;
        let result = op.execute(_ctx)?;
        Ok(Value::Int(result.changes_applied() as i64))
    }
}

/// bookmark redo — 重做书签操作。
pub struct BookmarkRedoCmd {
    pub args: BookmarkRedoArgs,
}

impl CommandSpec for BookmarkRedoCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        let op = bookmark_svc::BookmarkRedoOp::new(self.args.steps);
        use super::operation::Operation;
        let result = op.execute(_ctx)?;
        Ok(Value::Int(result.changes_applied() as i64))
    }
}

// ── 查询命令（List / Recent / Stats / Keys / All）────────────

/// bookmark list — 列出所有书签。
pub struct BookmarkListCmd {
    pub args: BookmarkListArgs,
}

impl CommandSpec for BookmarkListCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        bookmark_svc::list_bookmarks(self.args.tag.as_deref(), None)
    }
}

/// bookmark recent — 显示最近访问的书签。
pub struct BookmarkRecentCmd {
    pub args: BookmarkRecentArgs,
}

impl CommandSpec for BookmarkRecentCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        bookmark_svc::recent_bookmarks(
            self.args.limit,
            self.args.tag.as_deref(),
            self.args.workspace.as_deref(),
        )
    }
}

/// bookmark stats — 显示书签统计。
pub struct BookmarkStatsCmd {
    pub args: BookmarkStatsArgs,
}

impl CommandSpec for BookmarkStatsCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        bookmark_svc::stats_bookmarks()
    }
}

/// bookmark keys — 列出所有书签名称（用于 tab completion）。
pub struct BookmarkKeysCmd {
    pub args: BookmarkKeysArgs,
}

impl CommandSpec for BookmarkKeysCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        bookmark_svc::keys_bookmarks()
    }
}

/// bookmark all — 所有书签（机器输出）。
pub struct BookmarkAllCmd {
    pub args: BookmarkAllArgs,
}

impl CommandSpec for BookmarkAllCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        bookmark_svc::all_bookmarks(self.args.tag.as_deref())
    }
}

// ── 维护命令（Check / Gc / Dedup）────────────────────────────

/// bookmark check — 检查书签健康状态。
pub struct BookmarkCheckCmd {
    pub args: BookmarkCheckArgs,
}

impl CommandSpec for BookmarkCheckCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        bookmark_svc::check_bookmarks(self.args.days)
    }
}

/// bookmark gc — 清理死链。
pub struct BookmarkGcCmd {
    pub args: BookmarkGcArgs,
}

impl CommandSpec for BookmarkGcCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        bookmark_svc::gc_bookmarks(self.args.purge, self.args.dry_run, self.args.learned)
    }
}

/// bookmark dedup — 去重书签。
pub struct BookmarkDedupCmd {
    pub args: BookmarkDedupArgs,
}

impl CommandSpec for BookmarkDedupCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        bookmark_svc::dedup_bookmarks(&self.args.mode, self.args.yes)
    }
}

// ── I/O 命令（Export / Import）───────────────────────────────

/// bookmark export — 导出书签。
pub struct BookmarkExportCmd {
    pub args: BookmarkExportArgs,
}

impl CommandSpec for BookmarkExportCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        bookmark_svc::export_bookmarks(&self.args.format, self.args.out.as_deref())
    }
}

/// bookmark import — 导入书签。
pub struct BookmarkImportCmd {
    pub args: BookmarkImportArgs,
}

impl CommandSpec for BookmarkImportCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        bookmark_svc::import_bookmarks(
            &self.args.format,
            self.args.from.as_deref(),
            self.args.input.as_deref(),
            &self.args.mode,
            self.args.yes,
        )
    }
}

// ── 集成命令（Init / Learn）──────────────────────────────────

/// bookmark init — 生成 shell 集成脚本。
pub struct BookmarkInitCmd {
    pub args: BookmarkInitArgs,
}

impl CommandSpec for BookmarkInitCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        let valid_shells = ["powershell", "bash", "zsh", "fish"];
        if !valid_shells.contains(&self.args.shell.as_str()) {
            return Err(XunError::user(format!(
                "unsupported shell '{}', expected one of: {:?}",
                self.args.shell, valid_shells
            )));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        bookmark_svc::init_bookmark(&self.args.shell, self.args.cmd.as_deref())
    }
}

/// bookmark learn — 学习路径。
pub struct BookmarkLearnCmd {
    pub args: BookmarkLearnArgs,
}

impl CommandSpec for BookmarkLearnCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.path.is_empty() {
            return Err(XunError::user("path is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        bookmark_svc::learn_bookmark(&self.args.path)?;
        Ok(Value::Null)
    }
}

// ── Tag 子命令 ───────────────────────────────────────────────

/// bookmark tag add — 添加标签。
pub struct TagAddCmd {
    pub args: TagAddArgs,
}

impl CommandSpec for TagAddCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.name.is_empty() {
            return Err(XunError::user("bookmark name is required"));
        }
        if self.args.tags.is_empty() {
            return Err(XunError::user("tags are required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        bookmark_svc::tag_add(&self.args.name, &self.args.tags)
    }
}

/// bookmark tag add-batch — 批量添加标签。
pub struct TagAddBatchCmd {
    pub args: TagAddBatchArgs,
}

impl CommandSpec for TagAddBatchCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.names.is_empty() {
            return Err(XunError::user("at least one bookmark name is required"));
        }
        if self.args.tags.is_empty() {
            return Err(XunError::user("tags are required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        bookmark_svc::tag_add_batch(&self.args.names, &self.args.tags)
    }
}

/// bookmark tag remove — 移除标签。
pub struct TagRemoveCmd {
    pub args: TagRemoveArgs,
}

impl CommandSpec for TagRemoveCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.name.is_empty() {
            return Err(XunError::user("bookmark name is required"));
        }
        if self.args.tags.is_empty() {
            return Err(XunError::user("tags are required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        bookmark_svc::tag_remove(&self.args.name, &self.args.tags)
    }
}

/// bookmark tag list — 列出所有标签。
pub struct TagListCmd {
    pub args: TagListArgs,
}

impl CommandSpec for TagListCmd {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        bookmark_svc::tag_list()
    }
}

/// bookmark tag rename — 重命名标签。
pub struct TagRenameCmd {
    pub args: TagRenameArgs,
}

impl CommandSpec for TagRenameCmd {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.old.is_empty() || self.args.new.is_empty() {
            return Err(XunError::user("both old and new tag names are required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        bookmark_svc::tag_rename(&self.args.old, &self.args.new)
    }
}
