//! ShellIntegration — Shell 集成抽象
//!
//! 将 alias、function、completion 渲染为不同 shell 的脚本语法。
//! 支持 PowerShell 和 Bash。

/// Shell 集成 trait。
///
/// 每个 shell 实现此 trait，提供 alias/function/completion 的脚本渲染。
pub trait ShellIntegration {
    /// Shell 名称标识。
    fn shell_name(&self) -> &str;

    /// 渲染 alias 定义。
    fn render_alias(&self, name: &str, command: &str) -> String;

    /// 渲染 function 定义。
    fn render_function(&self, name: &str, body: &str) -> String;

    /// 渲染 completion 定义。
    fn render_completion(&self, name: &str, completions: &[&str]) -> String;
}

/// PowerShell 实现。
pub struct PowerShell;

impl ShellIntegration for PowerShell {
    fn shell_name(&self) -> &str {
        "powershell"
    }

    fn render_alias(&self, name: &str, command: &str) -> String {
        format!("Set-Alias -Name {name} -Value \"{command}\"")
    }

    fn render_function(&self, name: &str, body: &str) -> String {
        format!("function {name} {{\n    {body}\n}}")
    }

    fn render_completion(&self, name: &str, completions: &[&str]) -> String {
        let items: Vec<String> = completions
            .iter()
            .map(|c| format!("'{c}'"))
            .collect();
        format!(
            "Register-ArgumentCompleter -CommandName {name} -ScriptBlock {{\n    param($commandName, $parameterName, $wordToComplete)\n    @({items}) | Where-Object {{ $_ -like \"$wordToComplete*\" }} | ForEach-Object {{ [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_) }}\n}}",
            items = items.join(", ")
        )
    }
}

/// Bash 实现。
pub struct BashShell;

impl ShellIntegration for BashShell {
    fn shell_name(&self) -> &str {
        "bash"
    }

    fn render_alias(&self, name: &str, command: &str) -> String {
        format!("alias {name}='{command}'")
    }

    fn render_function(&self, name: &str, body: &str) -> String {
        format!("function {name}() {{\n    {body}\n}}")
    }

    fn render_completion(&self, name: &str, completions: &[&str]) -> String {
        format!(
            "complete -W \"{words}\" {name}",
            words = completions.join(" "),
            name = name,
        )
    }
}
