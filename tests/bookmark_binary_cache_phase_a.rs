#![cfg(windows)]

#[path = "support/mod.rs"]
mod common;

use common::{TestEnv, run_ok};

fn stdout_text(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n")
}

#[test]
fn binary_cache_is_internal_acceleration_layer_only() {
    let env = TestEnv::new();

    let bookmark_help = run_ok(env.cmd().args(["bookmark", "--help"]));
    let bookmark_stdout = stdout_text(&bookmark_help);
    assert!(
        !bookmark_stdout.contains("\n  cache"),
        "bookmark help unexpectedly exposes cache command:\n{bookmark_stdout}"
    );

    let root_help = run_ok(env.cmd().arg("--help"));
    let root_stdout = stdout_text(&root_help);
    assert!(
        !root_stdout.contains("\n  cache"),
        "top-level help unexpectedly exposes cache command:\n{root_stdout}"
    );
}
