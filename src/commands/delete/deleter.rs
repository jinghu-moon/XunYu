use super::reboot_delete;
use super::winapi;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Outcome {
    Ok,
    OkAttr,
    OkOwn,
    OkNtd,
    OkFch,
    OkReboot,
    WhatIf,
    Error(u32),
}

impl Outcome {
    pub(crate) fn is_success(&self) -> bool {
        matches!(
            self,
            Outcome::Ok
                | Outcome::OkAttr
                | Outcome::OkOwn
                | Outcome::OkNtd
                | Outcome::OkFch
                | Outcome::OkReboot
                | Outcome::WhatIf
        )
    }

    pub(crate) fn is_error(&self) -> bool {
        matches!(self, Outcome::Error(_))
    }

    pub(crate) fn is_deferred(&self) -> bool {
        matches!(self, Outcome::OkNtd | Outcome::OkReboot)
    }

    pub(crate) fn label(&self) -> &'static str {
        match self {
            Outcome::Ok => "Deleted",
            Outcome::OkAttr => "Deleted (attr reset)",
            Outcome::OkOwn => "Deleted (take ownership)",
            Outcome::OkNtd => "Deferred (delete on close)",
            Outcome::OkFch => "Deleted (forced handle close)",
            Outcome::OkReboot => "Scheduled (delete on reboot)",
            Outcome::WhatIf => "WhatIf",
            Outcome::Error(_) => "Failed",
        }
    }

    pub(crate) fn error_desc(code: u32) -> String {
        match code {
            5 => "Access denied".into(),
            32 => "File is in use".into(),
            2 => "File not found".into(),
            206 => "Path too long".into(),
            c => format!("Win32 error {}", c),
        }
    }
}

pub(crate) fn try_delete_from_level(
    path: &str,
    start_level: u8,
    handle_snapshot: &[winapi::SysHandleEntry],
) -> Outcome {
    let mut err: u32 = 0;

    if start_level <= 1 {
        err = winapi::delete_file(path);
        if err == 0 {
            return Outcome::Ok;
        }
    }

    if start_level <= 2 {
        winapi::set_normal_attrs(path);
        err = winapi::delete_file(path);
        if err == 0 {
            return Outcome::OkAttr;
        }
    }

    if start_level <= 3 && (err == 5 || start_level == 3) && winapi::take_ownership_and_grant(path)
    {
        err = winapi::delete_file(path);
        if err == 0 {
            return Outcome::OkOwn;
        }
    }

    if start_level <= 4
        && (err == 32 || err == 5 || start_level == 4)
        && winapi::mark_delete_on_close(path)
    {
        return Outcome::OkNtd;
    }

    if start_level <= 5 && (err == 32 || start_level == 5) {
        let closed = winapi::force_close_external_handles(path, handle_snapshot);
        if closed > 0 {
            err = winapi::delete_file(path);
            if err == 0 {
                return Outcome::OkFch;
            }
        }
    }

    if start_level == 6 && reboot_delete::schedule_delete_on_reboot(path) {
        return Outcome::OkReboot;
    }

    Outcome::Error(winapi::get_last_error())
}
