use super::*;

pub(super) fn cmd_view(args: AclViewCmd) -> CliResult {
    let path = Path::new(&args.path);
    let snap = acl::reader::get_acl(path).map_err(map_acl_err)?;

    print_path_header(path);
    print_acl_summary(&snap);

    if args.detail {
        ui_println!("");
        for (i, e) in snap.entries.iter().enumerate() {
            ui_println!(
                "#{} {} {}",
                i + 1,
                match e.ace_type {
                    AceType::Allow => "Allow",
                    AceType::Deny => "Deny",
                },
                e.principal
            );
            ui_println!(
                "  Rights: {} | Inherit: {} | Prop: {} | Source: {}{}",
                e.rights_display(),
                e.inheritance,
                e.propagation,
                if e.is_inherited {
                    "inherited"
                } else {
                    "explicit"
                },
                if e.is_orphan { " | orphan" } else { "" }
            );
            ui_println!("  SID: {}", e.raw_sid);
        }
        ui_println!("");
    } else {
        ui_println!("");
        ui_println!(
            "{: <5}  {: <8}  {: <40}  {: <16}  {}",
            "Type",
            "Source",
            "Principal",
            "Rights",
            "Orphan"
        );
        ui_println!("{}", "-".repeat(80));
        for e in &snap.entries {
            let t = match e.ace_type {
                AceType::Allow => "Allow",
                AceType::Deny => "Deny",
            };
            let src = if e.is_inherited {
                "inherit"
            } else {
                "explicit"
            };
            let prin = acl::parse::truncate(&e.principal, 40);
            let orph = if e.is_orphan { "yes" } else { "" };
            ui_println!(
                "{: <5}  {: <8}  {: <40}  {: <16}  {}",
                t,
                src,
                prin,
                e.rights_display(),
                orph
            );
        }
        ui_println!("");
    }

    if let Some(dest) = args.export {
        let dest = PathBuf::from(dest);
        let n = acl::export::export_acl_csv(&snap, &dest).map_err(map_acl_err)?;
        ui_println!("Exported {} entries to {}", n, dest.display());
    }

    Ok(())
}

pub(super) fn cmd_diff(args: AclDiffCmd) -> CliResult {
    let path = Path::new(&args.path);
    let reference = Path::new(&args.reference);

    let snap_a = acl::reader::get_acl(path).map_err(map_acl_err)?;
    let snap_b = acl::reader::get_acl(reference).map_err(map_acl_err)?;

    let diff = acl::diff::diff_acl(&snap_a, &snap_b);

    print_path_header(path);
    ui_println!("Reference: {}", reference.display());

    if diff.owner_diff.is_some() {
        ui_println!("Owner differs");
    }
    if diff.inherit_diff.is_some() {
        ui_println!("Inheritance differs");
    }

    ui_println!("Only in A: {}", diff.only_in_a.len());
    ui_println!("Only in B: {}", diff.only_in_b.len());
    ui_println!("Common: {}", diff.common_count);

    if let Some(dest) = args.output {
        let dest = PathBuf::from(dest);
        let n = acl::export::export_diff_csv(&diff, &dest).map_err(map_acl_err)?;
        ui_println!("Exported {} rows to {}", n, dest.display());
    }

    let cfg = load_acl_runtime_config();
    let audit = audit_log(&cfg);
    audit
        .append(&AuditEntry::ok(
            "Diff",
            path.to_string_lossy(),
            format!(
                "Ref={} OnlyA={} OnlyB={}",
                reference.display(),
                diff.only_in_a.len(),
                diff.only_in_b.len()
            ),
        ))
        .map_err(map_acl_err)?;

    Ok(())
}

pub(super) fn cmd_effective(args: AclEffectiveCmd) -> CliResult {
    let path = Path::new(&args.path);
    let snap = acl::reader::get_acl(path).map_err(map_acl_err)?;

    let (sids, label) = if let Some(u) = args.user.as_deref() {
        let sid = acl::effective::resolve_user_sid(u).map_err(map_acl_err)?;
        (vec![sid], format!("User: {u}"))
    } else {
        let sids = acl::effective::get_current_user_sids();
        (sids, format!("User: {}", acl::audit::current_user()))
    };

    let ea = acl::effective::compute_effective_access(&snap, &sids);

    ui_println!("\n{label}");
    ui_println!("Path: {}", path.display());
    ui_println!("");
    ui_println!("{: <12}  {}", "Right", "Result");
    ui_println!("{}", "-".repeat(24));

    for (lbl, state) in [
        ("Read", &ea.read),
        ("Write", &ea.write),
        ("Execute", &ea.execute),
        ("Delete", &ea.delete),
        ("ChangePerms", &ea.change_perms),
        ("TakeOwnership", &ea.take_ownership),
    ] {
        let result = match state {
            acl::types::TriState::Allow => "Allow",
            acl::types::TriState::Deny => "Deny",
            acl::types::TriState::NoRule => "NoRule",
        };
        ui_println!("{: <12}  {}", lbl, result);
    }

    ui_println!("");
    ui_println!(
        "Allow: {:#010x}  Deny: {:#010x}  Effective: {:#010x}",
        ea.allow_mask,
        ea.deny_mask,
        ea.effective_mask
    );

    if args.user.is_some() {
        ui_println!("Note: specified user only; group memberships not included.");
    }
    ui_println!("");

    Ok(())
}
