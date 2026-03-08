use argh::FromArgs;

#[cfg(feature = "dashboard")]
/// Start web dashboard server.
#[derive(FromArgs)]
#[argh(subcommand, name = "serve")]
pub struct ServeCmd {
    /// listen port (default: 9527)
    #[argh(option, short = 'p', default = "9527")]
    pub port: u16,
}
