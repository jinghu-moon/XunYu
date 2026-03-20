// tests/test_brn.rs
//
// Integration tests for `xun brn` batch rename — TDD Ph0

#![cfg(feature = "batch_rename")]

use std::fs;
use std::path::Path;

use tempfile::TempDir;

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn make_files(dir: &Path, names: &[&str]) {
    for name in names {
        fs::write(dir.join(name), b"").unwrap();
    }
}

fn file_exists(dir: &Path, name: &str) -> bool {
    dir.join(name).exists()
}

use xun::batch_rename::testing as brn;
#[allow(unused_imports)]
use xun::batch_rename::{compute::RenameMode, types::CaseStyle};

// ─── Ph0-1: 两阶段执行框架 ───────────────────────────────────────────────────

#[test]
fn ph0_1_empty_dir_returns_empty_ops() {
    let dir = TempDir::new().unwrap();
    let files = brn::collect_files(dir.path().to_str().unwrap(), &[], false).unwrap();
    assert!(files.is_empty(), "empty dir should yield no files");
}

#[test]
fn ph0_1_single_file_generates_correct_op() {
    let dir = TempDir::new().unwrap();
    make_files(dir.path(), &["hello world.txt"]);

    let files = brn::collect_files(dir.path().to_str().unwrap(), &[], false).unwrap();
    assert_eq!(files.len(), 1);

    let mode = xun::batch_rename::compute::RenameMode::Case(
        xun::batch_rename::types::CaseStyle::Kebab,
    );
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops.len(), 1);
    let to_name = ops[0].to.file_name().unwrap().to_str().unwrap();
    assert_eq!(to_name, "hello-world.txt");
}

#[test]
fn ph0_1_dryrun_does_not_rename_files() {
    let dir = TempDir::new().unwrap();
    make_files(dir.path(), &["hello world.txt"]);

    let files = brn::collect_files(dir.path().to_str().unwrap(), &[], false).unwrap();
    let mode = xun::batch_rename::compute::RenameMode::Case(
        xun::batch_rename::types::CaseStyle::Kebab,
    );
    let _ = brn::compute_ops(&files, &mode).unwrap();
    // dry-run: do NOT apply
    assert!(file_exists(dir.path(), "hello world.txt"), "original must still exist after dry-run");
    assert!(!file_exists(dir.path(), "hello-world.txt"), "renamed must not exist after dry-run");
}

#[test]
fn ph0_1_apply_actually_renames_file() {
    let dir = TempDir::new().unwrap();
    make_files(dir.path(), &["hello world.txt"]);

    let files = brn::collect_files(dir.path().to_str().unwrap(), &[], false).unwrap();
    let mode = xun::batch_rename::compute::RenameMode::Case(
        xun::batch_rename::types::CaseStyle::Kebab,
    );
    let ops = brn::compute_ops(&files, &mode).unwrap();

    for op in &ops {
        fs::rename(&op.from, &op.to).unwrap();
    }

    assert!(!file_exists(dir.path(), "hello world.txt"), "original must be gone");
    assert!(file_exists(dir.path(), "hello-world.txt"), "renamed must exist");
}

#[test]
fn ph0_1_preflight_conflict_prevents_rename() {
    use xun::batch_rename::conflict::detect_conflicts;
    use xun::batch_rename::types::RenameOp;
    use std::path::PathBuf;

    let dir = TempDir::new().unwrap();
    // Construct two ops manually that both map to the same target
    // (simulates two distinct source files both becoming "hello-world.txt")
    let target = dir.path().join("hello-world.txt");
    let ops = vec![
        RenameOp { from: dir.path().join("hello world.txt"), to: target.clone() },
        RenameOp { from: dir.path().join("Hello World.txt"), to: target.clone() },
    ];
    let conflicts = detect_conflicts(&ops, false);
    assert!(!conflicts.is_empty(), "duplicate target must be detected");
}

// ─── Ph1-1: --from/--to 字面量替换 ──────────────────────────────────────────

#[test]
fn ph1_1_single_replace_space_to_underscore() {
    use xun::batch_rename::compute::{RenameMode, ReplacePair};
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/my file.txt")];
    let mode = RenameMode::Replace(vec![ReplacePair { from: " ".into(), to: "_".into() }]);
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "my_file.txt");
}

#[test]
fn ph1_1_multiple_pairs_applied_in_order() {
    use xun::batch_rename::compute::{RenameMode, ReplacePair};
    use std::path::PathBuf;

    // "my file (2024).txt" -> "my_file_2024.txt"
    let files = vec![PathBuf::from("/dir/my file (2024).txt")];
    let mode = RenameMode::Replace(vec![
        ReplacePair { from: " ".into(), to: "_".into() },
        ReplacePair { from: "(".into(), to: "".into() },
        ReplacePair { from: ")".into(), to: "".into() },
    ]);
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "my_file_2024.txt");
}

#[test]
fn ph1_1_empty_to_deletes_match() {
    use xun::batch_rename::compute::{RenameMode, ReplacePair};
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/file(copy).txt")];
    let mode = RenameMode::Replace(vec![
        ReplacePair { from: "(".into(), to: "".into() },
        ReplacePair { from: "copy".into(), to: "".into() },
        ReplacePair { from: ")".into(), to: "".into() },
    ]);
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "file.txt");
}

#[test]
fn ph1_1_no_match_is_noop() {
    use xun::batch_rename::compute::{RenameMode, ReplacePair};
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/hello.txt")];
    let mode = RenameMode::Replace(vec![ReplacePair { from: "xyz".into(), to: "abc".into() }]);
    let ops = brn::compute_ops(&files, &mode).unwrap();
    // from == to (no-op), will be filtered by the pipeline
    assert_eq!(
        ops[0].from.file_name().unwrap().to_str().unwrap(),
        ops[0].to.file_name().unwrap().to_str().unwrap(),
        "no match must produce from==to noop"
    );
}

#[test]
fn ph1_1_replace_only_affects_stem_not_extension() {
    use xun::batch_rename::compute::{RenameMode, ReplacePair};
    use std::path::PathBuf;

    // Replace "txt" in stem — extension must not be touched
    let files = vec![PathBuf::from("/dir/my_txt_file.txt")];
    let mode = RenameMode::Replace(vec![ReplacePair { from: "txt".into(), to: "doc".into() }]);
    let ops = brn::compute_ops(&files, &mode).unwrap();
    let result = ops[0].to.file_name().unwrap().to_str().unwrap();
    // stem changes, extension stays
    assert_eq!(result, "my_doc_file.txt");
}


#[test]
fn ph0_5_case_only_change_injects_tmp() {
    use xun::batch_rename::ntfs_case::normalize_case_ops;
    use xun::batch_rename::types::RenameOp;
    use std::path::PathBuf;

    // photo.JPG -> photo.jpg — only case differs
    let ops = vec![RenameOp {
        from: PathBuf::from("photo.JPG"),
        to: PathBuf::from("photo.jpg"),
    }];
    let result = normalize_case_ops(ops);
    assert_eq!(result.len(), 2, "case-only rename must expand to 2 ops (via tmp)");
    // First op: original -> tmp
    assert_ne!(result[0].to, PathBuf::from("photo.jpg"), "first step must go to tmp");
    // Second op: tmp -> final
    assert_eq!(result[1].to, PathBuf::from("photo.jpg"), "second step must reach final target");
}

#[test]
fn ph0_5_different_name_no_tmp_injected() {
    use xun::batch_rename::ntfs_case::normalize_case_ops;
    use xun::batch_rename::types::RenameOp;
    use std::path::PathBuf;

    // photo.JPG -> other.jpg — not a case-only change
    let ops = vec![RenameOp {
        from: PathBuf::from("photo.JPG"),
        to: PathBuf::from("other.jpg"),
    }];
    let result = normalize_case_ops(ops);
    assert_eq!(result.len(), 1, "non-case-only rename must not inject tmp");
}

