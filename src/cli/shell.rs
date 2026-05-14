use clap::Args;

/// Initialize shell integration (print wrapper function).
#[derive(Args, Debug, Clone)]
pub struct InitCmd {
    /// shell type: powershell | bash | zsh
    pub shell: String,
}

/// Generate shell completion script.
#[derive(Args, Debug, Clone)]
pub struct CompletionCmd {
    /// shell type: powershell | bash | zsh | fish
    pub shell: String,
}

/// Internal completion entry (shell-pre-tokenized args).
#[derive(Args, Debug, Clone)]
pub struct CompleteCmd {
    /// pre-tokenized args after command name
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,
}
