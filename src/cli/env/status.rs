use clap::Args;

#[derive(Args, Debug, Clone)]
/// Show env subsystem status overview.
pub struct EnvStatusCmd {
    /// scope: user|system|all
    #[arg(long, default_value = "all")]
    pub scope: String,

    /// output format: text|json
    #[arg(long, default_value = "text")]
    pub format: String,
}
