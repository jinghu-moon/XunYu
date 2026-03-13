use super::*;

pub(super) fn cmd_add(args: AclAddCmd) -> CliResult {
    let path = Path::new(&args.path);
    print_path_header(path);
    ui_println!("Add permission entry");

    let principal = if let Some(p) = args.principal {
        p
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

    let _ = acl::writer::lookup_account_sid(&principal).map_err(map_acl_err)?;

    let rights_mask = if let Some(r) = args.rights {
        acl::parse::parse_rights(&r).map_err(map_acl_err)?
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

    let ace_type = if let Some(t) = args.ace_type {
        acl::parse::parse_ace_type(&t).map_err(map_acl_err)?
    } else if !can_interact() {
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

    let inheritance = if let Some(inh) = args.inherit {
        acl::parse::parse_inheritance(&inh).map_err(map_acl_err)?
    } else if !path.is_dir() || !can_interact() {
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
    };

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

    if !prompt_confirm("Confirm add?", true, args.yes)? {
        ui_println!("Cancelled");
        return Ok(());
    }

    acl::writer::add_rule(
        path,
        &principal,
        rights_mask,
        ace_type.clone(),
        inheritance,
        PropagationFlags::NONE,
    )
    .map_err(map_acl_err)?;

    ui_println!("Added");

    let cfg = load_acl_runtime_config();
    let audit = audit_log(&cfg);
    audit
        .append(&AuditEntry::ok(
            "AddPermission",
            path.to_string_lossy(),
            format!(
                "Principal={principal} Rights={} Type={ace_type} Inherit={inheritance}",
                acl::types::rights_short(rights_mask)
            ),
        ))
        .map_err(map_acl_err)?;

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
