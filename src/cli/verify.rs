use clap::Args;

/// Verify an xunbak container.
#[derive(Args, Debug, Clone)]
pub struct VerifyCmd {
    /// xunbak container path
    pub path: String,

    /// verify level: quick | full | manifest-only | existence-only | paranoid
    #[arg(long)]
    pub level: Option<String>,

    /// output machine-readable JSON
    #[arg(long)]
    pub json: bool,
}
