use std::fs;

use xun::path_guard::{PathIssueKind, PathPolicy, validate_paths};

#[test]
fn validate_paths_reports_existing_and_missing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("ok.txt");
    fs::write(&file, "ok").expect("write");

    let policy = PathPolicy::for_read();
    let ok = validate_paths(vec![file.to_string_lossy().to_string()], &policy);
    assert_eq!(ok.ok.len(), 1);
    assert!(ok.issues.is_empty());

    let missing = dir.path().join("missing.txt");
    let missing = validate_paths(vec![missing.to_string_lossy().to_string()], &policy);
    assert_eq!(missing.ok.len(), 0);
    assert_eq!(missing.issues.len(), 1);
    assert_eq!(missing.issues[0].kind, PathIssueKind::NotFound);
}

#[test]
fn validate_paths_blocks_system_dir_when_safety_check() {
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
    let target = format!("{windir}\\System32");
    let policy = PathPolicy::for_write();

    let result = validate_paths(vec![target], &policy);
    assert!(result.ok.is_empty());
    assert_eq!(result.issues.len(), 1);
    assert_eq!(result.issues[0].kind, PathIssueKind::AccessDenied);
}

#[test]
fn validate_paths_supports_file_list_and_csv_inputs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file_a = dir.path().join("a.txt");
    let file_b = dir.path().join("b.txt");
    fs::write(&file_a, "a").expect("write a");
    fs::write(&file_b, "b").expect("write b");

    let list_file = dir.path().join("paths.txt");
    let list_content = format!(
        "# comment\n{}\n\n{}\n",
        file_a.to_string_lossy(),
        file_b.to_string_lossy()
    );
    fs::write(&list_file, list_content).expect("write list");

    let raw = fs::read_to_string(&list_file).expect("read list");
    let file_paths: Vec<String> = raw
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect();

    let policy = PathPolicy::for_read();
    let from_file = validate_paths(file_paths, &policy);
    assert_eq!(from_file.ok.len(), 2);
    assert!(from_file.issues.is_empty());

    let csv = format!("{},{}", file_a.to_string_lossy(), file_b.to_string_lossy());
    let csv_paths: Vec<String> = csv
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    let from_csv = validate_paths(csv_paths, &policy);
    assert_eq!(from_csv.ok.len(), 2);
    assert!(from_csv.issues.is_empty());
}

#[test]
fn validate_paths_unc_optional() {
    let Ok(unc) = std::env::var("XUN_TEST_UNC") else {
        return;
    };
    let policy = PathPolicy::for_read();
    let result = validate_paths(vec![unc.clone()], &policy);
    assert!(
        result.issues.is_empty(),
        "UNC path should be valid when XUN_TEST_UNC is set: {:?}",
        result.issues
    );
    assert_eq!(result.ok.len(), 1);
}
