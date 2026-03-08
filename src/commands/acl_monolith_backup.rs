use std::path::{Path, PathBuf};

use dialoguer::{Confirm, FuzzySelect, Input, MultiSelect, Select};
use indicatif::{ProgressBar, ProgressStyle};

use crate::acl;
use crate::acl::audit::{AuditEntry, AuditLog};
use crate::acl::error::AclError;
use crate::acl::types::{AceType, InheritanceFlags, PropagationFlags, RIGHTS_TABLE};
use crate::cli::{
    AclAddCmd, AclAuditCmd, AclBackupCmd, AclBatchCmd, AclCmd, AclConfigCmd, AclCopyCmd,
    AclDiffCmd, AclEffectiveCmd, AclInheritCmd, AclOrphansCmd, AclOwnerCmd, AclPurgeCmd,
    AclRemoveCmd, AclRepairCmd, AclRestoreCmd, AclSubCommand, AclViewCmd,
};
use crate::config::{AclConfig, load_config, save_config};
use crate::output::{CliError, CliResult, apply_pretty_table_style, can_interact, print_table};
use crate::runtime;
use comfy_table::{Attribute, Cell, Color, Table};

pub(crate) fn cmd_acl(args: AclCmd) -> CliResult {
    match args.cmd {
        AclSubCommand::View(a) => cmd_view(a),
        AclSubCommand::Add(a) => cmd_add(a),
        AclSubCommand::Remove(a) => cmd_remove(a),
        AclSubCommand::Purge(a) => cmd_purge(a),
        AclSubCommand::Diff(a) => cmd_diff(a),
        AclSubCommand::Batch(a) => cmd_batch(a),
        AclSubCommand::Effective(a) => cmd_effective(a),
        AclSubCommand::Copy(a) => cmd_copy(a),
        AclSubCommand::Backup(a) => cmd_backup(a),
        AclSubCommand::Restore(a) => cmd_restore(a),
        AclSubCommand::Inherit(a) => cmd_inherit(a),
        AclSubCommand::Owner(a) => cmd_owner(a),
        AclSubCommand::Orphans(a) => cmd_orphans(a),
        AclSubCommand::Repair(a) => cmd_repair(a),
        AclSubCommand::Audit(a) => cmd_audit(a),
        AclSubCommand::Config(a) => cmd_config(a),
    }
}

struct AclRuntimeConfig {
    cfg: AclConfig,
    audit_log_path: PathBuf,
    export_path: PathBuf,
}

fn normalize_acl_config(cfg: AclConfig) -> AclConfig {
    let defaults = AclConfig::default();
    let mut out = cfg;
    if out.throttle_limit == 0 {
        out.throttle_limit = defaults.throttle_limit;
    }
    if out.chunk_size == 0 {
        out.chunk_size = defaults.chunk_size;
    }
    if out.audit_log_path.trim().is_empty() {
        out.audit_log_path = defaults.audit_log_path;
    }
    if out.export_path.trim().is_empty() {
        out.export_path = defaults.export_path;
    }
    if out.default_owner.trim().is_empty() {
        out.default_owner = defaults.default_owner;
    }
    if out.max_audit_lines == 0 {
        out.max_audit_lines = defaults.max_audit_lines;
    }
    out
}

fn load_acl_runtime_config() -> AclRuntimeConfig {
    let cfg = load_config();
    let acl_cfg = normalize_acl_config(cfg.acl);
    AclRuntimeConfig {
        audit_log_path: PathBuf::from(&acl_cfg.audit_log_path),
        export_path: PathBuf::from(&acl_cfg.export_path),
        cfg: acl_cfg,
    }
}

fn audit_log(cfg: &AclRuntimeConfig) -> AuditLog {
    AuditLog::new(cfg.audit_log_path.clone(), cfg.cfg.max_audit_lines)
}

fn map_acl_err(err: anyhow::Error) -> CliError {
    let mut details: Vec<String> = Vec::new();
    if let Some(acl_err) = err.downcast_ref::<AclError>() {
        if acl_err.is_access_denied() {
            details.push("Hint: Run as Administrator for ACL write/repair operations.".to_string());
        }
    }
    CliError {
        code: 1,
        message: format!("{err:#}"),
        details,
    }
}

fn prompt_confirm(prompt: &str, default: bool, yes: bool) -> CliResult<bool> {
    if yes {
        return Ok(true);
    }
    if !can_interact() {
        return Err(CliError::with_details(
            2,
            "Interactive confirmation required.".to_string(),
            &["Fix: Run in an interactive terminal or pass -y to skip confirmation."],
        ));
    }
    Confirm::new()
        .with_prompt(prompt)
        .default(default)
        .interact()
        .map_err(|e| CliError::new(1, format!("Failed to read confirmation: {e}")))
}

fn ensure_interactive(label: &str) -> CliResult {
    if can_interact() {
        Ok(())
    } else {
        Err(CliError::with_details(
            2,
            format!("{label} requires interactive mode."),
            &["Fix: Run in an interactive terminal."],
        ))
    }
}

fn print_path_header(path: &Path) {
    ui_println!("\nPath: {}", path.display());
}

fn print_acl_summary(snapshot: &acl::types::AclSnapshot) {
    let allow = snapshot.allow_count();
    let deny = snapshot.deny_count();
    let orphan = snapshot.orphan_count();
    let explicit = snapshot.explicit_count();
    let inherited = snapshot.inherited_count();
    ui_println!(
        "Owner: {} | Inherit: {}",
        snapshot.owner,
        if snapshot.is_protected {
            "disabled"
        } else {
            "enabled"
        }
    );
    ui_println!(
        "Total: {} (Allow {} / Deny {})  Explicit {}  Inherited {}  Orphan {}",
        snapshot.entries.len(),
        allow,
        deny,
        explicit,
        inherited,
        orphan
    );
}