#[test]
fn ph0_5_completely_different_name_no_tmp() {
    use xun::batch_rename::ntfs_case::normalize_case_ops;
    use xun::batch_rename::types::RenameOp;
    use std::path::PathBuf;

    let ops = vec![RenameOp {
        from: PathBuf::from("hello.txt"),
        to: PathBuf::from("world.txt"),
    }];
    let result = normalize_case_ops(ops);
    assert_eq!(result.len(), 1);
}

// ─── Ph0-6: 文件扫描与自然排序 ───────────────────────────────────────────────

#[test]
fn ph0_6_natural_sort_numbers() {
    use xun::batch_rename::natural_sort::natural_cmp;

    let mut names = vec!["track_10", "track_2", "track_1", "track_20"];
    names.sort_by(|a, b| natural_cmp(a, b));
    assert_eq!(names, ["track_1", "track_2", "track_10", "track_20"]);
}

#[test]
fn ph0_6_natural_sort_mixed() {
    use xun::batch_rename::natural_sort::natural_cmp;

    let mut names = vec!["file10", "file2", "file1", "file20"];
    names.sort_by(|a, b| natural_cmp(a, b));
    assert_eq!(names, ["file1", "file2", "file10", "file20"]);
}

#[test]
fn ph0_6_collect_returns_files_only() {
    use std::fs;
    let dir = TempDir::new().unwrap();
    make_files(dir.path(), &["a.txt", "b.txt"]);
    fs::create_dir(dir.path().join("subdir")).unwrap();

    let files = brn::collect_files(dir.path().to_str().unwrap(), &[], false).unwrap();
    assert_eq!(files.len(), 2, "must return files only, not dirs");
    assert!(files.iter().all(|f| f.is_file()));
}

#[test]
fn ph0_6_collect_recursive() {
    use std::fs;
    let dir = TempDir::new().unwrap();
    make_files(dir.path(), &["a.txt"]);
    fs::create_dir(dir.path().join("sub")).unwrap();
    fs::write(dir.path().join("sub").join("b.txt"), b"").unwrap();

    let files = brn::collect_files(dir.path().to_str().unwrap(), &[], true).unwrap();
    assert_eq!(files.len(), 2, "recursive must include subdir files");
}

#[test]
fn ph0_6_collect_empty_dir() {
    let dir = TempDir::new().unwrap();
    let files = brn::collect_files(dir.path().to_str().unwrap(), &[], false).unwrap();
    assert!(files.is_empty());
}

#[test]
fn ph0_6_collect_nonexistent_dir_errors() {
    let result = brn::collect_files("/nonexistent_path_xyz", &[], false);
    assert!(result.is_err());
}


#[test]
fn ph0_4_two_node_cycle_injects_tmp() {
    use xun::batch_rename::cycle_break::break_cycles;
    use xun::batch_rename::types::RenameOp;
    use std::path::PathBuf;

    // a -> b, b -> a
    let ops = vec![
        RenameOp { from: PathBuf::from("a.txt"), to: PathBuf::from("b.txt") },
        RenameOp { from: PathBuf::from("b.txt"), to: PathBuf::from("a.txt") },
    ];
    let result = break_cycles(ops, &[]);
    // Must have 3 ops: a->tmp, b->a, tmp->b (or equivalent)
    assert_eq!(result.len(), 3, "two-node cycle must expand to 3 ops, got: {}", result.len());
    // The final ops must not form a cycle
    let froms: Vec<_> = result.iter().map(|o| o.from.to_string_lossy().into_owned()).collect();
    let tos: Vec<_> = result.iter().map(|o| o.to.to_string_lossy().into_owned()).collect();
    // No from should equal a to that points back to it (simple cycle check)
    for op in &result {
        let from_s = op.from.to_string_lossy();
        let to_s = op.to.to_string_lossy();
        let back = result.iter().any(|o2| o2.from.to_string_lossy() == to_s && o2.to.to_string_lossy() == from_s);
        assert!(!back, "cycle still present after break_cycles: {:?} <-> {:?}", from_s, to_s);
    }
    let _ = (froms, tos);
}

#[test]
fn ph0_4_three_node_cycle_injects_tmp() {
    use xun::batch_rename::cycle_break::break_cycles;
    use xun::batch_rename::types::RenameOp;
    use std::path::PathBuf;

    // a -> b -> c -> a
    let ops = vec![
        RenameOp { from: PathBuf::from("a.txt"), to: PathBuf::from("b.txt") },
        RenameOp { from: PathBuf::from("b.txt"), to: PathBuf::from("c.txt") },
        RenameOp { from: PathBuf::from("c.txt"), to: PathBuf::from("a.txt") },
    ];
    let result = break_cycles(ops, &[]);
    assert_eq!(result.len(), 4, "three-node cycle must expand to 4 ops");
}

#[test]
fn ph0_4_tmp_name_not_in_existing_files() {
    use xun::batch_rename::cycle_break::break_cycles;
    use xun::batch_rename::types::RenameOp;
    use std::path::PathBuf;

    let existing = vec![PathBuf::from("__xun_brn_tmp_clash__")];
    let ops = vec![
        RenameOp { from: PathBuf::from("a.txt"), to: PathBuf::from("b.txt") },
        RenameOp { from: PathBuf::from("b.txt"), to: PathBuf::from("a.txt") },
    ];
    let result = break_cycles(ops, &existing);
    // The injected tmp name must not be "__xun_brn_tmp_clash__"
    let tmp_ops: Vec<_> = result.iter()
        .filter(|o| o.from.to_string_lossy().contains("__xun_brn_tmp_"))
        .collect();
    assert!(!tmp_ops.is_empty(), "must inject a tmp op");
    for op in &tmp_ops {
        assert_ne!(
            op.from.to_string_lossy().as_ref(),
            "__xun_brn_tmp_clash__",
            "tmp name must not clash with existing files"
        );
    }
}

#[test]
fn ph0_4_no_cycle_ops_unchanged() {
    use xun::batch_rename::cycle_break::break_cycles;
    use xun::batch_rename::types::RenameOp;
    use std::path::PathBuf;

    // Linear chain: a -> b -> c (no cycle)
    let ops = vec![
        RenameOp { from: PathBuf::from("a.txt"), to: PathBuf::from("b.txt") },
        RenameOp { from: PathBuf::from("c.txt"), to: PathBuf::from("d.txt") },
    ];
    let result = break_cycles(ops, &[]);
    assert_eq!(result.len(), 2, "no-cycle ops must be unchanged");
}


#[test]
fn ph0_3_illegal_char_detected() {
    use xun::batch_rename::preflight::{preflight_check, PreflightError};
    use xun::batch_rename::types::RenameOp;
    use std::path::PathBuf;

    // Use "*" — not interpreted as drive separator on Windows
    let ops = vec![RenameOp {
        from: PathBuf::from("a.txt"),
        to: PathBuf::from("a*b.txt"),
    }];
    let errors = preflight_check(&ops, false);
    assert!(
        errors.iter().any(|e| matches!(e, PreflightError::IllegalChar { .. })),
        "'*' in target name must be ILLEGAL_CHAR, got: {:?}",
        errors
    );
}

#[test]
fn ph0_3_reserved_name_detected() {
    use xun::batch_rename::preflight::{preflight_check, PreflightError};
    use xun::batch_rename::types::RenameOp;
    use std::path::PathBuf;

    let cases = ["CON.txt", "NUL.log", "COM1.txt", "LPT9.rs", "AUX", "PRN.md"];
    for name in &cases {
        let ops = vec![RenameOp {
            from: PathBuf::from("a.txt"),
            to: PathBuf::from(name),
        }];
        let errors = preflight_check(&ops, false);
        assert!(
            errors.iter().any(|e| matches!(e, PreflightError::ReservedName { .. })),
            "'{}' must be RESERVED_NAME, got: {:?}",
            name,
            errors
        );
    }
}

