use super::classify::classify_mode;
use super::*;

pub fn shell_alias_to_shim(alias: &ShellAlias) -> String {
    match classify_mode(&alias.command, alias.mode) {
        ShimKind::Exe { path, fixed_args } => {
            let mut out = format!("type = exe\npath = {}\n", path);
            if let Some(args) = fixed_args
                && !args.trim().is_empty()
            {
                out.push_str(&format!("args = {}\n", args));
            }
            out
        }
        ShimKind::Cmd { command } => {
            format!("type = cmd\ncmd = {}\nwait = true\n", command)
        }
    }
}

pub fn app_alias_to_shim(alias: &AppAlias) -> String {
    let mut out = format!("type = exe\npath = {}\n", alias.exe);
    if let Some(args) = alias.args.as_deref() {
        let args = args.trim();
        if !args.is_empty() {
            out.push_str(&format!("args = {}\n", args));
        }
    }
    out
}

pub(super) fn shell_alias_to_shim_with_template(alias: &ShellAlias) -> (String, bool) {
    match classify_mode(&alias.command, alias.mode) {
        ShimKind::Exe { path, fixed_args } => {
            let mut out = format!("type = exe\npath = {}\n", path);
            if let Some(args) = fixed_args
                && !args.trim().is_empty()
            {
                out.push_str(&format!("args = {}\n", args));
            }
            (out, is_gui_exe_path(&path))
        }
        ShimKind::Cmd { command } => (
            format!("type = cmd\ncmd = {}\nwait = true\n", command),
            false,
        ),
    }
}

pub(super) fn is_gui_exe_path(path: &str) -> bool {
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
