use argh::FromArgs;

/// Incremental project backup. Alias: `bak`.
#[derive(FromArgs, Clone, Debug, PartialEq, Eq)]
#[argh(subcommand, name = "backup")]
pub struct BackupCmd {
    #[argh(subcommand)]
    pub cmd: Option<BackupSubCommand>,

    /// backup description
    #[argh(option, short = 'm')]
    pub msg: Option<String>,
    /// working directory (default: cwd)
    #[argh(option, short = 'C')]
    pub dir: Option<String>,

    /// write to a single-file .xunbak container
    #[argh(option)]
    pub container: Option<String>,

    /// compression profile for xunbak mode: none | zstd | zstd:N | lz4 | lzma | auto
    #[argh(option)]
    #[cfg_attr(not(feature = "xunbak"), allow(dead_code))]
    pub compression: Option<String>,

    /// split xunbak output into numbered volumes, e.g. 64M / 2G
    #[argh(option)]
    #[cfg_attr(not(feature = "xunbak"), allow(dead_code))]
    pub split_size: Option<String>,

    /// dry run (no copy/zip/cleanup)
    #[argh(switch)]
    pub dry_run: bool,

    /// list selected source files without writing output
    #[argh(switch)]
    pub list: bool,

    /// skip compression for this run
    #[argh(switch)]
    pub no_compress: bool,

    /// override max backups
    #[argh(option)]
    pub retain: Option<usize>,

    /// add include path (repeatable or comma separated)
    #[argh(option)]
    pub include: Vec<String>,

    /// add exclude path (repeatable or comma separated)
    #[argh(option)]
    pub exclude: Vec<String>,

    /// incremental backup: only copy new/modified files
    #[argh(switch)]
    pub incremental: bool,

    /// skip creating a new backup when no changes are detected
    #[argh(switch)]
    pub skip_if_unchanged: bool,
}

#[derive(FromArgs, Clone, Debug, PartialEq, Eq)]
#[argh(subcommand)]
pub enum BackupSubCommand {
    Create(BackupCreateCmd),
    Restore(BackupRestoreCmd),
    Convert(BackupConvertCmd),
    List(BackupListCmd),
    Verify(BackupVerifyCmd),
    Find(BackupFindCmd),
}

/// List available backups.
#[derive(FromArgs, Clone, Debug, PartialEq, Eq)]
#[argh(subcommand, name = "list")]
pub struct BackupListCmd {
    /// output machine-readable JSON
    #[argh(switch)]
    pub json: bool,
}

/// Verify integrity of a directory backup.
#[derive(FromArgs, Clone, Debug, PartialEq, Eq)]
#[argh(subcommand, name = "verify")]
pub struct BackupVerifyCmd {
    /// backup name
    #[argh(positional)]
    pub name: String,

    /// output machine-readable JSON
    #[argh(switch)]
    pub json: bool,
}

/// Find backups by tag or other metadata filters.
#[derive(FromArgs, Clone, Debug, PartialEq, Eq)]
#[argh(subcommand, name = "find")]
pub struct BackupFindCmd {
    /// tag filter
    #[argh(positional)]
    pub tag: Option<String>,

    /// lower bound time filter: RFC3339 | YYYY-MM-DD | YYYY-MM-DD HH:MM:SS
    #[argh(option)]
    pub since: Option<String>,

    /// upper bound time filter: RFC3339 | YYYY-MM-DD | YYYY-MM-DD HH:MM:SS
    #[argh(option)]
    pub until: Option<String>,

    /// output machine-readable JSON
    #[argh(switch)]
    pub json: bool,
}

/// Create a new backup artifact.
#[derive(FromArgs, Clone, Debug, PartialEq, Eq)]
#[argh(subcommand, name = "create")]
pub struct BackupCreateCmd {
    /// backup description
    #[argh(option, short = 'm')]
    pub msg: Option<String>,

    /// working directory (default: cwd)
    #[argh(option, short = 'C')]
    pub dir: Option<String>,

    /// target artifact format: dir | xunbak | zip | 7z
    #[argh(option)]
    pub format: Option<String>,

    /// output target path or base name for artifact formats that require it
    #[argh(option, short = 'o')]
    pub output: Option<String>,

    /// compression profile for xunbak mode: none | zstd | zstd:N | lz4 | lzma | auto
    #[argh(option)]
    pub compression: Option<String>,

    /// split output into numbered volumes, e.g. 64M / 2G
    #[argh(option)]
    pub split_size: Option<String>,

    /// enable solid compression for 7z
    #[argh(switch)]
    pub solid: bool,

    /// output method, interpreted by target format
    #[argh(option)]
    pub method: Option<String>,

    /// compression level
    #[argh(option)]
    pub level: Option<u32>,

    /// dry run (no copy/zip/cleanup)
    #[argh(switch)]
    pub dry_run: bool,

