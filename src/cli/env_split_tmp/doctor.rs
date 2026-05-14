use clap::Args;

#[derive(Args, Debug, Clone)]
/// Run environment checks (alias of doctor).
pub struct EnvCheckCmd {
    /// scope: user|system|all
    #[arg(long, default_value = "all")]
    pub scope: String,

    /// apply fixes
    #[arg(long)]
    pub fix: bool,

    /// output format: text|json
    #[arg(long, default_value = "text")]
    pub format: String,
}

#[derive(Args, Debug, Clone)]
/// Run environment health checks.
pub struct EnvDoctorCmd {
    /// scope: user|system|all
    #[arg(long, default_value = "user")]
    pub scope: String,

    /// apply fixes
    #[arg(long)]
    pub fix: bool,

    /// output format: text|json
    #[arg(long, default_value = "text")]
    pub format: String,
}

#[derive(Args, Debug, Clone)]
/// Show env audit log entries.
pub struct EnvAuditCmd {
    /// max rows, 0 for all
    #[arg(long, default_value_t = 50)]
    pub limit: usize,

    /// output format: text|json
    #[arg(long, default_value = "text")]
    pub format: String,
}

#[derive(Args, Debug, Clone)]
/// Watch env variable changes by polling.
pub struct EnvWatchCmd {
    /// scope: user|system|all
    #[arg(long, default_value = "all")]
    pub scope: String,

    /// poll interval in milliseconds
    #[arg(long, default_value_t = 2000)]
    pub interval_ms: u64,

    /// output format: text|json
    #[arg(long, default_value = "text")]
    pub format: String,

    /// run one poll cycle and exit
    #[arg(long)]
    pub once: bool,
}
