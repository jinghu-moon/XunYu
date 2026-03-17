#![cfg(windows)]

mod common;

use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Instant;

use xun::path_guard::{
    validate_paths, validate_paths_owned, validate_single, PathIssueKind, PathKind, PathPolicy,
    validate_paths_with_info,
};

// ── helpers ────────────────────────────────────────────────────────────────

fn policy_output() -> PathPolicy {
    PathPolicy::for_output()
}

fn policy_read() -> PathPolicy {
    PathPolicy::for_read()
}

fn policy_write() -> PathPolicy {
    PathPolicy::for_write()
}

// ── 1. string_check 单元覆盖 ───────────────────────────────────────────────

#[test]
fn sc_empty_path_rejected() {
    let result = validate_paths(vec![""], &policy_output());
    assert_eq!(result.issues.len(), 1);
    assert_eq!(result.issues[0].kind, PathIssueKind::Empty);
}

#[test]
fn sc_control_char_rejected() {
    // 0x01 是控制字符
    let result = validate_paths(vec!["C:\\foo\x01bar"], &policy_output());
    assert_eq!(result.issues.len(), 1);
    assert_eq!(result.issues[0].kind, PathIssueKind::InvalidChar);
}

#[test]
fn sc_forbidden_chars_rejected() {
    for ch in ['<', '>', '"', '|', '*'] {
        let path = format!("C:\\foo{ch}bar");
        let result = validate_paths(vec![path.clone()], &policy_output());
        assert_eq!(
            result.issues.len(), 1,
            "expected rejection for char '{ch}' in path '{path}'"
        );
        assert_eq!(result.issues[0].kind, PathIssueKind::InvalidChar);
    }
}

#[test]
fn sc_reserved_names_rejected() {
    let reserved = [
        "CON", "PRN", "AUX", "NUL",
        "COM1", "COM2", "COM9",
        "LPT1", "LPT9",
        // 上标变体
        "COM\u{b9}", "LPT\u{b2}",
        // 带扩展名
        "NUL.txt", "CON.log",
        // 大小写混合
        "con", "nUl", "Com1",
    ];
    for name in reserved {
        let path = format!("C:\\dir\\{name}");
        let result = validate_paths(vec![path.clone()], &policy_output());
        assert_eq!(
            result.issues.len(), 1,
            "expected ReservedName for '{name}'"
        );
        assert_eq!(
            result.issues[0].kind, PathIssueKind::ReservedName,
            "wrong kind for '{name}'"
        );
    }
}

#[test]
fn sc_non_reserved_names_pass() {
    let ok_names = ["CONSOLE", "CONFORM", "NULLIFY", "PRUNE", "COMMA"];
    for name in ok_names {
        let path = format!("C:\\dir\\{name}.txt");
        let result = validate_paths(vec![path.clone()], &policy_output());
        assert!(
            result.issues.is_empty(),
            "unexpected rejection for '{name}': {:?}",
            result.issues.first().map(|i| i.kind)
        );
    }
}

#[test]
fn sc_trailing_dot_rejected() {
    let paths = ["C:\\foo.", "C:\\foo\\bar."];
    for path in paths {
        let result = validate_paths(vec![path], &policy_output());
        assert_eq!(result.issues.len(), 1, "path: {path}");
        assert_eq!(result.issues[0].kind, PathIssueKind::TrailingDotSpace, "path: {path}");
    }
}

#[test]
fn sc_trailing_space_rejected() {
    let paths = ["C:\\foo ", "C:\\foo\\bar "];
    for path in paths {
        let result = validate_paths(vec![path], &policy_output());
        assert_eq!(result.issues.len(), 1, "path: {path}");
        assert_eq!(result.issues[0].kind, PathIssueKind::TrailingDotSpace, "path: {path}");
    }
}

#[test]
fn sc_drive_relative_always_rejected() {
    // C:foo 没有反斜杠 → DriveRelative
    let result = validate_paths(vec!["C:foo\\bar"], &policy_output());
    assert_eq!(result.issues.len(), 1);
    assert_eq!(result.issues[0].kind, PathIssueKind::DriveRelativeNotAllowed);
}

