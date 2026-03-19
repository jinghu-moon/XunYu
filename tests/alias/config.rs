//! alias config 读写测试：TOML 持久化、原子写、回读校验、备份回滚

use super::common::*;
use crate::common::*;

// ── setup ────────────────────────────────────────────────────────────────────

#[test]
fn alias_setup_creates_dirs_and_toml() {
    let env = TestEnv::new();
    do_setup(&env);

    assert!(aliases_toml(&env).exists(), "aliases.toml not created");
    assert!(shims_dir(&env).exists(), "shims dir not created");
}

#[test]
fn alias_setup_idempotent() {
    let env = TestEnv::new();
    do_setup(&env);
    // 第二次 setup 不报错，不破坏现有配置
    do_setup(&env);
    assert!(aliases_toml(&env).exists());
}

#[test]
fn alias_setup_skips_disabled_shells() {
    let env = TestEnv::new();
    run_ok(alias_cmd(&env).args(["alias", "setup", "--no-ps", "--no-cmd"]));
    // 不要求 PS / CMD profile 写入成功，只要不崩溃即可
    assert!(aliases_toml(&env).exists());
}

// ── add / persist ─────────────────────────────────────────────────────────────

#[test]
fn alias_add_persists_to_toml() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args([
        "alias",
        "add",
        "gs",
        "git status",
        "--desc",
        "git status",
        "--tag",
        "git",
    ]));

    let toml = read_toml(&env);
    assert!(toml.contains("[alias.gs]"), "alias section missing");
    assert!(toml.contains("git status"), "command missing");
    assert!(toml.contains("git"), "tag missing");
}

#[test]
fn alias_add_mode_stored_correctly() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "piped", "git log | head", "--mode", "cmd"]));

    let toml = read_toml(&env);
    assert!(toml.contains("mode = \"cmd\""), "mode not persisted");
}

#[test]
fn alias_add_rejects_duplicate_without_force() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status"]));
    let out = run_err(alias_cmd(&env).args(["alias", "add", "gs", "git diff"]));
    let err = stderr_str(&out);
    assert!(
        err.contains("already exists") || err.contains("force"),
        "expected duplicate error: {err}"
    );
}

#[test]
fn alias_add_force_overwrites() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status"]));
    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git stash", "--force"]));

    let toml = read_toml(&env);
    assert!(toml.contains("git stash"), "overwrite failed");
    assert!(!toml.contains("git status"), "old value still present");
}

#[test]
fn alias_add_force_replaces_existing_app_alias() {
    let env = TestEnv::new();
    do_setup(&env);

    let exe = make_fake_exe(&env, "switch_app");
    run_ok(alias_cmd(&env).args([
        "alias",
        "app",
        "add",
        "switcher",
        exe.to_str().unwrap(),
        "--no-apppaths",
    ]));

    run_ok(alias_cmd(&env).args(["alias", "add", "switcher", "git status", "--force"]));

    let toml = read_toml(&env);
    assert!(toml.contains("[alias.switcher]"), "shell alias missing after force replace");
    assert!(!toml.contains("[app.switcher]"), "app alias should be removed after force replace");
    assert_shim_exists(&env, "switcher");
    assert_file_contains(&cmd_macrofile(&env), "doskey switcher=git status $*");
}

#[test]
fn alias_add_shell_filter_stored() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "psonly", "Get-Process", "--shell", "ps"]));

    let toml = read_toml(&env);
    assert!(toml.contains("shells"), "shells field missing");
    assert!(toml.contains("ps"), "shell filter not persisted");
}

// ── rm ────────────────────────────────────────────────────────────────────────

#[test]
fn alias_rm_removes_entry() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status"]));
    run_ok(alias_cmd(&env).args(["alias", "rm", "gs"]));

    let toml = read_toml(&env);
    assert!(!toml.contains("[alias.gs]"), "entry not removed from TOML");
}

#[test]
fn alias_rm_nonexistent_is_graceful() {
    let env = TestEnv::new();
    do_setup(&env);
    // 删除不存在的 alias 不应报错（打印 Not found）
    run_ok(alias_cmd(&env).args(["alias", "rm", "nonexistent"]));
}

#[test]
fn alias_rm_multiple_names() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "a1", "cmd1"]));
    run_ok(alias_cmd(&env).args(["alias", "add", "a2", "cmd2"]));
    run_ok(alias_cmd(&env).args(["alias", "rm", "a1", "a2"]));

    let toml = read_toml(&env);
    assert!(!toml.contains("[alias.a1]"));
    assert!(!toml.contains("[alias.a2]"));
}

// ── export / import ───────────────────────────────────────────────────────────

#[test]
fn alias_export_produces_valid_toml() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status"]));

    let out_path = env.root.join("export.toml");
    run_ok(alias_cmd(&env).args(["alias", "export", "-o", out_path.to_str().unwrap()]));

    let content = std::fs::read_to_string(&out_path).unwrap();
    assert!(
        content.contains("[alias.gs]"),
        "exported TOML missing alias"
    );
}

#[test]
fn alias_import_adds_entries() {
    let src_env = TestEnv::new();
    do_setup(&src_env);
    run_ok(alias_cmd(&src_env).args(["alias", "add", "gs", "git status"]));
    run_ok(alias_cmd(&src_env).args(["alias", "add", "gp", "git push"]));

    let export_path = src_env.root.join("export.toml");
    run_ok(alias_cmd(&src_env).args(["alias", "export", "-o", export_path.to_str().unwrap()]));

    let dst_env = TestEnv::new();
    do_setup(&dst_env);
    run_ok(alias_cmd(&dst_env).args(["alias", "import", export_path.to_str().unwrap()]));

    let toml = read_toml(&dst_env);
    assert!(toml.contains("[alias.gs]"));
    assert!(toml.contains("[alias.gp]"));
}

