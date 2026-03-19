use std::fs;
use std::thread;
use std::time::Duration;

use super::common::*;
use crate::common::*;

#[test]
fn alias_setup_writes_core_shell_artifacts() {
    let env = TestEnv::new();
    do_setup(&env);

    let cmd_path = cmd_macrofile(&env);
    let ps_path = powershell_profile(&env);

    assert!(cmd_path.exists(), "cmd macrofile not created");
    assert!(ps_path.exists(), "powershell profile not created");
    assert_file_contains(&cmd_path, "REM === XUN_ALIAS BEGIN ===");
    assert_file_contains(&cmd_path, "REM === XUN_ALIAS END ===");
    assert_file_contains(&ps_path, "# === XUN_ALIAS BEGIN ===");
    assert_file_contains(&ps_path, "# === XUN_ALIAS END ===");
}

#[test]
fn shell_filter_limits_backend_output() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "psonly", "Get-Process", "--shell", "ps"]));

    let cmd_path = cmd_macrofile(&env);
    let ps_path = powershell_profile(&env);
    assert_file_contains(&ps_path, "Set-Alias psonly Get-Process");
    assert_file_not_contains(&cmd_path, "doskey psonly=");
}

#[test]
fn shell_filter_force_update_moves_alias_between_backends() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "switcher", "Write-Host hi", "--shell", "ps"]));
    assert_file_contains(
        &powershell_profile(&env),
        "function switcher { Write-Host hi @args }",
    );
    assert_file_not_contains(&cmd_macrofile(&env), "doskey switcher=");

    run_ok(alias_cmd(&env).args([
        "alias", "add", "switcher", "echo hi", "--shell", "cmd", "--force",
    ]));

    assert_file_contains(&cmd_macrofile(&env), "doskey switcher=echo hi $*");
    assert_file_not_contains(&powershell_profile(&env), "function switcher {");
    assert_file_not_contains(&powershell_profile(&env), "Set-Alias switcher ");
}

#[test]
fn powershell_profile_renders_simple_and_complex_aliases() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "g", "git"]));
    run_ok(alias_cmd(&env).args(["alias", "add", "gst", "git status"]));

    let ps_path = powershell_profile(&env);
    assert_file_contains(&ps_path, "Set-Alias g git");
    assert_file_contains(&ps_path, "function gst { git status @args }");
}

#[test]
fn app_alias_is_rendered_into_shell_artifacts() {
    let env = TestEnv::new();
    do_setup(&env);

    let exe = make_fake_exe(&env, "editor");
    let exe_str = exe.to_str().unwrap().to_string();
    run_ok(alias_cmd(&env).args([
        "alias",
        "app",
        "add",
        "editor",
        &exe_str,
        "--args",
        "--flag",
        "--no-apppaths",
    ]));

    let cmd_path = cmd_macrofile(&env);
    let ps_path = powershell_profile(&env);
    assert_file_contains(&cmd_path, "doskey editor=");
    assert_file_contains(&cmd_path, &exe_str);
    assert_file_contains(&cmd_path, "--flag $*");
    assert_file_contains(&ps_path, "function editor { Start-Process '");
    assert_file_contains(&ps_path, &exe_str);
    assert_file_contains(&ps_path, "@('--flag') + $args");
}

#[test]
fn alias_sync_recreates_missing_shell_artifacts() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gst", "git status"]));

    let cmd_path = cmd_macrofile(&env);
    let ps_path = powershell_profile(&env);
    assert_file_contains(&cmd_path, "doskey gst=git status $*");
    assert_file_contains(&ps_path, "function gst { git status @args }");

    fs::remove_file(&cmd_path).unwrap();
    fs::remove_file(&ps_path).unwrap();

    run_ok(alias_cmd(&env).args(["alias", "sync"]));

    assert!(cmd_path.exists(), "cmd macrofile not recreated by sync");
    assert!(ps_path.exists(), "powershell profile not recreated by sync");
    assert_file_contains(&cmd_path, "doskey gst=git status $*");
    assert_file_contains(&ps_path, "function gst { git status @args }");
}