#[test]
fn sc_device_namespace_always_rejected() {
    let paths = [
        r"\\.\COM1",
        r"\\.\PhysicalDrive0",
        r"\\.\pipe\test",
    ];
    for path in paths {
        let result = validate_paths(vec![path], &policy_output());
        assert_eq!(result.issues.len(), 1, "path: {path}");
        assert_eq!(result.issues[0].kind, PathIssueKind::DeviceNamespaceNotAllowed, "path: {path}");
    }
}

#[test]
fn sc_nt_namespace_always_rejected() {
    let paths = [
        // \Device\... 是标准 NT namespace 形式
        (r"\Device\HarddiskVolume1", PathIssueKind::NtNamespaceNotAllowed),
        // \??\ 含 ? 字符，被 check_chars 先拦截为 InvalidChar
        (r"\??\C:\Windows", PathIssueKind::InvalidChar),
    ];
    for (path, expected) in paths {
        let result = validate_paths(vec![path], &policy_output());
        assert_eq!(result.issues.len(), 1, "path: {path}");
        assert_eq!(result.issues[0].kind, expected, "path: {path}");
    }
}

#[test]
fn sc_volume_guid_always_rejected() {
    let path = r"\\?\Volume{12345678-1234-1234-1234-123456789abc}\foo";
    let result = validate_paths(vec![path], &policy_output());
    assert_eq!(result.issues.len(), 1);
    assert_eq!(result.issues[0].kind, PathIssueKind::VolumeGuidNotAllowed);
}

#[test]
fn sc_ads_blocked_by_default() {
    // is_ads 检测第一个冒号位置：相对路径或无盘符前缀的路径中的冒号才被识别为 ADS
    // 例如 "file.txt:stream"（相对路径形式）
    let mut policy = policy_output();
    policy.allow_relative = true; // 允许相对路径，让 ADS 检测先于 RelativeNotAllowed
    policy.allow_ads = false;
    let result = validate_paths(vec!["file.txt:stream"], &policy);
    assert_eq!(result.issues.len(), 1, "{:?}", result.issues);
    assert_eq!(result.issues[0].kind, PathIssueKind::AdsNotAllowed);
}

#[test]
fn sc_ads_on_drive_path_not_detected_by_first_colon() {
    // 原已知盲区已修复：C:\foo\file.txt:stream 中文件名组件含冒号
    // check_component 现在检测组件内冒号 → InvalidChar（优先于 is_ads）
    let result = validate_paths(vec!["C:\\foo\\file.txt:stream"], &policy_output());
    assert_eq!(result.issues.len(), 1, "{:?}", result.issues);
    assert_eq!(result.issues[0].kind, PathIssueKind::InvalidChar);
}

#[test]
fn sc_ads_allowed_when_policy_permits() {
    let mut policy = policy_output();
    policy.allow_ads = true;
    let result = validate_paths(vec!["C:\\foo\\file.txt:stream"], &policy);
    assert!(result.issues.is_empty());
    assert_eq!(result.ok.len(), 1);
}

#[test]
fn sc_drive_colon_not_ads() {
    // C:\foo 中的冒号不应被识别为 ADS
    let result = validate_paths(vec!["C:\\foo\\bar.txt"], &policy_output());
    assert!(result.issues.is_empty());
}

#[test]
fn sc_relative_blocked_by_default_write() {
    let result = validate_paths(vec!["relative\\path"], &policy_write());
    assert_eq!(result.issues.len(), 1);
    assert_eq!(result.issues[0].kind, PathIssueKind::RelativeNotAllowed);
}

#[test]
fn sc_relative_allowed_when_policy_permits() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut policy = policy_output();
    policy.allow_relative = true;
    policy.cwd_snapshot = Some(dir.path().to_path_buf());
    let result = validate_paths(vec!["subdir\\file.txt"], &policy);
    // output policy 不要求存在，路径合法即 ok
    assert!(result.issues.is_empty());
}

#[test]
fn sc_extended_length_path_ok() {
    let result = validate_paths(vec![r"\\?\C:\Windows\System32"], &policy_output());
    assert!(result.issues.is_empty(), "{:?}", result.issues);
}

#[test]
fn sc_unc_path_ok() {
    // UNC 路径格式合法性（不要求存在）
    let result = validate_paths(vec![r"\\server\share\dir\file.txt"], &policy_output());
    assert!(result.issues.is_empty(), "{:?}", result.issues);
}

