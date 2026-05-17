//! Backup CLI 定义（clap derive）
//!
//! 新架构的 backup 命令定义，替代 argh 版本。

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use crate::xun_core::table_row::TableRow;
use crate::xun_core::value::{ColumnDef, Value, ValueKind};

/// 增量项目备份。别名: `bak`。
#[derive(Parser, Debug, Clone)]
#[command(
    name = "backup",
    about = "Incremental project backup",
    after_help = "EXAMPLES:\n    \
        xun backup                     # backup current directory\n    \
        xun backup -m \"before refactor\"  # backup with description\n    \
        xun backup --include src,docs  # backup only src and docs\n    \
        xun backup --dry-run           # preview without writing\n    \
        xun backup list                # list existing backups\n    \
        xun backup restore --latest    # restore the latest backup"
)]
pub struct BackupCmd {
    #[command(subcommand)]
    pub cmd: Option<BackupSubCommand>,

    /// 备份描述
    #[arg(short = 'm', long)]
    pub msg: Option<String>,

    /// 工作目录（默认当前目录）
    #[arg(short = 'C', long)]
    pub dir: Option<String>,

    /// 写入单文件 .xunbak 容器
    #[arg(long)]
    pub container: Option<String>,

    /// 压缩配置
    #[arg(long)]
    pub compression: Option<String>,

    /// 分卷大小（如 64M / 2G）
    #[arg(long)]
    pub split_size: Option<String>,

    /// 干运行（不实际复制/压缩/清理）
    #[arg(long)]
    pub dry_run: bool,

    /// 列出选中的源文件而不写入输出
    #[arg(long)]
    pub list: bool,

    /// 跳过压缩
    #[arg(long)]
    pub no_compress: bool,

    /// 覆盖最大备份数 (1..1000)
    #[arg(long)]
    pub retain: Option<usize>,

    /// 包含路径（可重复或逗号分隔）
    #[arg(long)]
    pub include: Vec<String>,

    /// 排除路径（可重复或逗号分隔）
    #[arg(long)]
    pub exclude: Vec<String>,

    /// 增量备份：仅复制新增/修改文件
    #[arg(long)]
    pub incremental: bool,

    /// 无变更时跳过创建新备份
    #[arg(long)]
    pub skip_if_unchanged: bool,

    /// diff 模式：auto | hash | meta
    #[arg(long)]
    pub diff_mode: Option<String>,

    /// 输出 JSON 摘要
    #[arg(long)]
    pub json: bool,
}

/// Backup 子命令枚举。
#[derive(Subcommand, Debug, Clone)]
pub enum BackupSubCommand {
    /// 创建新备份
    #[command(name = "add", alias = "create")]
    Add(BackupCreateCmd),
    /// 从备份恢复
    Restore(BackupRestoreCmd),
    /// 转换备份格式
    Convert(BackupConvertCmd),
    /// 列出可用备份
    List(BackupListCmd),
    /// 验证备份完整性
    Verify(BackupVerifyCmd),
    /// 按标签查找备份
    Find(BackupFindCmd),
}

/// backup create 参数。
#[derive(Parser, Debug, Clone)]
pub struct BackupCreateCmd {
    /// 备份描述
    #[arg(short = 'm', long)]
    pub msg: Option<String>,
    /// 工作目录
    #[arg(short = 'C', long)]
    pub dir: Option<String>,
    /// 目标格式：dir | xunbak | zip | 7z
    #[arg(long)]
    pub format: Option<String>,
    /// 输出路径
    #[arg(short = 'o', long)]
    pub output: Option<String>,
    /// 压缩配置
    #[arg(long)]
    pub compression: Option<String>,
    /// 分卷大小（如 64M / 2G）
    #[arg(long)]
    pub split_size: Option<String>,
    /// 固实压缩
    #[arg(long)]
    pub solid: bool,
    /// 压缩方法
    #[arg(long)]
    pub method: Option<String>,
    /// 压缩级别
    #[arg(long)]
    pub level: Option<u32>,
    /// 干运行
    #[arg(long)]
    pub dry_run: bool,
    /// 列出文件
    #[arg(long)]
    pub list: bool,
    /// 跳过压缩
    #[arg(long)]
    pub no_compress: bool,
    /// 覆盖最大备份数 (1..1000)
    #[arg(long)]
    pub retain: Option<usize>,
    /// 包含路径
    #[arg(long)]
    pub include: Vec<String>,
    /// 排除路径
    #[arg(long)]
    pub exclude: Vec<String>,
    /// 增量备份
    #[arg(long)]
    pub incremental: bool,
    /// 无变更跳过
    #[arg(long)]
    pub skip_if_unchanged: bool,
    /// diff 模式
    #[arg(long)]
    pub diff_mode: Option<String>,
    /// 进度模式
    #[arg(long)]
    pub progress: Option<String>,
    /// 输出 JSON
    #[arg(long)]
    pub json: bool,
    /// 禁用 sidecar
    #[arg(long)]
    pub no_sidecar: bool,
}

