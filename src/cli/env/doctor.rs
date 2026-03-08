use argh::FromArgs;

#[derive(FromArgs)]
#[argh(subcommand, name = "check")]
/// Run environment checks (alias of doctor).
pub struct EnvCheckCmd {
    /// scope: user|system|all
    #[argh(option, default = "String::from(\"all\")")]
    pub scope: String,

    /// apply fixes
    #[argh(switch)]
    pub fix: bool,

    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "doctor")]
/// Run environment health checks.
pub struct EnvDoctorCmd {
    /// scope: user|system|all
    #[argh(option, default = "String::from(\"user\")")]
    pub scope: String,

    /// apply fixes
    #[argh(switch)]
    pub fix: bool,

    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "audit")]
/// Show env audit log entries.
pub struct EnvAuditCmd {
    /// max rows, 0 for all
    #[argh(option, default = "50")]
    pub limit: usize,

    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "watch")]
/// Watch env variable changes by polling.
pub struct EnvWatchCmd {
    /// scope: user|system|all
    #[argh(option, default = "String::from(\"all\")")]
    pub scope: String,

    /// poll interval in milliseconds
    #[argh(option, default = "2000")]
    pub interval_ms: u64,

    /// output format: text|json
    #[argh(option, default = "String::from(\"text\")")]
    pub format: String,

    /// run one poll cycle and exit
    #[argh(switch)]
    pub once: bool,
}
