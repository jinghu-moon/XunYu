pub const USER_ENV_SUBKEY: &str = "Environment";
pub const SYSTEM_ENV_SUBKEY: &str =
    "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment";

#[cfg(windows)]
mod imp {
    use std::collections::HashSet;
    use std::io;

    use windows_sys::Win32::UI::WindowsAndMessaging::{
        HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE,
    };
    use winreg::enums::{
        HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE, REG_EXPAND_SZ, REG_SZ,
    };
    use winreg::{RegKey, RegValue};

    use crate::env_core::types::{EnvError, EnvResult, EnvScope, EnvVar, SnapshotEntry};
    use crate::env_core::var_type::infer_var_kind;

    use super::{SYSTEM_ENV_SUBKEY, USER_ENV_SUBKEY};

    fn invalid_scope(scope: EnvScope) -> EnvError {
        EnvError::ScopeNotWritable(scope)
    }

    fn map_io_error(err: io::Error, scope: EnvScope, op: &str) -> EnvError {
        if err.kind() == io::ErrorKind::PermissionDenied {
            EnvError::PermissionDenied(format!("{} {} failed: {}", op, scope, err))
        } else {
            EnvError::Io(err)
        }
    }

    fn encode_utf16_le_nul(value: &str) -> Vec<u8> {
        value
            .encode_utf16()
            .chain(std::iter::once(0))
            .flat_map(|u| u.to_le_bytes())
            .collect()
    }

