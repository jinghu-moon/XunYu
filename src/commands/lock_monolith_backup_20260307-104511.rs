use crate::cli::{LockCmd, LockSubCommand, LockWhoCmd, MvCmd, RenFileCmd};
use crate::output::{CliError, CliResult};
use crate::output::{apply_pretty_table_style, emit_warning, print_table};
use crate::windows::handle_query::get_locking_processes;
use crate::windows::restart_manager::{LockQueryError, LockerInfo};
use comfy_table::{Attribute, Cell, Color, Table};
use std::path::Path;

pub(crate) fn cmd_lock(args: LockCmd) -> CliResult {
    match args.cmd {
        LockSubCommand::Who(a) => cmd_lock_who(a),
    }
}

fn report_lock_query_error(prefix: &str, err: &LockQueryError) {
    let msg = format!("{prefix}: {err}");
    let hint = format!("Hint: {}", err.guidance());
    emit_warning(&msg, &[hint.as_str()]);
}

const FORCE_KILL_WAIT_MS: u32 = 500;
const UNLOCK_MAX_RETRIES: u64 = 3;

fn try_force_kill_process(pid: u32, name: &str) -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE, TerminateProcess,
            WaitForSingleObject,
        };

        let handle = unsafe {
            OpenProcess(
                PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
                0,
                pid,
            )
        };
        if !handle.is_null() {
            let killed = unsafe { TerminateProcess(handle, 1) } != 0;
            let _ = unsafe { WaitForSingleObject(handle, FORCE_KILL_WAIT_MS) };
            unsafe {
                CloseHandle(handle);
            }
            if killed {
                return true;
            }
        }
    }

    let status = std::process::Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    match status {
        Ok(s) if s.success() => true,
        Ok(s) => {
            let msg = format!("taskkill failed for PID {pid} ({name})");
            let detail = format!("Status: {s}");
            emit_warning(&msg, &[detail.as_str()]);
            false
        }
        Err(e) => {
            let msg = format!("taskkill error for PID {pid} ({name})");
            let detail = format!("Details: {e}");
            emit_warning(&msg, &[detail.as_str()]);
            false
        }
    }
}

fn list_lockers_or_default(path: &Path, err_prefix: &str) -> Vec<LockerInfo> {
    let timeout_ms = env_u64("XUN_LOCK_QUERY_TIMEOUT_MS").unwrap_or(15_000);
    match get_locking_processes_with_timeout(path, timeout_ms) {
        Ok(lockers) => lockers,
        Err(err) => {
            report_lock_query_error(err_prefix, &err);
            Vec::new()
        }
    }
}

fn print_lockers(lockers: &[LockerInfo]) {
    ui_println!("File is locked by {} process(es):", lockers.len());
    for l in lockers {
        ui_println!("  - PID: {} ({})", l.pid, l.name);
        if is_critical_process_name(&l.name) {
            ui_println!("    [WARNING] Critical system process! Killing is unsafe.");
        }
    }
}

fn is_critical_process_name(name: &str) -> bool {
    let name_lower = name.to_lowercase();
    CRITICAL_PROCESSES.iter().any(|&c| name_lower == c)
}

fn ensure_force_kill_authorized(force_kill: bool, yes: bool) -> CliResult {
    if force_kill {
        return Ok(());
    }
    if !yes && crate::output::can_interact() {
        let run = dialoguer::Confirm::new()
            .with_prompt("Force kill these processes?")
            .interact()
            .unwrap_or(false);
        if !run {
            return Err(CliError::new(
                crate::util::EXIT_LOCKED_UNAUTHORIZED,
                "Cancelled.",
            ));
        }
        return Ok(());
    }
    Err(CliError::with_details(
        crate::util::EXIT_LOCKED_UNAUTHORIZED,
        "No --force-kill provided and non-interactive. Operation aborted.",
        &["Fix: Re-run with --force-kill, or run in an interactive terminal and confirm."],
    ))
}

fn kill_lockers(lockers: &[LockerInfo], prefix: &str) {
    for l in lockers {
        ui_println!("{prefix} {} (PID: {})...", l.name, l.pid);
        let _ = try_force_kill_process(l.pid, &l.name);
    }
}