#[test]
fn ph0_3_cycle_detected_two_nodes() {
    use xun::batch_rename::preflight::{preflight_check, PreflightError};
    use xun::batch_rename::types::RenameOp;
    use std::path::PathBuf;

    // a -> b, b -> a
    let ops = vec![
        RenameOp { from: PathBuf::from("a.txt"), to: PathBuf::from("b.txt") },
        RenameOp { from: PathBuf::from("b.txt"), to: PathBuf::from("a.txt") },
    ];
    let errors = preflight_check(&ops, false);
    assert!(
        errors.iter().any(|e| matches!(e, PreflightError::Cycle { .. })),
        "a->b, b->a must be CYCLE, got: {:?}",
        errors
    );
}

#[test]
fn ph0_3_cycle_detected_three_nodes() {
    use xun::batch_rename::preflight::{preflight_check, PreflightError};
    use xun::batch_rename::types::RenameOp;
    use std::path::PathBuf;

    // a -> b -> c -> a
    let ops = vec![
        RenameOp { from: PathBuf::from("a.txt"), to: PathBuf::from("b.txt") },
        RenameOp { from: PathBuf::from("b.txt"), to: PathBuf::from("c.txt") },
        RenameOp { from: PathBuf::from("c.txt"), to: PathBuf::from("a.txt") },
    ];
    let errors = preflight_check(&ops, false);
    assert!(
        errors.iter().any(|e| matches!(e, PreflightError::Cycle { .. })),
        "a->b->c->a must be CYCLE, got: {:?}",
        errors
    );
}

#[test]
fn ph0_3_all_errors_returned_no_short_circuit() {
    use xun::batch_rename::preflight::{preflight_check, PreflightError};
    use xun::batch_rename::types::RenameOp;
    use std::path::PathBuf;

    // One illegal char op + one reserved name op → both errors returned
    let ops = vec![
        RenameOp { from: PathBuf::from("a.txt"), to: PathBuf::from("a*b.txt") },
        RenameOp { from: PathBuf::from("b.txt"), to: PathBuf::from("CON.txt") },
    ];
    let errors = preflight_check(&ops, false);
    let has_illegal = errors.iter().any(|e| matches!(e, PreflightError::IllegalChar { .. }));
    let has_reserved = errors.iter().any(|e| matches!(e, PreflightError::ReservedName { .. }));
    assert!(has_illegal && has_reserved, "must return both errors, got: {:?}", errors);
}

#[test]
fn ph0_3_clean_ops_pass_preflight() {
    use xun::batch_rename::preflight::preflight_check;
    use xun::batch_rename::types::RenameOp;
    use std::path::PathBuf;

    let ops = vec![
        RenameOp { from: PathBuf::from("hello world.txt"), to: PathBuf::from("hello-world.txt") },
    ];
    let errors = preflight_check(&ops, false);
    assert!(errors.is_empty(), "clean op must pass preflight, got: {:?}", errors);
}

// ─── Ph0-2: undo 文件读写 ────────────────────────────────────────────────────

#[test]
fn ph0_2_apply_writes_undo_file() {
    use xun::batch_rename::undo::UndoRecord;

    let dir = TempDir::new().unwrap();
    let records = vec![UndoRecord {
        from: dir.path().join("b.txt").to_string_lossy().into_owned(),
        to: dir.path().join("a.txt").to_string_lossy().into_owned(),
    }];
    brn::write_undo(dir.path(), &records).unwrap();

    let undo_path = dir.path().join(".xun-brn-undo.json");
    assert!(undo_path.exists(), "undo file must be written");
    let content = fs::read_to_string(&undo_path).unwrap();
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(
        parsed[0]["from"].as_str().unwrap(),
        dir.path().join("b.txt").to_str().unwrap()
    );
}

#[test]
fn ph0_2_second_apply_overwrites_undo_file() {
    use xun::batch_rename::undo::UndoRecord;

    let dir = TempDir::new().unwrap();
    let records1 = vec![UndoRecord {
        from: "b.txt".to_owned(),
        to: "a.txt".to_owned(),
    }];
    let records2 = vec![
        UndoRecord { from: "d.txt".to_owned(), to: "c.txt".to_owned() },
        UndoRecord { from: "f.txt".to_owned(), to: "e.txt".to_owned() },
    ];
    brn::write_undo(dir.path(), &records1).unwrap();
    brn::write_undo(dir.path(), &records2).unwrap();

    let content = fs::read_to_string(dir.path().join(".xun-brn-undo.json")).unwrap();
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed.len(), 2, "second write must overwrite first");
}

#[test]
fn ph0_2_undo_restores_files() {
    use xun::batch_rename::undo::UndoRecord;

    let dir = TempDir::new().unwrap();
    let original = dir.path().join("a.txt");
    let renamed = dir.path().join("b.txt");
    fs::write(&renamed, b"data").unwrap();

    let records = vec![UndoRecord {
        from: renamed.to_string_lossy().into_owned(),
        to: original.to_string_lossy().into_owned(),
    }];
    brn::write_undo(dir.path(), &records).unwrap();
    brn::run_undo(dir.path().to_str().unwrap()).unwrap();

    assert!(original.exists(), "original must be restored");
    assert!(!renamed.exists(), "renamed must be gone after undo");
}

#[test]
fn ph0_2_undo_missing_file_gives_friendly_error() {
    let dir = TempDir::new().unwrap();
    let result = brn::run_undo(dir.path().to_str().unwrap());
    assert!(result.is_err());
    let msg = result.unwrap_err();
    assert!(
        msg.contains("not found") || msg.contains("Nothing to undo"),
        "error must mention not found: {}",
        msg
    );
}

#[test]
fn ph0_2_undo_corrupt_file_gives_error() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join(".xun-brn-undo.json"), b"not json!").unwrap();
    let result = brn::run_undo(dir.path().to_str().unwrap());
    assert!(result.is_err());
}

// ─── Ph1-2: --strip-suffix ───────────────────────────────────────────────────

#[test]
fn ph1_2_strip_suffix_removes_stem_suffix() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/file_v2.txt")];
    let mode = RenameMode::StripSuffix("_v2".into());
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "file.txt");
}

#[test]
fn ph1_2_strip_suffix_no_match_is_noop() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/file.txt")];
    let mode = RenameMode::StripSuffix("_v2".into());
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(
        ops[0].from.file_name().unwrap().to_str().unwrap(),
        ops[0].to.file_name().unwrap().to_str().unwrap()
    );
}

// ─── Ph1-3: --ext-from/--ext-to ──────────────────────────────────────────────

#[test]
fn ph1_3_ext_rename_basic() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/photo.jpeg")];
    let mode = RenameMode::RenameExt { from: "jpeg".into(), to: "jpg".into() };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "photo.jpg");
}

#[test]
fn ph1_3_ext_rename_case_insensitive_match() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/photo.JPEG")];
    let mode = RenameMode::RenameExt { from: "jpeg".into(), to: "jpg".into() };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "photo.jpg");
}

#[test]
fn ph1_3_ext_rename_no_match_is_noop() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/photo.png")];
    let mode = RenameMode::RenameExt { from: "jpeg".into(), to: "jpg".into() };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(
        ops[0].from.file_name().unwrap().to_str().unwrap(),
        ops[0].to.file_name().unwrap().to_str().unwrap()
    );
}


// ─── Ph1-4: --ext-case ───────────────────────────────────────────────────────

