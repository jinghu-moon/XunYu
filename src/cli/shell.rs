use argh::FromArgs;

/// Initialize shell integration (print wrapper function).
#[derive(FromArgs)]
#[argh(subcommand, name = "init")]
pub struct InitCmd {
    /// shell type: powershell | bash | zsh
    #[argh(positional)]
    pub shell: String,
}

/// Generate shell completion script.
#[derive(FromArgs)]
#[argh(subcommand, name = "completion")]
pub struct CompletionCmd {
    /// shell type: powershell | bash | zsh | fish
    #[argh(positional)]
    pub shell: String,
}

/// Internal completion entry (shell-pre-tokenized args).
#[derive(FromArgs)]
#[argh(subcommand, name = "__complete")]
pub struct CompleteCmd {
    /// pre-tokenized args after command name
    #[argh(positional, greedy)]
    pub args: Vec<String>,
}