    fn decode_utf16_le(bytes: &[u8]) -> String {
        if bytes.len() < 2 {
            return String::new();
        }
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|ch| u16::from_le_bytes([ch[0], ch[1]]))
            .take_while(|c| *c != 0)
            .collect();
        String::from_utf16_lossy(&units)
    }

    fn open_scope_read(scope: EnvScope) -> Result<RegKey, EnvError> {
        match scope {
            EnvScope::User => RegKey::predef(HKEY_CURRENT_USER)
                .open_subkey(USER_ENV_SUBKEY)
                .map_err(|e| map_io_error(e, scope, "open")),
            EnvScope::System => RegKey::predef(HKEY_LOCAL_MACHINE)
                .open_subkey(SYSTEM_ENV_SUBKEY)
                .map_err(|e| map_io_error(e, scope, "open")),
            EnvScope::All => Err(invalid_scope(scope)),
        }
    }

    fn open_scope_write(scope: EnvScope) -> Result<RegKey, EnvError> {
        match scope {
            EnvScope::User => RegKey::predef(HKEY_CURRENT_USER)
                .open_subkey_with_flags(USER_ENV_SUBKEY, KEY_READ | KEY_WRITE)
                .map_err(|e| map_io_error(e, scope, "open(write)")),
            EnvScope::System => RegKey::predef(HKEY_LOCAL_MACHINE)
                .open_subkey_with_flags(SYSTEM_ENV_SUBKEY, KEY_READ | KEY_WRITE)
                .map_err(|e| map_io_error(e, scope, "open(write)")),
            EnvScope::All => Err(invalid_scope(scope)),
        }
    }

    fn raw_to_string(raw: &RegValue) -> String {
        decode_utf16_le(&raw.bytes)
    }

    fn should_expand(name: &str, value: &str) -> bool {
        name.eq_ignore_ascii_case("PATH") || value.contains('%')
    }

    fn write_value_with_type(
        key: &RegKey,
        name: &str,
        value: &str,
        reg_type: u32,
    ) -> EnvResult<()> {
        if reg_type == REG_EXPAND_SZ as u32 || should_expand(name, value) {
            key.set_raw_value(
                name,
                &RegValue {
                    bytes: encode_utf16_le_nul(value),
                    vtype: REG_EXPAND_SZ,
                },
            )
            .map_err(EnvError::Io)
        } else {
            key.set_value(name, &value.to_string())
                .map_err(EnvError::Io)
        }
    }

    pub fn broadcast_env_change() {
        let target: Vec<u16> = "Environment\0".encode_utf16().collect();
        unsafe {
            let _ = SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                0,
                target.as_ptr() as isize,
                SMTO_ABORTIFHUNG,
                3_000,
                std::ptr::null_mut(),
            );
        }
    }

    pub fn list_scope(scope: EnvScope) -> EnvResult<Vec<EnvVar>> {
        let key = open_scope_read(scope)?;
        let mut out = Vec::new();
        for item in key.enum_values() {
            let (name, value) = item.map_err(EnvError::Io)?;
            let raw_value = raw_to_string(&value);
            out.push(EnvVar {
                scope,
                inferred_kind: infer_var_kind(&name, &raw_value),
                name,
                raw_value,
                reg_type: value.vtype as u32,
            });
        }
        out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(out)
    }

    pub fn list_vars(scope: EnvScope) -> EnvResult<Vec<EnvVar>> {
        match scope {
            EnvScope::User | EnvScope::System => list_scope(scope),
            EnvScope::All => {
                let mut all = list_scope(EnvScope::User)?;
                all.extend(list_scope(EnvScope::System)?);
                Ok(all)
            }
        }
    }

    pub fn get_var(scope: EnvScope, name: &str) -> EnvResult<Option<EnvVar>> {
        if !scope.is_writable() {
            return Err(invalid_scope(scope));
        }
        let key = open_scope_read(scope)?;
        match key.get_raw_value(name) {
            Ok(raw) => {
                let raw_value = raw_to_string(&raw);
                let inferred_kind = infer_var_kind(name, &raw_value);
                Ok(Some(EnvVar {
                    scope,
                    name: name.to_string(),
                    raw_value,
                    reg_type: raw.vtype as u32,
                    inferred_kind,
                }))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(map_io_error(e, scope, "get")),
        }
    }

    pub fn set_var(scope: EnvScope, name: &str, value: &str) -> EnvResult<()> {
        if !scope.is_writable() {
            return Err(invalid_scope(scope));
        }
        let key = open_scope_write(scope)?;
        let reg_type = if should_expand(name, value) {
            REG_EXPAND_SZ as u32
        } else {
            REG_SZ as u32
        };
        write_value_with_type(&key, name, value, reg_type)?;
        broadcast_env_change();
        Ok(())
    }

    pub fn delete_var(scope: EnvScope, name: &str) -> EnvResult<bool> {
        if !scope.is_writable() {
            return Err(invalid_scope(scope));
        }
        let key = open_scope_write(scope)?;
        match key.delete_value(name) {
            Ok(_) => {
                broadcast_env_change();
                Ok(true)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(map_io_error(e, scope, "delete")),
        }
    }

    pub fn get_path_entries(scope: EnvScope) -> EnvResult<Vec<String>> {
        if !scope.is_writable() {
            return Err(invalid_scope(scope));
        }
        let path = get_var(scope, "PATH")?;
        let mut out = Vec::new();
        if let Some(path) = path {
            out.extend(
                path.raw_value
                    .split(';')
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(str::to_string),
            );
        }
        Ok(out)
    }

    pub fn set_path_entries(scope: EnvScope, entries: &[String]) -> EnvResult<()> {
        if !scope.is_writable() {
            return Err(invalid_scope(scope));
        }
        let key = open_scope_write(scope)?;
        let joined = entries.join(";");
        write_value_with_type(&key, "PATH", &joined, REG_EXPAND_SZ as u32)?;
        broadcast_env_change();
        Ok(())
    }

    pub fn add_path_entry(scope: EnvScope, entry: &str, head: bool) -> EnvResult<bool> {
        let mut path = get_path_entries(scope)?;
        if path.iter().any(|v| v.eq_ignore_ascii_case(entry)) {
            return Ok(false);
        }
        if head {
            path.insert(0, entry.to_string());
        } else {
            path.push(entry.to_string());
        }
        set_path_entries(scope, &path)?;
        Ok(true)
    }

    pub fn remove_path_entry(scope: EnvScope, entry: &str) -> EnvResult<bool> {
        let mut path = get_path_entries(scope)?;
        let before = path.len();
        path.retain(|v| !v.eq_ignore_ascii_case(entry));
        if path.len() == before {
            return Ok(false);
        }
        set_path_entries(scope, &path)?;
        Ok(true)
    }

    pub fn replace_scope(scope: EnvScope, vars: &[SnapshotEntry]) -> EnvResult<()> {
        if !scope.is_writable() {
            return Err(invalid_scope(scope));
        }
        let key = open_scope_write(scope)?;

        let existing_names: Vec<String> = key
            .enum_values()
            .filter_map(Result::ok)
            .map(|(name, _)| name)
            .collect();
        for name in &existing_names {
            let _ = key.delete_value(name);
        }

        let mut dedupe = HashSet::new();
        for v in vars {
            let upper = v.name.to_uppercase();
            if !dedupe.insert(upper) {
                continue;
            }
            write_value_with_type(&key, &v.name, &v.raw_value, v.reg_type)?;
        }

        broadcast_env_change();
        Ok(())
    }
}

#[cfg(not(windows))]
mod imp {
    use crate::env_core::types::{EnvError, EnvResult, EnvScope, EnvVar, SnapshotEntry};

    fn unsupported() -> EnvError {
        EnvError::UnsupportedPlatform
    }

    pub fn broadcast_env_change() {}

    pub fn list_scope(_scope: EnvScope) -> EnvResult<Vec<EnvVar>> {
        Err(unsupported())
    }

    pub fn list_vars(_scope: EnvScope) -> EnvResult<Vec<EnvVar>> {
        Err(unsupported())
    }

    pub fn get_var(_scope: EnvScope, _name: &str) -> EnvResult<Option<EnvVar>> {
        Err(unsupported())
    }

    pub fn set_var(_scope: EnvScope, _name: &str, _value: &str) -> EnvResult<()> {
        Err(unsupported())
    }

    pub fn delete_var(_scope: EnvScope, _name: &str) -> EnvResult<bool> {
        Err(unsupported())
    }

    pub fn get_path_entries(_scope: EnvScope) -> EnvResult<Vec<String>> {
        Err(unsupported())
    }

    pub fn set_path_entries(_scope: EnvScope, _entries: &[String]) -> EnvResult<()> {
        Err(unsupported())
    }

    pub fn add_path_entry(_scope: EnvScope, _entry: &str, _head: bool) -> EnvResult<bool> {
        Err(unsupported())
    }

    pub fn remove_path_entry(_scope: EnvScope, _entry: &str) -> EnvResult<bool> {
        Err(unsupported())
    }

    pub fn replace_scope(_scope: EnvScope, _vars: &[SnapshotEntry]) -> EnvResult<()> {
        Err(unsupported())
    }
}

pub use imp::{
    add_path_entry, delete_var, get_path_entries, get_var, list_scope, list_vars,
    remove_path_entry, replace_scope, set_path_entries, set_var,
};
