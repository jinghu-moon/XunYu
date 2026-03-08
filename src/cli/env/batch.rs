use argh::FromArgs;

#[derive(FromArgs)]
#[argh(subcommand, name = "batch")]
/// Batch operations.
pub struct EnvBatchCmd {
    #[argh(subcommand)]
    pub cmd: EnvBatchSubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum EnvBatchSubCommand {
    Set(EnvBatchSetCmd),
    Delete(EnvBatchDeleteCmd),
    Rename(EnvBatchRenameCmd),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "set")]
/// Batch set KEY=VALUE pairs.
pub struct EnvBatchSetCmd {
    /// scope: user|system
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// preview only, do not write
    #[argh(switch)]
    pub dry_run: bool,

    /// items like KEY=VALUE
    #[argh(positional)]
    pub items: Vec<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "delete")]
/// Batch delete names.
pub struct EnvBatchDeleteCmd {
    /// scope: user|system
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// preview only, do not write
    #[argh(switch)]
    pub dry_run: bool,

    /// variable names
    #[argh(positional)]
    pub names: Vec<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "rename")]
/// Rename one variable.
pub struct EnvBatchRenameCmd {
    /// scope: user|system
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// preview only, do not write
    #[argh(switch)]
    pub dry_run: bool,

    /// old variable name
    #[argh(positional)]
    pub old: String,

    /// new variable name
    #[argh(positional)]
    pub new: String,
}
