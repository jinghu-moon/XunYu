#![cfg(windows)]

#[path = "support/mod.rs"]
mod common;

use common::{run_ok, TestEnv};
use std::fs;

fn stdout_text(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n")
}

fn read_store(env: &TestEnv) -> String {
    fs::read_to_string(env.root.join(".xun.bookmark.json")).unwrap()
}

#[test]
fn learn_creates_learned_entry() {
    let env = TestEnv::new();
    fs::write(
        env.root.join(".xun.config.json"),
        r#"{"bookmark":{"excludeDirs":[]}}"#,
    )
    .unwrap();
    let dir = env.root.join("learned-target");
    fs::create_dir_all(&dir).unwrap();

    run_ok(
        env.cmd()
            .arg("bookmark")
            .arg("learn")
            .arg("--path")
            .arg(dir.to_string_lossy().to_string()),
    );

    let raw = read_store(&env);
    assert!(raw.contains("Learned"));
    assert!(raw.contains("learned-target"));
}

#[test]
fn learn_disabled_by_config_noops() {
    let env = TestEnv::new();
    fs::write(
        env.root.join(".xun.config.json"),
        r#"{"bookmark":{"autoLearn":{"enabled":false}}}"#,
    )
    .unwrap();
    let dir = env.root.join("disabled-target");
    fs::create_dir_all(&dir).unwrap();

    run_ok(
        env.cmd()
            .arg("bookmark")
            .arg("learn")
            .arg("--path")
            .arg(dir.to_string_lossy().to_string()),
    );

    assert!(!env.root.join(".xun.bookmark.json").exists());
}

#[test]
fn import_from_autojump_writes_imported_entries() {
    let env = TestEnv::new();
    let autojump = env.root.join("autojump.txt");
    fs::write(&autojump, "12\tC:/work/client-api\n").unwrap();

    run_ok(
        env.cmd()
            .arg("bookmark")
            .arg("import")
            .arg("--from")
            .arg("autojump")
            .arg("-i")
            .arg(&autojump),
    );

    let raw = read_store(&env);
    assert!(raw.contains("Imported"));
    assert!(raw.contains("client-api"));
}

#[test]
fn init_powershell_contains_expected_integration_bits() {
    let env = TestEnv::new();
    let out = run_ok(
        env.cmd()
            .arg("bookmark")
            .arg("init")
            .arg("powershell")
            .arg("--cmd")
            .arg("j"),
    );
    let stdout = stdout_text(&out);
    assert!(stdout.contains("function bm"));
    assert!(stdout.contains("function j"));
    assert!(stdout.contains("function ji"));
    assert!(stdout.contains("function jo"));
    assert!(stdout.contains("function joi"));
    assert!(stdout.contains("Start-Process"));
    assert!(stdout.contains("Register-ArgumentCompleter"));
    assert!(!stdout.contains("Start-Job"));
}

#[test]
fn init_bash_contains_root_and_query_completion() {
    let env = TestEnv::new();
    let out = run_ok(env.cmd().arg("bookmark").arg("init").arg("bash"));
    let stdout = stdout_text(&out);
    assert!(stdout.contains("function bm()"));
    assert!(stdout.contains("_bm_root_complete"));
    assert!(stdout.contains("_bm_query_complete"));
    assert!(!stdout.contains("eval \"$(xun bookmark init bash)\""));
}