pub(crate) fn unlock_and_retry<F>(
    path: &Path,
    force_kill: bool,
    yes: bool,
    initial_error: &std::io::Error,
    lock_query_error_prefix: &str,
    mut op: F,
) -> CliResult
where
    F: FnMut() -> Result<(), std::io::Error>,
{
    ui_println!("Operation failed. Checking lockers...");
    let lockers = list_lockers_or_default(path, lock_query_error_prefix);
    if lockers.is_empty() {
        return Err(CliError::with_details(
            crate::util::EXIT_ACCESS_DENIED,
            format!("Still failed: no locking processes found. OsError: {initial_error}"),
            &["Hint: Close the process using the file, or retry later."],
        ));
    }

    print_lockers(&lockers);
    ensure_force_kill_authorized(force_kill, yes)?;
    kill_lockers(&lockers, "Attempting to kill");

    let mut last_err: Option<std::io::Error> = None;
    for attempt in 0..UNLOCK_MAX_RETRIES {
        std::thread::sleep(std::time::Duration::from_millis(500 + attempt * 300));
        match op() {
            Ok(_) => return Ok(()),
            Err(err) => {
                last_err = Some(err);
                let retry_lockers = get_locking_processes(&[path]).unwrap_or_default();
                kill_lockers(&retry_lockers, "Retry kill");
            }
        }
    }

    let err = last_err.unwrap_or_else(|| std::io::Error::other("unknown unlock operation failure"));
    Err(CliError::new(
        crate::util::EXIT_UNLOCK_FAILED,
        format!("Still failed after unlocking: {err}"),
    ))
}

pub(crate) fn cmd_lock_who(args: LockWhoCmd) -> CliResult {
    let path = Path::new(&args.path);
    if !path.exists() {
        return Err(CliError::with_details(
            2,
            format!("File or directory not found: {}", args.path),
            &["Fix: Check the path and try again."],
        ));
    }

    if crate::runtime::is_verbose() {
        let _ = crate::windows::handle_query::ensure_debug_privilege();
    }

    // Fast path: for files, if we can open with no sharing, treat as unlocked.
    if path.is_file() && try_open_exclusive(path) {
        if args.format != "json" && args.format != "tsv" {
            ui_println!("No locking processes found.");
        }
        return Ok(());
    }

    let timeout_ms = env_u64("XUN_LOCK_QUERY_TIMEOUT_MS").unwrap_or(15_000);
    let lockers = match get_locking_processes_with_timeout(path, timeout_ms) {
        Ok(l) => l,
        Err(e) => {
            ui_println!("Lock query timed out.");
            report_lock_query_error("Failed to query lock status", &e);
            return Err(CliError::new(1, "Lock query timed out."));
        }
    };

    if lockers.is_empty() {
        if args.format != "json" && args.format != "tsv" {
            ui_println!("No locking processes found.");
        }
        return Ok(());
    }

    if args.format == "json" {
        let json_arr: Vec<_> = lockers
            .iter()
            .map(|l| {
                serde_json::json!({
                    "pid": l.pid,
                    "name": l.name,
                    "type": l.app_type,
                })
            })
            .collect();
        crate::output::ui_println(format_args!(
            "{}",
            serde_json::to_string(&json_arr).unwrap_or_default()
        ));
        return Ok(());
    }

    if args.format == "tsv" {
        for l in &lockers {
            crate::output::ui_println(format_args!("{}\t{}\t{}", l.pid, l.name, l.app_type));
        }
        return Ok(());
    }

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("PID")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Process Name")
            .add_attribute(Attribute::Bold)
            .fg(Color::Yellow),
        Cell::new("App Type")
            .add_attribute(Attribute::Bold)
            .fg(Color::Magenta),
    ]);

    for l in &lockers {
        table.add_row(vec![
            Cell::new(l.pid).fg(Color::Cyan),
            Cell::new(&l.name).fg(Color::Yellow),
            Cell::new(l.app_type).fg(Color::Magenta),
        ]);
    }
    print_table(&table);
    Ok(())
}