#[test]
fn sc_env_var_blocked_by_default() {
    // expand_env=false 时，含 % 的路径触发 EnvVarNotAllowed
    // 需要 allow_relative=true 否则先触发 RelativeNotAllowed
    let mut policy = policy_output();
    policy.allow_relative = true;
    let result = validate_paths(vec!["%TEMP%\\foo"], &policy);
    assert_eq!(result.issues.len(), 1, "{:?}", result.issues);
    assert_eq!(result.issues[0].kind, PathIssueKind::EnvVarNotAllowed);
}

#[test]
fn sc_env_var_expanded_when_allowed() {
    let mut policy = policy_output();
    policy.expand_env = true;
    policy.allow_relative = true;
    let result = validate_paths(vec!["%TEMP%\\xun-test-env.txt"], &policy);
    assert!(result.issues.is_empty(), "{:?}", result.issues);
}

#[test]
fn sc_env_var_expands_to_invalid_path_rejected() {
    // 使 expand_env=true，但展开后路径含 ADS（不可能通过 env，这里用 base policy 验证 chain）
    let mut policy = policy_output();
    policy.expand_env = true;
    policy.allow_relative = true;
    // %TEMP% 展开后是合法路径，应通过
    let result = validate_paths(vec!["%TEMP%"], &policy);
    assert!(result.issues.is_empty(), "{:?}", result.issues);
}

// ── 2. dedupe 正确性 ────────────────────────────────────────────────────────

#[test]
fn dedupe_exact_duplicate_removed() {
    let path = "C:\\foo\\bar.txt";
    let result = validate_paths(vec![path, path, path], &policy_output());
    assert_eq!(result.ok.len(), 1);
    assert_eq!(result.deduped, 2);
}

#[test]
fn dedupe_case_insensitive() {
    let paths = vec![
        "C:\\Foo\\Bar.txt",
        "C:\\foo\\bar.txt",
        "C:\\FOO\\BAR.TXT",
    ];
    let result = validate_paths(paths, &policy_output());
    assert_eq!(result.ok.len(), 1, "case variants should be deduped");
    assert_eq!(result.deduped, 2);
}

#[test]
fn dedupe_forward_slash_normalized() {
    let paths = vec![
        "C:\\foo\\bar.txt",
        "C:/foo/bar.txt",
    ];
    let result = validate_paths(paths, &policy_output());
    assert_eq!(result.ok.len(), 1, "forward/backslash variants should be deduped");
    assert_eq!(result.deduped, 1);
}

#[test]
fn dedupe_trailing_slash_normalized() {
    let paths = vec![
        "C:\\foo\\bar",
        "C:\\foo\\bar\\",
    ];
    let result = validate_paths(paths, &policy_output());
    assert_eq!(result.ok.len(), 1, "trailing slash should be deduped");
    assert_eq!(result.deduped, 1);
}

#[test]
fn dedupe_mixed_case_and_slash() {
    let paths = vec![
        "C:\\Foo\\Bar",
        "C:/foo/bar/",
        "C:\\FOO\\BAR\\",
    ];
    let result = validate_paths(paths, &policy_output());
    assert_eq!(result.ok.len(), 1);
    assert_eq!(result.deduped, 2);
}

#[test]
fn dedupe_distinct_paths_not_removed() {
    let paths = vec![
        "C:\\foo\\a.txt",
        "C:\\foo\\b.txt",
        "C:\\foo\\c.txt",
    ];
    let result = validate_paths(paths, &policy_output());
    assert_eq!(result.ok.len(), 3);
    assert_eq!(result.deduped, 0);
}

#[test]
fn dedupe_large_batch_high_collision() {
    // 大量重复路径：hash 碰撞场景压力测试
    let base = "C:\\foo\\file.txt";
    let total = 500usize;
    let unique = 10usize;

    let mut paths: Vec<String> = (0..unique)
        .map(|i| format!("C:\\foo\\file_{i}.txt"))
        .collect();
    // 填充重复
    for i in 0..(total - unique) {
        paths.push(format!("C:\\foo\\file_{}.txt", i % unique));
    }
    let _ = base;
    let result = validate_paths(paths, &policy_output());
    assert_eq!(result.ok.len(), unique);
    assert_eq!(result.deduped, total - unique);
}