/// backup restore 参数。
#[derive(Parser, Debug, Clone)]
pub struct BackupRestoreCmd {
    /// 备份名或路径
    pub name_or_path: String,
    /// 恢复单个文件
    #[arg(long)]
    pub file: Option<String>,
    /// 按 glob 模式恢复
    #[arg(long)]
    pub glob: Option<String>,
    /// 恢复到指定目录
    #[arg(long)]
    pub to: Option<String>,
    /// 恢复前快照当前状态
    #[arg(long)]
    pub snapshot: bool,
    /// 项目根目录
    #[arg(short = 'C', long)]
    pub dir: Option<String>,
    /// 干运行
    #[arg(long)]
    pub dry_run: bool,
    /// 跳过确认
    #[arg(short = 'y', long)]
    pub yes: bool,
    /// 输出 JSON
    #[arg(long)]
    pub json: bool,
}

/// backup convert 参数。
#[derive(Parser, Debug, Clone)]
pub struct BackupConvertCmd {
    /// 输入备份路径
    pub artifact: String,
    /// 目标格式
    #[arg(long)]
    pub format: String,
    /// 输出路径
    #[arg(short = 'o', long)]
    pub output: String,
    /// 包含文件
    #[arg(long)]
    pub file: Vec<String>,
    /// 包含 glob
    #[arg(long)]
    pub glob: Vec<String>,
    /// 从文件读取包含模式
    #[arg(long)]
    pub patterns_from: Vec<String>,
    /// 分卷大小
    #[arg(long)]
    pub split_size: Option<String>,
    /// 固实压缩
    #[arg(long)]
    pub solid: bool,
    /// 压缩方法
    #[arg(long)]
    pub method: Option<String>,
    /// 压缩级别
    #[arg(long)]
    pub level: Option<u32>,
    /// 线程数
    #[arg(long)]
    pub threads: Option<u32>,
    /// 密码
    #[arg(long)]
    pub password: Option<String>,
    /// 加密头部
    #[arg(long)]
    pub encrypt_header: bool,
    /// 覆盖模式
    #[arg(long)]
    pub overwrite: Option<String>,
    /// 干运行
    #[arg(long)]
    pub dry_run: bool,
    /// 列出文件
    #[arg(long)]
    pub list: bool,
    /// 验证源
    #[arg(long)]
    pub verify_source: Option<String>,
    /// 验证输出
    #[arg(long)]
    pub verify_output: Option<String>,
    /// 进度模式
    #[arg(long)]
    pub progress: Option<String>,
    /// 输出 JSON
    #[arg(long)]
    pub json: bool,
    /// 禁用 sidecar
    #[arg(long)]
    pub no_sidecar: bool,
}

/// backup list 参数。
#[derive(Parser, Debug, Clone)]
pub struct BackupListCmd {
    /// 输出 JSON
    #[arg(long)]
    pub json: bool,
}

/// backup verify 参数。
#[derive(Parser, Debug, Clone)]
pub struct BackupVerifyCmd {
    /// 备份名
    pub name: String,
    /// 输出 JSON
    #[arg(long)]
    pub json: bool,
}

/// backup find 参数。
#[derive(Parser, Debug, Clone)]
pub struct BackupFindCmd {
    /// 标签过滤
    pub tag: Option<String>,
    /// 起始时间过滤
    #[arg(long)]
    pub since: Option<String>,
    /// 结束时间过滤
    #[arg(long)]
    pub until: Option<String>,
    /// 输出 JSON
    #[arg(long)]
    pub json: bool,
}

// ============================================================
// BackupEntry — 备份列表输出类型
// ============================================================

/// 备份条目。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupEntry {
    /// 备份名称
    pub name: String,
    /// 创建时间
    pub created: String,
    /// 大小（字节）
    pub size: u64,
    /// 文件数
    pub file_count: usize,
    /// 描述
    pub description: String,
}

impl BackupEntry {
    pub fn new(
        name: impl Into<String>,
        created: impl Into<String>,
        size: u64,
        file_count: usize,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            created: created.into(),
            size,
            file_count,
            description: description.into(),
        }
    }
}

