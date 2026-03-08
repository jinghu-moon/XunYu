use super::*;

pub(super) fn parse_key_value_items(
    items: &[String],
    flag_name: &str,
) -> CliResult<Vec<(String, String)>> {
    let mut out = Vec::new();
    for item in items {
        let Some((name, value)) = item.split_once('=') else {
            return Err(CliError::with_details(
                2,
                format!("invalid {} item '{}'", flag_name, item),
                &[r#"Fix: use KEY=VALUE, e.g. --set JAVA_HOME=C:\Java\jdk"#],
            ));
        };
        let key = name.trim();
        if key.is_empty() {
            return Err(CliError::new(
                2,
                format!("invalid {} item '{}': empty key", flag_name, item),
            ));
        }
        out.push((key.to_string(), value.to_string()));
    }
    Ok(out)
}

pub(super) fn parse_scope(raw: &str) -> CliResult<EnvScope> {
    EnvScope::from_str(raw).map_err(map_env_err)
}

pub(super) fn parse_writable_scope(raw: &str) -> CliResult<EnvScope> {
    let scope = parse_scope(raw)?;
    if !scope.is_writable() {
        return Err(CliError::with_details(
            2,
            format!("scope '{}' is not writable", scope),
            &["Fix: Use --scope user|system for write operations."],
        ));
    }
    Ok(scope)
}

pub(super) fn parse_format(raw: &str) -> CliResult<ListFormat> {
    let mut format = parse_list_format(raw).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("invalid format '{}'", raw),
            &["Fix: Use auto|table|tsv|json."],
        )
    })?;
    if format == ListFormat::Auto {
        format = if prefer_table_output() {
            ListFormat::Table
        } else {
            ListFormat::Tsv
        };
    }
    Ok(format)
}

pub(super) fn map_env_err(err: EnvError) -> CliError {
    CliError::new(err.exit_code(), err.to_string())
}

pub(super) fn prompt_confirm(prompt: &str, yes: bool) -> CliResult<bool> {
    if yes {
        return Ok(true);
    }
    if !can_interact() {
        return Err(CliError::with_details(
            2,
            "interactive confirmation required".to_string(),
            &["Fix: Run in terminal and confirm, or pass -y."],
        ));
    }
    Confirm::new()
        .with_prompt(prompt)
        .default(false)
        .interact()
        .map_err(|e| CliError::new(1, format!("confirmation failed: {}", e)))
}
