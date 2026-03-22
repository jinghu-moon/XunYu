use crate::cli::RmCmd;

#[cfg(feature = "lock")]
use crate::commands::lock::unlock_and_retry;

use crate::output::{CliError, CliResult};

pub(crate) fn cmd_rm(args: RmCmd) -> CliResult {
    #[cfg(not(feature = "lock"))]
    let _ = args.yes;
    let _ = &args.format;

    let mut policy = crate::path_guard::PathPolicy::for_write();
    policy.must_exist = true;
    let validation = crate::path_guard::validate_paths(vec![args.path.clone()], &policy);
    if !validation.issues.is_empty() {
        let details: Vec<String> = validation
            .issues
            .iter()
            .map(|i| format!("{} ({})", i.raw, i.detail))
            .collect();
        return Err(CliError::with_details(
            2,
            "Invalid path.".to_string(),
            &details,
        ));
    }
    let Some(path_buf) = validation.ok.into_iter().next() else {
        return Err(CliError::with_details(
            2,
            format!("File or directory not found: {}", args.path),
            &["Hint: Check the path exists, or run from the correct directory."],
        ));
    };
    let path = path_buf.as_path();

    #[cfg(feature = "protect")]
    if let Err(msg) =
        crate::protect::check_protection(path, "delete", args.force, args.reason.as_deref())
    {
        return Err(CliError::with_details(
            crate::util::EXIT_ACCESS_DENIED,
            format!("Protection check failed: {msg}"),
            &["Fix: Add --force with a reason, or update protect rules to allow this operation."],
        ));
    }

    if args.dry_run {
        ui_println!("DRY RUN: would delete {:?}", path);
        return Ok(());
    }

    if args.on_reboot {
        if let Err(e) = crate::windows::reboot_ops::schedule_delete_on_reboot(path) {
            return Err(CliError::new(
                crate::util::EXIT_ACCESS_DENIED,
                format!("Failed to schedule reboot delete: OS Error {e}"),
            ));
        }
        ui_println!(
            "Successfully scheduled deletion on next reboot: {}",
            args.path
        );
        crate::security::audit::audit_log(
            "schedule_reboot_delete",
            &args.path,
            "cli",
            "",
            "success",
            args.reason.as_deref().unwrap_or(""),
        );
        return Err(CliError::new(crate::util::EXIT_REBOOT_SCHEDULED, ""));
    }

    let is_dir = path.is_dir();
    let delete = || {
        if is_dir {
            std::fs::remove_dir_all(path)
        } else {
            std::fs::remove_file(path)
        }
    };
    let res = delete();

    if let Err(e) = res {
        #[cfg(feature = "lock")]
        {
            if args.unlock {
                unlock_and_retry(
                    path,
                    args.force_kill,
                    args.yes,
                    &e,
                    "Lock query unavailable; fallback to plain delete failure",
                    delete,
                )?;
                ui_println!("Successfully unlocked and deleted: {}", args.path);
                crate::security::audit::audit_log(
                    "force_delete",
                    &args.path,
                    "cli",
                    format!("force_kill={}", args.force_kill),
                    "success",
                    args.reason.as_deref().unwrap_or(""),
                );
                return Ok(());
            }
            return Err(CliError::with_details(
                crate::util::EXIT_ACCESS_DENIED,
                format!("Delete failed: {e}."),
                &["Fix: Re-run with --unlock, or close the process locking the file."],
            ));
        }
        #[cfg(not(feature = "lock"))]
        {
            return Err(CliError::new(
                crate::util::EXIT_ACCESS_DENIED,
                format!("Delete failed: {e}"),
            ));
        }
    } else {
        ui_println!("Deleted: {}", args.path);
        if args.force && cfg!(feature = "protect") {
            crate::security::audit::audit_log(
                "protected_delete",
                &args.path,
                "cli",
                "bypassed",
                "success",
                args.reason.as_deref().unwrap_or(""),
            );
        }
    }
    Ok(())
}
