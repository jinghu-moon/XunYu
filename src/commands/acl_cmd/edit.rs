use super::*;
use rayon::prelude::*;
use std::time::{Duration, Instant};

fn acl_timing_enabled() -> bool {
    std::env::var("XUN_ACL_TIMING")
        .ok()
        .map(|v| {
            let v = v.trim().to_ascii_lowercase();
            matches!(v.as_str(), "1" | "true" | "yes" | "on")
        })
        .unwrap_or(false)
}

fn update_min_max(min: &mut Option<Duration>, max: &mut Option<Duration>, value: Duration) {
    match min {
        Some(current) => {
            if value < *current {
                *current = value;
            }
        }
        None => {
            *min = Some(value);
        }
    }
    match max {
        Some(current) => {
            if value > *current {
                *current = value;
            }
        }
        None => {
            *max = Some(value);
        }
    }
}

pub(super) fn cmd_add(args: AclAddCmd) -> CliResult {
    let timing_enabled = acl_timing_enabled();
    let total_start = Instant::now();
    let paths_start = Instant::now();
    let mut paths: Vec<PathBuf> = Vec::new();
    if let Some(file) = args.file.as_deref() {
        let raw = std::fs::read_to_string(file)
            .map_err(|e| CliError::new(1, format!("Failed to read file: {e}")))?;
        paths.extend(
            raw.lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .map(PathBuf::from),
        );
    }
    if let Some(list) = args.paths.as_deref() {
        paths.extend(
            list.split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(PathBuf::from),
        );
    }
    if let Some(path) = args.path.as_deref() {
        paths.push(PathBuf::from(path));
    }
    paths.sort();
    paths.dedup();
    if paths.is_empty() {
        return Err(CliError::with_details(
            2,
            "add requires --path or --file or --paths".to_string(),
            &[
                "Fix: Use `xun acl add -p <path>` or `--file <txt>` or `--paths a,b,c`.",
            ],
        ));
    }
    let paths_elapsed = paths_start.elapsed();

    let is_batch = args.file.is_some() || args.paths.is_some();
    if !is_batch {
        if let Some(path) = paths.first() {
            print_path_header(path);
        }
    }
    ui_println!("Add permission entry");
    if is_batch {
        ui_println!("Paths: {}", paths.len());
    }

    let principal = if let Some(p) = args.principal {
        p
    } else if is_batch {
        return Err(CliError::with_details(
            2,
            "add requires --principal in batch mode.".to_string(),
            &["Fix: Provide --principal with --file/--paths."],
        ));
    } else {
        ensure_interactive("Add")?;
        Input::new()
            .with_prompt("Principal (e.g. BUILTIN\\Users or DOMAIN\\alice)")
            .validate_with(|s: &String| {
                acl::writer::lookup_account_sid(s)
                    .map(|_| ())
                    .map_err(|_| format!("Invalid principal: {s}"))
            })
            .interact_text()
            .map_err(|e| CliError::new(1, format!("Failed to read principal: {e}")))?
    };

    let principal_start = Instant::now();
    let principal_sid = acl::writer::lookup_account_sid(&principal).map_err(map_acl_err)?;
    let principal_elapsed = principal_start.elapsed();

    let rights_start = Instant::now();
    let rights_mask = if let Some(r) = args.rights {
        acl::parse::parse_rights(&r).map_err(map_acl_err)?
    } else if is_batch {
        return Err(CliError::with_details(
            2,
            "add requires --rights in batch mode.".to_string(),
            &["Fix: Provide --rights with --file/--paths."],
        ));
    } else {
        ensure_interactive("Add")?;
        let descs: Vec<String> = RIGHTS_TABLE
            .iter()
            .map(|&(_, s, d)| format!("{:<16}  {}", s, d))
            .collect();
        let idx = Select::new()
            .with_prompt("Rights")
            .items(&descs)
            .default(0)
            .interact()
            .map_err(|e| CliError::new(1, format!("Failed to select rights: {e}")))?;
        RIGHTS_TABLE[idx].0
    };
    let rights_elapsed = rights_start.elapsed();

    let ace_start = Instant::now();
    let ace_type = if let Some(t) = args.ace_type {
        acl::parse::parse_ace_type(&t).map_err(map_acl_err)?
    } else if is_batch || !can_interact() {
        AceType::Allow
    } else {
        let ch = ["Allow", "Deny"];
        let idx = Select::new()
            .with_prompt("Access type")
            .items(&ch)
            .default(0)
            .interact()
            .map_err(|e| CliError::new(1, format!("Failed to select access type: {e}")))?;
        if idx == 0 {
            AceType::Allow
        } else {
            AceType::Deny
        }
    };
    let ace_elapsed = ace_start.elapsed();

    let inherit_start = Instant::now();
    let inheritance = if let Some(inh) = args.inherit {
        acl::parse::parse_inheritance(&inh).map_err(map_acl_err)?
    } else if is_batch {
        InheritanceFlags::NONE
    } else {
        let is_dir = paths
            .first()
            .map(|p| p.is_dir())
            .unwrap_or(false);
        if !is_dir || !can_interact() {
            InheritanceFlags::NONE
        } else {
            let ch = [
                "BothInherit    - children dirs and files",
                "ContainerOnly  - directories only",
                "ObjectOnly     - files only",
                "None           - no inheritance",
            ];
            let idx = Select::new()
                .with_prompt("Inheritance")
                .items(&ch)
                .default(0)
                .interact()
                .map_err(|e| CliError::new(1, format!("Failed to select inheritance: {e}")))?;
            match idx {
                0 => InheritanceFlags::BOTH,
                1 => InheritanceFlags::CONTAINER_INHERIT,
                2 => InheritanceFlags::OBJECT_INHERIT,
                _ => InheritanceFlags::NONE,
            }
        }
    };
    let inherit_elapsed = inherit_start.elapsed();

    ui_println!(
        "Will add: {} {}  Rights {}  Inherit {}",
        match ace_type {
            AceType::Allow => "Allow",
            AceType::Deny => "Deny",
        },
        principal,
        acl::types::rights_short(rights_mask),
        inheritance
    );

    let confirm = if is_batch {
        format!("Confirm add on {} paths?", paths.len())
    } else {
        "Confirm add?".to_string()
    };
    let confirm_start = Instant::now();
    let confirmed = prompt_confirm(&confirm, true, args.yes)?;
    let confirm_elapsed = confirm_start.elapsed();
    if !confirmed {
        ui_println!("Cancelled");
        return Ok(());
    }

    let cfg = load_acl_runtime_config();
    let audit = audit_log(&cfg);
    let mut add_elapsed = Duration::from_millis(0);
    let mut add_min: Option<Duration> = None;
    let mut add_max: Option<Duration> = None;
    let mut add_count = 0usize;
    let mut audit_elapsed = Duration::from_millis(0);
    let mut audit_min: Option<Duration> = None;
    let mut audit_max: Option<Duration> = None;
    let mut audit_flushes = 0usize;
    let mut audit_rotate_elapsed = Duration::from_millis(0);
    let mut audit_rotate_count = 0usize;
    let audit_batch_size = 64usize;

    if is_batch {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(cfg.cfg.throttle_limit.max(1))
            .build()
            .map_err(|e| CliError::new(1, format!("Failed to create thread pool: {e}")))?;

        let results: Vec<(PathBuf, anyhow::Result<()>, Duration)> = pool.install(|| {
            paths
                .par_iter()
                .map(|path| {
                    let add_start = Instant::now();
                    let result = acl::writer::add_rule_with_sid_bytes(
                        path,
                        &principal_sid,
                        rights_mask,
                        ace_type.clone(),
                        inheritance,
                        PropagationFlags::NONE,
                    );
                    let add_dur = add_start.elapsed();
                    (path.clone(), result, add_dur)
                })
                .collect()
        });

        let mut failures: Vec<(PathBuf, anyhow::Error)> = Vec::new();
        let mut success_entries: Vec<AuditEntry> = Vec::new();

        for (path, result, add_dur) in results {
            add_elapsed += add_dur;
            add_count += 1;
            update_min_max(&mut add_min, &mut add_max, add_dur);
            match result {
                Ok(()) => {
                    success_entries.push(AuditEntry::ok(
                        "AddPermission",
                        path.to_string_lossy(),
                        format!(
                            "Principal={principal} Rights={} Type={ace_type} Inherit={inheritance}",
                            acl::types::rights_short(rights_mask)
                        ),
                    ));
                }
                Err(err) => failures.push((path, err)),
            }
        }

        for chunk in success_entries.chunks(audit_batch_size) {
            let audit_start = Instant::now();
            audit.append_many_no_rotate(chunk).map_err(map_acl_err)?;
            let audit_dur = audit_start.elapsed();
            audit_elapsed += audit_dur;
            audit_flushes += 1;
            update_min_max(&mut audit_min, &mut audit_max, audit_dur);
        }

        let rotate_start = Instant::now();
        audit.rotate_if_needed().map_err(map_acl_err)?;
        audit_rotate_elapsed += rotate_start.elapsed();
        audit_rotate_count += 1;

        if !failures.is_empty() {
            let (first_path, first_err) = failures.remove(0);
            let mut err = map_acl_err(first_err);
            err.details.push(format!(
                "Batch failed: {} errors out of {}.",
                failures.len() + 1,
                paths.len()
            ));
            err.details.push(format!("First failed path: {}", first_path.display()));
            return Err(err);
        }
    } else {
        let mut pending: Vec<AuditEntry> = Vec::new();
        for path in &paths {
            let add_start = Instant::now();
            let result = acl::writer::add_rule_with_sid_bytes(
                path,
                &principal_sid,
                rights_mask,
                ace_type.clone(),
                inheritance,
                PropagationFlags::NONE,
            );
            let add_dur = add_start.elapsed();
            add_elapsed += add_dur;
            add_count += 1;
            update_min_max(&mut add_min, &mut add_max, add_dur);
            if let Err(err) = result {
                if !pending.is_empty() {
                    audit.append_many_no_rotate(&pending).map_err(map_acl_err)?;
                }
                let rotate_start = Instant::now();
                audit.rotate_if_needed().map_err(map_acl_err)?;
                audit_rotate_elapsed += rotate_start.elapsed();
                audit_rotate_count += 1;
                return Err(map_acl_err(err));
            }

            pending.push(AuditEntry::ok(
                "AddPermission",
                path.to_string_lossy(),
                format!(
                    "Principal={principal} Rights={} Type={ace_type} Inherit={inheritance}",
                    acl::types::rights_short(rights_mask)
                ),
            ));

            if pending.len() >= audit_batch_size {
                let audit_start = Instant::now();
                audit.append_many_no_rotate(&pending).map_err(map_acl_err)?;
                let audit_dur = audit_start.elapsed();
                audit_elapsed += audit_dur;
                audit_flushes += 1;
                update_min_max(&mut audit_min, &mut audit_max, audit_dur);
                pending.clear();
            }
        }

        if !pending.is_empty() {
            let audit_start = Instant::now();
            audit.append_many_no_rotate(&pending).map_err(map_acl_err)?;
            let audit_dur = audit_start.elapsed();
            audit_elapsed += audit_dur;
            audit_flushes += 1;
            update_min_max(&mut audit_min, &mut audit_max, audit_dur);
        }
        let rotate_start = Instant::now();
        audit.rotate_if_needed().map_err(map_acl_err)?;
        audit_rotate_elapsed += rotate_start.elapsed();
        audit_rotate_count += 1;
    }

    if is_batch {
        ui_println!("Added {} entries", paths.len());
    } else {
        ui_println!("Added");
    }

    if timing_enabled {
        let add_total_us = add_elapsed.as_micros();
        let audit_total_us = audit_elapsed.as_micros();
        let add_avg_us = if add_count == 0 {
            0
        } else {
            add_total_us / add_count as u128
        };
        let audit_avg_us = if audit_flushes == 0 {
            0
        } else {
            audit_total_us / audit_flushes as u128
        };
        eprintln!(
            "perf: acl_add paths={} batch={} paths_us={} principal_us={} rights_us={} ace_us={} inherit_us={} confirm_us={} add_us={} add_min_us={} add_max_us={} add_avg_us={} audit_us={} audit_flushes={} audit_min_us={} audit_max_us={} audit_avg_us={} audit_rotate_us={} audit_rotate_count={} total_us={}",
            paths.len(),
            if is_batch { "yes" } else { "no" },
            paths_elapsed.as_micros(),
            principal_elapsed.as_micros(),
            rights_elapsed.as_micros(),
            ace_elapsed.as_micros(),
            inherit_elapsed.as_micros(),
            confirm_elapsed.as_micros(),
            add_total_us,
            add_min.map(|d| d.as_micros()).unwrap_or(0),
            add_max.map(|d| d.as_micros()).unwrap_or(0),
            add_avg_us,
            audit_total_us,
            audit_flushes,
            audit_min.map(|d| d.as_micros()).unwrap_or(0),
            audit_max.map(|d| d.as_micros()).unwrap_or(0),
            audit_avg_us,
            audit_rotate_elapsed.as_micros(),
            audit_rotate_count,
            total_start.elapsed().as_micros()
        );
    }

    Ok(())
}