#[test]
fn ph1_4_ext_case_lower() {
    use xun::batch_rename::compute::RenameMode;
    use xun::batch_rename::types::CaseStyle;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/photo.JPG")];
    let mode = RenameMode::ExtCase(CaseStyle::Lower);
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "photo.jpg");
}

#[test]
fn ph1_4_ext_case_upper() {
    use xun::batch_rename::compute::RenameMode;
    use xun::batch_rename::types::CaseStyle;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/file.txt")];
    let mode = RenameMode::ExtCase(CaseStyle::Upper);
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "file.TXT");
}

#[test]
fn ph1_4_ext_case_no_change_is_noop() {
    use xun::batch_rename::compute::RenameMode;
    use xun::batch_rename::types::CaseStyle;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/file.txt")];
    let mode = RenameMode::ExtCase(CaseStyle::Lower);
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(
        ops[0].from.file_name().unwrap().to_str().unwrap(),
        ops[0].to.file_name().unwrap().to_str().unwrap()
    );
}

// ─── Ph1-7: --insert-at/--insert-str ─────────────────────────────────────────

#[test]
fn ph1_7_insert_at_position() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/20240315photo.jpg")];
    let mode = RenameMode::InsertAt { pos: 8, insert: "_".into() };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "20240315_photo.jpg");
}

#[test]
fn ph1_7_insert_at_start() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/photo.jpg")];
    let mode = RenameMode::InsertAt { pos: 0, insert: "IMG_".into() };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "IMG_photo.jpg");
}

#[test]
fn ph1_7_insert_at_beyond_stem_clamps_to_end() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/hi.txt")];
    let mode = RenameMode::InsertAt { pos: 999, insert: "_end".into() };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "hi_end.txt");
}

// ─── Ph1-5: --seq-pos/--seq-only ─────────────────────────────────────────────

#[test]
fn ph1_5_seq_prefix_position() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/photo.jpg")];
    let mode = RenameMode::SeqExt { start: 1, pad: 3, prefix: true, only: false };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "001_photo.jpg");
}

#[test]
fn ph1_5_seq_only_replaces_stem() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/DSC001.jpg")];
    let mode = RenameMode::SeqExt { start: 1, pad: 4, prefix: false, only: true };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "0001.jpg");
}

#[test]
fn ph1_5_seq_multiple_files_increment() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![
        PathBuf::from("/dir/a.jpg"),
        PathBuf::from("/dir/b.jpg"),
        PathBuf::from("/dir/c.jpg"),
    ];
    let mode = RenameMode::SeqExt { start: 1, pad: 3, prefix: true, only: false };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "001_a.jpg");
    assert_eq!(ops[1].to.file_name().unwrap().to_str().unwrap(), "002_b.jpg");
    assert_eq!(ops[2].to.file_name().unwrap().to_str().unwrap(), "003_c.jpg");
}

// ─── Ph1-8: --on-conflict ────────────────────────────────────────────────────

#[test]
fn ph1_8_on_conflict_abort_blocks_all() {
    use xun::batch_rename::conflict_strategy::OnConflict;
    use xun::batch_rename::types::RenameOp;
    use std::path::PathBuf;

    let target = PathBuf::from("hello-world.txt");
    let ops = vec![
        RenameOp { from: PathBuf::from("a.txt"), to: target.clone() },
        RenameOp { from: PathBuf::from("b.txt"), to: target.clone() },
    ];
    let result = brn::apply_conflict_strategy(ops, OnConflict::Abort, &[]);
    assert!(result.is_err(), "abort strategy must fail when conflict detected");
}

#[test]
fn ph1_8_on_conflict_skip_removes_conflicting() {
    use xun::batch_rename::conflict_strategy::OnConflict;
    use xun::batch_rename::types::RenameOp;
    use std::path::PathBuf;

    let target = PathBuf::from("hello-world.txt");
    let ops = vec![
        RenameOp { from: PathBuf::from("a.txt"), to: target.clone() },
        RenameOp { from: PathBuf::from("b.txt"), to: target.clone() },
        RenameOp { from: PathBuf::from("c.txt"), to: PathBuf::from("other.txt") },
    ];
    let result = brn::apply_conflict_strategy(ops, OnConflict::Skip, &[]).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].from, PathBuf::from("c.txt"));
}

#[test]
fn ph1_8_no_conflict_all_strategies_same() {
    use xun::batch_rename::conflict_strategy::OnConflict;
    use xun::batch_rename::types::RenameOp;
    use std::path::PathBuf;

    let ops = vec![
        RenameOp { from: PathBuf::from("a.txt"), to: PathBuf::from("x.txt") },
        RenameOp { from: PathBuf::from("b.txt"), to: PathBuf::from("y.txt") },
    ];
    for strategy in [OnConflict::Abort, OnConflict::Skip] {
        let result = brn::apply_conflict_strategy(
            ops.iter().map(|o| RenameOp { from: o.from.clone(), to: o.to.clone() }).collect(),
            strategy,
            &[]
        ).unwrap();
        assert_eq!(result.len(), 2);
    }
}

// ─── Ph1-9: 环形重命名集成验收 ───────────────────────────────────────────────

#[test]
fn ph1_9_two_file_swap_apply() {
    use xun::batch_rename::cycle_break::break_cycles;
    use xun::batch_rename::types::RenameOp;
    use std::path::PathBuf;

    let dir = TempDir::new().unwrap();
    let a = dir.path().join("a.txt");
    let b = dir.path().join("b.txt");
    fs::write(&a, b"content_a").unwrap();
    fs::write(&b, b"content_b").unwrap();

    // Swap: a->b, b->a (cycle)
    let ops = vec![
        RenameOp { from: a.clone(), to: b.clone() },
        RenameOp { from: b.clone(), to: a.clone() },
    ];
    let ops = break_cycles(ops, &[]);
    assert_eq!(ops.len(), 3, "two-node cycle must produce 3 ops");

    // Apply in order
    for op in &ops {
        fs::rename(&op.from, &op.to).unwrap();
    }

    // After swap: a.txt has content_b, b.txt has content_a
    assert_eq!(fs::read(&a).unwrap(), b"content_b");
    assert_eq!(fs::read(&b).unwrap(), b"content_a");

    // No tmp files left
    let remaining: Vec<_> = fs::read_dir(dir.path()).unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains("__xun_brn_tmp_"))
        .collect();
    assert!(remaining.is_empty(), "tmp files must not remain");
}

#[test]
fn ph1_9_three_node_cycle_apply() {
    use xun::batch_rename::cycle_break::break_cycles;
    use xun::batch_rename::types::RenameOp;
    use std::path::PathBuf;

    let dir = TempDir::new().unwrap();
    let a = dir.path().join("a.txt");
    let b = dir.path().join("b.txt");
    let c = dir.path().join("c.txt");
    fs::write(&a, b"aaa").unwrap();
    fs::write(&b, b"bbb").unwrap();
    fs::write(&c, b"ccc").unwrap();

    // a->b->c->a
    let ops = vec![
        RenameOp { from: a.clone(), to: b.clone() },
        RenameOp { from: b.clone(), to: c.clone() },
        RenameOp { from: c.clone(), to: a.clone() },
    ];
    let ops = break_cycles(ops, &[]);
    for op in &ops {
        fs::rename(&op.from, &op.to).unwrap();
    }

    assert_eq!(fs::read(&a).unwrap(), b"ccc");
    assert_eq!(fs::read(&b).unwrap(), b"aaa");
    assert_eq!(fs::read(&c).unwrap(), b"bbb");
}

// ─── Ph1-10: NTFS 大小写中转集成验收 ─────────────────────────────────────────

