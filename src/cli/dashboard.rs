use clap::Args;

#[cfg(feature = "dashboard")]
/// Start web dashboard server.
#[derive(Args, Debug, Clone)]
pub struct ServeCmd {
    /// listen port (default: 9527)
    #[arg(short = 'p', long, default_value_t = 9527)]
    pub port: u16,
}
