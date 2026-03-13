use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

#[derive(Clone, Copy, Debug)]
enum StepStatus {
    Ok,
    Fail,
    Skip,
}

struct StepResult {
    name: String,
    status: StepStatus,
    note: Option<String>,
}

impl StepResult {
    fn ok(name: &str) -> Self {
        Self {
            name: name.to_string(),
            status: StepStatus::Ok,
            note: None,
        }
    }

    fn skip(name: &str, note: &str) -> Self {
        Self {
            name: name.to_string(),
            status: StepStatus::Skip,
            note: Some(note.to_string()),
        }
    }

    fn fail(name: &str, note: &str) -> Self {
        Self {
            name: name.to_string(),
            status: StepStatus::Fail,
            note: Some(note.to_string()),
        }
    }
}

struct TestEnv {
    root: PathBuf,
    local_app_data: PathBuf,
    config_path: PathBuf,
    export_dir: PathBuf,
    xun_path: PathBuf,
}

impl TestEnv {
    fn new() -> Result<Self, String> {
        let root = if let Ok(v) = env::var("XUN_ACL_TEST_ROOT") {
            PathBuf::from(v)
        } else {
            let mut base = env::temp_dir();
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| format!("time error: {e}"))?
                .as_nanos();
            base.push(format!("xun-acl-test-{}-{nanos}", std::process::id()));
            base
        };

        fs::create_dir_all(&root).map_err(|e| format!("create root failed: {e}"))?;
        let local_app_data = root.join("LocalAppData");
        let export_dir = root.join("Exports");
        let config_path = root.join(".xun.config.json");
        fs::create_dir_all(&local_app_data).map_err(|e| format!("create LocalAppData: {e}"))?;
        fs::create_dir_all(&export_dir).map_err(|e| format!("create export dir: {e}"))?;

        let exe = env::current_exe().map_err(|e| format!("current_exe failed: {e}"))?;
        let exe_dir = exe
            .parent()
            .ok_or_else(|| "current_exe has no parent".to_string())?;
        let xun_path = exe_dir.join("xun.exe");
        if !xun_path.exists() {
            return Err(format!(
                "xun.exe not found at {} (build xun first)",
                xun_path.display()
            ));
        }

        Ok(Self {
            root,
            local_app_data,
            config_path,
            export_dir,
            xun_path,
        })
    }

    fn command(&self) -> Command {
        let mut cmd = Command::new(&self.xun_path);
        cmd.env("XUN_CONFIG", &self.config_path);
        cmd.env("USERPROFILE", &self.root);
        cmd.env("HOME", &self.root);
        cmd.env("LOCALAPPDATA", &self.local_app_data);
        cmd.env("XUN_NON_INTERACTIVE", "1");
        cmd.env_remove("XUN_UI");
        cmd
    }

    fn make_dir(&self, name: &str) -> Result<PathBuf, String> {
        let dir = self.root.join(name);
        fs::create_dir_all(&dir).map_err(|e| format!("create dir {name}: {e}"))?;
        fs::write(dir.join("sample.txt"), b"data")
            .map_err(|e| format!("write sample file: {e}"))?;
        Ok(dir)
    }

    fn audit_log_path(&self) -> PathBuf {
        self.local_app_data.join("xun").join("acl_audit.jsonl")
    }
}

fn run_cmd(env: &TestEnv, args: &[String]) -> Result<Output, String> {
    let mut cmd = env.command();
    cmd.args(args);
    cmd.output().map_err(|e| format!("spawn failed: {e}"))
}

fn output_note(out: &Output) -> String {
    let code = out.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let mut note = format!("exit={code}; stderr={stderr}");
    if note.len() > 400 {
        note.truncate(400);
    }
    note
}

fn run_ok<F>(name: &str, env: &TestEnv, args: &[String], check: F) -> StepResult
where
    F: FnOnce(&TestEnv, &Output) -> Result<(), String>,
{
    let out = match run_cmd(env, args) {
        Ok(o) => o,
        Err(e) => return StepResult::fail(name, &e),
    };
    if !out.status.success() {
        return StepResult::fail(name, &output_note(&out));
    }
    if let Err(e) = check(env, &out) {
        return StepResult::fail(name, &e);
    }
    StepResult::ok(name)
}