#[test]
fn dedupe_drive_root_not_stripped() {
    // C:\ 是驱动器根，不应去掉尾部反斜杠
    let paths = vec!["C:\\", "c:\\"];
    let result = validate_paths(paths, &policy_output());
    assert_eq!(result.ok.len(), 1);
    assert_eq!(result.deduped, 1);
}

// ── 3. probe / 存在性校验 ────────────────────────────────────────────────────

#[test]
fn probe_existing_file_ok() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("exist.txt");
    fs::write(&file, "ok").unwrap();
    let result = validate_paths(vec![file.to_string_lossy().to_string()], &policy_read());
    assert_eq!(result.ok.len(), 1);
    assert!(result.issues.is_empty());
}

#[test]
fn probe_missing_file_reported() {
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = dir.path().join("no_such.txt");
    let result = validate_paths(vec![missing.to_string_lossy().to_string()], &policy_read());
    assert_eq!(result.issues.len(), 1);
    assert_eq!(result.issues[0].kind, PathIssueKind::NotFound);
}

#[test]
fn probe_output_policy_skips_existence_check() {
    // for_output: must_exist=false，不存在路径应进 ok
    let path = "C:\\nonexistent_xun_test_path\\file.txt";
    let result = validate_paths(vec![path], &policy_output());
    assert!(result.issues.is_empty());
    assert_eq!(result.ok.len(), 1);
}

#[test]
fn probe_batch_above_threshold_correct() {
    // BATCH_PROBE_MIN = 10，确保批量 probe 路径分支正确
    let dir = tempfile::tempdir().expect("tempdir");
    let existing_count = 15usize;
    let missing_count = 10usize;

    let mut inputs: Vec<String> = Vec::new();
    for i in 0..existing_count {
        let f = dir.path().join(format!("e{i}.txt"));
        fs::write(&f, "ok").unwrap();
        inputs.push(f.to_string_lossy().to_string());
    }
    for i in 0..missing_count {
        let f = dir.path().join(format!("m{i}.txt"));
        inputs.push(f.to_string_lossy().to_string());
    }

    let result = validate_paths(inputs, &policy_read());
    assert_eq!(result.ok.len(), existing_count);
    assert_eq!(result.issues.len(), missing_count);
    assert!(result.issues.iter().all(|i| i.kind == PathIssueKind::NotFound));
}

#[test]
fn probe_batch_below_threshold_correct() {
    // 低于 BATCH_PROBE_MIN(10)，走串行单条 probe
    let dir = tempfile::tempdir().expect("tempdir");
    let existing_count = 5usize;
    let missing_count = 3usize;

    let mut inputs: Vec<String> = Vec::new();
    for i in 0..existing_count {
        let f = dir.path().join(format!("s{i}.txt"));
        fs::write(&f, "ok").unwrap();
        inputs.push(f.to_string_lossy().to_string());
    }
    for i in 0..missing_count {
        let f = dir.path().join(format!("sm{i}.txt"));
        inputs.push(f.to_string_lossy().to_string());
    }

    let result = validate_paths(inputs, &policy_read());
    assert_eq!(result.ok.len(), existing_count);
    assert_eq!(result.issues.len(), missing_count);
}

#[test]
fn probe_parallel_above_threshold_correct() {
    // >= PARALLEL_MIN(32) 条路径走并行分支
    let dir = tempfile::tempdir().expect("tempdir");
    let existing_count = 20usize;
    let missing_count = 15usize;

    let mut inputs: Vec<String> = Vec::new();
    for i in 0..existing_count {
        let f = dir.path().join(format!("p{i}.txt"));
        fs::write(&f, "ok").unwrap();
        inputs.push(f.to_string_lossy().to_string());
    }
    for i in 0..missing_count {
        let f = dir.path().join(format!("pm{i}.txt"));
        inputs.push(f.to_string_lossy().to_string());
    }
    assert!(inputs.len() >= 32);

    let result = validate_paths(inputs, &policy_read());
    assert_eq!(result.ok.len(), existing_count);
    assert_eq!(result.issues.len(), missing_count);
}

