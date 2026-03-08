#![cfg(all(windows, feature = "diff"))]

mod common;

use crate::common::*;
use std::fs;
use std::path::Path;

fn write_file(path: &Path, content: &str) {
    fs::write(path, content).unwrap();
}

#[test]
fn diff_strip_trailing_cr_reports_identical() {
    let env = TestEnv::new();
    let old = env.root.join("old.txt");
    let new = env.root.join("new.txt");

    write_file(&old, "a\r\nb\r\n");
    write_file(&new, "a\nb\n");

    let out = run_ok(env.cmd().args([
        "diff",
        old.to_str().unwrap(),
        new.to_str().unwrap(),
        "--strip-trailing-cr",
    ]));

    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("Files are identical"));
    assert!(err.contains("ignoring whitespace"));
}

#[test]
fn diff_ignore_blank_lines_treats_empty_as_identical() {
    let env = TestEnv::new();
    let old = env.root.join("old.txt");
    let new = env.root.join("new.txt");

    write_file(&old, "\n\n");
    write_file(&new, "");

    let out = run_ok(env.cmd().args([
        "diff",
        old.to_str().unwrap(),
        new.to_str().unwrap(),
        "--ignore-blank-lines",
    ]));

    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("Files are identical"));
    assert!(err.contains("ignoring whitespace"));
}

#[test]
fn diff_vue_multiline_open_tag_parsed() {
    let env = TestEnv::new();
    let old = env.root.join("old.vue");
    let new = env.root.join("new.vue");

    let old_src = r#"<script
lang="ts"
>
function foo() {
return 1
}
</script>
"#;
    let new_src = r#"<script
lang="ts"
>
function foo() {
return 2
}
</script>
"#;
    write_file(&old, old_src);
    write_file(&new, new_src);

    let out = run_ok(
        env.cmd()
            .args(["diff", old.to_str().unwrap(), new.to_str().unwrap()]),
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("script::foo"),
        "expected script section in output"
    );
    assert!(err.contains("-return 1"), "expected removed line in output");
    assert!(err.contains("+return 2"), "expected added line in output");
}

#[test]
fn diff_vue_ast_added_removed_include_lines() {
    let env = TestEnv::new();
    let old = env.root.join("old.vue");
    let new = env.root.join("new.vue");

    let old_src = r#"<script
lang="ts"
>
function foo() {
return 1
}
</script>
"#;
    let new_src = r#"<script
lang="ts"
>
function bar() {
return 2
}
</script>
"#;
    write_file(&old, old_src);
    write_file(&new, new_src);

    let out = run_ok(
        env.cmd()
            .args(["diff", old.to_str().unwrap(), new.to_str().unwrap()]),
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("-function foo()"),
        "expected removed function line"
    );
    assert!(
        err.contains("+function bar()"),
        "expected added function line"
    );
}