impl TableRow for BackupEntry {
    fn columns() -> Vec<ColumnDef> {
        vec![
            ColumnDef::new("name", ValueKind::String),
            ColumnDef::new("created", ValueKind::String),
            ColumnDef::new("size", ValueKind::Filesize),
            ColumnDef::new("file_count", ValueKind::Int),
            ColumnDef::new("description", ValueKind::String),
        ]
    }

    fn cells(&self) -> Vec<Value> {
        vec![
            Value::String(self.name.clone()),
            Value::String(self.created.clone()),
            Value::Filesize(self.size),
            Value::Int(self.file_count as i64),
            Value::String(self.description.clone()),
        ]
    }
}

// ── CommandSpec 实现 ──────────────────────────────────────────────

use super::command::CommandSpec;
use super::context::CmdContext;
use super::error::XunError;
use super::services::backup as backup_svc;

/// backup create（传统模式，无子命令）
pub struct BackupCreateCmdSpec {
    pub args: BackupCreateCmd,
}

impl CommandSpec for BackupCreateCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        backup_svc::create_backup_artifact(
            self.args.msg.as_deref(),
            self.args.dir.as_deref(),
            self.args.format.as_deref(),
            self.args.output.as_deref(),
            self.args.compression.as_deref(),
            self.args.split_size.as_deref(),
            self.args.dry_run,
            self.args.list,
            self.args.no_compress,
            self.args.retain,
            &self.args.include,
            &self.args.exclude,
            self.args.incremental,
            self.args.skip_if_unchanged,
            self.args.diff_mode.as_deref(),
            self.args.progress.as_deref(),
            self.args.json,
            self.args.no_sidecar,
        )
    }
}

/// backup restore
pub struct BackupRestoreCmdSpec {
    pub args: BackupRestoreCmd,
}

impl CommandSpec for BackupRestoreCmdSpec {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.name_or_path.is_empty() {
            return Err(XunError::user("backup name or path is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        backup_svc::restore_backup(
            &self.args.name_or_path,
            self.args.file.as_deref(),
            self.args.glob.as_deref(),
            self.args.to.as_deref(),
            self.args.snapshot,
            self.args.dir.as_deref(),
            self.args.dry_run,
            self.args.yes,
            self.args.json,
        )
    }
}

/// backup convert
pub struct BackupConvertCmdSpec {
    pub args: BackupConvertCmd,
}

impl CommandSpec for BackupConvertCmdSpec {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.artifact.is_empty() {
            return Err(XunError::user("artifact path is required"));
        }
        if self.args.format.is_empty() {
            return Err(XunError::user("target format is required"));
        }
        if self.args.output.is_empty() {
            return Err(XunError::user("output path is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        backup_svc::convert_backup(
            &self.args.artifact,
            &self.args.format,
            &self.args.output,
            &self.args.file,
            &self.args.glob,
            self.args.split_size.as_deref(),
            self.args.level,
            self.args.dry_run,
            self.args.list,
            self.args.json,
        )
    }
}

/// backup list
pub struct BackupListCmdSpec {
    pub args: BackupListCmd,
    pub dir: Option<String>,
}

impl CommandSpec for BackupListCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        backup_svc::list_backups(self.dir.as_deref(), self.args.json)
    }
}

/// backup verify
pub struct BackupVerifyCmdSpec {
    pub args: BackupVerifyCmd,
    pub dir: Option<String>,
}

impl CommandSpec for BackupVerifyCmdSpec {
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        if self.args.name.is_empty() {
            return Err(XunError::user("backup name is required"));
        }
        Ok(())
    }

    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        backup_svc::verify_backup(&self.args.name, self.dir.as_deref(), self.args.json)
    }
}

/// backup find
pub struct BackupFindCmdSpec {
    pub args: BackupFindCmd,
    pub dir: Option<String>,
}

impl CommandSpec for BackupFindCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        backup_svc::find_backup(
            self.args.tag.as_deref(),
            self.args.since.as_deref(),
            self.args.until.as_deref(),
            self.dir.as_deref(),
            self.args.json,
        )
    }
}

/// backup 默认（无子命令，走传统目录备份流程）
pub struct BackupDefaultCmdSpec {
    pub args: BackupCmd,
}

impl CommandSpec for BackupDefaultCmdSpec {
    fn run(&self, _ctx: &mut CmdContext) -> Result<Value, XunError> {
        backup_svc::create_backup(
            self.args.msg.as_deref(),
            self.args.dir.as_deref(),
            self.args.dry_run,
            self.args.list,
            self.args.no_compress,
            self.args.retain,
            &self.args.include,
            &self.args.exclude,
            self.args.incremental,
            self.args.skip_if_unchanged,
            self.args.diff_mode.as_deref(),
            self.args.json,
        )
    }
}