#[test]
fn ph1_10_ntfs_case_change_via_tmp() {
    use xun::batch_rename::ntfs_case::normalize_case_ops;
    use xun::batch_rename::types::RenameOp;

    let dir = TempDir::new().unwrap();
    let upper = dir.path().join("PHOTO.JPG");
    let lower = dir.path().join("PHOTO.jpg");
    fs::write(&upper, b"img").unwrap();

    let ops = vec![RenameOp { from: upper.clone(), to: lower.clone() }];
    let ops = normalize_case_ops(ops);
    assert_eq!(ops.len(), 2);

    for op in &ops {
        fs::rename(&op.from, &op.to).unwrap();
    }

    assert!(lower.exists(), "lowercase file must exist after rename");
    // No tmp should remain
    let tmp_remaining: Vec<_> = fs::read_dir(dir.path()).unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains("__xun_brn_ntfs_tmp_"))
        .collect();
    assert!(tmp_remaining.is_empty(), "ntfs tmp must not remain");
}

// ─── Ph1-11: 输出名合法性检测集成验收 ────────────────────────────────────────

#[test]
fn ph1_11_reserved_name_blocked() {
    use xun::batch_rename::preflight::{preflight_check, PreflightError};
    use xun::batch_rename::types::RenameOp;
    use std::path::PathBuf;

    let ops = vec![RenameOp {
        from: PathBuf::from("file.txt"),
        to: PathBuf::from("aux.txt"),
    }];
    let errors = preflight_check(&ops, false);
    assert!(
        errors.iter().any(|e| matches!(e, PreflightError::ReservedName { .. })),
        "'aux.txt' must fail with RESERVED_NAME"
    );
}

#[test]
fn ph1_11_illegal_char_blocked() {
    use xun::batch_rename::preflight::{preflight_check, PreflightError};
    use xun::batch_rename::types::RenameOp;
    use std::path::PathBuf;

    let ops = vec![RenameOp {
        from: PathBuf::from("file.txt"),
        to: PathBuf::from("file*.txt"),
    }];
    let errors = preflight_check(&ops, false);
    assert!(
        errors.iter().any(|e| matches!(e, PreflightError::IllegalChar { .. })),
        "'*' must fail with ILLEGAL_CHAR"
    );
}

#[test]
fn ph1_11_clean_name_passes() {
    use xun::batch_rename::preflight::preflight_check;
    use xun::batch_rename::types::RenameOp;
    use std::path::PathBuf;

    let ops = vec![RenameOp {
        from: PathBuf::from("file.txt"),
        to: PathBuf::from("clean-file.txt"),
    }];
    let errors = preflight_check(&ops, false);
    assert!(errors.is_empty());
}

// ─── Ph3-1: --strip-brackets ─────────────────────────────────────────────────

#[test]
fn ph3_1_strip_round_brackets() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/song (2024) (official).mp3")];
    let mode = RenameMode::StripBrackets { round: true, square: false, curly: false };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "song.mp3");
}

#[test]
fn ph3_1_strip_square_brackets() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/movie [BluRay].mkv")];
    let mode = RenameMode::StripBrackets { round: false, square: true, curly: false };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "movie.mkv");
}

#[test]
fn ph3_1_strip_trims_whitespace() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/song  (extra)  .mp3")];
    let mode = RenameMode::StripBrackets { round: true, square: false, curly: false };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "song.mp3");
}

// ─── Ph3-2: --trim ────────────────────────────────────────────────────────────

#[test]
fn ph3_2_trim_whitespace() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/  hello  .txt")];
    let mode = RenameMode::Trim { chars: None };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "hello.txt");
}

#[test]
fn ph3_2_trim_specific_chars() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/___hello___.txt")];
    let mode = RenameMode::Trim { chars: Some("_".into()) };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "hello.txt");
}

#[test]
fn ph3_2_trim_no_change_is_noop() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/hello.txt")];
    let mode = RenameMode::Trim { chars: None };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(
        ops[0].from.file_name().unwrap().to_str().unwrap(),
        ops[0].to.file_name().unwrap().to_str().unwrap()
    );
}

// ─── Ph3-4: --case title 词首大写 ───────────────────────────────────────────

#[test]
fn ph3_4_case_title_basic() {
    use xun::batch_rename::compute::RenameMode;
    use xun::batch_rename::types::CaseStyle;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/hello world file.txt")];
    let mode = RenameMode::Case(CaseStyle::Title);
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "Hello World File.txt");
}

#[test]
fn ph3_4_case_title_kebab() {
    // kebab 连接词按 - 分词
    use xun::batch_rename::compute::RenameMode;
    use xun::batch_rename::types::CaseStyle;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/hello-world.txt")];
    let mode = RenameMode::Case(CaseStyle::Title);
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "Hello-World.txt");
}

#[test]
fn ph3_4_case_title_no_change_is_noop() {
    // 已经是 title case → from == to
    use xun::batch_rename::compute::RenameMode;
    use xun::batch_rename::types::CaseStyle;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/Hello World.txt")];
    let mode = RenameMode::Case(CaseStyle::Title);
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(
        ops[0].from.file_name().unwrap().to_str().unwrap(),
        ops[0].to.file_name().unwrap().to_str().unwrap()
    );
}

#[test]
fn ph3_3_slice_take_prefix() {
    // --slice ":8" → 取前8字符
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/very_long_filename.txt")];
    let mode = RenameMode::Slice { start: None, end: Some(8) };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "very_lon.txt");
}

#[test]
fn ph3_3_slice_drop_prefix() {
    // --slice "4:" → 去掉前4字符
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/001_file.txt")];
    let mode = RenameMode::Slice { start: Some(4), end: None };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "file.txt");
}

#[test]
fn ph3_3_slice_drop_suffix() {
    // --slice ":-3" → 去掉末尾3字符（Python 切片语义）
    // stem "file_v2" (7 chars), end=-3 → chars[0..4] = "file"
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/file_v2.txt")];
    let mode = RenameMode::Slice { start: None, end: Some(-3) };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "file.txt");
}

#[test]
fn ph3_3_slice_range() {
    // --slice "2:5" → 取第2到4字符（Python 切片语义）
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/abcdefgh.txt")];
    let mode = RenameMode::Slice { start: Some(2), end: Some(5) };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "cde.txt");
}

#[test]
fn ph3_3_slice_negative_start() {
    // 负数 start: 取末尾2字符
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/file.txt")];
    let mode = RenameMode::Slice { start: Some(-2), end: None };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "le.txt");
}

#[test]
fn ph3_3_slice_clamp_overflow() {
    // 超出词干长度 → clamp，不报错
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/ab.txt")];
    let mode = RenameMode::Slice { start: Some(0), end: Some(100) };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "ab.txt");
}

// ─── Ph3-5: --filter 按名称过滤 ──────────────────────────────────────────────

#[test]
fn ph3_5_filter_by_extension_glob() {
    let dir = TempDir::new().unwrap();
    make_files(dir.path(), &["a.jpg", "b.jpg", "c.txt", "d.png"]);

    let files = brn::collect_files_filtered(
        dir.path().to_str().unwrap(), &[], false,
        Some("*.jpg"), None,
    ).unwrap();
    assert_eq!(files.len(), 2);
    assert!(files.iter().all(|f| f.extension().unwrap() == "jpg"));
}

#[test]
fn ph3_5_filter_by_name_glob() {
    let dir = TempDir::new().unwrap();
    make_files(dir.path(), &["IMG_001.jpg", "IMG_002.jpg", "photo.jpg"]);

    let files = brn::collect_files_filtered(
        dir.path().to_str().unwrap(), &[], false,
        Some("IMG_*"), None,
    ).unwrap();
    assert_eq!(files.len(), 2);
}

#[test]
fn ph3_5_filter_no_match_returns_empty() {
    let dir = TempDir::new().unwrap();
    make_files(dir.path(), &["a.txt", "b.txt"]);

    let files = brn::collect_files_filtered(
        dir.path().to_str().unwrap(), &[], false,
        Some("*.jpg"), None,
    ).unwrap();
    assert!(files.is_empty());
}

