use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};
use std::process::Command;

#[test]
fn test_cli_help() {
    let mut cmd = Command::new(get_cargo_bin("xun"));
    cmd.arg("--help");
    assert_cmd_snapshot!(cmd);
}

#[test]
fn test_cli_version() {
    let mut cmd = Command::new(get_cargo_bin("xun"));
    cmd.arg("--version");
    insta::with_settings!({
        filters => vec![
            (r"xun \d+\.\d+\.\d+", "xun [VERSION]")
        ]
    }, {
        assert_cmd_snapshot!(cmd);
    });
}

#[test]
fn test_cli_invalid_command() {
    let mut cmd = Command::new(get_cargo_bin("xun"));
    cmd.arg("invalid-command-name");
    assert_cmd_snapshot!(cmd);
}
