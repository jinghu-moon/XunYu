use argh::FromArgs;

/// Verify an xunbak container.
#[derive(FromArgs)]
#[argh(subcommand, name = "verify")]
pub struct VerifyCmd {
    /// xunbak container path
    #[argh(positional)]
    pub path: String,

    /// verify level: quick | full | paranoid
    #[argh(option)]
    pub level: Option<String>,

    /// output machine-readable JSON
    #[argh(switch)]
    pub json: bool,
}
