use super::io::{atomic_write_bytes, files_equal, link_template};
use super::render::{is_gui_exe_path, shell_alias_to_shim_with_template};
use super::*;

pub fn sync_shell_alias(
    shims_dir: &Path,
    template_console: &Path,
    template_gui: &Path,
    name: &str,
    alias: &ShellAlias,
) -> Result<()> {
    let (shim_content, use_gui_template) = shell_alias_to_shim_with_template(alias);
    create_shim(
        shims_dir,
        template_console,
        template_gui,
        name,
        &shim_content,
        use_gui_template,
    )
}

pub fn sync_app_alias(
    shims_dir: &Path,
    template_console: &Path,
    template_gui: &Path,
    name: &str,
    alias: &AppAlias,
) -> Result<()> {
    let shim_content = app_alias_to_shim(alias);
    let use_gui_template = is_gui_exe_path(&alias.exe);
    create_shim(
        shims_dir,
        template_console,
        template_gui,
        name,
        &shim_content,
        use_gui_template,
    )
}

pub fn config_to_sync_entries(cfg: &Config) -> Vec<SyncEntry> {
    let mut entries = Vec::with_capacity(cfg.alias.len() + cfg.app.len());
    for (name, alias) in &cfg.alias {
        let (shim_content, use_gui_template) = shell_alias_to_shim_with_template(alias);
        entries.push(SyncEntry {
            name: name.clone(),
            shim_content,
            use_gui_template,
        });
    }
    for (name, alias) in &cfg.app {
        entries.push(SyncEntry {
            name: name.clone(),
            shim_content: app_alias_to_shim(alias),
            use_gui_template: is_gui_exe_path(&alias.exe),
        });
    }
    entries
}

pub(crate) fn create_shim(
    shims_dir: &Path,
    template_console: &Path,
    template_gui: &Path,
    name: &str,
    shim_content: &str,
    use_gui_template: bool,
) -> Result<()> {
    fs::create_dir_all(shims_dir)
        .with_context(|| format!("Failed to create shims dir: {}", shims_dir.display()))?;

    let exe_path = shims_dir.join(format!("{name}.exe"));
    let shim_path = shims_dir.join(format!("{name}.shim"));
    let template = if use_gui_template && template_gui.is_file() {
        template_gui
    } else {
        template_console
    };

    if exe_path.is_file()
        && shim_path.is_file()
        && fs::read_to_string(&shim_path)
            .map(|v| v == shim_content)
            .unwrap_or(false)
        && files_equal(&exe_path, template)
    {
        return Ok(());
    }

    if exe_path.exists() {
        let _ = fs::remove_file(&exe_path);
    }
    if !link_template(template, &exe_path)? {
        fs::copy(template, &exe_path).with_context(|| {
            format!(
                "Failed to copy shim template: {} -> {}",
                template.display(),
                exe_path.display()
            )
        })?;
    }

    atomic_write_bytes(&shim_path, shim_content.as_bytes())?;
    Ok(())
}

pub fn remove_shim(shims_dir: &Path, name: &str) -> Result<()> {
    let exe = shims_dir.join(format!("{name}.exe"));
    let shim = shims_dir.join(format!("{name}.shim"));
    if exe.exists() {
        fs::remove_file(&exe).with_context(|| format!("Failed to remove {}", exe.display()))?;
    }
    if shim.exists() {
        fs::remove_file(&shim).with_context(|| format!("Failed to remove {}", shim.display()))?;
    }
    Ok(())
}

pub fn sync_all(
    entries: &[SyncEntry],
    shims_dir: &Path,
    template_console: &Path,
    template_gui: &Path,
) -> Result<SyncReport> {
    let mut report = SyncReport::default();
    let expected: HashSet<&str> = entries.iter().map(|v| v.name.as_str()).collect();

    for entry in entries {
        match create_shim(
            shims_dir,
            template_console,
            template_gui,
            &entry.name,
            &entry.shim_content,
            entry.use_gui_template,
        ) {
            Ok(()) => report.created.push(entry.name.clone()),
            Err(err) => report.errors.push((entry.name.clone(), err.to_string())),
        }
    }

    if shims_dir.is_dir() {
        for file in fs::read_dir(shims_dir)
            .with_context(|| format!("Failed to read shims dir: {}", shims_dir.display()))?
            .flatten()
        {
            let path = file.path();
            if path.extension().and_then(|v| v.to_str()) != Some("shim") {
                continue;
            }
            let Some(name) = path.file_stem().and_then(|v| v.to_str()) else {
                continue;
            };
            if expected.contains(name) {
                continue;
            }
            match remove_shim(shims_dir, name) {
                Ok(()) => report.removed.push(name.to_string()),
                Err(err) => report.errors.push((name.to_string(), err.to_string())),
            }
        }
    }

    Ok(report)
}