#[test]
fn alias_import_skips_duplicates_without_force() {
    let env = TestEnv::new();
    do_setup(&env);
    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status"]));

    let export_path = env.root.join("export.toml");
    run_ok(alias_cmd(&env).args(["alias", "export", "-o", export_path.to_str().unwrap()]));

    // 再修改 gs
    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git stash", "--force"]));

    // import 不带 --force，gs 应保留 git stash
    run_ok(alias_cmd(&env).args(["alias", "import", export_path.to_str().unwrap()]));

    let toml = read_toml(&env);
    assert!(
        toml.contains("git stash"),
        "import should skip existing entry"
    );
}

#[test]
fn alias_import_force_overwrites_existing() {
    let env = TestEnv::new();
    do_setup(&env);
    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git status"]));

    let export_path = env.root.join("export.toml");
    run_ok(alias_cmd(&env).args(["alias", "export", "-o", export_path.to_str().unwrap()]));

    run_ok(alias_cmd(&env).args(["alias", "add", "gs", "git stash", "--force"]));

    // import --force 应用 export 中的旧值
    run_ok(alias_cmd(&env).args(["alias", "import", export_path.to_str().unwrap(), "--force"]));

    let toml = read_toml(&env);
    assert!(toml.contains("git status"), "force import should overwrite");
}

// ── 名称校验 ──────────────────────────────────────────────────────────────────

#[test]
fn alias_add_rejects_name_with_space() {
    let env = TestEnv::new();
    do_setup(&env);
    let out = alias_cmd(&env)
        .args(["alias", "add", "hello world", "echo hi"])
        .output()
        .unwrap();
    assert!(!out.status.success(), "should reject name with space");
    assert!(
        combined_str(&out).contains("space"),
        "error message should mention space"
    );
}

#[test]
fn alias_add_rejects_name_with_slash() {
    let env = TestEnv::new();
    do_setup(&env);
    let out = alias_cmd(&env)
        .args(["alias", "add", "bad/name", "echo hi"])
        .output()
        .unwrap();
    assert!(!out.status.success(), "should reject name with slash");
    assert!(
        combined_str(&out).contains("invalid character"),
        "error message should mention invalid character"
    );
}

#[test]
fn alias_add_rejects_name_starting_with_dot() {
    let env = TestEnv::new();
    do_setup(&env);
    let out = alias_cmd(&env)
        .args(["alias", "add", ".hidden", "echo hi"])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "should reject name starting with dot"
    );
}

#[test]
fn alias_add_accepts_valid_names() {
    let env = TestEnv::new();
    do_setup(&env);
    run_ok(alias_cmd(&env).args(["alias", "add", "valid-name_123", "echo hi"]));
    run_ok(alias_cmd(&env).args(["alias", "add", "héllo", "echo hi"]));
    let toml = read_toml(&env);
    assert!(toml.contains("valid-name_123"));
    assert!(toml.contains("héllo"));
}

#[test]
fn app_add_rejects_invalid_name() {
    let env = TestEnv::new();
    do_setup(&env);
    let exe = make_fake_exe(&env, "myapp");
    let out = alias_cmd(&env)
        .args([
            "alias",
            "app",
            "add",
            "bad:name",
            exe.to_str().unwrap(),
            "--no-apppaths",
        ])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "app add should reject name with colon"
    );
    assert!(
        combined_str(&out).contains("invalid character"),
        "error message should mention invalid character"
    );
}
#[test]
fn alias_import_force_replaces_existing_app_alias() {
    let env = TestEnv::new();
    do_setup(&env);

    let exe = make_fake_exe(&env, "import_switch_app");
    run_ok(alias_cmd(&env).args([
        "alias",
        "app",
        "add",
        "switcher",
        exe.to_str().unwrap(),
        "--no-apppaths",
    ]));

    let import_path = env.root.join("import_force_shell.toml");
    std::fs::write(
        &import_path,
        "[alias.switcher]\ncommand = \"git status\"\nmode = \"auto\"\n",
    )
    .unwrap();

    run_ok(alias_cmd(&env).args([
        "alias",
        "import",
        import_path.to_str().unwrap(),
        "--force",
    ]));

    let toml = read_toml(&env);
    assert!(toml.contains("[alias.switcher]"));
    assert!(!toml.contains("[app.switcher]"));
}

#[test]
fn alias_import_force_replaces_existing_shell_alias_with_app() {
    let env = TestEnv::new();
    do_setup(&env);

    run_ok(alias_cmd(&env).args(["alias", "add", "switcher", "git status"]));

    let exe = make_fake_exe(&env, "import_switcher_app");
    let import_path = env.root.join("import_force_app.toml");
    std::fs::write(
        &import_path,
        format!(
            "[app.switcher]\nexe = \"{}\"\nregister_apppaths = false\n",
            exe.to_str().unwrap().replace('\\', "\\\\")
        ),
    )
    .unwrap();

    run_ok(alias_cmd(&env).args([
        "alias",
        "import",
        import_path.to_str().unwrap(),
        "--force",
    ]));

    let toml = read_toml(&env);
    assert!(toml.contains("[app.switcher]"));
    assert!(!toml.contains("[alias.switcher]"));
}