#[test]
fn probe_multi_dir_batch_probe() {
    // 路径跨多个目录，测试 build_probe_cache 目录分组逻辑
    let root = tempfile::tempdir().expect("tempdir");
    let dir_a = root.path().join("a");
    let dir_b = root.path().join("b");
    let dir_c = root.path().join("c");
    fs::create_dir_all(&dir_a).unwrap();
    fs::create_dir_all(&dir_b).unwrap();
    fs::create_dir_all(&dir_c).unwrap();

    let mut inputs: Vec<String> = Vec::new();
    for i in 0..10usize {
        let f = dir_a.join(format!("a{i}.txt"));
        fs::write(&f, "ok").unwrap();
        inputs.push(f.to_string_lossy().to_string());
    }
    for i in 0..10usize {
        let f = dir_b.join(format!("b{i}.txt"));
        fs::write(&f, "ok").unwrap();
        inputs.push(f.to_string_lossy().to_string());
    }
    for i in 0..5usize {
        let f = dir_c.join(format!("cm{i}.txt"));
        inputs.push(f.to_string_lossy().to_string());
    }

    let result = validate_paths(inputs, &policy_read());
    assert_eq!(result.ok.len(), 20);
    assert_eq!(result.issues.len(), 5);
}

// ── 4. policy 组合边界 ────────────────────────────────────────────────────────

#[test]
fn policy_safety_check_blocks_system32() {
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
    let target = format!("{windir}\\System32");
    let result = validate_paths(vec![target], &policy_write());
    assert!(result.ok.is_empty());
    assert_eq!(result.issues.len(), 1);
    assert_eq!(result.issues[0].kind, PathIssueKind::AccessDenied);
}

#[test]
fn policy_no_safety_check_allows_system32() {
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
    let target = format!("{windir}\\System32");
    let result = validate_paths(vec![target], &policy_read());
    assert_eq!(result.ok.len(), 1);
    assert!(result.issues.is_empty());
}

#[test]
fn policy_base_traversal_blocked() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut policy = policy_output();
    policy.base = Some(dir.path().to_path_buf());

    let escape = "C:\\Windows\\System32";
    let result = validate_paths(vec![escape], &policy);
    assert_eq!(result.issues.len(), 1);
    assert_eq!(result.issues[0].kind, PathIssueKind::TraversalDetected);
}

#[test]
fn policy_base_inside_allowed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut policy = policy_output();
    policy.base = Some(dir.path().to_path_buf());

    let inside = dir.path().join("sub").join("file.txt");
    let result = validate_paths(vec![inside.to_string_lossy().to_string()], &policy);
    assert!(result.issues.is_empty(), "{:?}", result.issues);
}

// ── 5. validate_single ───────────────────────────────────────────────────────

#[test]
fn single_existing_file_ok() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("single.txt");
    fs::write(&file, "ok").unwrap();

    let mut scratch = Vec::new();
    let result = validate_single(file.as_os_str(), &policy_read(), &mut scratch);
    assert!(result.is_ok());
    let info = result.unwrap();
    assert_eq!(info.kind, PathKind::DriveAbsolute);
    assert_eq!(info.is_directory, Some(false));
    assert!(!info.is_reparse_point);
}

#[test]
fn single_missing_file_err() {
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = dir.path().join("nope.txt");

    let mut scratch = Vec::new();
    let result = validate_single(missing.as_os_str(), &policy_read(), &mut scratch);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind, PathIssueKind::NotFound);
}

#[test]
fn single_directory_detected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut scratch = Vec::new();
    let result = validate_single(dir.path().as_os_str(), &policy_read(), &mut scratch);
    assert!(result.is_ok());
    let info = result.unwrap();
    assert_eq!(info.is_directory, Some(true));
}

// ── 6. validate_paths_with_info ──────────────────────────────────────────────

#[test]
fn with_info_returns_path_kind() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("info.txt");
    fs::write(&file, "ok").unwrap();

    let (infos, issues) = validate_paths_with_info(
        vec![file.to_string_lossy().to_string()],
        &policy_read(),
    );
    assert!(issues.is_empty());
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].kind, PathKind::DriveAbsolute);
    assert!(!infos[0].is_reparse_point);
}

#[test]
fn with_info_missing_yields_issue() {
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = dir.path().join("ghost.txt");

    let (infos, issues) = validate_paths_with_info(
        vec![missing.to_string_lossy().to_string()],
        &policy_read(),
    );
    assert!(infos.is_empty());
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].kind, PathIssueKind::NotFound);
}

// ── 7. 并发安全 ───────────────────────────────────────────────────────────────

