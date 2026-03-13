use super::*;

pub(super) fn cmd_orphans(args: AclOrphansCmd) -> CliResult {
    let path = Path::new(&args.path);
    let cfg = load_acl_runtime_config();
    let audit = audit_log(&cfg);

    let bar = if runtime::is_quiet() {
        ProgressBar::hidden()
    } else {
        let b = ProgressBar::new_spinner();
        b.set_style(
            ProgressStyle::with_template("{spinner:.cyan} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner()),
        );
        b
    };
    bar.set_message(format!("Scanning orphans... {}", path.display()));
    bar.enable_steady_tick(std::time::Duration::from_millis(80));

    let orphans = acl::orphan::scan_orphans(path, args.recursive, &cfg.cfg).map_err(map_acl_err)?;
    bar.finish_and_clear();

    if orphans.is_empty() {
        ui_println!("No orphan SIDs found.");
        audit
            .append(&AuditEntry::ok(
                "ScanOrphans",
                path.to_string_lossy(),
                "Found=0",
            ))
            .map_err(map_acl_err)?;
        return Ok(());
    }

    ui_println!("Found {} orphan SIDs", orphans.len());
    for o in orphans.iter().take(20) {
        ui_println!(
            "  SID: {}  {}  {}",
            o.ace.raw_sid,
            o.ace.ace_type,
            acl::parse::truncate_left(&o.path.to_string_lossy(), 48)
        );
    }
    if orphans.len() > 20 {
        ui_println!("  ... {} more", orphans.len() - 20);
    }

    audit
        .append(&AuditEntry::ok(
            "ScanOrphans",
            path.to_string_lossy(),
            format!("Found={}", orphans.len()),
        ))
        .map_err(map_acl_err)?;

    let export_path = args.output.as_ref().map(PathBuf::from).unwrap_or_else(|| {
        cfg.export_path.join(format!(
            "ACLOrphans_{}.csv",
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        ))
    });

    match args.action.as_str() {
        "export" => {
            let n = acl::export::export_orphans_csv(&orphans, &export_path).map_err(map_acl_err)?;
            ui_println!("Exported {} rows to {}", n, export_path.display());
        }
        "delete" => {
            if !prompt_confirm(
                &format!("Delete {} orphan SIDs?", orphans.len()),
                false,
                args.yes,
            )? {
                ui_println!("Cancelled");
                return Ok(());
            }
            let (ok, fail) = acl::orphan::purge_orphan_sids(&orphans);
            ui_println!("Removed {} / Failed {}", ok, fail);
            audit
                .append(&AuditEntry::ok(
                    "PurgeOrphans",
                    path.to_string_lossy(),
                    format!("Cleaned={ok} Failed={fail}"),
                ))
                .map_err(map_acl_err)?;
        }
        "both" => {
            let n = acl::export::export_orphans_csv(&orphans, &export_path).map_err(map_acl_err)?;
            ui_println!("Exported {} rows to {}", n, export_path.display());
            if !prompt_confirm(
                &format!("Delete {} orphan SIDs?", orphans.len()),
                false,
                args.yes,
            )? {
                ui_println!("Delete skipped");
                return Ok(());
            }
            let (ok, fail) = acl::orphan::purge_orphan_sids(&orphans);
            ui_println!("Removed {} / Failed {}", ok, fail);
            audit
                .append(&AuditEntry::ok(
                    "PurgeOrphans",
                    path.to_string_lossy(),
                    format!("Cleaned={ok} Failed={fail}"),
                ))
                .map_err(map_acl_err)?;
        }
        _ => {
            ui_println!("Use --action export|delete|both for follow-up actions.");
        }
    }

    Ok(())
}

pub(super) fn cmd_repair(args: AclRepairCmd) -> CliResult {
    let path = Path::new(&args.path);
    let cfg = load_acl_runtime_config();
    let audit = audit_log(&cfg);

    print_path_header(path);
    ui_println!("Force repair (take ownership + grant FullControl)");
    ui_println!("This operation is destructive and cannot be undone.");

    if !prompt_confirm("Confirm repair?", false, args.yes)? {
        ui_println!("Cancelled");
        return Ok(());
    }

    let stats = match acl::repair::force_repair(path, &cfg.cfg, runtime::is_quiet()) {
        Ok(s) => s,
        Err(e) => {
            if args.export_errors {
                let dest = acl::export::error_csv_filename(&cfg.export_path, "repair");
                let mut wtr = csv::Writer::from_path(&dest)
                    .map_err(|e| map_acl_err(anyhow::Error::new(e)))?;
                wtr.write_record(["Path", "Error"])
                    .map_err(|e| map_acl_err(anyhow::Error::new(e)))?;
                let msg = format!("{e:#}");
                wtr.write_record([&path.to_string_lossy(), msg.as_str()])
                    .map_err(|e| map_acl_err(anyhow::Error::new(e)))?;
                wtr.flush()
                    .map_err(|e| map_acl_err(anyhow::Error::new(e)))?;
                ui_println!("Exported 1 errors to {}", dest.display());
            }
            return Err(map_acl_err(e));
        }
    };

    ui_println!("");
    ui_println!("{}", stats.summary());

    let has_fail = stats.total_fail() > 0;
    if has_fail {
        ui_println!("Failures (first 8):");
        for (p, e) in stats.owner_fail.iter().chain(stats.acl_fail.iter()).take(8) {
            ui_println!("  {} : {}", p.display(), e);
        }
        let rest = stats.total_fail().saturating_sub(8);
        if rest > 0 {
            ui_println!("  ... {} more", rest);
        }

        if args.export_errors {
            let dest = acl::export::error_csv_filename(&cfg.export_path, "repair");
            let n = acl::export::export_repair_errors_csv(&stats, &dest).map_err(map_acl_err)?;
            ui_println!("Exported {} errors to {}", n, dest.display());
        }
    }

    let repair_entry = if has_fail {
        AuditEntry::fail(
            "ForceRepair",
            path.to_string_lossy(),
            stats.summary(),
            "partial failure",
        )
    } else {
        AuditEntry::ok("ForceRepair", path.to_string_lossy(), stats.summary())
    };
    audit.append(&repair_entry).map_err(map_acl_err)?;

    Ok(())
}