fn env_u64(key: &str) -> Option<u64> {
    std::env::var(key).ok().and_then(|v| v.parse::<u64>().ok())
}

fn try_open_exclusive(path: &Path) -> bool {
    let mut opts = std::fs::OpenOptions::new();
    opts.read(true).write(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        opts.share_mode(0);
    }
    opts.open(path).is_ok()
}

fn get_locking_processes_with_timeout(
    path: &Path,
    timeout_ms: u64,
) -> Result<
    Vec<crate::windows::restart_manager::LockerInfo>,
    crate::windows::restart_manager::LockQueryError,
> {
    use std::sync::mpsc;
    use std::time::Duration;

    let (tx, rx) = mpsc::channel();
    let p = path.to_path_buf();
    std::thread::spawn(move || {
        let res = get_locking_processes(&[p.as_path()]);
        let _ = tx.send(res);
    });

    match rx.recv_timeout(Duration::from_millis(timeout_ms)) {
        Ok(res) => res,
        Err(_) => Err(crate::windows::restart_manager::LockQueryError::from_win32(
            crate::windows::restart_manager::ERROR_SEM_TIMEOUT_CODE,
            crate::windows::restart_manager::LockQueryStage::StartSession,
            "timeout",
        )),
    }
}

const CRITICAL_PROCESSES: &[&str] = &[
    "csrss.exe",
    "wininit.exe",
    "lsass.exe",
    "services.exe",
    "smss.exe",
    "winlogon.exe",
    "explorer.exe",
];

pub(crate) fn cmd_mv(args: MvCmd) -> CliResult {
    do_move(
        Path::new(&args.src),
        Path::new(&args.dst),
        args.unlock,
        args.force_kill,
        args.dry_run,
        args.yes,
        args.force,
        args.reason.as_deref(),
    )
}

pub(crate) fn cmd_ren_file(args: RenFileCmd) -> CliResult {
    do_move(
        Path::new(&args.src),
        Path::new(&args.dst),
        args.unlock,
        args.force_kill,
        args.dry_run,
        args.yes,
        args.force,
        args.reason.as_deref(),
    )
}

fn do_move(
    src: &Path,
    dst: &Path,
    unlock: bool,
    force_kill: bool,
    dry_run: bool,
    yes: bool,
    force: bool,
    reason: Option<&str>,
) -> CliResult {
    if !src.exists() {
        emit_warning(
            format!("Source not found: {}", src.display()),
            &["Hint: Check the path exists, or use an absolute path."],
        );
        return Ok(());
    }

    #[cfg(feature = "protect")]
    if let Err(msg) = crate::protect::check_protection(src, "move", force, reason) {
        return Err(CliError::with_details(
            crate::util::EXIT_ACCESS_DENIED,
            format!("Protection check failed: {msg}"),
            &["Fix: Add --force with a reason, or update protect rules to allow this operation."],
        ));
    }
    if dry_run {
        ui_println!("DRY RUN: would move {:?} to {:?}", src, dst);
        return Ok(());
    }

    if let Err(e) = std::fs::rename(src, dst) {
        if !unlock {
            return Err(CliError::with_details(
                crate::util::EXIT_ACCESS_DENIED,
                format!("Move failed: {e}."),
                &["Fix: Re-run with --unlock, or close the process locking the file."],
            ));
        }

        unlock_and_retry(
            src,
            force_kill,
            yes,
            &e,
            "Lock query unavailable; fallback to plain move failure",
            || std::fs::rename(src, dst),
        )?;
        ui_println!("Successfully unlocked and moved {:?} to {:?}", src, dst);
        crate::security::audit::audit_log(
            "force_move",
            &src.to_string_lossy(),
            "cli",
            format!("target={:?} force_kill={}", dst, force_kill),
            "success",
            reason.unwrap_or(""),
        );
    } else {
        ui_println!("Moved {:?} to {:?}", src, dst);
        if force && cfg!(feature = "protect") {
            crate::security::audit::audit_log(
                "protected_move",
                &src.to_string_lossy(),
                "cli",
                format!("target={:?}", dst),
                "success",
                reason.unwrap_or(""),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