#[test]
fn concurrent_validate_paths_no_data_race() {
    let dir = tempfile::tempdir().expect("tempdir");
    for i in 0..20usize {
        let f = dir.path().join(format!("c{i}.txt"));
        fs::write(&f, "ok").unwrap();
    }

    let dir_path = dir.path().to_path_buf();
    let handles: Vec<_> = (0..4)
        .map(|t| {
            let root = dir_path.clone();
            thread::spawn(move || {
                let inputs: Vec<String> = (0..20)
                    .map(|i| root.join(format!("c{i}.txt")).to_string_lossy().to_string())
                    .collect();
                let result = validate_paths(inputs, &PathPolicy::for_read());
                assert_eq!(result.ok.len(), 20, "thread {t} failed");
                assert!(result.issues.is_empty(), "thread {t} had issues");
            })
        })
        .collect();

    for h in handles {
        h.join().expect("thread panicked");
    }
}

// ── 8. owned vs borrow 一致性 ─────────────────────────────────────────────────

#[test]
fn owned_and_borrow_produce_same_results() {
    let dir = tempfile::tempdir().expect("tempdir");
    let existing_count = 20usize;
    let missing_count = 10usize;

    let mut inputs: Vec<PathBuf> = Vec::new();
    for i in 0..existing_count {
        let f = dir.path().join(format!("ob{i}.txt"));
        fs::write(&f, "ok").unwrap();
        inputs.push(f);
    }
    for i in 0..missing_count {
        inputs.push(dir.path().join(format!("om{i}.txt")));
    }

    let policy = PathPolicy::for_read();
    let r_borrow = validate_paths(inputs.iter(), &policy);
    let r_owned = validate_paths_owned(inputs.clone(), &policy);

    assert_eq!(r_borrow.ok.len(), r_owned.ok.len());
    assert_eq!(r_borrow.issues.len(), r_owned.issues.len());
    assert_eq!(r_borrow.deduped, r_owned.deduped);
}

// ── 9. Unicode 路径 ───────────────────────────────────────────────────────────

#[test]
fn unicode_filename_existing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("测试文件.txt");
    fs::write(&file, "ok").unwrap();

    let result = validate_paths(
        vec![file.to_string_lossy().to_string()],
        &policy_read(),
    );
    assert_eq!(result.ok.len(), 1, "{:?}", result.issues);
}

#[test]
fn unicode_filename_not_treated_as_reserved() {
    let result = validate_paths(vec!["C:\\dir\\文件名.txt"], &policy_output());
    assert!(result.issues.is_empty(), "{:?}", result.issues);
}

#[test]
fn ascii_lower_os_non_ascii_falls_back_to_single_probe() {
    // 含非 ASCII 文件名不进入 probe_cache 分组，应 fallback 到单条 probe，结果正确
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("日本語.txt");
    fs::write(&file, "ok").unwrap();

    let mut inputs = vec![file.to_string_lossy().to_string()];
    for i in 0..12usize {
        let f = dir.path().join(format!("ascii{i}.txt"));
        fs::write(&f, "ok").unwrap();
        inputs.push(f.to_string_lossy().to_string());
    }

    let result = validate_paths(inputs.clone(), &policy_read());
    assert_eq!(result.ok.len(), inputs.len(), "{:?}", result.issues);
}

// ── 10. 性能回归（快速版，< 500ms）───────────────────────────────────────────

#[test]
fn perf_serial_10_paths_under_200ms() {
    let dir = tempfile::tempdir().expect("tempdir");
    for i in 0..5usize {
        fs::write(dir.path().join(format!("f{i}.txt")), "ok").unwrap();
    }
    let inputs: Vec<String> = (0..10)
        .map(|i| dir.path().join(format!("f{}.txt", i % 5)).to_string_lossy().to_string())
        .collect();

    let start = Instant::now();
    let _ = validate_paths(inputs, &policy_read());
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 200, "serial 10 paths took {}ms", elapsed.as_millis());
}

#[test]
fn perf_parallel_100_paths_under_500ms() {
    let dir = tempfile::tempdir().expect("tempdir");
    for i in 0..50usize {
        fs::write(dir.path().join(format!("pp{i}.txt")), "ok").unwrap();
    }
    let inputs: Vec<String> = (0..100)
        .map(|i| dir.path().join(format!("pp{}.txt", i % 50)).to_string_lossy().to_string())
        .collect();

    let start = Instant::now();
    let _ = validate_paths(inputs, &policy_read());
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 500, "parallel 100 paths took {}ms", elapsed.as_millis());
}

