use super::*;

pub(super) fn cmd_batch(args: AclBatchCmd) -> CliResult {
    let paths: Vec<PathBuf> = if let Some(file) = args.file.as_deref() {
        let raw = std::fs::read_to_string(file)
            .map_err(|e| CliError::new(1, format!("Failed to read file: {e}")))?;
        raw.lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(PathBuf::from)
            .collect()
    } else if let Some(list) = args.paths.as_deref() {
        list.split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .collect()
    } else {
        return Err(CliError::with_details(
            2,
            "batch requires --file or --paths".to_string(),
            &["Fix: Use `xun acl batch --file <txt>` or `--paths a,b,c`."],
        ));
    };

    if paths.is_empty() {
        return Err(CliError::new(2, "No paths provided."));
    }

    if !prompt_confirm(
        &format!("Batch execute '{}' on {} paths?", args.action, paths.len()),
        false,
        args.yes,
    )? {
        ui_println!("Cancelled");
        return Ok(());
    }

    let cfg = load_acl_runtime_config();
    let audit = audit_log(&cfg);
    let export_dir = args
        .output
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| cfg.export_path.clone());

    let bar = if runtime::is_quiet() {
        ProgressBar::hidden()
    } else {
        let b = ProgressBar::new(paths.len() as u64);
        b.set_style(
            ProgressStyle::with_template(
                "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>4}/{len:4}  {msg}",
            )
            .unwrap_or_else(|_| ProgressStyle::default_bar()),
        );
        b
    };

    let mut errors: Vec<(PathBuf, String)> = Vec::new();

    for path in &paths {
        bar.set_message(acl::parse::truncate(&path.to_string_lossy(), 40));

        let result: anyhow::Result<()> = match args.action.to_lowercase().replace('-', "").as_str()
        {
            "repair" => acl::repair::force_repair(path, &cfg.cfg, true).and_then(|s| {
                if s.total_fail() > 0 {
                    anyhow::bail!("{}", s.summary());
                }
                Ok(())
            }),
            "backup" => acl::reader::get_acl(path).and_then(|snap| {
                let leaf = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "root".into());
                let dest = acl::export::backup_filename(&export_dir, &leaf);
                acl::export::backup_acl(&snap, &dest)
            }),
            "orphans" => acl::orphan::scan_orphans(path, true, &cfg.cfg).map(|_| ()),
            "inheritreset" | "resetinherit" => {
                acl::writer::set_access_rule_protection(path, false, true)
            }
            other => Err(anyhow::anyhow!("unknown batch action '{other}'")),
        };

        if let Err(e) = result {
            errors.push((path.clone(), format!("{e:#}")));
        }
        bar.inc(1);
    }

    bar.finish_with_message("done");

    ui_println!(
        "Success: {}  Failed: {}",
        paths.len() - errors.len(),
        errors.len()
    );

    if !errors.is_empty() {
        for (p, e) in errors.iter().take(5) {
            ui_println!("  {} : {}", p.display(), e);
        }
        if errors.len() > 5 {
            ui_println!("  ... and {} more", errors.len() - 5);
        }

        let dest = acl::export::error_csv_filename(&export_dir, &args.action);
        let mut wtr =
            csv::Writer::from_path(&dest).map_err(|e| map_acl_err(anyhow::Error::new(e)))?;
        wtr.write_record(["Path", "Error"])
            .map_err(|e| map_acl_err(anyhow::Error::new(e)))?;
        for (p, e) in &errors {
            wtr.write_record([&p.to_string_lossy().into_owned(), e.as_str()])
                .map_err(|e| map_acl_err(anyhow::Error::new(e)))?;
        }
        wtr.flush()
            .map_err(|e| map_acl_err(anyhow::Error::new(e)))?;
        ui_println!("Errors exported to {}", dest.display());
    }

    let batch_entry = if errors.is_empty() {
        AuditEntry::ok(
            "Batch",
            paths
                .first()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            format!(
                "Action={} Total={} Errors={}",
                args.action,
                paths.len(),
                errors.len()
            ),
        )
    } else {
        AuditEntry::fail(
            "Batch",
            paths
                .first()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            format!(
                "Action={} Total={} Errors={}",
                args.action,
                paths.len(),
                errors.len()
            ),
            "partial failure",
        )
    };
    audit.append(&batch_entry).map_err(map_acl_err)?;

    Ok(())
}

pub(super) fn cmd_backup(args: AclBackupCmd) -> CliResult {
    let path = Path::new(&args.path);
    let cfg = load_acl_runtime_config();
    let snap = acl::reader::get_acl(path).map_err(map_acl_err)?;

    let dest = if let Some(p) = args.output {
        PathBuf::from(p)
    } else {
        let leaf = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "root".into());
        acl::export::backup_filename(&cfg.export_path, &leaf)
    };

    acl::export::backup_acl(&snap, &dest).map_err(map_acl_err)?;
    ui_println!(
        "Backed up {} entries -> {}",
        snap.entries.len(),
        dest.display()
    );

    let audit = audit_log(&cfg);
    audit
        .append(&AuditEntry::ok(
            "BackupAcl",
            path.to_string_lossy(),
            format!("Dest={} Entries={}", dest.display(), snap.entries.len()),
        ))
        .map_err(map_acl_err)?;

    Ok(())
}

pub(super) fn cmd_restore(args: AclRestoreCmd) -> CliResult {
    let path = Path::new(&args.path);
    let from = Path::new(&args.from);

    print_path_header(path);
    ui_println!("Restore ACL from {}", from.display());
    ui_println!("Target ACL will be overwritten.");

    if !prompt_confirm("Confirm restore?", false, args.yes)? {
        ui_println!("Cancelled");
        return Ok(());
    }

    acl::export::restore_acl(from, path).map_err(map_acl_err)?;
    ui_println!("ACL restored");

    let cfg = load_acl_runtime_config();
    let audit = audit_log(&cfg);
    audit
        .append(&AuditEntry::ok(
            "RestoreAcl",
            path.to_string_lossy(),
            format!("Source={}", from.display()),
        ))
        .map_err(map_acl_err)?;

    Ok(())
}
