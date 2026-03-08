#![cfg(windows)]

mod common;

use std::process::Command;

use common::{TestEnv, run_ok};

#[test]
fn init_powershell_exposes_xy_and_xyu() {
    let env = TestEnv::new();
    let out = run_ok(env.cmd().args(["init", "powershell"]));
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(stdout.contains("Set-Alias xyu xun"));
    assert!(stdout.contains("Set-Alias xy xun"));
    assert!(stdout.contains("Register-ArgumentCompleter -CommandName 'xyu'"));
    assert!(stdout.contains("Register-ArgumentCompleter -CommandName 'xy'"));
}

#[test]
fn completion_bash_supports_xy_and_xyu() {
    let env = TestEnv::new();
    let out = run_ok(env.cmd().args(["completion", "bash"]));
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(stdout.contains(
        r#"if [[ "$cmd" != "xun" && "$cmd" != "x" && "$cmd" != "xyu" && "$cmd" != "xy" ]]"#
    ));
    assert!(stdout.contains("complete -F _xun_complete xun x xyu xy z o delete rename"));
}

#[test]
fn xyu_binary_reports_its_own_name() {
    let out = Command::new(env!("CARGO_BIN_EXE_xyu"))
        .arg("--version")
        .output()
        .expect("xyu should run");

    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("xyu "));
}