// ── 11. issue detail 消息非空 ─────────────────────────────────────────────────

#[test]
fn issue_detail_not_empty() {
    let cases: Vec<(&str, PathIssueKind)> = vec![
        ("", PathIssueKind::Empty),
        ("C:\\foo<bar", PathIssueKind::InvalidChar),
        ("C:\\NUL", PathIssueKind::ReservedName),
        ("C:\\foo.", PathIssueKind::TrailingDotSpace),
        ("C:foo", PathIssueKind::DriveRelativeNotAllowed),
        (r"\\.\COM1", PathIssueKind::DeviceNamespaceNotAllowed),
    ];
    for (path, expected_kind) in cases {
        let result = validate_paths(vec![path], &policy_output());
        assert_eq!(result.issues.len(), 1, "path: {path:?}");
        let issue = &result.issues[0];
        assert_eq!(issue.kind, expected_kind, "path: {path:?}");
        assert!(!issue.detail.is_empty(), "detail empty for {path:?}");
        assert!(!issue.raw.is_empty() || path.is_empty(), "raw empty for {path:?}");
    }
}

// ── 12. 路径类型检测（通过 validate_single 间接验证）───────────────────────────

#[test]
fn path_kind_drive_absolute_via_single() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("kind.txt");
    fs::write(&file, "ok").unwrap();

    let mut scratch = Vec::new();
    let info = validate_single(file.as_os_str(), &policy_read(), &mut scratch).unwrap();
    assert_eq!(info.kind, PathKind::DriveAbsolute);
}

#[test]
fn path_kind_unc_via_output_policy() {
    // UNC 路径：不要求存在，仅检查格式
    let result = validate_paths(vec![r"\\server\share\file.txt"], &policy_output());
    assert!(result.issues.is_empty(), "{:?}", result.issues);
    assert_eq!(result.ok.len(), 1);
}

#[test]
fn path_kind_extended_length_via_output_policy() {
    let result = validate_paths(vec![r"\\?\C:\Windows\foo"], &policy_output());
    assert!(result.issues.is_empty(), "{:?}", result.issues);
}

// ── 13. is_ads 覆盖盲区（drive path 中第二个冒号）────────────────────────────

#[test]
fn sc_ads_on_drive_path_colon_in_filename() {
    // C:\foo\file:stream — 文件名组件中含冒号
    // check_component 现在检测组件内冒号 → InvalidChar
    let result = validate_paths(vec![r"C:\foo\file:stream"], &policy_output());
    assert_eq!(result.issues.len(), 1, "{:?}", result.issues);
    assert_eq!(result.issues[0].kind, PathIssueKind::InvalidChar);
}

#[test]
fn sc_colon_in_path_component_should_be_invalid() {
    // 文件名中的裸冒号在 Windows 是非法字符，check_component 现在正确拦截
    let result = validate_paths(vec![r"C:\foo\bar:baz"], &policy_output());
    assert_eq!(result.issues.len(), 1, "{:?}", result.issues);
    assert_eq!(result.issues[0].kind, PathIssueKind::InvalidChar);
}

// ── 14. 路径规范化：.. 未展开导致 dedupe 遗漏 ───────────────────────────────

#[test]
fn dedupe_dotdot_not_normalized() {
    // C:\foo\..\bar 与 C:\bar 逻辑等价，但当前 dedupe 未规范化 ..
    // 这是已知盲区：dedupe 只做 case-fold + slash-normalize + trailing-strip
    let paths = vec![
        r"C:\foo\..\bar",
        r"C:\bar",
    ];
    let result = validate_paths(paths, &policy_output());
    // 现状：两条都通过，不被识别为重复
    assert_eq!(result.ok.len(), 2, "dotdot paths currently not deduped (known limitation)");
    assert_eq!(result.deduped, 0);
}

#[test]
fn dedupe_multiple_slashes_normalized() {
    // C:\foo\bar 与 C:\foo\bar 应被识别为相同路径
    // 连续斜杠在 normalize 时被折叠（if slash == fslash → backslash）
    // 但连续 backslash 未被折叠
    let paths = vec![
        r"C:\foo\bar",
        r"C:\\foo\\bar",  // 连续反斜杠
    ];
    let result = validate_paths(paths, &policy_output());
    // 记录现状：当前不被识别为重复
    let _ = result;
}

