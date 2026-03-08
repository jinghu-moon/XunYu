#![cfg(all(windows, feature = "redirect"))]

mod common;

use common::*;
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::process::Stdio;

fn write_basic_redirect_config(env: &TestEnv) {
    let cfg = r#"
{
  "redirect": {
    "profiles": {
      "default": {
        "rules": [
          { "name": "Images", "match": { "ext": ["jpg","png"] }, "dest": "./Images" },
          { "name": "Reports", "match": { "glob": "report_*" }, "dest": "./Reports" }
        ],
        "unmatched": "skip",
        "on_conflict": "rename_new"
      }
    }
  }
}
"#;
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();
}

fn parse_last_tx_from_audit(env: &TestEnv) -> String {
    let log = fs::read_to_string(env.audit_path()).unwrap_or_default();
    for line in log.lines().rev() {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let action = v.get("action").and_then(Value::as_str).unwrap_or("");
        if !action.starts_with("redirect_") {
            continue;
        }
        let params = v.get("params").and_then(Value::as_str).unwrap_or("");
        if let Some(idx) = params.find("tx=") {
            let rest = &params[(idx + 3)..];
            let end = rest.find(' ').unwrap_or(rest.len());
            let tx = rest[..end].trim();
            if !tx.is_empty() {
                return tx.to_string();
            }
        }
    }
    panic!("missing tx in audit log");
}