#[test]
fn alias_import_mixed_entries_rebuilds_shell_artifacts() {
    let src_env = TestEnv::new();
    do_setup(&src_env);

    let exe = make_fake_exe(&src_env, "code_like");
    run_ok(alias_cmd(&src_env).args(["alias", "add", "gs", "git status"]));
    run_ok(alias_cmd(&src_env).args([
        "alias",
        "app",
        "add",
        "code",
        exe.to_str().unwrap(),
        "--no-apppaths",
    ]));

    let export_path = src_env.root.join("mixed_export.toml");
    run_ok(alias_cmd(&src_env).args(["alias", "export", "-o", export_path.to_str().unwrap()]));

    let dst_env = TestEnv::new();
    do_setup(&dst_env);
    run_ok(alias_cmd(&dst_env).args(["alias", "import", export_path.to_str().unwrap()]));

    let toml = read_toml(&dst_env);
    assert!(
        toml.contains("[alias.gs]"),
        "shell alias missing after import"
    );
    assert!(
        toml.contains("[app.code]"),
        "app alias missing after import"
    );
    assert_shim_exists(&dst_env, "gs");
    assert_shim_exists(&dst_env, "code");
    assert_file_contains(&cmd_macrofile(&dst_env), "doskey gs=git status $*");
    assert_file_contains(&cmd_macrofile(&dst_env), "doskey code=");
    assert_file_contains(
        &powershell_profile(&dst_env),
        "function gs { git status @args }",
    );
    assert_file_contains(
        &powershell_profile(&dst_env),
        "function code { Start-Process '",
    );
}

#[test]
fn alias_sync_does_not_rewrite_shell_artifacts_when_unchanged() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status"]));
    run_ok(alias_cmd(&env).args(["alias", "sync"]));

    let cmd_path = cmd_macrofile(&env);
    let ps_path = powershell_profile(&env);
    let cmd_mtime_1 = fs::metadata(&cmd_path).unwrap().modified().unwrap();
    let ps_mtime_1 = fs::metadata(&ps_path).unwrap().modified().unwrap();

    thread::sleep(Duration::from_millis(50));
    run_ok(alias_cmd(&env).args(["alias", "sync"]));

    let cmd_mtime_2 = fs::metadata(&cmd_path).unwrap().modified().unwrap();
    let ps_mtime_2 = fs::metadata(&ps_path).unwrap().modified().unwrap();

    assert_eq!(
        cmd_mtime_1, cmd_mtime_2,
        "cmd macrofile should not be rewritten"
    );
    assert_eq!(
        ps_mtime_1, ps_mtime_2,
        "powershell profile should not be rewritten"
    );
}
#[test]
fn alias_import_without_changes_does_not_rewrite_shell_artifacts() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status"]));

    let export_path = env.root.join("import_same.toml");
    run_ok(alias_cmd(&env).args(["alias", "export", "-o", export_path.to_str().unwrap()]));

    let cmd_path = cmd_macrofile(&env);
    let ps_path = powershell_profile(&env);
    let cmd_mtime_1 = fs::metadata(&cmd_path).unwrap().modified().unwrap();
    let ps_mtime_1 = fs::metadata(&ps_path).unwrap().modified().unwrap();

    thread::sleep(Duration::from_millis(50));
    run_ok(alias_cmd(&env).args(["alias", "import", export_path.to_str().unwrap()]));

    let cmd_mtime_2 = fs::metadata(&cmd_path).unwrap().modified().unwrap();
    let ps_mtime_2 = fs::metadata(&ps_path).unwrap().modified().unwrap();

    assert_eq!(cmd_mtime_1, cmd_mtime_2, "cmd macrofile should not be rewritten on noop import");
    assert_eq!(ps_mtime_1, ps_mtime_2, "powershell profile should not be rewritten on noop import");
}