// ─── Ph3-6: --exclude 排除匹配文件 ───────────────────────────────────────────

#[test]
fn ph3_6_exclude_hidden_files() {
    let dir = TempDir::new().unwrap();
    make_files(dir.path(), &[".hidden", "visible.txt", ".config"]);

    let files = brn::collect_files_filtered(
        dir.path().to_str().unwrap(), &[], false,
        None, Some(".*"),
    ).unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].file_name().unwrap().to_str().unwrap() == "visible.txt");
}

#[test]
fn ph3_6_exclude_bak_files() {
    let dir = TempDir::new().unwrap();
    make_files(dir.path(), &["a.txt", "b.bak", "c.txt", "d.bak"]);

    let files = brn::collect_files_filtered(
        dir.path().to_str().unwrap(), &[], false,
        None, Some("*.bak"),
    ).unwrap();
    assert_eq!(files.len(), 2);
    assert!(files.iter().all(|f| f.extension().unwrap() != "bak"));
}

#[test]
fn ph3_6_filter_and_exclude_combined() {
    let dir = TempDir::new().unwrap();
    make_files(dir.path(), &["a.jpg", "b.jpg", "thumb_c.jpg", "d.txt"]);

    // filter *.jpg, exclude thumb_*
    let files = brn::collect_files_filtered(
        dir.path().to_str().unwrap(), &[], false,
        Some("*.jpg"), Some("thumb_*"),
    ).unwrap();
    assert_eq!(files.len(), 2);
    assert!(files.iter().all(|f| {
        let n = f.file_name().unwrap().to_str().unwrap();
        n.ends_with(".jpg") && !n.starts_with("thumb_")
    }));
}

// ─── Ph3-7: --depth 递归深度控制 ─────────────────────────────────────────────

#[test]
fn ph3_7_depth_1_only_current_dir() {
    use std::fs;
    let dir = TempDir::new().unwrap();
    make_files(dir.path(), &["a.txt"]);
    fs::create_dir(dir.path().join("sub")).unwrap();
    fs::write(dir.path().join("sub").join("b.txt"), b"").unwrap();

    let files = brn::collect_files_depth(
        dir.path().to_str().unwrap(), &[], Some(1), None, None,
    ).unwrap();
    assert_eq!(files.len(), 1, "depth=1 should only return current dir files");
}

#[test]
fn ph3_7_depth_2_includes_subdir() {
    use std::fs;
    let dir = TempDir::new().unwrap();
    make_files(dir.path(), &["a.txt"]);
    fs::create_dir(dir.path().join("sub")).unwrap();
    fs::write(dir.path().join("sub").join("b.txt"), b"").unwrap();
    fs::create_dir(dir.path().join("sub").join("deep")).unwrap();
    fs::write(dir.path().join("sub").join("deep").join("c.txt"), b"").unwrap();

    let files = brn::collect_files_depth(
        dir.path().to_str().unwrap(), &[], Some(2), None, None,
    ).unwrap();
    assert_eq!(files.len(), 2, "depth=2 includes root + one subdir level");
}

#[test]
fn ph3_7_depth_none_unlimited() {
    use std::fs;
    let dir = TempDir::new().unwrap();
    make_files(dir.path(), &["a.txt"]);
    fs::create_dir(dir.path().join("sub")).unwrap();
    fs::create_dir(dir.path().join("sub").join("deep")).unwrap();
    fs::write(dir.path().join("sub").join("deep").join("c.txt"), b"").unwrap();

    let files = brn::collect_files_depth(
        dir.path().to_str().unwrap(), &[], None, None, None,
    ).unwrap();
    assert_eq!(files.len(), 2, "depth=None means unlimited recursion");
}

// ─── Ph3-8: --insert-date 插入文件日期 ───────────────────────────────────────

#[test]
fn ph3_8_insert_date_prefix_default_fmt() {
    // 默认格式 %Y%m%d 前缀，使用 mtime
    use xun::batch_rename::compute::RenameMode;

    let dir = TempDir::new().unwrap();
    let path = dir.path().join("photo.jpg");
    fs::write(&path, b"").unwrap();

    let files = vec![path.clone()];
    let mode = RenameMode::InsertDate {
        fmt: "%Y%m%d".into(),
        use_ctime: false,
        prefix: true,
    };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    let new_name = ops[0].to.file_name().unwrap().to_str().unwrap();
    // Should match YYYYMMDD_photo.jpg
    assert!(new_name.ends_with("_photo.jpg"), "got: {}", new_name);
    assert_eq!(new_name.len(), "20260320_photo.jpg".len(), "date prefix length wrong: {}", new_name);
}

#[test]
fn ph3_8_insert_date_suffix() {
    use xun::batch_rename::compute::RenameMode;

    let dir = TempDir::new().unwrap();
    let path = dir.path().join("photo.jpg");
    fs::write(&path, b"").unwrap();

    let files = vec![path.clone()];
    let mode = RenameMode::InsertDate {
        fmt: "%Y%m%d".into(),
        use_ctime: false,
        prefix: false,
    };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    let new_name = ops[0].to.file_name().unwrap().to_str().unwrap();
    // Should match photo_YYYYMMDD.jpg
    assert!(new_name.starts_with("photo_"), "got: {}", new_name);
    assert!(new_name.ends_with(".jpg"), "got: {}", new_name);
}

#[test]
fn ph3_8_insert_date_custom_fmt() {
    use xun::batch_rename::compute::RenameMode;

    let dir = TempDir::new().unwrap();
    let path = dir.path().join("file.txt");
    fs::write(&path, b"").unwrap();

    let files = vec![path.clone()];
    let mode = RenameMode::InsertDate {
        fmt: "%Y-%m-%d".into(),
        use_ctime: false,
        prefix: true,
    };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    let new_name = ops[0].to.file_name().unwrap().to_str().unwrap();
    // Should match YYYY-MM-DD_file.txt
    assert!(new_name.ends_with("_file.txt"), "got: {}", new_name);
    // Date part has dashes
    let date_part = &new_name[..new_name.len() - "_file.txt".len()];
    assert_eq!(date_part.len(), 10, "date part should be YYYY-MM-DD: {}", date_part);
    assert!(date_part.contains('-'), "got: {}", date_part);
}

// ─── Ph3-9: --normalize-seq 序号规范化 ───────────────────────────────────────

#[test]
fn ph3_9_normalize_seq_pad_single_digit() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/track_1.mp3")];
    let mode = RenameMode::NormalizeSeq { pad: 3 };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "track_001.mp3");
}

#[test]
fn ph3_9_normalize_seq_pad_two_digit() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/track_10.mp3")];
    let mode = RenameMode::NormalizeSeq { pad: 3 };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "track_010.mp3");
}

#[test]
fn ph3_9_normalize_seq_already_wide() {
    // 已满足宽度，不变
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/track_100.mp3")];
    let mode = RenameMode::NormalizeSeq { pad: 3 };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "track_100.mp3");
}

#[test]
fn ph3_9_normalize_seq_last_group_only() {
    // 只处理最后一组数字
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/track_1_v2.mp3")];
    let mode = RenameMode::NormalizeSeq { pad: 3 };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "track_1_v002.mp3");
}

#[test]
fn ph3_9_normalize_seq_no_number_is_noop() {
    // 无数字 → noop (from == to)
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/readme.txt")];
    let mode = RenameMode::NormalizeSeq { pad: 3 };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(
        ops[0].from.file_name().unwrap().to_str().unwrap(),
        ops[0].to.file_name().unwrap().to_str().unwrap()
    );
}

// ─── Ph3-10: --seq-by 排序依据 ───────────────────────────────────────────────