#[test]
fn redirect_explain_prints_rule_details_without_real_file() {
    let env = TestEnv::new();
    write_basic_redirect_config(&env);

    let out = run_ok(env.cmd().args([
        "redirect",
        "--explain",
        "2026-02_report.jpg",
        "--format",
        "tsv",
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Rule \"Images\":"));
    assert!(stderr.contains("ext=jpg matched"));
    assert!(stderr.contains("Rule \"Reports\":"));
    assert!(stderr.contains("glob=\"report_*\""));
    assert!(stderr.contains("Result: would match"));
}

#[test]
fn redirect_stats_prints_coverage_summary_to_stderr() {
    let env = TestEnv::new();
    write_basic_redirect_config(&env);

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.jpg"), "img").unwrap();

    let out = run_ok(env.cmd().args([
        "redirect",
        src.to_str().unwrap(),
        "--stats",
        "--format",
        "tsv",
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Rules coverage:"));
    assert!(stderr.contains("Images:"));
}

#[test]
fn redirect_dry_run_prints_summary_to_stderr() {
    let env = TestEnv::new();
    write_basic_redirect_config(&env);

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.jpg"), "img").unwrap();

    let out = run_ok(env.cmd().args([
        "redirect",
        src.to_str().unwrap(),
        "--dry-run",
        "--format",
        "tsv",
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Dry run complete:"));
    assert!(stderr.contains("Run without --dry-run to execute."));
}

#[test]
fn redirect_table_shows_reason_column_when_verbose() {
    let env = TestEnv::new();

    let cfg = r#"
{
  "redirect": {
    "profiles": {
      "default": {
        "rules": [
          { "name": "Images", "match": { "ext": ["jpg"] }, "dest": "C:\\Windows\\Temp\\XunTest" }
        ],
        "unmatched": "skip",
        "on_conflict": "rename_new"
      }
    }
  }
}
"#;
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.jpg"), "img").unwrap();

    let out = run_ok(env.cmd().args([
        "--no-color",
        "-v",
        "redirect",
        src.to_str().unwrap(),
        "--format",
        "table",
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Reason"));
    assert!(stderr.to_ascii_lowercase().contains("unsafe_dest"));
}

#[test]
fn redirect_missing_source_error_includes_hint() {
    let env = TestEnv::new();
    write_basic_redirect_config(&env);

    let out = run_err(
        env.cmd()
            .args(["redirect", env.root.join("missing").to_str().unwrap()]),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Error: Source directory not found"));
    assert!(stderr.contains("Hint:"));
}

#[test]
fn redirect_invalid_format_suggests_did_you_mean() {
    let env = TestEnv::new();
    write_basic_redirect_config(&env);

    let out = run_err(env.cmd().args(["redirect", "--log", "--format", "jsn"]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Error: Invalid format: jsn"));
    assert!(stderr.contains("Did you mean: \"json\"?"));
}

#[test]
fn redirect_profile_typo_suggests_did_you_mean() {
    let env = TestEnv::new();
    write_basic_redirect_config(&env);

    let out = run_err(
        env.cmd()
            .args(["redirect", "--profile", "defualt", "--validate"]),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Error: Redirect profile not found"));
    assert!(stderr.contains("Did you mean: \"default\"?"));
}

#[test]
fn redirect_on_conflict_typo_suggests_did_you_mean() {
    let env = TestEnv::new();

    let cfg = r#"
{
  "redirect": {
    "profiles": {
      "default": {
        "rules": [
          { "name": "Images", "match": { "ext": ["jpg"] }, "dest": "./Images" }
        ],
        "unmatched": "skip",
        "on_conflict": "renmae_new"
      }
    }
  }
}
"#;
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();

    let out = run_err(env.cmd().args(["redirect", "--validate"]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Unsupported on_conflict"));
    assert!(stderr.contains("Did you mean: \"rename_new\"?"));
}

#[test]
fn redirect_log_returns_recent_tx_summaries_as_json() {
    let env = TestEnv::new();
    write_basic_redirect_config(&env);

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.jpg"), "img").unwrap();
    run_ok(
        env.cmd()
            .args(["redirect", src.to_str().unwrap(), "--format", "tsv"]),
    );

    let last_tx = parse_last_tx_from_audit(&env);
    let out = run_ok(
        env.cmd()
            .args(["redirect", "--log", "--last", "5", "--format", "json"]),
    );
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let arr = v.as_array().unwrap();
    assert!(
        arr.iter()
            .any(|x| x.get("tx").and_then(Value::as_str) == Some(&last_tx)),
        "expected tx to appear in --log results"
    );
}

#[test]
fn redirect_validate_exits_ok_for_valid_profile() {
    let env = TestEnv::new();
    write_basic_redirect_config(&env);

    run_ok(env.cmd().args(["redirect", "--validate"]));
}

#[test]
fn redirect_confirm_runs_in_non_interactive_with_yes() {
    let env = TestEnv::new();
    write_basic_redirect_config(&env);

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.jpg"), "img").unwrap();

    let out = run_ok(env.cmd().args([
        "redirect",
        src.to_str().unwrap(),
        "--confirm",
        "--yes",
        "--format",
        "tsv",
    ]));
    assert!(src.join("Images").join("a.jpg").exists());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Preview:"));
}

#[test]
fn redirect_review_requires_interactive_mode() {
    let env = TestEnv::new();
    write_basic_redirect_config(&env);

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.jpg"), "img").unwrap();

    let out = run_err(
        env.cmd()
            .args(["redirect", src.to_str().unwrap(), "--review"]),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Error: --review requires interactive mode."));
}

#[test]
fn redirect_simulate_reads_names_from_stdin() {
    let env = TestEnv::new();
    write_basic_redirect_config(&env);

    let mut cmd = env.cmd();
    cmd.args(["redirect", "--simulate", "--format", "tsv"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"a.jpg\nreport_2026.pdf\nrandom.xyz\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("a.jpg\tImages\t"));
    assert!(stdout.contains("report_2026.pdf\tReports\t"));
    assert!(stdout.contains("random.xyz\t(none)\t"));
}

#[test]
fn redirect_watch_status_reads_status_file_in_source_dir() {
    let env = TestEnv::new();
    write_basic_redirect_config(&env);

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();

    let status_path = src.join(".xun_watch_status.json");
    fs::write(
        &status_path,
        r#"{
  "pid": 1234,
  "tx": "redirect_1_2",
  "profile": "default",
  "source": "C:\\\\tmp",
  "started_ts": 1,
  "last_scan_ts": 2,
  "batches": 3,
  "events_processed": 4,
  "retry_queue": ["locked.docx"],
  "errors": 0
}"#,
    )
    .unwrap();

    let out = run_ok(env.cmd().args([
        "redirect",
        src.to_str().unwrap(),
        "--watch",
        "--status",
        "--format",
        "json",
    ]));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v.get("pid").and_then(Value::as_u64), Some(1234));
    assert_eq!(v.get("tx").and_then(Value::as_str), Some("redirect_1_2"));
}

#[test]
fn redirect_plan_writes_plan_file_without_side_effects() {
    let env = TestEnv::new();
    write_basic_redirect_config(&env);

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.jpg"), "img").unwrap();

    let plan_path = env.root.join("plan.json");
    run_ok(env.cmd().args([
        "redirect",
        src.to_str().unwrap(),
        "--plan",
        plan_path.to_str().unwrap(),
        "--format",
        "tsv",
    ]));

    assert!(plan_path.exists());
    assert!(src.join("a.jpg").exists(), "plan should not move files");
    assert!(!src.join("Images").join("a.jpg").exists());

    let v: Value = serde_json::from_str(&fs::read_to_string(&plan_path).unwrap()).unwrap();
    assert_eq!(v.get("version").and_then(Value::as_u64), Some(1));
    assert_eq!(v.get("profile").and_then(Value::as_str), Some("default"));
    let items = v.get("items").and_then(Value::as_array).unwrap();
    assert_eq!(items.len(), 1);
    assert!(
        items[0]
            .get("src")
            .and_then(Value::as_str)
            .unwrap()
            .ends_with("a.jpg")
    );
}

#[test]
fn redirect_apply_executes_plan_and_skips_stale_items() {
    let env = TestEnv::new();
    write_basic_redirect_config(&env);

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();
    let src_file = src.join("a.jpg");
    fs::write(&src_file, "img").unwrap();

    let plan_path = env.root.join("plan.json");
    run_ok(env.cmd().args([
        "redirect",
        src.to_str().unwrap(),
        "--plan",
        plan_path.to_str().unwrap(),
        "--format",
        "tsv",
    ]));

    fs::write(&src_file, "img-changed-and-longer").unwrap();

    let out = run_ok(env.cmd().args([
        "redirect",
        "--apply",
        plan_path.to_str().unwrap(),
        "--format",
        "tsv",
    ]));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("\tskipped\tstale") || stdout.contains("\tskipped\tstale\r\n"),
        "expected apply to skip stale item, got stdout={stdout:?}"
    );
    assert!(src_file.exists(), "stale item should not be moved");
}
