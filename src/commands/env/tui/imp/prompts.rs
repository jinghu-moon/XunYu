use super::*;

pub(super) fn prompt_new_var() -> CliResult<Option<(String, String)>> {
    prompt_interactive(|| {
        let name: String = Input::new().with_prompt("Variable name").interact_text()?;
        if name.trim().is_empty() {
            return Ok(None);
        }
        let value: String = Input::new().with_prompt("Variable value").interact_text()?;
        Ok(Some((name, value)))
    })
}

pub(super) fn prompt_edit_var(name: &str, current: &str) -> CliResult<Option<String>> {
    prompt_interactive(|| {
        let value: String = Input::new()
            .with_prompt(format!("Edit {}", name))
            .default(current.to_string())
            .interact_text()?;
        Ok(Some(value))
    })
}

pub(super) fn prompt_path_entry(prompt: &str) -> CliResult<Option<String>> {
    prompt_interactive(|| {
        let entry: String = Input::new().with_prompt(prompt).interact_text()?;
        if entry.trim().is_empty() {
            Ok(None)
        } else {
            Ok(Some(entry))
        }
    })
}

pub(super) fn prompt_text(prompt: &str, default: &str) -> CliResult<String> {
    prompt_interactive(|| {
        Input::new()
            .with_prompt(prompt)
            .default(default.to_string())
            .interact_text()
    })
}

pub(super) fn prompt_yes_no(prompt: &str) -> CliResult<bool> {
    prompt_interactive(|| Confirm::new().with_prompt(prompt).default(false).interact())
}

pub(super) fn prompt_export_target() -> CliResult<Option<(ExportFormat, PathBuf)>> {
    prompt_interactive(|| {
        let formats = ["json", "env", "reg", "csv"];
        let idx = Select::new()
            .with_prompt("Export format")
            .items(&formats)
            .default(0)
            .interact()?;
        let format = match formats[idx] {
            "json" => ExportFormat::Json,
            "env" => ExportFormat::Env,
            "reg" => ExportFormat::Reg,
            _ => ExportFormat::Csv,
        };
        let path: String = Input::new()
            .with_prompt("Output file path")
            .interact_text()?;
        if path.trim().is_empty() {
            Ok(None)
        } else {
            Ok(Some((format, PathBuf::from(path))))
        }
    })
}

pub(super) fn prompt_import_source() -> CliResult<Option<(PathBuf, ImportStrategy, bool)>> {
    prompt_interactive(|| {
        let path: String = Input::new()
            .with_prompt("Import file path")
            .interact_text()?;
        if path.trim().is_empty() {
            return Ok(None);
        }
        let modes = ["merge", "overwrite"];
        let mode_idx = Select::new()
            .with_prompt("Import mode")
            .items(&modes)
            .default(0)
            .interact()?;
        let strategy = if mode_idx == 0 {
            ImportStrategy::Merge
        } else {
            ImportStrategy::Overwrite
        };
        let dry_run = Confirm::new()
            .with_prompt("Dry run only?")
            .default(true)
            .interact()?;
        Ok(Some((PathBuf::from(path), strategy, dry_run)))
    })
}

pub(super) fn prompt_interactive<T, F>(f: F) -> CliResult<T>
where
    F: FnOnce() -> Result<T, dialoguer::Error>,
{
    disable_raw_mode().map_err(|e| CliError::new(1, format!("{e}")))?;
    execute!(io::stdout(), LeaveAlternateScreen).map_err(|e| CliError::new(1, format!("{e}")))?;
    let result = f().map_err(|e| CliError::new(1, format!("prompt failed: {}", e)));
    execute!(io::stdout(), EnterAlternateScreen).map_err(|e| CliError::new(1, format!("{e}")))?;
    enable_raw_mode().map_err(|e| CliError::new(1, format!("{e}")))?;
    FORCE_FULL_REDRAW.store(true, Ordering::SeqCst);
    result
}