#[test]
fn ph3_10_seq_by_name_natural_order() {
    // 按文件名自然排序
    use xun::batch_rename::collect::sort_files_by;
    use std::path::PathBuf;

    let mut files = vec![
        PathBuf::from("/dir/track_10.mp3"),
        PathBuf::from("/dir/track_2.mp3"),
        PathBuf::from("/dir/track_1.mp3"),
    ];
    sort_files_by(&mut files, xun::batch_rename::collect::SortBy::Name);
    let names: Vec<_> = files.iter().map(|f| f.file_name().unwrap().to_str().unwrap()).collect();
    assert_eq!(names, ["track_1.mp3", "track_2.mp3", "track_10.mp3"]);
}

#[test]
fn ph3_10_seq_by_mtime_order() {
    // 按修改时间排序（最早的在前）
    use xun::batch_rename::collect::sort_files_by;
    use std::thread;
    use std::time::Duration;

    let dir = TempDir::new().unwrap();
    let a = dir.path().join("a.txt");
    let b = dir.path().join("b.txt");
    fs::write(&a, b"").unwrap();
    thread::sleep(Duration::from_millis(50));
    fs::write(&b, b"").unwrap();

    let mut files = vec![b.clone(), a.clone()];
    sort_files_by(&mut files, xun::batch_rename::collect::SortBy::Mtime);
    assert_eq!(files[0], a, "a was written first, should sort before b");
    assert_eq!(files[1], b);
}

// ─── Ph3-12: --output-format json/csv ────────────────────────────────────────

#[test]
fn ph3_12_ops_to_json_structure() {
    use xun::batch_rename::output_format::ops_to_json;
    use std::path::PathBuf;
    use xun::batch_rename::types::RenameOp;

    let ops = vec![
        RenameOp { from: PathBuf::from("/dir/a.txt"), to: PathBuf::from("/dir/b.txt") },
        RenameOp { from: PathBuf::from("/dir/c.txt"), to: PathBuf::from("/dir/d.txt") },
    ];
    let json = ops_to_json(&ops, 0);
    let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    assert_eq!(v["total"], 2);
    assert_eq!(v["effective"], 2);
    assert_eq!(v["skipped"], 0);
    assert_eq!(v["ops"][0]["from"], "/dir/a.txt");
    assert_eq!(v["ops"][0]["to"], "/dir/b.txt");
}

#[test]
fn ph3_12_ops_to_json_skipped_noop() {
    // noop ops (from==to) should not appear in ops, counted in skipped
    use xun::batch_rename::output_format::ops_to_json;
    use std::path::PathBuf;
    use xun::batch_rename::types::RenameOp;

    let ops = vec![
        RenameOp { from: PathBuf::from("/dir/a.txt"), to: PathBuf::from("/dir/b.txt") },
        RenameOp { from: PathBuf::from("/dir/c.txt"), to: PathBuf::from("/dir/c.txt") }, // noop
    ];
    let json = ops_to_json(&ops, 0);
    let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    assert_eq!(v["total"], 2);
    assert_eq!(v["effective"], 1);
    assert_eq!(v["skipped"], 1);
    assert_eq!(v["ops"].as_array().unwrap().len(), 1);
}

#[test]
fn ph3_12_ops_to_csv_format() {
    use xun::batch_rename::output_format::ops_to_csv;
    use std::path::PathBuf;
    use xun::batch_rename::types::RenameOp;

    let ops = vec![
        RenameOp { from: PathBuf::from("/dir/a.txt"), to: PathBuf::from("/dir/b.txt") },
    ];
    let csv = ops_to_csv(&ops);
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines[0], "from,to", "first line must be header");
    assert!(lines[1].contains("/dir/a.txt"), "got: {}", lines[1]);
    assert!(lines[1].contains("/dir/b.txt"), "got: {}", lines[1]);
}

// ─── Ph1-6: --template 模板命名 ──────────────────────────────────────────────

#[test]
fn ph1_6_template_stem_ext_noop() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/photo.jpg")];
    let mode = RenameMode::Template { tpl: "{stem}{ext}".into(), start: 1, pad: 3 };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(
        ops[0].from.file_name().unwrap().to_str().unwrap(),
        ops[0].to.file_name().unwrap().to_str().unwrap()
    );
}

#[test]
fn ph1_6_template_upper() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/hello.txt")];
    let mode = RenameMode::Template { tpl: "{upper}{ext}".into(), start: 1, pad: 3 };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "HELLO.txt");
}

#[test]
fn ph1_6_template_lower() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/HELLO.txt")];
    let mode = RenameMode::Template { tpl: "{lower}{ext}".into(), start: 1, pad: 3 };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "hello.txt");
}

#[test]
fn ph1_6_template_seq_and_stem() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![
        PathBuf::from("/dir/a.jpg"),
        PathBuf::from("/dir/b.jpg"),
    ];
    let mode = RenameMode::Template { tpl: "{n}_{stem}{ext}".into(), start: 1, pad: 3 };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "001_a.jpg");
    assert_eq!(ops[1].to.file_name().unwrap().to_str().unwrap(), "002_b.jpg");
}

#[test]
fn ph1_6_template_parent() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/mydir/photo.jpg")];
    let mode = RenameMode::Template { tpl: "{parent}_{n}{ext}".into(), start: 1, pad: 2 };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "mydir_01.jpg");
}

// ─── Ph5-1: --remove-chars 删除指定字符 ──────────────────────────────────────

#[test]
fn ph5_1_remove_chars_basic() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/file[1](copy) v2.txt")];
    let mode = RenameMode::RemoveChars { chars: "[]() ".into() };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "file1copyv2.txt");
}

#[test]
fn ph5_1_remove_chars_digits() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/track01.mp3")];
    let mode = RenameMode::RemoveChars { chars: "0123456789".into() };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "track.mp3");
}

#[test]
fn ph5_1_remove_chars_no_ext_effect() {
    // 不影响扩展名
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/file123.mp3")];
    let mode = RenameMode::RemoveChars { chars: "123".into() };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "file.mp3");
}

// ─── Ph5-3: --add-ext 添加扩展名 ─────────────────────────────────────────────

#[test]
fn ph5_3_add_ext_no_extension() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/Makefile")];
    let mode = RenameMode::AddExt { ext: "txt".into() };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "Makefile.txt");
}

#[test]
fn ph5_3_add_ext_already_has_ext_is_noop() {
    // 已有扩展名 → noop
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/photo.jpg")];
    let mode = RenameMode::AddExt { ext: "txt".into() };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(
        ops[0].from.file_name().unwrap().to_str().unwrap(),
        ops[0].to.file_name().unwrap().to_str().unwrap()
    );
}

#[test]
fn ph5_3_add_ext_with_dot_prefix() {
    // .txt 等价于 txt
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![PathBuf::from("/dir/README")];
    let mode = RenameMode::AddExt { ext: ".md".into() };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "README.md");
}

// ─── Ph5-5: --renumber 重新排序编号 ──────────────────────────────────────────

#[test]
fn ph5_5_renumber_basic() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![
        PathBuf::from("/dir/track_3.mp3"),
        PathBuf::from("/dir/track_7.mp3"),
        PathBuf::from("/dir/track_15.mp3"),
    ];
    let mode = RenameMode::Renumber { start: 1, pad: 3 };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "track_001.mp3");
    assert_eq!(ops[1].to.file_name().unwrap().to_str().unwrap(), "track_002.mp3");
    assert_eq!(ops[2].to.file_name().unwrap().to_str().unwrap(), "track_003.mp3");
}

#[test]
fn ph5_5_renumber_start_10() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    let files = vec![
        PathBuf::from("/dir/a_1.txt"),
        PathBuf::from("/dir/a_5.txt"),
    ];
    let mode = RenameMode::Renumber { start: 10, pad: 3 };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(ops[0].to.file_name().unwrap().to_str().unwrap(), "a_010.txt");
    assert_eq!(ops[1].to.file_name().unwrap().to_str().unwrap(), "a_011.txt");
}