pub(super) fn cmd_remove(args: AclRemoveCmd) -> CliResult {
    let path = Path::new(&args.path);
    let snap = acl::reader::get_acl(path).map_err(map_acl_err)?;

    let explicit: Vec<_> = snap
        .entries
        .iter()
        .filter(|e| !e.is_inherited)
        .cloned()
        .collect();
    if explicit.is_empty() {
        ui_println!("No explicit ACE entries to remove.");
        return Ok(());
    }

    let has_filters = args.principal.is_some()
        || args.raw_sid.is_some()
        || args.rights.is_some()
        || args.ace_type.is_some();

    if has_filters {
        if args.principal.is_none() && args.raw_sid.is_none() {
            return Err(CliError::with_details(
                2,
                "remove requires --principal or --raw-sid for non-interactive mode.".to_string(),
                &[
                    "Fix: Provide --principal or --raw-sid (optional: --rights, --ace-type).",
                ],
            ));
        }

        let rights_mask = if let Some(r) = args.rights.as_deref() {
            Some(acl::parse::parse_rights(r).map_err(map_acl_err)?)
        } else {
            None
        };

        let ace_type = if let Some(t) = args.ace_type.as_deref() {
            Some(acl::parse::parse_ace_type(t).map_err(map_acl_err)?)
        } else {
            None
        };

        let to_remove: Vec<_> = explicit
            .iter()
            .filter(|e| {
                if let Some(principal) = args.principal.as_deref() {
                    if !e.principal.eq_ignore_ascii_case(principal) {
                        return false;
                    }
                }
                if let Some(raw_sid) = args.raw_sid.as_deref() {
                    if !e.raw_sid.eq_ignore_ascii_case(raw_sid) {
                        return false;
                    }
                }
                if let Some(mask) = rights_mask {
                    if e.rights_mask != mask {
                        return false;
                    }
                }
                if let Some(ref ty) = ace_type {
                    if &e.ace_type != ty {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        if to_remove.is_empty() {
            ui_println!("No matching explicit ACE entries.");
            return Ok(());
        }

        print_path_header(path);
        ui_println!("Remove explicit permission entries");

        if !prompt_confirm("Confirm remove?", false, args.yes)? {
            ui_println!("Cancelled");
            return Ok(());
        }

        let removed = acl::writer::remove_rules(path, &to_remove).map_err(map_acl_err)?;
        ui_println!("Removed {} entries.", removed);

        let cfg = load_acl_runtime_config();
        let audit = audit_log(&cfg);
        audit
            .append(&AuditEntry::ok(
                "RemovePermission",
                path.to_string_lossy(),
                format!("Removed={removed}"),
            ))
            .map_err(map_acl_err)?;

        return Ok(());
    }

    ensure_interactive("Remove")?;

    print_path_header(path);
    ui_println!("Remove explicit permission entries");

    let labels: Vec<String> = explicit
        .iter()
        .map(|e| {
            format!(
                "{} | {} | {} | {}",
                e.principal,
                e.ace_type,
                e.rights_display(),
                e.raw_sid
            )
        })
        .collect();

    let selections = MultiSelect::new()
        .with_prompt("Select entries to remove")
        .items(&labels)
        .interact()
        .map_err(|e| CliError::new(1, format!("Failed to select entries: {e}")))?;

    if selections.is_empty() {
        ui_println!("No entries selected.");
        return Ok(());
    }

    let to_remove: Vec<_> = selections
        .into_iter()
        .map(|i| explicit[i].clone())
        .collect();

    let removed = acl::writer::remove_rules(path, &to_remove).map_err(map_acl_err)?;
    ui_println!("Removed {} entries.", removed);

    let cfg = load_acl_runtime_config();
    let audit = audit_log(&cfg);
    audit
        .append(&AuditEntry::ok(
            "RemovePermission",
            path.to_string_lossy(),
            format!("Removed={removed}"),
        ))
        .map_err(map_acl_err)?;

    Ok(())
}

pub(super) fn cmd_purge(args: AclPurgeCmd) -> CliResult {
    let path = Path::new(&args.path);
    let principal = if let Some(p) = args.principal {
        p
    } else {
        ensure_interactive("Purge")?;
        let snap = acl::reader::get_acl(path).map_err(map_acl_err)?;
        let mut principals: Vec<String> = snap
            .entries
            .iter()
            .filter(|e| !e.is_inherited)
            .map(|e| e.principal.clone())
            .collect();
        principals.sort();
        principals.dedup();
        if principals.is_empty() {
            ui_println!("No explicit principals found.");
            return Ok(());
        }
        let idx = FuzzySelect::new()
            .with_prompt("Select principal to purge")
            .items(&principals)
            .default(0)
            .interact()
            .map_err(|e| CliError::new(1, format!("Failed to select principal: {e}")))?;
        principals[idx].clone()
    };

    if !prompt_confirm(
        &format!("Purge all explicit ACEs for '{principal}'?"),
        false,
        args.yes,
    )? {
        ui_println!("Cancelled");
        return Ok(());
    }

    let removed = acl::writer::purge_principal(path, &principal).map_err(map_acl_err)?;
    ui_println!("Purged {} entries for {}", removed, principal);

    let cfg = load_acl_runtime_config();
    let audit = audit_log(&cfg);
    audit
        .append(&AuditEntry::ok(
            "PurgePrincipal",
            path.to_string_lossy(),
            format!("Principal={principal} Removed={removed}"),
        ))
        .map_err(map_acl_err)?;

    Ok(())
}

pub(super) fn cmd_copy(args: AclCopyCmd) -> CliResult {
    let path = Path::new(&args.path);
    let reference = Path::new(&args.reference);

    print_path_header(path);
    ui_println!("Copy ACL from {}", reference.display());
    ui_println!("Target ACL will be overwritten.");

    if !prompt_confirm("Confirm copy?", false, args.yes)? {
        ui_println!("Cancelled");
        return Ok(());
    }

    acl::writer::copy_acl(reference, path).map_err(map_acl_err)?;

    ui_println!("ACL copied");

    let cfg = load_acl_runtime_config();
    let audit = audit_log(&cfg);
    audit
        .append(&AuditEntry::ok(
            "CopyAcl",
            path.to_string_lossy(),
            format!("Source={}", reference.display()),
        ))
        .map_err(map_acl_err)?;

    Ok(())
}

pub(super) fn cmd_inherit(args: AclInheritCmd) -> CliResult {
    let path = Path::new(&args.path);
    let snap = acl::reader::get_acl(path).map_err(map_acl_err)?;

    print_path_header(path);
    ui_println!(
        "Current inheritance: {}",
        if snap.is_protected {
            "disabled"
        } else {
            "enabled"
        }
    );

    let (break_it, preserve_copies) = if args.disable {
        (true, args.preserve)
    } else if args.enable {
        (false, false)
    } else {
        ensure_interactive("Inherit")?;
        if snap.is_protected {
            let choices = ["Restore inheritance", "Cancel"];
            let idx = Select::new()
                .with_prompt("Select action")
                .items(&choices)
                .default(0)
                .interact()
                .map_err(|e| CliError::new(1, format!("Failed to select: {e}")))?;
            if idx == 1 {
                ui_println!("Cancelled");
                return Ok(());
            }
            (false, false)
        } else {
            let choices = [
                "Break inheritance and keep inherited entries",
                "Break inheritance and remove inherited entries",
                "Cancel",
            ];
            let idx = Select::new()
                .with_prompt("Select action")
                .items(&choices)
                .default(0)
                .interact()
                .map_err(|e| CliError::new(1, format!("Failed to select: {e}")))?;
            match idx {
                0 => (true, true),
                1 => (true, false),
                _ => {
                    ui_println!("Cancelled");
                    return Ok(());
                }
            }
        }
    };

    acl::writer::set_access_rule_protection(path, break_it, preserve_copies)
        .map_err(map_acl_err)?;
    ui_println!(
        "Inheritance {}",
        if break_it { "disabled" } else { "enabled" }
    );

    let cfg = load_acl_runtime_config();
    let audit = audit_log(&cfg);
    audit
        .append(&AuditEntry::ok(
            "SetInheritance",
            path.to_string_lossy(),
            format!("Protected={break_it} Preserve={preserve_copies}"),
        ))
        .map_err(map_acl_err)?;

    Ok(())
}

pub(super) fn cmd_owner(args: AclOwnerCmd) -> CliResult {
    let path = Path::new(&args.path);
    let snap = acl::reader::get_acl(path).map_err(map_acl_err)?;

    print_path_header(path);
    ui_println!("Current owner: {}", snap.owner);

    let new_owner = if let Some(o) = args.set {
        o
    } else {
        ensure_interactive("Owner")?;
        Input::new()
            .with_prompt("New owner")
            .with_initial_text(&snap.owner)
            .validate_with(|s: &String| {
                acl::writer::lookup_account_sid(s)
                    .map(|_| ())
                    .map_err(|_| format!("Invalid owner: {s}"))
            })
            .interact_text()
            .map_err(|e| CliError::new(1, format!("Failed to read owner: {e}")))?
    };

    if new_owner == snap.owner {
        ui_println!("Owner unchanged.");
        return Ok(());
    }

    if !prompt_confirm("Confirm change owner?", true, args.yes)? {
        ui_println!("Cancelled");
        return Ok(());
    }

    acl::writer::set_owner(path, &new_owner).map_err(map_acl_err)?;
    ui_println!("Owner updated");

    let cfg = load_acl_runtime_config();
    let audit = audit_log(&cfg);
    audit
        .append(&AuditEntry::ok(
            "SetOwner",
            path.to_string_lossy(),
            format!("Old={} New={new_owner}", snap.owner),
        ))
        .map_err(map_acl_err)?;

    Ok(())
}