    /// list selected source files without writing output
    #[argh(switch)]
    pub list: bool,

    /// skip compression for this run
    #[argh(switch)]
    pub no_compress: bool,

    /// override max backups
    #[argh(option)]
    pub retain: Option<usize>,

    /// add include path (repeatable or comma separated)
    #[argh(option)]
    pub include: Vec<String>,

    /// add exclude path (repeatable or comma separated)
    #[argh(option)]
    pub exclude: Vec<String>,

    /// incremental backup: only copy new/modified files
    #[argh(switch)]
    pub incremental: bool,

    /// skip creating a new backup when no changes are detected
    #[argh(switch)]
    pub skip_if_unchanged: bool,

    /// progress mode: auto | always | off
    #[argh(option)]
    pub progress: Option<String>,

    /// output machine-readable JSON summary
    #[argh(switch)]
    pub json: bool,

    /// disable writing __xunyu__/export_manifest.json sidecar for dir/zip/7z outputs
    #[argh(switch)]
    pub no_sidecar: bool,
}

/// Restore files from a backup artifact.
#[derive(FromArgs, Clone, Debug, PartialEq, Eq)]
#[argh(subcommand, name = "restore")]
pub struct BackupRestoreCmd {
    /// backup artifact path or backup name
    #[argh(positional)]
    pub name_or_path: String,

    /// restore a single file (relative path, e.g. src/main.rs)
    #[argh(option)]
    pub file: Option<String>,

    /// restore files matching glob pattern (e.g. '**/*.ts')
    #[argh(option)]
    pub glob: Option<String>,

    /// restore to this directory instead of the project root
    #[argh(option)]
    pub to: Option<String>,

    /// snapshot current state before restoring (creates a pre_restore backup)
    #[argh(switch)]
    pub snapshot: bool,

    /// project root (default: cwd)
    #[argh(option, short = 'C')]
    pub dir: Option<String>,

    /// dry run: show what would be restored without writing files
    #[argh(switch)]
    pub dry_run: bool,

    /// skip confirmation prompt
    #[argh(switch, short = 'y')]
    pub yes: bool,

    /// output machine-readable JSON summary
    #[argh(switch)]
    pub json: bool,

}

/// Convert one backup artifact into another artifact format.
#[derive(FromArgs, Clone, Debug, PartialEq, Eq)]
#[argh(subcommand, name = "convert")]
pub struct BackupConvertCmd {
    /// input backup artifact path
    #[argh(positional)]
    pub artifact: String,

    /// target artifact format: dir | xunbak | zip | 7z
    #[argh(option)]
    pub format: String,

    /// output target path or base name
    #[argh(option, short = 'o')]
    pub output: String,

    /// include a single relative path from the source artifact
    #[argh(option)]
    pub file: Vec<String>,

    /// include paths matching glob patterns from the source artifact
    #[argh(option)]
    pub glob: Vec<String>,

    /// read additional glob patterns from files
    #[argh(option)]
    pub patterns_from: Vec<String>,

    /// split output into numbered volumes, e.g. 64M / 2G
    #[argh(option)]
    pub split_size: Option<String>,

    /// enable solid compression for 7z
    #[argh(switch)]
    pub solid: bool,

    /// output method, interpreted by target format
    #[argh(option)]
    pub method: Option<String>,

    /// compression level
    #[argh(option)]
    pub level: Option<u32>,

    /// compression threads
    #[argh(option)]
    pub threads: Option<u32>,

    /// password for 7z encryption
    #[argh(option)]
    pub password: Option<String>,

    /// encrypt 7z header
    #[argh(switch)]
    pub encrypt_header: bool,

    /// overwrite policy: ask | replace | fail
    #[argh(option)]
    pub overwrite: Option<String>,

    /// dry run: show what would be converted without writing files
    #[argh(switch)]
    pub dry_run: bool,

    /// list selected items without writing output
    #[argh(switch)]
    pub list: bool,

    /// verify source mode: quick | full | paranoid | off
    #[argh(option)]
    pub verify_source: Option<String>,

    /// verify output mode: on | off
    #[argh(option)]
    pub verify_output: Option<String>,

    /// progress mode: auto | always | off
    #[argh(option)]
    pub progress: Option<String>,

    /// output machine-readable JSON summary
    #[argh(switch)]
    pub json: bool,

    /// disable writing __xunyu__/export_manifest.json sidecar for dir/zip/7z outputs
    #[argh(switch)]
    pub no_sidecar: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> BackupCmd {
        <BackupCmd as argh::FromArgs>::from_args(&["backup"], args).expect("parse backup cmd")
    }

    #[test]
    fn parse_backup_create_subcommand() {
        let cmd = parse(&["create", "-C", "src", "--format", "zip", "-o", "out.zip"]);
        assert!(matches!(cmd.cmd, Some(BackupSubCommand::Create(_))));
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
}