// ─── Ph5-7: --normalize-unicode Unicode 规范化 ────────────────────────────────

#[test]
fn ph5_7_normalize_nfc_already_nfc_is_noop() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    // Pure ASCII → already NFC, noop
    let files = vec![PathBuf::from("/dir/hello.txt")];
    let mode = RenameMode::NormalizeUnicode { form: "nfc".into() };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    assert_eq!(
        ops[0].from.file_name().unwrap().to_str().unwrap(),
        ops[0].to.file_name().unwrap().to_str().unwrap()
    );
}

#[test]
fn ph5_7_normalize_nfc_from_nfd() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    // NFD: 'e' + combining acute accent (U+0301) = \u{65}\u{301}
    // NFC: precomposed 'é' = \u{e9}
    let nfd_name = "caf\u{65}\u{301}.txt"; // "café" in NFD
    let files = vec![PathBuf::from(format!("/dir/{}", nfd_name))];
    let mode = RenameMode::NormalizeUnicode { form: "nfc".into() };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    let new_name = ops[0].to.file_name().unwrap().to_str().unwrap();
    // NFC form: é is single codepoint U+00E9
    assert_eq!(new_name, "caf\u{e9}.txt");
}

#[test]
fn ph5_7_normalize_nfkc_fullwidth() {
    use xun::batch_rename::compute::RenameMode;
    use std::path::PathBuf;

    // Fullwidth 'ａ' (U+FF41) → 'a' under NFKC
    let fw_name = "\u{FF41}bc.txt";
    let files = vec![PathBuf::from(format!("/dir/{}", fw_name))];
    let mode = RenameMode::NormalizeUnicode { form: "nfkc".into() };
    let ops = brn::compute_ops(&files, &mode).unwrap();
    let new_name = ops[0].to.file_name().unwrap().to_str().unwrap();
    assert_eq!(new_name, "abc.txt");
}

// ─── Ph5-8: --undo --steps 多步 undo 历史 ────────────────────────────────────

#[test]
fn ph5_8_append_undo_builds_history() {
    use xun::batch_rename::undo::{append_undo, UndoRecord, read_undo_history};

    let dir = TempDir::new().unwrap();

    // First batch
    let r1 = vec![UndoRecord { from: "a.txt".into(), to: "b.txt".into() }];
    brn::append_undo(dir.path(), &r1).unwrap();

    // Second batch
    let r2 = vec![UndoRecord { from: "c.txt".into(), to: "d.txt".into() }];
    brn::append_undo(dir.path(), &r2).unwrap();

    let history = brn::read_undo_history(dir.path()).unwrap();
    assert_eq!(history.len(), 2, "two batches in history");
}

#[test]
fn ph5_8_undo_steps_reverses_last_n() {
    use xun::batch_rename::undo::{append_undo, UndoRecord};

    let dir = TempDir::new().unwrap();

    // Create real files for undo
    let a = dir.path().join("a.txt");
    let b = dir.path().join("b.txt");
    let c = dir.path().join("c.txt");
    let d = dir.path().join("d.txt");
    fs::write(&b, b"from_a").unwrap(); // after first rename a->b
    fs::write(&d, b"from_c").unwrap(); // after second rename c->d

    // Record history: batch1 renamed a->b, batch2 renamed c->d
    let r1 = vec![UndoRecord { from: a.to_string_lossy().into(), to: b.to_string_lossy().into() }];
    brn::append_undo(dir.path(), &r1).unwrap();
    let r2 = vec![UndoRecord { from: c.to_string_lossy().into(), to: d.to_string_lossy().into() }];
    brn::append_undo(dir.path(), &r2).unwrap();

    // Undo last 1 step: reverse c->d (i.e., rename d back to c)
    brn::run_undo_steps(dir.path().to_str().unwrap(), 1).unwrap();

    assert!(c.exists(), "c.txt should be restored");
    assert!(!d.exists(), "d.txt should be gone");
    assert!(b.exists(), "b.txt should still exist (not undone)");
}

#[test]
fn ph5_8_undo_steps_exceeds_history() {
    use xun::batch_rename::undo::{append_undo, UndoRecord};

    let dir = TempDir::new().unwrap();
    let a = dir.path().join("a.txt");
    let b = dir.path().join("b.txt");
    fs::write(&b, b"").unwrap();

    let r1 = vec![UndoRecord { from: a.to_string_lossy().into(), to: b.to_string_lossy().into() }];
    brn::append_undo(dir.path(), &r1).unwrap();

    // steps=5 but only 1 in history → undo all, no error
    let result = brn::run_undo_steps(dir.path().to_str().unwrap(), 5);
    assert!(result.is_ok(), "exceeding steps should not error: {:?}", result);
}

// ─── 多操作组合 (chain) ───────────────────────────────────────────────────────

#[test]
fn chain_prefix_then_case() {
    // prefix "raw_" then case kebab: "My File.txt" → "raw_My File" → "raw-my-file.txt"
    use std::path::PathBuf;
    let files = vec![PathBuf::from("/dir/My File.txt")];
    let steps = vec![
        RenameMode::Prefix("raw_".into()),
        RenameMode::Case(CaseStyle::Kebab),
    ];
    let ops = brn::compute_ops_chain(&files, &steps).unwrap();
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].to.file_name().unwrap(), "raw-my-file.txt");
}

#[test]
fn chain_replace_then_suffix() {
    // replace " " with "_" then suffix "_v2": "my file.txt" → "my_file" → "my_file_v2.txt"
    use std::path::PathBuf;
    use xun::batch_rename::compute::ReplacePair;
    let files = vec![PathBuf::from("/dir/my file.txt")];
    let steps = vec![
        RenameMode::Replace(vec![ReplacePair { from: " ".into(), to: "_".into() }]),
        RenameMode::Suffix("_v2".into()),
    ];
    let ops = brn::compute_ops_chain(&files, &steps).unwrap();
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].to.file_name().unwrap(), "my_file_v2.txt");
}

#[test]
fn chain_single_step_equals_compute_ops() {
    // single step chain should produce same result as compute_ops
    use std::path::PathBuf;
    let files = vec![PathBuf::from("/dir/hello world.txt")];
    let mode = RenameMode::Case(CaseStyle::Snake);
    let single = brn::compute_ops(&files, &mode).unwrap();
    let chain = brn::compute_ops_chain(&files, &[RenameMode::Case(CaseStyle::Snake)]).unwrap();
    assert_eq!(
        single[0].to.file_name().unwrap(),
        chain[0].to.file_name().unwrap()
    );
}

#[test]
fn chain_empty_steps_returns_noop() {
    // empty steps → all ops are from==to (noop)
    use std::path::PathBuf;
    let files = vec![PathBuf::from("/dir/a.txt"), PathBuf::from("/dir/b.txt")];
    let ops = brn::compute_ops_chain(&files, &[]).unwrap();
    assert_eq!(ops.len(), 2);
    assert!(ops.iter().all(|o| o.from == o.to), "empty chain should be noop");
}

#[test]
fn chain_three_steps() {
    // strip suffix "_old" then prefix "new_" then case upper
    // "report_old.txt" → "report" (strip) → "new_report" (prefix) → "NEW-REPORT.txt" (upper)
    use std::path::PathBuf;
    let files = vec![PathBuf::from("/dir/report_old.txt")];
    let steps = vec![
        RenameMode::StripSuffix("_old".into()),
        RenameMode::Prefix("new_".into()),
        RenameMode::Case(CaseStyle::Upper),
    ];
    let ops = brn::compute_ops_chain(&files, &steps).unwrap();
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].to.file_name().unwrap(), "NEW_REPORT.txt");
}