fn cmd_view(args: AclViewCmd) -> CliResult {
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
fn cmd_add(args: AclAddCmd) -> CliResult {
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

fn cmd_remove(args: AclRemoveCmd) -> CliResult {
    ensure_interactive("Remove")?;
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

fn cmd_purge(args: AclPurgeCmd) -> CliResult {
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

fn cmd_diff(args: AclDiffCmd) -> CliResult {
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
fn cmd_batch(args: AclBatchCmd) -> CliResult {
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

fn cmd_effective(args: AclEffectiveCmd) -> CliResult {
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

fn cmd_copy(args: AclCopyCmd) -> CliResult {
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

fn cmd_backup(args: AclBackupCmd) -> CliResult {
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

fn cmd_restore(args: AclRestoreCmd) -> CliResult {
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
fn cmd_inherit(args: AclInheritCmd) -> CliResult {
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

fn cmd_owner(args: AclOwnerCmd) -> CliResult {
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

fn cmd_orphans(args: AclOrphansCmd) -> CliResult {
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

fn cmd_repair(args: AclRepairCmd) -> CliResult {
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

    let stats =
        acl::repair::force_repair(path, &cfg.cfg, runtime::is_quiet()).map_err(map_acl_err)?;

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

fn cmd_audit(args: AclAuditCmd) -> CliResult {
    let cfg = load_acl_runtime_config();
    let audit = audit_log(&cfg);

    if let Some(dest) = args.export {
        let dest = PathBuf::from(dest);
        let n = audit.export_csv(&dest).map_err(map_acl_err)?;
        ui_println!("Exported {} rows to {}", n, dest.display());
        return Ok(());
    }

    let entries = audit.tail(args.tail).map_err(map_acl_err)?;
    if entries.is_empty() {
        ui_println!("No audit entries.");
        return Ok(());
    }

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Time")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Action").add_attribute(Attribute::Bold),
        Cell::new("Status").add_attribute(Attribute::Bold),
        Cell::new("Path").add_attribute(Attribute::Bold),
        Cell::new("Details").add_attribute(Attribute::Bold),
    ]);

    for e in &entries {
        let status = if e.success { "ok" } else { "fail" };
        table.add_row(vec![
            Cell::new(&e.ts),
            Cell::new(&e.action),
            Cell::new(status),
            Cell::new(acl::parse::truncate_left(&e.path, 40)),
            Cell::new(acl::parse::truncate(&e.details, 40)),
        ]);
    }

    print_table(&table);
    Ok(())
}

fn cmd_config(args: AclConfigCmd) -> CliResult {
    let mut cfg = load_config();

    if !args.set.is_empty() {
        if args.set.len() != 2 {
            return Err(CliError::with_details(
                2,
                "--set requires KEY VALUE".to_string(),
                &["Fix: Use `xun acl config --set throttle_limit 8`."],
            ));
        }
        let key = args.set[0].as_str();
        let value = args.set[1].as_str();
        match key {
            "throttle_limit" => {
                cfg.acl.throttle_limit = value
                    .parse::<usize>()
                    .map_err(|_| CliError::new(2, "Invalid throttle_limit"))?;
            }
            "chunk_size" => {
                cfg.acl.chunk_size = value
                    .parse::<usize>()
                    .map_err(|_| CliError::new(2, "Invalid chunk_size"))?;
            }
            "audit_log_path" => {
                cfg.acl.audit_log_path = value.to_string();
            }
            "export_path" => {
                cfg.acl.export_path = value.to_string();
            }
            "default_owner" => {
                cfg.acl.default_owner = value.to_string();
            }
            "max_audit_lines" => {
                cfg.acl.max_audit_lines = value
                    .parse::<usize>()
                    .map_err(|_| CliError::new(2, "Invalid max_audit_lines"))?;
            }
            _ => {
                return Err(CliError::with_details(
                    2,
                    format!("Unknown key: {key}"),
                    &[
                        "Valid keys: throttle_limit, chunk_size, audit_log_path, export_path, default_owner, max_audit_lines",
                    ],
                ));
            }
        }

        save_config(&cfg).map_err(|e| CliError::new(1, format!("Failed to save config: {e}")))?;
        ui_println!("ACL config updated.");
        return Ok(());
    }

    let acl_cfg = normalize_acl_config(cfg.acl);

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Key")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Value").add_attribute(Attribute::Bold),
    ]);
    table.add_row(vec![
        Cell::new("throttle_limit"),
        Cell::new(acl_cfg.throttle_limit),
    ]);
    table.add_row(vec![Cell::new("chunk_size"), Cell::new(acl_cfg.chunk_size)]);
    table.add_row(vec![
        Cell::new("audit_log_path"),
        Cell::new(acl_cfg.audit_log_path),
    ]);
    table.add_row(vec![
        Cell::new("export_path"),
        Cell::new(acl_cfg.export_path),
    ]);
    table.add_row(vec![
        Cell::new("default_owner"),
        Cell::new(acl_cfg.default_owner),
    ]);
    table.add_row(vec![
        Cell::new("max_audit_lines"),
        Cell::new(acl_cfg.max_audit_lines),
    ]);
    print_table(&table);
    Ok(())
}
