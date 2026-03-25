#[path = "../support/mod.rs"]
mod common;

use common::{TestEnv, run_err, run_ok};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

fn prepare_fake_sevenzip_home(env: &TestEnv, name: &str) -> PathBuf {
    let home = env.root.join(name);
    fs::create_dir_all(home.join("Formats")).unwrap();
    fs::write(home.join("7zFM.exe"), b"7zfm").unwrap();
    home
}

fn prepare_fake_build_root(env: &TestEnv, config: &str, payload: &[u8]) -> PathBuf {
    let root = env.root.join("plugin-build");
    fs::create_dir_all(root.join(config)).unwrap();
    fs::write(root.join(config).join("xunbak.dll"), payload).unwrap();
    root
}

fn prepare_assoc_store_path(env: &TestEnv, name: &str) -> PathBuf {
    env.root.join(name).join("assoc-store.json")
}

#[derive(Deserialize)]
struct AssocStoreData {
    user: std::collections::BTreeMap<String, String>,
}

fn read_assoc_store(path: &PathBuf) -> AssocStoreData {
    let content = fs::read_to_string(path).unwrap();
    serde_json::from_str(&content).unwrap()
}

fn write_assoc_store(path: &PathBuf, user: &[(&str, String)]) {
    let user = user
        .iter()
        .map(|(key, value)| (key.to_string(), value.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        path,
        serde_json::to_string_pretty(&serde_json::json!({
            "user": user,
            "classes": {}
        }))
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn cli_xunbak_plugin_install_copies_dll_to_explicit_sevenzip_home() {
    let env = TestEnv::new();
    let sevenzip_home = prepare_fake_sevenzip_home(&env, "sevenzip-install");
    let build_root = prepare_fake_build_root(&env, "Debug", b"plugin-debug");

    let out = run_ok(
        env.cmd()
            .env("XUN_XUNBAK_PLUGIN_BUILD_ROOT", &build_root)
            .args([
                "xunbak",
                "plugin",
                "install",
                "--sevenzip-home",
                sevenzip_home.to_str().unwrap(),
            ]),
    );

    assert_eq!(
        fs::read(sevenzip_home.join("Formats").join("xunbak.dll")).unwrap(),
        b"plugin-debug"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Installed:"));
}

#[test]
fn cli_xunbak_plugin_install_errors_when_build_artifact_is_missing() {
    let env = TestEnv::new();
    let sevenzip_home = prepare_fake_sevenzip_home(&env, "sevenzip-missing-artifact");
    let build_root = env.root.join("missing-plugin-build");
    fs::create_dir_all(&build_root).unwrap();

    let out = run_err(
        env.cmd()
            .env("XUN_XUNBAK_PLUGIN_BUILD_ROOT", &build_root)
            .args([
                "xunbak",
                "plugin",
                "install",
                "--sevenzip-home",
                sevenzip_home.to_str().unwrap(),
            ]),
    );

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Plugin build artifact not found"));
}

#[test]
fn cli_xunbak_plugin_install_rejects_existing_dll_when_no_overwrite_is_set() {
    let env = TestEnv::new();
    let sevenzip_home = prepare_fake_sevenzip_home(&env, "sevenzip-no-overwrite");
    let build_root = prepare_fake_build_root(&env, "Debug", b"plugin-debug");
    fs::write(
        sevenzip_home.join("Formats").join("xunbak.dll"),
        b"existing-plugin",
    )
    .unwrap();

    let out = run_err(
        env.cmd()
            .env("XUN_XUNBAK_PLUGIN_BUILD_ROOT", &build_root)
            .args([
                "xunbak",
                "plugin",
                "install",
                "--sevenzip-home",
                sevenzip_home.to_str().unwrap(),
                "--no-overwrite",
            ]),
    );

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Target plugin already exists"));
    assert_eq!(
        fs::read(sevenzip_home.join("Formats").join("xunbak.dll")).unwrap(),
        b"existing-plugin"
    );
}

#[test]
fn cli_xunbak_plugin_install_errors_for_invalid_explicit_sevenzip_home() {
    let env = TestEnv::new();
    let build_root = prepare_fake_build_root(&env, "Debug", b"plugin-debug");
    let invalid_home = env.root.join("not-a-sevenzip-home");
    fs::create_dir_all(&invalid_home).unwrap();

    let out = run_err(
        env.cmd()
            .env("XUN_XUNBAK_PLUGIN_BUILD_ROOT", &build_root)
            .args([
                "xunbak",
                "plugin",
                "install",
                "--sevenzip-home",
                invalid_home.to_str().unwrap(),
            ]),
    );

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Invalid --sevenzip-home"));
}

#[test]
fn cli_xunbak_plugin_install_with_associate_writes_current_user_binding() {
    let env = TestEnv::new();
    let sevenzip_home = prepare_fake_sevenzip_home(&env, "sevenzip-associate");
    let build_root = prepare_fake_build_root(&env, "Debug", b"plugin-debug");
    let assoc_store = prepare_assoc_store_path(&env, "sevenzip-associate");

    let out = run_ok(
        env.cmd()
            .env("XUN_XUNBAK_PLUGIN_BUILD_ROOT", &build_root)
            .env("XUN_XUNBAK_PLUGIN_TEST_ASSOC_FILE", &assoc_store)
            .args([
                "xunbak",
                "plugin",
                "install",
                "--sevenzip-home",
                sevenzip_home.to_str().unwrap(),
                "--associate",
            ]),
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Associated: .xunbak"));
    let data = read_assoc_store(&assoc_store);
    let expected_command = format!("\"{}\" \"%1\"", sevenzip_home.join("7zFM.exe").display());
    assert_eq!(
        data.user.get(".xunbak").map(String::as_str),
        Some("XunYu.xunbak")
    );
    assert_eq!(
        data.user
            .get("XunYu.xunbak\\shell\\open\\command")
            .map(String::as_str),
        Some(expected_command.as_str())
    );
}

#[test]
fn cli_xunbak_plugin_install_with_associate_is_idempotent() {
    let env = TestEnv::new();
    let sevenzip_home = prepare_fake_sevenzip_home(&env, "sevenzip-associate-repeat");
    let build_root = prepare_fake_build_root(&env, "Debug", b"plugin-debug");
    let assoc_store = prepare_assoc_store_path(&env, "sevenzip-associate-repeat");

    run_ok(
        env.cmd()
            .env("XUN_XUNBAK_PLUGIN_BUILD_ROOT", &build_root)
            .env("XUN_XUNBAK_PLUGIN_TEST_ASSOC_FILE", &assoc_store)
            .args([
                "xunbak",
                "plugin",
                "install",
                "--sevenzip-home",
                sevenzip_home.to_str().unwrap(),
                "--associate",
            ]),
    );
    let first = fs::read_to_string(&assoc_store).unwrap();

    run_ok(
        env.cmd()
            .env("XUN_XUNBAK_PLUGIN_BUILD_ROOT", &build_root)
            .env("XUN_XUNBAK_PLUGIN_TEST_ASSOC_FILE", &assoc_store)
            .args([
                "xunbak",
                "plugin",
                "install",
                "--sevenzip-home",
                sevenzip_home.to_str().unwrap(),
                "--associate",
            ]),
    );
    let second = fs::read_to_string(&assoc_store).unwrap();

    assert_eq!(first, second);
}

#[test]
fn cli_xunbak_plugin_install_with_associate_reports_clear_error_and_rollback_hint() {
    let env = TestEnv::new();
    let sevenzip_home = prepare_fake_sevenzip_home(&env, "sevenzip-associate-error");
    let build_root = prepare_fake_build_root(&env, "Debug", b"plugin-debug");
    let assoc_store = prepare_assoc_store_path(&env, "sevenzip-associate-error");
    fs::create_dir_all(assoc_store.parent().unwrap()).unwrap();
    fs::write(&assoc_store, "{not-json").unwrap();

    let out = run_err(
        env.cmd()
            .env("XUN_XUNBAK_PLUGIN_BUILD_ROOT", &build_root)
            .env("XUN_XUNBAK_PLUGIN_TEST_ASSOC_FILE", &assoc_store)
            .args([
                "xunbak",
                "plugin",
                "install",
                "--sevenzip-home",
                sevenzip_home.to_str().unwrap(),
                "--associate",
            ]),
    );

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Failed to associate .xunbak with 7-Zip"));
    assert!(stderr.contains("Rollback:"));
}

#[test]
fn cli_xunbak_plugin_uninstall_removes_plugin_dll() {
    let env = TestEnv::new();
    let sevenzip_home = prepare_fake_sevenzip_home(&env, "sevenzip-uninstall");
    fs::write(
        sevenzip_home.join("Formats").join("xunbak.dll"),
        b"installed-plugin",
    )
    .unwrap();

    let out = run_ok(env.cmd().args([
        "xunbak",
        "plugin",
        "uninstall",
        "--sevenzip-home",
        sevenzip_home.to_str().unwrap(),
    ]));

    assert!(!sevenzip_home.join("Formats").join("xunbak.dll").exists());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Removed:"));
}

#[test]
fn cli_xunbak_plugin_uninstall_is_idempotent_when_dll_is_missing() {
    let env = TestEnv::new();
    let sevenzip_home = prepare_fake_sevenzip_home(&env, "sevenzip-uninstall-idempotent");

    let out = run_ok(env.cmd().args([
        "xunbak",
        "plugin",
        "uninstall",
        "--sevenzip-home",
        sevenzip_home.to_str().unwrap(),
    ]));

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Plugin not installed"));
}

#[test]
fn cli_xunbak_plugin_uninstall_with_remove_association_clears_managed_binding() {
    let env = TestEnv::new();
    let sevenzip_home = prepare_fake_sevenzip_home(&env, "sevenzip-remove-associate");
    let assoc_store = prepare_assoc_store_path(&env, "sevenzip-remove-associate");
    write_assoc_store(
        &assoc_store,
        &[
            (".xunbak", "XunYu.xunbak".to_string()),
            (
                "XunYu.xunbak\\shell\\open\\command",
                format!("\"{}\" \"%1\"", sevenzip_home.join("7zFM.exe").display()),
            ),
        ],
    );

    let out = run_ok(
        env.cmd()
            .env("XUN_XUNBAK_PLUGIN_TEST_ASSOC_FILE", &assoc_store)
            .args([
                "xunbak",
                "plugin",
                "uninstall",
                "--sevenzip-home",
                sevenzip_home.to_str().unwrap(),
                "--remove-association",
            ]),
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Association removed: .xunbak"));
    let data = read_assoc_store(&assoc_store);
    assert!(data.user.is_empty());
}

#[test]
fn cli_xunbak_plugin_uninstall_with_remove_association_preserves_third_party_binding() {
    let env = TestEnv::new();
    let sevenzip_home = prepare_fake_sevenzip_home(&env, "sevenzip-keep-third-party");
    let assoc_store = prepare_assoc_store_path(&env, "sevenzip-keep-third-party");
    write_assoc_store(
        &assoc_store,
        &[
            (".xunbak", "ThirdParty.xunbak".to_string()),
            (
                "ThirdParty.xunbak\\shell\\open\\command",
                "\"C:\\Tools\\Other.exe\" \"%1\"".to_string(),
            ),
        ],
    );

    let out = run_ok(
        env.cmd()
            .env("XUN_XUNBAK_PLUGIN_TEST_ASSOC_FILE", &assoc_store)
            .args([
                "xunbak",
                "plugin",
                "uninstall",
                "--sevenzip-home",
                sevenzip_home.to_str().unwrap(),
                "--remove-association",
            ]),
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Association kept: current .xunbak binding is not 7-Zip"));
    let data = read_assoc_store(&assoc_store);
    assert_eq!(
        data.user.get(".xunbak").map(String::as_str),
        Some("ThirdParty.xunbak")
    );
}

#[test]
fn cli_xunbak_plugin_doctor_reports_plugin_state() {
    let env = TestEnv::new();
    let sevenzip_home = prepare_fake_sevenzip_home(&env, "sevenzip-doctor");
    fs::write(
        sevenzip_home.join("Formats").join("xunbak.dll"),
        b"installed-plugin",
    )
    .unwrap();

    let out = run_ok(
        env.cmd()
            .env("XUN_XUNBAK_PLUGIN_TEST_FILE_VERSION", "24.09")
            .env(
                "XUN_XUNBAK_PLUGIN_TEST_7ZI_OUTPUT",
                "Codecs:\n 0 ED     40202 BZip2\n 0 ED         0 Copy\n",
            )
            .args([
                "xunbak",
                "plugin",
                "doctor",
                "--sevenzip-home",
                sevenzip_home.to_str().unwrap(),
            ]),
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("xunbak plugin doctor"));
    assert!(stdout.contains("7-Zip Home:"));
    assert!(stdout.contains("7-Zip FM:"));
    assert!(stdout.contains("version 24.09"));
    assert!(stdout.contains("Plugin DLL:"));
    assert!(stdout.contains("Association:"));
    assert!(stdout.contains("7z ZSTD Codec: not-detected"));
    assert!(stdout.contains("Suggestions:"));
    assert!(stdout.contains("did not report ZSTD codec support"));
}
