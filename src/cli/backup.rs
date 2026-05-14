use clap::{Args, Parser, Subcommand};

/// Incremental project backup. Alias: `bak`.
#[derive(Parser, Debug, Clone, PartialEq, Eq)]
pub struct BackupCmd {
    #[command(subcommand)]
    pub cmd: Option<BackupSubCommand>,

    /// backup description
    #[arg(short = 'm', long)]
    pub msg: Option<String>,

    /// working directory (default: cwd)
    #[arg(short = 'C', long)]
    pub dir: Option<String>,

    /// write to a single-file .xunbak container
    #[arg(long)]
    pub container: Option<String>,

    /// compression profile for xunbak mode
    #[arg(long)]
    #[cfg_attr(not(feature = "xunbak"), allow(dead_code))]
    pub compression: Option<String>,

    /// split xunbak output into numbered volumes, e.g. 64M / 2G
    #[arg(long)]
    #[cfg_attr(not(feature = "xunbak"), allow(dead_code))]
    pub split_size: Option<String>,

    /// dry run (no copy/zip/cleanup)
    #[arg(long)]
    pub dry_run: bool,

    /// list selected source files without writing output
    #[arg(long)]
    pub list: bool,

    /// skip compression for this run
    #[arg(long)]
    pub no_compress: bool,

    /// override max backups
    #[arg(long)]
    pub retain: Option<usize>,

    /// add include path (repeatable or comma separated)
    #[arg(long)]
    pub include: Vec<String>,

    /// add exclude path (repeatable or comma separated)
    #[arg(long)]
    pub exclude: Vec<String>,

    /// incremental backup: only copy new/modified files
    #[arg(long)]
    pub incremental: bool,

    /// skip creating a new backup when no changes are detected
    #[arg(long)]
    pub skip_if_unchanged: bool,

    /// diff mode for traditional backup: auto | hash | meta
    #[arg(long)]
    pub diff_mode: Option<String>,

    /// output machine-readable JSON summary
    #[arg(long)]
    pub json: bool,
}

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum BackupSubCommand {
    #[command(name = "add", alias = "create")]
    Add(BackupCreateCmd),
    Restore(BackupRestoreCmd),
    Convert(BackupConvertCmd),
    List(BackupListCmd),
    Verify(BackupVerifyCmd),
    Find(BackupFindCmd),
}

/// List available backups.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct BackupListCmd {
    /// output machine-readable JSON
    #[arg(long)]
    pub json: bool,
}

/// Verify integrity of a directory backup.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct BackupVerifyCmd {
    /// backup name
    pub name: String,

    /// output machine-readable JSON
    #[arg(long)]
    pub json: bool,
}

/// Find backups by tag or other metadata filters.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct BackupFindCmd {
    /// tag filter
    pub tag: Option<String>,

    /// lower bound time filter: RFC3339 | YYYY-MM-DD | YYYY-MM-DD HH:MM:SS
    #[arg(long)]
    pub since: Option<String>,

    /// upper bound time filter: RFC3339 | YYYY-MM-DD | YYYY-MM-DD HH:MM:SS
    #[arg(long)]
    pub until: Option<String>,

    /// output machine-readable JSON
    #[arg(long)]
    pub json: bool,
}

/// Create a new backup artifact.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct BackupCreateCmd {
    /// backup description
    #[arg(short = 'm', long)]
    pub msg: Option<String>,

    /// working directory (default: cwd)
    #[arg(short = 'C', long)]
    pub dir: Option<String>,

    /// target artifact format: dir | xunbak | zip | 7z
    #[arg(long)]
    pub format: Option<String>,

    /// output target path or base name for artifact formats that require it
    #[arg(short = 'o', long)]
    pub output: Option<String>,

    /// compression profile for xunbak mode
    #[arg(long)]
    pub compression: Option<String>,

    /// split output into numbered volumes, e.g. 64M / 2G
    #[arg(long)]
    pub split_size: Option<String>,

    /// enable solid compression for 7z
    #[arg(long)]
    pub solid: bool,

    /// output method, interpreted by target format
    #[arg(long)]
    pub method: Option<String>,

    /// compression level
    #[arg(long)]
    pub level: Option<u32>,

    /// dry run (no copy/zip/cleanup)
    #[arg(long)]
    pub dry_run: bool,

    /// list selected source files without writing output
    #[arg(long)]
    pub list: bool,

    /// skip compression for this run
    #[arg(long)]
    pub no_compress: bool,

    /// override max backups
    #[arg(long)]
    pub retain: Option<usize>,

    /// add include path (repeatable or comma separated)
    #[arg(long)]
    pub include: Vec<String>,

    /// add exclude path (repeatable or comma separated)
    #[arg(long)]
    pub exclude: Vec<String>,

    /// incremental backup: only copy new/modified files
    #[arg(long)]
    pub incremental: bool,

    /// skip creating a new backup when no changes are detected
    #[arg(long)]
    pub skip_if_unchanged: bool,

    /// diff mode for traditional backup: auto | hash | meta
    #[arg(long)]
    pub diff_mode: Option<String>,

    /// progress mode: auto | always | off
    #[arg(long)]
    pub progress: Option<String>,

    /// output machine-readable JSON summary
    #[arg(long)]
    pub json: bool,

    /// disable writing __xunyu__/export_manifest.json sidecar for dir/zip/7z outputs
    #[arg(long)]
    pub no_sidecar: bool,
}

