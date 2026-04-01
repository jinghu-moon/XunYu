#![cfg(windows)]

#[path = "support/mod.rs"]
mod common;

use common::{run_err, run_ok, TestEnv};

fn stdout_text(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n")
}

#[test]
fn bookmark_subcommand_is_registered() {
    let env = TestEnv::new();
    let out = run_ok(env.cmd().arg("bookmark").arg("--help"));
    let stdout = stdout_text(&out);
    for expected in [
        "z",
        "zi",
        "o",
        "oi",
        "set",
        "save",
        "delete",
        "tag",
        "pin",
        "unpin",
        "rename",
        "list",
        "recent",
        "stats",
        "check",
        "gc",
        "dedup",
        "export",
        "import",
        "init",
        "touch",
    ] {
        assert!(
            stdout.contains(&format!("\n  {expected}")),
            "missing subcommand {expected} in:\n{stdout}"
        );
    }
}

#[test]
fn top_level_help_hides_legacy_bookmark_commands() {
    let env = TestEnv::new();
    let out = run_ok(env.cmd().arg("--help"));
    let stdout = stdout_text(&out);
    assert!(stdout.contains("\n  bookmark"));
    for legacy in ["\n  z ", "\n  open ", "\n  ws ", "\n  save ", "\n  fuzzy "] {
        assert!(
            !stdout.contains(legacy),
            "legacy command still visible: {legacy}\n{stdout}"
        );
    }
}

#[test]
fn workspace_subcommand_is_absent() {
    let env = TestEnv::new();
    let out = run_err(env.cmd().arg("bookmark").arg("workspace").arg("--help"));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("Run xun --help") || stderr.contains("unrecognized"),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn bash_completion_uses_bookmark_namespace() {
    let env = TestEnv::new();
    let out = run_ok(env.cmd().arg("completion").arg("bash"));
    let stdout = stdout_text(&out);
    assert!(stdout.contains("local subcommands=\"bookmark"));
    assert!(stdout.contains("local bookmark_sub=\"z zi o oi open save"));
    assert!(!stdout.contains("local subcommands=\"init completion config ctx list z open ws save"));
}
