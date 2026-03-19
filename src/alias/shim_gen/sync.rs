use super::io::{atomic_write_bytes, files_equal, files_equal_path, link_template};
use super::render::{is_gui_exe_path, shell_alias_to_shim_with_template};
use super::*;

struct TemplateAsset<'a> {
    path: &'a Path,
    bytes: Vec<u8>,
}

struct TemplateCache<'a> {
    console: TemplateAsset<'a>,
    gui: TemplateAsset<'a>,
}

fn load_template_cache<'a>(template_console: &'a Path, template_gui: &'a Path) -> Result<TemplateCache<'a>> {
    Ok(TemplateCache {
        console: TemplateAsset {
            path: template_console,
            bytes: fs::read(template_console).with_context(|| {
                format!("Failed to read shim template: {}", template_console.display())
            })?,
        },
        gui: TemplateAsset {
            path: template_gui,
            bytes: fs::read(template_gui).with_context(|| {
                format!("Failed to read shim template: {}", template_gui.display())
            })?,
        },
    })
}

pub fn shell_alias_to_sync_entry(name: &str, alias: &ShellAlias) -> SyncEntry {
    let (shim_content, use_gui_template) = shell_alias_to_shim_with_template(alias);
    SyncEntry {
        name: name.to_string(),
        shim_content,
        use_gui_template,
    }
}

pub fn app_alias_to_sync_entry(name: &str, alias: &AppAlias) -> SyncEntry {
    SyncEntry {
        name: name.to_string(),
        shim_content: app_alias_to_shim(alias),
        use_gui_template: is_gui_exe_path(&alias.exe),
    }
}

fn create_shim_with_cache(
    shims_dir: &Path,
    templates: &TemplateCache<'_>,
    name: &str,
    shim_content: &str,
    use_gui_template: bool,
) -> Result<()> {
    fs::create_dir_all(shims_dir)
        .with_context(|| format!("Failed to create shims dir: {}", shims_dir.display()))?;

    let exe_path = shims_dir.join(format!("{name}.exe"));
    let shim_path = shims_dir.join(format!("{name}.shim"));
    let template = if use_gui_template && templates.gui.path.is_file() {
        &templates.gui
    } else {
        &templates.console
    };

    if exe_path.is_file()
        && shim_path.is_file()
        && fs::read_to_string(&shim_path)
            .map(|v| v == shim_content)
            .unwrap_or(false)
        && files_equal(&exe_path, template.bytes.as_slice())
    {
        return Ok(());
    }

    if exe_path.exists() {
        let _ = fs::remove_file(&exe_path);
    }
    if !link_template(template.path, &exe_path)? {
        atomic_write_bytes(&exe_path, template.bytes.as_slice())?;
    }

    atomic_write_bytes(&shim_path, shim_content.as_bytes())?;
    Ok(())
}

fn create_shim_direct(
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
        && files_equal_path(&exe_path, template)
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

pub fn sync_shell_alias(
    shims_dir: &Path,
    template_console: &Path,
    template_gui: &Path,
    name: &str,
    alias: &ShellAlias,
) -> Result<()> {
    let entry = shell_alias_to_sync_entry(name, alias);
    create_shim_direct(
        shims_dir,
        template_console,
        template_gui,
        &entry.name,
        &entry.shim_content,
        entry.use_gui_template,
    )
}

pub fn sync_app_alias(
    shims_dir: &Path,
    template_console: &Path,
    template_gui: &Path,
    name: &str,
    alias: &AppAlias,
) -> Result<()> {
    let entry = app_alias_to_sync_entry(name, alias);
    create_shim_direct(
        shims_dir,
        template_console,
        template_gui,
        &entry.name,
        &entry.shim_content,
        entry.use_gui_template,
    )
}

pub fn config_to_sync_entries(cfg: &Config) -> Vec<SyncEntry> {
    let mut entries = Vec::with_capacity(cfg.alias.len() + cfg.app.len());
    for (name, alias) in &cfg.alias {
        entries.push(shell_alias_to_sync_entry(name, alias));
    }
    for (name, alias) in &cfg.app {
        entries.push(app_alias_to_sync_entry(name, alias));
    }
    entries
}

pub fn sync_entries(
    entries: &[SyncEntry],
    shims_dir: &Path,
    template_console: &Path,
    template_gui: &Path,
) -> Result<SyncReport> {
    let cache = load_template_cache(template_console, template_gui)?;
    let mut report = SyncReport::default();

    for entry in entries {
        match create_shim_with_cache(
            shims_dir,
            &cache,
            &entry.name,
            &entry.shim_content,
            entry.use_gui_template,
        ) {
            Ok(()) => report.created.push(entry.name.clone()),
            Err(err) => report.errors.push((entry.name.clone(), err.to_string())),
        }
    }

    Ok(report)
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
    let expected: HashSet<&str> = entries.iter().map(|v| v.name.as_str()).collect();
    let mut report = sync_entries(entries, shims_dir, template_console, template_gui)?;

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
