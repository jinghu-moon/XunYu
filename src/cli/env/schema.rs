use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
/// Manage env schema rules.
pub struct EnvSchemaCmd {
    #[command(subcommand)]
    pub cmd: EnvSchemaSubCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum EnvSchemaSubCommand {
    Show(EnvSchemaShowCmd),
    AddRequired(EnvSchemaAddRequiredCmd),
    AddRegex(EnvSchemaAddRegexCmd),
    AddEnum(EnvSchemaAddEnumCmd),
    Remove(EnvSchemaRemoveCmd),
    Reset(EnvSchemaResetCmd),
}

#[derive(Args, Debug, Clone)]
/// Show current schema.
pub struct EnvSchemaShowCmd {
    /// output format: text|json
    #[arg(long, default_value = "text")]
    pub format: String,
}

#[derive(Args, Debug, Clone)]
/// Add or replace required rule.
pub struct EnvSchemaAddRequiredCmd {
    /// variable pattern, supports * and ?
    pub pattern: String,

    /// mark as warning when not strict
    #[arg(long)]
    pub warn_only: bool,
}

#[derive(Args, Debug, Clone)]
/// Add or replace regex rule.
pub struct EnvSchemaAddRegexCmd {
    /// variable pattern, supports * and ?
    pub pattern: String,

    /// regex expression
    pub regex: String,

    /// mark as warning when not strict
    #[arg(long)]
    pub warn_only: bool,
}

#[derive(Args, Debug, Clone)]
/// Add or replace enum rule.
pub struct EnvSchemaAddEnumCmd {
    /// variable pattern, supports * and ?
    pub pattern: String,

    /// allowed values, one or more
    pub values: Vec<String>,

    /// mark as warning when not strict
    #[arg(long)]
    pub warn_only: bool,
}

#[derive(Args, Debug, Clone)]
/// Remove one rule by pattern.
pub struct EnvSchemaRemoveCmd {
    /// rule pattern
    pub pattern: String,
}

#[derive(Args, Debug, Clone)]
/// Reset schema to empty.
pub struct EnvSchemaResetCmd {
    /// skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}