fn run_expect_err(name: &str, env: &TestEnv, args: &[String], needle: &str) -> StepResult {
    let out = match run_cmd(env, args) {
        Ok(o) => o,
        Err(e) => return StepResult::fail(name, &e),
    };
    if out.status.success() {
        return StepResult::fail(name, "command unexpectedly succeeded");
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    if !stderr.contains(needle) {
        return StepResult::fail(name, &format!("missing expected error: {needle}"));
    }
    StepResult::ok(name)
}

fn read_audit_actions(env: &TestEnv) -> Vec<String> {
    let path = env.audit_log_path();
    let raw = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    raw.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|v| v.get("action").and_then(|a| a.as_str()).map(|s| s.to_string()))
        .collect()
}

fn count_acl_backups(dir: &Path) -> usize {
    let entries = match fs::read_dir(dir) {
        Ok(v) => v,
        Err(_) => return 0,
    };
    entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            path.extension().map(|v| v == "json").unwrap_or(false)
                && e.file_name().to_string_lossy().starts_with("ACL_")
        })
        .count()
}

#[allow(deprecated)]
fn is_admin() -> bool {
    unsafe { windows_sys::Win32::UI::Shell::IsUserAnAdmin() != 0 }
}

fn main() {
    let env = match TestEnv::new() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("acl_test init failed: {e}");
            std::process::exit(2);
        }
    };

    println!("ACL test root: {}", env.root.display());
    println!("Xun path: {}", env.xun_path.display());

    let dir_a = env.make_dir("acl_a").unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let dir_b = env.make_dir("acl_b").unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });

    let mut results: Vec<StepResult> = Vec::new();

    results.push(run_ok(
        "acl view",
        &env,
        &vec![
            "acl".to_string(),
            "view".to_string(),
            "-p".to_string(),
            dir_a.to_string_lossy().into_owned(),
        ],
        |_, _| Ok(()),
    ));

    results.push(run_ok(
        "acl view detail",
        &env,
        &vec![
            "acl".to_string(),
            "view".to_string(),
            "-p".to_string(),
            dir_a.to_string_lossy().into_owned(),
            "--detail".to_string(),
        ],
        |_, _| Ok(()),
    ));

    let export_acl = env.root.join("acl_view.csv");
    results.push(run_ok(
        "acl view export",
        &env,
        &vec![
            "acl".to_string(),
            "view".to_string(),
            "-p".to_string(),
            dir_a.to_string_lossy().into_owned(),
            "--export".to_string(),
            export_acl.to_string_lossy().into_owned(),
        ],
        |_, _| {
            if export_acl.exists() {
                Ok(())
            } else {
                Err("export file not created".to_string())
            }
        },
    ));

    let export_diff = env.root.join("acl_diff.csv");
    results.push(run_ok(
        "acl diff export",
        &env,
        &vec![
            "acl".to_string(),
            "diff".to_string(),
            "-p".to_string(),
            dir_a.to_string_lossy().into_owned(),
            "-r".to_string(),
            dir_b.to_string_lossy().into_owned(),
            "-o".to_string(),
            export_diff.to_string_lossy().into_owned(),
        ],
        |_, _| {
            if export_diff.exists() {
                Ok(())
            } else {
                Err("diff export not created".to_string())
            }
        },
    ));

    results.push(run_ok(
        "acl effective",
        &env,
        &vec![
            "acl".to_string(),
            "effective".to_string(),
            "-p".to_string(),
            dir_a.to_string_lossy().into_owned(),
        ],
        |_, _| Ok(()),
    ));

    results.push(run_ok(
        "acl config set",
        &env,
        &vec![
            "acl".to_string(),
            "config".to_string(),
            "--set".to_string(),
            "throttle_limit".to_string(),
            "8".to_string(),
        ],
        |env, _| {
            let raw = fs::read_to_string(&env.config_path)
                .map_err(|e| format!("read config failed: {e}"))?;
            let v: Value = serde_json::from_str(&raw)
                .map_err(|e| format!("parse config failed: {e}"))?;
            let cur = v
                .get("acl")
                .and_then(|a| a.get("throttle_limit"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            if cur == 8 {
                Ok(())
            } else {
                Err(format!("unexpected throttle_limit: {cur}"))
            }
        },
    ));

    results.push(run_ok(
        "acl orphans",
        &env,
        &vec![
            "acl".to_string(),
            "orphans".to_string(),
            "-p".to_string(),
            dir_a.to_string_lossy().into_owned(),
            "--action".to_string(),
            "none".to_string(),
        ],
        |_, _| Ok(()),
    ));

    results.push(run_ok(
        "acl add",
        &env,
        &vec![
            "acl".to_string(),
            "add".to_string(),
            "-p".to_string(),
            dir_a.to_string_lossy().into_owned(),
            "--principal".to_string(),
            "BUILTIN\\Users".to_string(),
            "--rights".to_string(),
            "Read".to_string(),
            "--ace-type".to_string(),
            "Allow".to_string(),
            "--inherit".to_string(),
            "None".to_string(),
            "-y".to_string(),
        ],
        |env, _| {
            let actions = read_audit_actions(env);
            if actions.iter().any(|a| a == "AddPermission") {
                Ok(())
            } else {
                Err("missing AddPermission audit entry".to_string())
            }
        },
    ));

    results.push(run_ok(
        "acl purge",
        &env,
        &vec![
            "acl".to_string(),
            "purge".to_string(),
            "-p".to_string(),
            dir_a.to_string_lossy().into_owned(),
            "--principal".to_string(),
            "BUILTIN\\Users".to_string(),
            "-y".to_string(),
        ],
        |env, _| {
            let actions = read_audit_actions(env);
            if actions.iter().any(|a| a == "PurgePrincipal") {
                Ok(())
            } else {
                Err("missing PurgePrincipal audit entry".to_string())
            }
        },
    ));

    results.push(run_ok(
        "acl inherit disable",
        &env,
        &vec![
            "acl".to_string(),
            "inherit".to_string(),
            "-p".to_string(),
            dir_a.to_string_lossy().into_owned(),
            "--disable".to_string(),
        ],
        |env, _| {
            let actions = read_audit_actions(env);
            if actions.iter().any(|a| a == "SetInheritance") {
                Ok(())
            } else {
                Err("missing SetInheritance audit entry".to_string())
            }
        },
    ));

    results.push(run_ok(
        "acl copy",
        &env,
        &vec![
            "acl".to_string(),
            "copy".to_string(),
            "-p".to_string(),
            dir_b.to_string_lossy().into_owned(),
            "-r".to_string(),
            dir_a.to_string_lossy().into_owned(),
            "-y".to_string(),
        ],
        |env, _| {
            let actions = read_audit_actions(env);
            if actions.iter().any(|a| a == "CopyAcl") {
                Ok(())
            } else {
                Err("missing CopyAcl audit entry".to_string())
            }
        },
    ));

    let backup = env.root.join("acl_backup.json");
    results.push(run_ok(
        "acl backup",
        &env,
        &vec![
            "acl".to_string(),
            "backup".to_string(),
            "-p".to_string(),
            dir_a.to_string_lossy().into_owned(),
            "-o".to_string(),
            backup.to_string_lossy().into_owned(),
        ],
        |_, _| {
            if backup.exists() {
                Ok(())
            } else {
                Err("backup file not created".to_string())
            }
        },
    ));

    results.push(run_ok(
        "acl restore",
        &env,
        &vec![
            "acl".to_string(),
            "restore".to_string(),
            "-p".to_string(),
            dir_b.to_string_lossy().into_owned(),
            "--from".to_string(),
            backup.to_string_lossy().into_owned(),
            "-y".to_string(),
        ],
        |env, _| {
            let actions = read_audit_actions(env);
            if actions.iter().any(|a| a == "RestoreAcl") {
                Ok(())
            } else {
                Err("missing RestoreAcl audit entry".to_string())
            }
        },
    ));

    let batch_paths = format!(
        "{},{}",
        dir_a.to_string_lossy(),
        dir_b.to_string_lossy()
    );
    results.push(run_ok(
        "acl batch backup",
        &env,
        &vec![
            "acl".to_string(),
            "batch".to_string(),
            "--paths".to_string(),
            batch_paths,
            "--action".to_string(),
            "backup".to_string(),
            "--output".to_string(),
            env.export_dir.to_string_lossy().into_owned(),
            "-y".to_string(),
        ],
        |env, _| {
            let count = count_acl_backups(&env.export_dir);
            if count >= 2 {
                Ok(())
            } else {
                Err(format!("expected backups, found {count}"))
            }
        },
    ));

    let audit_export = env.root.join("acl_audit.csv");
    results.push(run_ok(
        "acl audit export",
        &env,
        &vec![
            "acl".to_string(),
            "audit".to_string(),
            "--export".to_string(),
            audit_export.to_string_lossy().into_owned(),
        ],
        |_, _| {
            if audit_export.exists() {
                Ok(())
            } else {
                Err("audit export not created".to_string())
            }
        },
    ));

    results.push(run_expect_err(
        "acl remove (non-interactive)",
        &env,
        &vec![
            "acl".to_string(),
            "remove".to_string(),
            "-p".to_string(),
            dir_a.to_string_lossy().into_owned(),
        ],
        "requires interactive mode",
    ));

    if is_admin() {
        results.push(run_ok(
            "acl owner (admin)",
            &env,
            &vec![
                "acl".to_string(),
                "owner".to_string(),
                "-p".to_string(),
                dir_a.to_string_lossy().into_owned(),
                "--set".to_string(),
                "BUILTIN\\Administrators".to_string(),
                "-y".to_string(),
            ],
            |env, out| {
                let stderr = String::from_utf8_lossy(&out.stderr);
                if stderr.contains("Owner unchanged.") {
                    return Ok(());
                }
                let actions = read_audit_actions(env);
                if actions.iter().any(|a| a == "SetOwner") {
                    Ok(())
                } else {
                    Err("missing SetOwner audit entry".to_string())
                }
            },
        ));

        results.push(run_ok(
            "acl repair (admin)",
            &env,
            &vec![
                "acl".to_string(),
                "repair".to_string(),
                "-p".to_string(),
                dir_a.to_string_lossy().into_owned(),
                "-y".to_string(),
            ],
            |env, _| {
                let actions = read_audit_actions(env);
                if actions.iter().any(|a| a == "ForceRepair") {
                    Ok(())
                } else {
                    Err("missing ForceRepair audit entry".to_string())
                }
            },
        ));
    } else {
        results.push(StepResult::skip(
            "acl owner (admin)",
            "not running as admin",
        ));
        results.push(StepResult::skip(
            "acl repair (admin)",
            "not running as admin",
        ));
    }

    println!("\nACL test summary:");
    let mut ok = 0usize;
    let mut fail = 0usize;
    let mut skip = 0usize;
    for r in &results {
        match r.status {
            StepStatus::Ok => {
                ok += 1;
                println!("[OK]   {}", r.name);
            }
            StepStatus::Skip => {
                skip += 1;
                println!("[SKIP] {} - {}", r.name, r.note.as_deref().unwrap_or(""));
            }
            StepStatus::Fail => {
                fail += 1;
                println!("[FAIL] {} - {}", r.name, r.note.as_deref().unwrap_or(""));
            }
        }
    }
    println!(
        "\nTotal: {}  Ok: {}  Fail: {}  Skip: {}",
        results.len(),
        ok,
        fail,
        skip
    );
    println!("Root: {}", env.root.display());

    if fail > 0 {
        std::process::exit(1);
    }
}
