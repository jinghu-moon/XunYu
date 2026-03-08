use crate::common::*;
use std::fs;

#[test]
fn tree_invalid_sort_fails() {
    let env = TestEnv::new();
    let root = env.root.join("tree_sort");
    fs::create_dir_all(&root).unwrap();

    let out = run_err(env.cmd().args([
        "tree",
        root.to_str().unwrap(),
        "--sort",
        "nope",
        "--no-clip",
    ]));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("Invalid sort"));
}

#[test]
fn tree_no_clip_outputs() {
    let env = TestEnv::new();
    let root = env.root.join("tree");
    let sub = root.join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(root.join("file.txt"), "hi").unwrap();

    let out = run_ok(
        env.cmd()
            .args(["tree", root.to_str().unwrap(), "-d", "1", "--no-clip"]),
    );
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("sub/"));
    assert!(s.contains("file.txt"));
}

#[test]
fn tree_output_file_matches_stdout() {
    let env = TestEnv::new();
    let root = env.root.join("tree2");
    let sub = root.join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(root.join("a.txt"), "hi").unwrap();
    fs::write(sub.join("b.txt"), "ok").unwrap();

    let out_path = env.root.join("tree.txt");
    let output = run_ok(env.cmd().args([
        "tree",
        root.to_str().unwrap(),
        "-d",
        "2",
        "-o",
        out_path.to_str().unwrap(),
        "--no-clip",
    ]));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let file = fs::read_to_string(&out_path).unwrap_or_default();

    let out_lines: Vec<&str> = stdout.lines().collect();
    let file_lines: Vec<&str> = file.lines().collect();
    assert_eq!(out_lines, file_lines);
}

#[test]
fn tree_xunignore_excludes_entries() {
    let env = TestEnv::new();
    let root = env.root.join("tree_ignore");
    let sub = root.join("sub");
    let keep = root.join("keep");
    fs::create_dir_all(&sub).unwrap();
    fs::create_dir_all(&keep).unwrap();
    fs::write(root.join(".xunignore"), "sub/\n").unwrap();

    let out = run_ok(
        env.cmd()
            .args(["tree", root.to_str().unwrap(), "--plain", "--no-clip"]),
    );
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(!s.contains("sub/"));
    assert!(s.contains("keep/"));
}

#[test]
fn tree_plain_max_items_limits_output() {
    let env = TestEnv::new();
    let root = env.root.join("tree_limit");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "1").unwrap();
    fs::write(root.join("b.txt"), "2").unwrap();
    fs::write(root.join("c.txt"), "3").unwrap();

    let out = run_ok(env.cmd().args([
        "tree",
        root.to_str().unwrap(),
        "--plain",
        "--max-items",
        "2",
        "--no-clip",
    ]));
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.lines().count() <= 2);
}

#[test]
fn tree_plain_has_no_unicode_box_drawing_chars() {
    let env = TestEnv::new();
    let root = env.root.join("tree_plain_unicode");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "1").unwrap();

    let out = run_ok(
        env.cmd()
            .args(["tree", root.to_str().unwrap(), "--plain", "--no-clip"]),
    );
    let s = String::from_utf8_lossy(&out.stdout);

    // Ensure the plain output doesn't contain box drawing characters like:
    // "├──", "└──", "│".
    assert!(
        !s.contains('\u{251c}'),
        "unexpected unicode branch mid in:\n{s}"
    );
    assert!(
        !s.contains('\u{2514}'),
        "unexpected unicode branch end in:\n{s}"
    );
    assert!(!s.contains('\u{2502}'), "unexpected unicode pipe in:\n{s}");
}

#[test]
fn tree_size_outputs_human_readable_sizes() {
    let env = TestEnv::new();
    let root = env.root.join("tree_size");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("size.txt"), "1234567890").unwrap(); // 10B

    let out = run_ok(env.cmd().args([
        "tree",
        root.to_str().unwrap(),
        "--plain",
        "--size",
        "--no-clip",
    ]));
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(
        s.contains("[10B] size.txt"),
        "expected size output, got: {s}"
    );
}

#[test]
fn tree_stats_only_outputs_stats_without_tree_lines() {
    let env = TestEnv::new();
    let root = env.root.join("tree_stats_only");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "1").unwrap();

    let out = run_ok(
        env.cmd()
            .args(["tree", root.to_str().unwrap(), "--stats-only", "--no-clip"]),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.trim().is_empty(),
        "expected empty stdout, got:\n{stdout}"
    );

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("- Path:"), "unexpected stderr:\n{stderr}");
    assert!(stderr.contains("- Lines:"), "unexpected stderr:\n{stderr}");
    assert!(
        !stderr.contains("a.txt"),
        "tree lines should not be printed"
    );
}

#[test]
fn tree_exclude_pattern_filters_entries() {
    let env = TestEnv::new();
    let root = env.root.join("tree_exclude");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("keep.txt"), "1").unwrap();
    fs::write(root.join("drop.log"), "2").unwrap();

    let out = run_ok(env.cmd().args([
        "tree",
        root.to_str().unwrap(),
        "--plain",
        "--exclude",
        "*.log",
        "--no-clip",
    ]));
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("keep.txt"));
    assert!(!s.contains("drop.log"));
}

#[test]
fn tree_include_pattern_overrides_exclude_pattern() {
    let env = TestEnv::new();
    let root = env.root.join("tree_include");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("keep.log"), "1").unwrap();
    fs::write(root.join("drop.log"), "2").unwrap();

    let out = run_ok(env.cmd().args([
        "tree",
        root.to_str().unwrap(),
        "--plain",
        "--exclude",
        "*.log",
        "--include",
        "keep.log",
        "--no-clip",
    ]));
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("keep.log"));
    assert!(!s.contains("drop.log"));
}

#[test]
fn tree_invalid_path_fails() {
    let env = TestEnv::new();
    let missing = env.root.join("no-such-dir");
    let out = run_err(
        env.cmd()
            .args(["tree", missing.to_str().unwrap(), "--no-clip"]),
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("not a valid directory"));
}