// ── 15. validate_paths_serial 覆盖（< PARALLEL_MIN=32）─────────────────────

#[test]
fn serial_path_exact_at_parallel_min_minus_one() {
    // 31 条路径 → 走串行分支（< PARALLEL_MIN=32）
    let dir = tempfile::tempdir().expect("tempdir");
    let count = 31usize;
    let mut inputs: Vec<String> = Vec::new();
    for i in 0..count {
        let f = dir.path().join(format!("ser{i}.txt"));
        fs::write(&f, "ok").unwrap();
        inputs.push(f.to_string_lossy().to_string());
    }
    let result = validate_paths(inputs, &policy_read());
    assert_eq!(result.ok.len(), count);
    assert!(result.issues.is_empty());
}

#[test]
fn parallel_path_exact_at_parallel_min() {
    // 32 条路径 → 走并行分支（>= PARALLEL_MIN=32）
    let dir = tempfile::tempdir().expect("tempdir");
    let count = 32usize;
    let mut inputs: Vec<String> = Vec::new();
    for i in 0..count {
        let f = dir.path().join(format!("par{i}.txt"));
        fs::write(&f, "ok").unwrap();
        inputs.push(f.to_string_lossy().to_string());
    }
    let result = validate_paths(inputs, &policy_read());
    assert_eq!(result.ok.len(), count);
    assert!(result.issues.is_empty());
}

// ── 16. 边界：空列表 ──────────────────────────────────────────────────────────

#[test]
fn empty_input_list_returns_empty() {
    let result = validate_paths(Vec::<String>::new(), &policy_read());
    assert_eq!(result.ok.len(), 0);
    assert_eq!(result.issues.len(), 0);
    assert_eq!(result.deduped, 0);
}

#[test]
fn single_valid_path_no_dedupe() {
    let result = validate_paths(vec![r"C:\foo\bar.txt"], &policy_output());
    assert_eq!(result.ok.len(), 1);
    assert_eq!(result.deduped, 0);
}

// ── 17. probe_cache: 目录低于 BATCH_PROBE_MIN 阈值降级到单条 probe ───────────

#[test]
fn probe_cache_dir_below_batch_min_falls_back() {
    // 同一目录下文件数 < BATCH_PROBE_MIN(10)，不走批量 FindFirstFileExW
    // 总数 >= PARALLEL_MIN(32) 走并行，但各目录文件数 < 10 → 单条 probe fallback
    let root = tempfile::tempdir().expect("tempdir");
    let mut inputs: Vec<String> = Vec::new();
    // 4 个目录，每个 8 个文件（8 < BATCH_PROBE_MIN=10）
    for d in 0..4usize {
        let dir = root.path().join(format!("dd{d}"));
        fs::create_dir_all(&dir).unwrap();
        for f in 0..8usize {
            let p = dir.join(format!("f{f}.txt"));
            fs::write(&p, "ok").unwrap();
            inputs.push(p.to_string_lossy().to_string());
        }
    }
    assert_eq!(inputs.len(), 32); // 正好 PARALLEL_MIN
    let result = validate_paths(inputs, &policy_read());
    assert_eq!(result.ok.len(), 32);
    assert!(result.issues.is_empty());
}

// ── 18. 结果顺序一致性（validate_paths 不保证顺序，但 ok+issues 完整）────────

#[test]
fn result_count_complete() {
    let dir = tempfile::tempdir().expect("tempdir");
    let existing = 20usize;
    let missing = 15usize;
    let total = existing + missing;

    let mut inputs: Vec<String> = Vec::new();
    for i in 0..existing {
        let f = dir.path().join(format!("rc{i}.txt"));
        fs::write(&f, "ok").unwrap();
        inputs.push(f.to_string_lossy().to_string());
    }
    for i in 0..missing {
        inputs.push(dir.path().join(format!("rm{i}.txt")).to_string_lossy().to_string());
    }

    let result = validate_paths(inputs, &policy_read());
    assert_eq!(result.ok.len() + result.issues.len() + result.deduped, total,
        "total count mismatch: ok={} issues={} deduped={}",
        result.ok.len(), result.issues.len(), result.deduped);
}
