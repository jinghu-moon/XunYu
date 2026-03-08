use super::*;

pub(super) fn cmd_check(manager: &EnvManager, args: EnvCheckCmd) -> CliResult {
    cmd_doctor_like(manager, args.scope, args.fix, args.format, true)
}


pub(super) fn cmd_doctor(manager: &EnvManager, args: EnvDoctorCmd) -> CliResult {
    cmd_doctor_like(manager, args.scope, args.fix, args.format, false)
}

pub(super) fn cmd_doctor_like(
    manager: &EnvManager,
    scope_raw: String,
    fix: bool,
    format: String,
    use_check_alias: bool,
) -> CliResult {
    let scope = parse_scope(&scope_raw)?;
    if fix {
        let fixed = manager.doctor_fix(scope).map_err(map_env_err)?;
        if format.eq_ignore_ascii_case("json") {
            out_println!(
                "{}",
                serde_json::to_string_pretty(&fixed).unwrap_or_default()
            );
        } else {
            out_println!("doctor fixed: {} item(s)", fixed.fixed);
            for line in fixed.details {
                out_println!("  - {}", line);
            }
        }
        return Ok(());
    }
    let report = if use_check_alias {
        manager.check_run(scope).map_err(map_env_err)?
    } else {
        manager.doctor_run(scope).map_err(map_env_err)?
    };
    if format.eq_ignore_ascii_case("json") {
        out_println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_default()
        );
    } else {
        out_println!("{}", doctor::report_text(&report));
    }
    if !format.eq_ignore_ascii_case("json") {
        let code = doctor::doctor_exit_code(&report);
        if code > 0 {
            return Err(CliError::new(code, "doctor reported issues"));
        }
    }
    Ok(())
}



