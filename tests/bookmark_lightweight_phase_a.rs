#![cfg(windows)]

#[path = "support/mod.rs"]
mod common;

use common::{TestEnv, run_ok};

fn stdout_text(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n")
}

#[test]
fn lightweight_view_is_internal_acceleration_layer_only() {
    let env = TestEnv::new();

    let bookmark_help = run_ok(env.cmd().args(["bookmark", "--help"]));
    let bookmark_stdout = stdout_text(&bookmark_help).to_ascii_lowercase();
    assert!(!bookmark_stdout.contains("lightweight"));
    assert!(!bookmark_stdout.contains("runtime view"));
    assert!(!bookmark_stdout.contains("archived"));

    let root_help = run_ok(env.cmd().arg("--help"));
    let root_stdout = stdout_text(&root_help).to_ascii_lowercase();
    assert!(!root_stdout.contains("lightweight"));
    assert!(!root_stdout.contains("runtime view"));
    assert!(!root_stdout.contains("archived"));
}