/// Restore files from a backup artifact.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct BackupRestoreCmd {
    /// backup artifact path or backup name
    pub name_or_path: String,

    /// restore a single file (relative path, e.g. src/main.rs)
    #[arg(long)]
    pub file: Option<String>,

    /// restore files matching glob pattern (e.g. '**/*.ts')
    #[arg(long)]
    pub glob: Option<String>,

    /// restore to this directory instead of the project root
    #[arg(long)]
    pub to: Option<String>,

    /// snapshot current state before restoring (creates a pre_restore backup)
    #[arg(long)]
    pub snapshot: bool,

    /// project root (default: cwd)
    #[arg(short = 'C', long)]
    pub dir: Option<String>,

    /// dry run: show what would be restored without writing files
    #[arg(long)]
    pub dry_run: bool,

    /// skip confirmation prompt
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// output machine-readable JSON summary
    #[arg(long)]
    pub json: bool,
}

/// Convert one backup artifact into another artifact format.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct BackupConvertCmd {
    /// input backup artifact path
    pub artifact: String,

    /// target artifact format: dir | xunbak | zip | 7z
    #[arg(long)]
    pub format: String,

    /// output target path or base name
    #[arg(short = 'o', long)]
    pub output: String,

    /// include a single relative path from the source artifact
    #[arg(long)]
    pub file: Vec<String>,

    /// include paths matching glob patterns from the source artifact
    #[arg(long)]
    pub glob: Vec<String>,

    /// read additional glob patterns from files
    #[arg(long)]
    pub patterns_from: Vec<String>,

    /// split output into numbered volumes, e.g. 64M / 2G
    #[arg(long)]
    pub split_size: Option<String>,

    /// enable solid compression for 7z
    #[arg(long)]
    pub solid: bool,

    /// output method, interpreted by target format
    #[arg(long)]
    pub method: Option<String>,

    /// compression level
    #[arg(long)]
    pub level: Option<u32>,

    /// compression threads
    #[arg(long)]
    pub threads: Option<u32>,

    /// password for 7z encryption
    #[arg(long)]
    pub password: Option<String>,

    /// encrypt 7z header
    #[arg(long)]
    pub encrypt_header: bool,

    /// overwrite policy: ask | replace | fail
    #[arg(long)]
    pub overwrite: Option<String>,

    /// dry run: show what would be converted without writing files
    #[arg(long)]
    pub dry_run: bool,

    /// list selected items without writing output
    #[arg(long)]
    pub list: bool,

    /// verify source mode: quick | full | manifest-only | existence-only | paranoid | off
    #[arg(long)]
    pub verify_source: Option<String>,

    /// verify output mode: on | off
    #[arg(long)]
    pub verify_output: Option<String>,

    /// progress mode: auto | always | off
    #[arg(long)]
    pub progress: Option<String>,

    /// output machine-readable JSON summary
    #[arg(long)]
    pub json: bool,

    /// disable writing __xunyu__/export_manifest.json sidecar for dir/zip/7z outputs
    #[arg(long)]
    pub no_sidecar: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn parse(args: &[&str]) -> BackupCmd {
        let mut full_args = vec!["backup"];
        full_args.extend_from_slice(args);
        BackupCmd::try_parse_from(&full_args).expect("parse backup cmd")
    }

    #[test]
    fn parse_backup_create_subcommand() {
        let cmd = parse(&["create", "-C", "src", "--format", "zip", "-o", "out.zip"]);
        assert!(matches!(cmd.cmd, Some(BackupSubCommand::Add(_))));
    }

    #[test]
    fn parse_backup_restore_subcommand() {
        let cmd = parse(&["restore", "archive.xunbak", "--to", "out"]);
        assert!(matches!(cmd.cmd, Some(BackupSubCommand::Restore(_))));
    }

    #[test]
    fn parse_backup_convert_subcommand() {
        let cmd = parse(&[
            "convert",
            "archive.xunbak",
            "--format",
            "zip",
            "-o",
            "out.zip",
        ]);
        assert!(matches!(cmd.cmd, Some(BackupSubCommand::Convert(_))));
    }

    #[test]
    fn parse_backup_diff_mode_option() {
        for mode in ["auto", "hash", "meta"] {
            let cmd = parse(&["-C", "src", "--diff-mode", mode, "-m", "t"]);
            assert_eq!(cmd.diff_mode.as_deref(), Some(mode));
        }
    }
}
