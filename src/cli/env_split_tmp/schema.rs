use super::*;

pub struct EnvValidateCmd {
    /// scope: user|system|all
    #[argh(option, default = "String::from(\"all\")")]
    pub scope: String,

    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,

    /// treat warnings as errors
    #[argh(switch)]
    pub strict: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "schema")]
/// Manage env schema rules.
pub struct EnvSchemaCmd {
    #[argh(subcommand)]
    pub cmd: EnvSchemaSubCommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum EnvSchemaSubCommand {
    Show(EnvSchemaShowCmd),
    AddRequired(EnvSchemaAddRequiredCmd),
    AddRegex(EnvSchemaAddRegexCmd),
    AddEnum(EnvSchemaAddEnumCmd),
    Remove(EnvSchemaRemoveCmd),
    Reset(EnvSchemaResetCmd),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "show")]
/// Show current schema.
pub struct EnvSchemaShowCmd {
    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "add-required")]
/// Add or replace required rule.
pub struct EnvSchemaAddRequiredCmd {
    /// variable pattern, supports * and ?
    #[argh(positional)]
    pub pattern: String,

    /// mark as warning when not strict
    #[argh(switch)]
    pub warn_only: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "add-regex")]
/// Add or replace regex rule.
pub struct EnvSchemaAddRegexCmd {
    /// variable pattern, supports * and ?
    #[argh(positional)]
    pub pattern: String,

    /// regex expression
    #[argh(positional)]
    pub regex: String,

    /// mark as warning when not strict
    #[argh(switch)]
    pub warn_only: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "add-enum")]
/// Add or replace enum rule.
pub struct EnvSchemaAddEnumCmd {
    /// variable pattern, supports * and ?
    #[argh(positional)]
    pub pattern: String,

    /// allowed values, one or more
    #[argh(positional)]
    pub values: Vec<String>,

    /// mark as warning when not strict
    #[argh(switch)]
    pub warn_only: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "remove")]
/// Remove one rule by pattern.
pub struct EnvSchemaRemoveCmd {
    /// rule pattern
    #[argh(positional)]
    pub pattern: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "reset")]
/// Reset schema to empty.
pub struct EnvSchemaResetCmd {
    /// skip confirmation
    #[argh(switch, short = 'y')]
    pub yes: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "annotate")]
/// Manage variable annotations.

