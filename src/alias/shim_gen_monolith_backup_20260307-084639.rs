use std::collections::HashSet;
use std::fs;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::alias::config::{AliasMode, AppAlias, Config, ShellAlias};

const EMBEDDED_SHIM_TEMPLATE: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/alias_shim_template.bin"));

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ShimKind {
    Exe {
        path: String,
        fixed_args: Option<String>,
    },
    Cmd {
        command: String,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct SyncEntry {
    pub(crate) name: String,
    pub(crate) shim_content: String,
    pub(crate) use_gui_template: bool,
}

#[derive(Debug, Default)]
pub(crate) struct SyncReport {
    pub(crate) created: Vec<String>,
    pub(crate) removed: Vec<String>,
    pub(crate) errors: Vec<(String, String)>,
}

#[allow(dead_code)]
pub(crate) fn classify_command(command: &str) -> ShimKind {
    classify_mode(command, AliasMode::Auto)
}

pub(crate) fn classify_mode(command: &str, mode: AliasMode) -> ShimKind {
    let trimmed = command.trim();
    match mode {
        AliasMode::Cmd => {
            return ShimKind::Cmd {
                command: trimmed.to_string(),
            };
        }
        AliasMode::Exe => {
            if let Some((path, args)) = parse_exe_candidate(trimmed) {
                return ShimKind::Exe {
                    path,
                    fixed_args: args,
                };
            }
            return ShimKind::Cmd {
                command: trimmed.to_string(),
            };
        }
        AliasMode::Auto => {}
    }

    if contains_shell_operators(trimmed) {
        return ShimKind::Cmd {
            command: trimmed.to_string(),
        };
    }
    if let Some((path, args)) = parse_exe_candidate(trimmed) {
        return ShimKind::Exe {
            path,
            fixed_args: args,
        };
    }
    ShimKind::Cmd {
        command: trimmed.to_string(),
    }
}

pub(crate) fn shell_alias_to_shim(alias: &ShellAlias) -> String {
    match classify_mode(&alias.command, alias.mode) {
        ShimKind::Exe { path, fixed_args } => {
            let mut out = format!("type = exe\npath = {}\n", path);
            if let Some(args) = fixed_args {
                if !args.trim().is_empty() {
                    out.push_str(&format!("args = {}\n", args));
                }
            }
            out
        }
        ShimKind::Cmd { command } => {
            format!("type = cmd\ncmd = {}\nwait = true\n", command)
        }
    }
}

pub(crate) fn app_alias_to_shim(alias: &AppAlias) -> String {
    let mut out = format!("type = exe\npath = {}\n", alias.exe);
    if let Some(args) = alias.args.as_deref() {
        let args = args.trim();
        if !args.is_empty() {
            out.push_str(&format!("args = {}\n", args));
        }
    }
    out
}

pub(crate) fn config_to_sync_entries(cfg: &Config) -> Vec<SyncEntry> {
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

fn shell_alias_to_shim_with_template(alias: &ShellAlias) -> (String, bool) {
    match classify_mode(&alias.command, alias.mode) {
        ShimKind::Exe { path, fixed_args } => {
            let mut out = format!("type = exe\npath = {}\n", path);
            if let Some(args) = fixed_args {
                if !args.trim().is_empty() {
                    out.push_str(&format!("args = {}\n", args));
                }
            }
            (out, is_gui_exe_path(&path))
        }
        ShimKind::Cmd { command } => (
            format!("type = cmd\ncmd = {}\nwait = true\n", command),
            false,
        ),
    }
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

pub(crate) fn remove_shim(shims_dir: &Path, name: &str) -> Result<()> {
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

pub(crate) fn sync_all(
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

pub(crate) fn deploy_shim_templates(dest_console: &Path, dest_gui: &Path) -> Result<()> {
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

fn atomic_write_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create dir: {}", parent.display()))?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes)
        .with_context(|| format!("Failed to write temp file: {}", tmp.display()))?;
    replace_file(&tmp, path)
        .with_context(|| format!("Failed to replace file: {}", path.display()))?;
    Ok(())
}

fn files_equal(path_a: &Path, path_b: &Path) -> bool {
    let Ok(meta_a) = fs::metadata(path_a) else {
        return false;
    };
    let Ok(meta_b) = fs::metadata(path_b) else {
        return false;
    };
    if meta_a.len() != meta_b.len() {
        return false;
    }
    let Ok(bytes_a) = fs::read(path_a) else {
        return false;
    };
    let Ok(bytes_b) = fs::read(path_b) else {
        return false;
    };
    bytes_a == bytes_b
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

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            out.push(dir.join("alias-shim.exe"));
        }
    }
    out
}

fn patch_subsystem_gui(bytes: &mut [u8]) -> Result<()> {
    if bytes.len() < 0x40 {
        anyhow::bail!("PE image too small");
    }
    let mz = u16::from_le_bytes([bytes[0], bytes[1]]);
    if mz != 0x5a4d {
        anyhow::bail!("invalid MZ signature");
    }
    let e_lfanew =
        u32::from_le_bytes([bytes[0x3c], bytes[0x3d], bytes[0x3e], bytes[0x3f]]) as usize;
    if bytes.len() < e_lfanew + 4 + 20 + 70 {
        anyhow::bail!("PE header out of range");
    }
    let pe = u32::from_le_bytes([
        bytes[e_lfanew],
        bytes[e_lfanew + 1],
        bytes[e_lfanew + 2],
        bytes[e_lfanew + 3],
    ]);
    if pe != 0x0000_4550 {
        anyhow::bail!("invalid PE signature");
    }
    let opt = e_lfanew + 4 + 20;
    let magic = u16::from_le_bytes([bytes[opt], bytes[opt + 1]]);
    if magic != 0x10b && magic != 0x20b {
        anyhow::bail!("unsupported optional header magic");
    }
    let subsystem_off = opt + 68;
    bytes[subsystem_off] = 2;
    bytes[subsystem_off + 1] = 0;
    Ok(())
}

fn is_gui_exe_path(path: &str) -> bool {
    let mut file = match fs::File::open(path) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let mut dos = [0u8; 64];
    if file.read_exact(&mut dos).is_err() {
        return false;
    }
    if u16::from_le_bytes([dos[0], dos[1]]) != 0x5a4d {
        return false;
    }
    let e_lfanew = u32::from_le_bytes([dos[0x3c], dos[0x3d], dos[0x3e], dos[0x3f]]) as u64;

    let mut pe_sig = [0u8; 4];
    if file
        .seek(SeekFrom::Start(e_lfanew))
        .and_then(|_| file.read_exact(&mut pe_sig))
        .is_err()
    {
        return false;
    }
    if u32::from_le_bytes(pe_sig) != 0x0000_4550 {
        return false;
    }

    let opt = e_lfanew + 4 + 20;
    let mut magic_bytes = [0u8; 2];
    if file
        .seek(SeekFrom::Start(opt))
        .and_then(|_| file.read_exact(&mut magic_bytes))
        .is_err()
    {
        return false;
    }
    let magic = u16::from_le_bytes(magic_bytes);
    if magic != 0x10b && magic != 0x20b {
        return false;
    }

    let mut subsystem_bytes = [0u8; 2];
    if file
        .seek(SeekFrom::Start(opt + 68))
        .and_then(|_| file.read_exact(&mut subsystem_bytes))
        .is_err()
    {
        return false;
    }
    let subsystem = u16::from_le_bytes(subsystem_bytes);
    subsystem == 2
}

#[cfg(windows)]
fn replace_file(from: &Path, to: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let mut from_w: Vec<u16> = from.as_os_str().encode_wide().collect();
    from_w.push(0);
    let mut to_w: Vec<u16> = to.as_os_str().encode_wide().collect();
    to_w.push(0);

    let ok = unsafe {
        MoveFileExW(
            from_w.as_ptr(),
            to_w.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(from: &Path, to: &Path) -> io::Result<()> {
    fs::rename(from, to)
}

fn parse_exe_candidate(command: &str) -> Option<(String, Option<String>)> {
    let mut parts = command.splitn(2, char::is_whitespace);
    let exe = parts.next()?.trim();
    let rest = parts.next().map(str::trim).filter(|v| !v.is_empty());

    let path = Path::new(exe);
    if path.is_absolute() && path.is_file() {
        return Some((exe.to_string(), rest.map(str::to_string)));
    }

    find_in_path(exe).map(|p| (p, rest.map(str::to_string)))
}

fn contains_shell_operators(command: &str) -> bool {
    command
        .chars()
        .any(|ch| matches!(ch, '|' | '&' | '<' | '>' | ';' | '`'))
}

fn find_in_path(exe: &str) -> Option<String> {
    let path_var = std::env::var("PATH").ok()?;
    let candidates = executable_candidates(exe);
    for dir in path_var.split(';') {
        if dir.is_empty() {
            continue;
        }
        let base = Path::new(dir);
        for name in &candidates {
            let p = base.join(name);
            if p.is_file() {
                return Some(p.to_string_lossy().into_owned());
            }
        }
    }
    None
}

fn executable_candidates(exe: &str) -> Vec<String> {
    if Path::new(exe).extension().is_some() {
        return vec![exe.to_string()];
    }
    vec![
        format!("{exe}.exe"),
        format!("{exe}.cmd"),
        format!("{exe}.bat"),
        format!("{exe}.com"),
    ]
}

#[cfg(windows)]
fn link_template(src: &Path, dst: &Path) -> Result<bool> {
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Storage::FileSystem::CreateHardLinkW;

    let mut src_w: Vec<u16> = src.as_os_str().encode_wide().collect();
    src_w.push(0);
    let mut dst_w: Vec<u16> = dst.as_os_str().encode_wide().collect();
    dst_w.push(0);
    let ok = unsafe { CreateHardLinkW(dst_w.as_ptr(), src_w.as_ptr(), std::ptr::null()) };
    Ok(ok != 0)
}

#[cfg(not(windows))]
fn link_template(_src: &Path, _dst: &Path) -> Result<bool> {
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alias::config::{AliasMode, ShellAlias};

    #[test]
    fn auto_mode_detects_cmd_operators() {
        let kind = classify_mode("git status | findstr M", AliasMode::Auto);
        assert!(matches!(kind, ShimKind::Cmd { .. }));
    }

    #[test]
    fn mode_cmd_forces_cmd() {
        let kind = classify_mode("notepad.exe", AliasMode::Cmd);
        assert!(matches!(kind, ShimKind::Cmd { .. }));
    }

    #[test]
    fn shell_alias_shim_contains_mode() {
        let alias = ShellAlias {
            command: "echo hi".to_string(),
            desc: None,
            tags: vec![],
            shells: vec![],
            mode: AliasMode::Cmd,
        };
        let text = shell_alias_to_shim(&alias);
        assert!(text.contains("type = cmd"));
        assert!(text.contains("wait = true"));
    }
}
