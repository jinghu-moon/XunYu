use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
/// Batch operations.
pub struct EnvBatchCmd {
    #[command(subcommand)]
    pub cmd: EnvBatchSubCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum EnvBatchSubCommand {
    Set(EnvBatchSetCmd),
    Delete(EnvBatchDeleteCmd),
    Rename(EnvBatchRenameCmd),
}

#[derive(Args, Debug, Clone)]
/// Batch set KEY=VALUE pairs.
pub struct EnvBatchSetCmd {
    /// scope: user|system
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// preview only, do not write
    #[arg(long)]
    pub dry_run: bool,

    /// items like KEY=VALUE
    pub items: Vec<String>,
}

#[derive(Args, Debug, Clone)]
/// Batch delete names.
pub struct EnvBatchDeleteCmd {
    /// scope: user|system
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// preview only, do not write
    #[arg(long)]
    pub dry_run: bool,

    /// variable names
    pub names: Vec<String>,
}

#[derive(Args, Debug, Clone)]
/// Rename one variable.
pub struct EnvBatchRenameCmd {
    /// scope: user|system
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// preview only, do not write
    #[arg(long)]
    pub dry_run: bool,

    /// old variable name
    pub old: String,

    /// new variable name
    pub new: String,
}
