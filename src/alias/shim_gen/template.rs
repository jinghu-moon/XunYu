use super::io::atomic_write_bytes;
use super::pe_patch::patch_subsystem_gui;
use super::*;

pub fn deploy_shim_templates(dest_console: &Path, dest_gui: &Path) -> Result<()> {
    if let Some(parent) = dest_console.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create template dir: {}", parent.display()))?;
    }
    if let Some(parent) = dest_gui.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create template dir: {}", parent.display()))?;
    }

    if !EMBEDDED_SHIM_TEMPLATE.is_empty() {
        if !dest_console.is_file()
            || fs::metadata(dest_console)
                .map(|m| m.len() as usize != EMBEDDED_SHIM_TEMPLATE.len())
                .unwrap_or(true)
            || fs::read(dest_console)
                .map(|v| v.as_slice() != EMBEDDED_SHIM_TEMPLATE)
                .unwrap_or(true)
        {
            atomic_write_bytes(dest_console, EMBEDDED_SHIM_TEMPLATE)?;
        }
        let mut gui_bytes = EMBEDDED_SHIM_TEMPLATE.to_vec();
        patch_subsystem_gui(&mut gui_bytes)?;
        if !dest_gui.is_file()
            || fs::read(dest_gui)
                .map(|v| v.as_slice() != gui_bytes.as_slice())
                .unwrap_or(true)
        {
            atomic_write_bytes(dest_gui, &gui_bytes)?;
        }
        return Ok(());
    }

    if dest_console.exists()
        && dest_console
            .metadata()
            .map(|m| m.len() > 0)
            .unwrap_or(false)
    {
        if !dest_gui.exists() {
            let mut gui_bytes = fs::read(dest_console).with_context(|| {
                format!(
                    "Failed to read console template: {}",
                    dest_console.display()
                )
            })?;
            patch_subsystem_gui(&mut gui_bytes)?;
            atomic_write_bytes(dest_gui, &gui_bytes)?;
        }
        return Ok(());
    }

    for candidate in shim_template_candidates() {
        if !candidate.exists() {
            continue;
        }
        let bytes = fs::read(&candidate).with_context(|| {
            format!(
                "Failed to read shim template candidate: {}",
                candidate.display()
            )
        })?;
        if bytes.is_empty() {
            continue;
        }
        atomic_write_bytes(dest_console, &bytes)?;
        let mut gui_bytes = bytes;
        patch_subsystem_gui(&mut gui_bytes)?;
        atomic_write_bytes(dest_gui, &gui_bytes)?;
        return Ok(());
    }

    anyhow::bail!(
        "alias-shim.exe not found. Build it first: cargo build -p alias-shim --profile release-shim"
    )
}

fn shim_template_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(v) = std::env::var("XUN_ALIAS_SHIM_TEMPLATE") {
        out.push(PathBuf::from(v));
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    out.push(
        root.join("target")
            .join("release-shim")
            .join("alias-shim.exe"),
    );
    out.push(
        root.join("target")
            .join("release-shim")
            .join("deps")
            .join("alias_shim.exe"),
    );

    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        out.push(dir.join("alias-shim.exe"));
    }
    out
}
