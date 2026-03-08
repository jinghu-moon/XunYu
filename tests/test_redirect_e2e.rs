#![cfg(all(windows, feature = "redirect"))]

mod common;

use common::*;
use serde_json::Value;
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows_sys::Win32::Foundation::FILETIME;
use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, OPEN_EXISTING, SetFileTime,
};

fn write_redirect_config(env: &TestEnv) {
    let cfg = r#"
{
  "redirect": {
    "profiles": {
      "default": {
        "rules": [
          { "name": "Images", "match": { "ext": ["jpg", "png"] }, "dest": "./Images" },
          { "name": "Reports", "match": { "glob": "report_*" }, "dest": "./Reports" }
        ],
        "unmatched": "skip",
        "on_conflict": "rename_new"
      }
    }
  }
}
"#;
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();
}

fn to_wide(s: &Path) -> Vec<u16> {
    s.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn filetime_from_unix_secs(secs: u64) -> FILETIME {
    // Windows FILETIME: 100ns intervals since 1601-01-01
    const EPOCH_DIFF_SECS: u64 = 11_644_473_600;
    let intervals = (secs.saturating_add(EPOCH_DIFF_SECS)).saturating_mul(10_000_000);
    FILETIME {
        dwLowDateTime: (intervals & 0xFFFF_FFFF) as u32,
        dwHighDateTime: (intervals >> 32) as u32,
    }
}

fn set_mtime(path: &Path, unix_secs: u64) {
    let wide = to_wide(path);
    // Use direct Win32 to avoid extra deps in tests.
    let h = unsafe {
        CreateFileW(
            wide.as_ptr(),
            0x40000000, // GENERIC_WRITE
            0x00000007, // FILE_SHARE_READ|WRITE|DELETE
            std::ptr::null_mut(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            std::ptr::null_mut(),
        )
    };
    if h == INVALID_HANDLE_VALUE {
        panic!("set_mtime: CreateFileW failed");
    }
    let ft = filetime_from_unix_secs(unix_secs);
    let ok = unsafe { SetFileTime(h, std::ptr::null(), std::ptr::null(), &ft as *const _) };
    unsafe { CloseHandle(h) };
    assert!(ok != 0, "set_mtime: SetFileTime failed");
}

fn set_ctime_mtime(path: &Path, created_unix_secs: u64, modified_unix_secs: u64) {
    let wide = to_wide(path);
    let h = unsafe {
        CreateFileW(
            wide.as_ptr(),
            0x40000000, // GENERIC_WRITE
            0x00000007, // FILE_SHARE_READ|WRITE|DELETE
            std::ptr::null_mut(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            std::ptr::null_mut(),
        )
    };
    if h == INVALID_HANDLE_VALUE {
        panic!("set_ctime_mtime: CreateFileW failed");
    }
    let c = filetime_from_unix_secs(created_unix_secs);
    let m = filetime_from_unix_secs(modified_unix_secs);
    let ok = unsafe { SetFileTime(h, &c as *const _, std::ptr::null(), &m as *const _) };
    unsafe { CloseHandle(h) };
    assert!(ok != 0, "set_ctime_mtime: SetFileTime failed");
}

fn year_month_from_unix_secs(secs: u64) -> (u16, u16) {
    let ft = filetime_from_unix_secs(secs);
    let mut st = windows_sys::Win32::Foundation::SYSTEMTIME {
        wYear: 0,
        wMonth: 0,
        wDayOfWeek: 0,
        wDay: 0,
        wHour: 0,
        wMinute: 0,
        wSecond: 0,
        wMilliseconds: 0,
    };
    let ok = unsafe {
        windows_sys::Win32::System::Time::FileTimeToSystemTime(&ft as *const _, &mut st as *mut _)
    };
    assert!(ok != 0, "FileTimeToSystemTime failed");
    (st.wYear, st.wMonth)
}

#[test]
fn redirect_moves_by_ext_to_dest_dir() {
    let env = TestEnv::new();
    write_redirect_config(&env);

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.jpg"), "img").unwrap();
    fs::write(src.join("b.txt"), "t").unwrap();

    let out = run_ok(
        env.cmd()
            .args(["redirect", src.to_str().unwrap(), "--format", "json"]),
    );
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let arr = v.as_array().unwrap();
    assert!(
        arr.iter()
            .any(|x| x["src"].as_str().unwrap().ends_with("a.jpg"))
    );
    for row in arr {
        assert!(row.get("action").is_some());
        assert!(row.get("src").and_then(Value::as_str).is_some());
        assert!(row.get("dst").and_then(Value::as_str).is_some());
        assert!(row.get("rule").and_then(Value::as_str).is_some());
        assert!(row.get("result").and_then(Value::as_str).is_some());
        assert!(row.get("reason").and_then(Value::as_str).is_some());
    }

    assert!(!src.join("a.jpg").exists(), "jpg should be moved out");
    assert!(
        src.join("Images").join("a.jpg").exists(),
        "jpg should be moved into Images"
    );
    assert!(src.join("b.txt").exists(), "unmatched file should remain");
}

#[test]
fn redirect_dry_run_has_no_side_effects() {
    let env = TestEnv::new();
    write_redirect_config(&env);

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.jpg"), "img").unwrap();

    run_ok(env.cmd().args([
        "redirect",
        src.to_str().unwrap(),
        "--dry-run",
        "--format",
        "tsv",
    ]));

    assert!(src.join("a.jpg").exists(), "dry run should not move file");
    assert!(
        !src.join("Images").join("a.jpg").exists(),
        "dry run should not create destination file"
    );
}

#[test]
fn redirect_copy_keeps_source_file() {
    let env = TestEnv::new();
    write_redirect_config(&env);

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.jpg"), "img").unwrap();

    run_ok(env.cmd().args([
        "redirect",
        src.to_str().unwrap(),
        "--copy",
        "--format",
        "tsv",
    ]));

    assert!(src.join("a.jpg").exists(), "copy should keep source file");
    assert!(
        src.join("Images").join("a.jpg").exists(),
        "copy should create dest file"
    );
}

#[test]
fn redirect_rename_new_conflict_adds_suffix() {
    let env = TestEnv::new();
    write_redirect_config(&env);

    let src = env.root.join("src");
    fs::create_dir_all(src.join("Images")).unwrap();
    fs::write(src.join("a.jpg"), "img1").unwrap();
    fs::write(src.join("Images").join("a.jpg"), "existing").unwrap();

    run_ok(
        env.cmd()
            .args(["redirect", src.to_str().unwrap(), "--format", "json"]),
    );

    assert!(!src.join("a.jpg").exists(), "source should be moved");
    assert!(
        src.join("Images").join("a (1).jpg").exists(),
        "conflict should be renamed"
    );
}

#[test]
fn redirect_rename_date_conflict_adds_timestamp_suffix() {
    let env = TestEnv::new();

    let cfg = r#"
{
  "redirect": {
    "profiles": {
      "default": {
        "rules": [
          { "name": "Images", "match": { "ext": ["jpg"] }, "dest": "./Images" }
        ],
        "unmatched": "skip",
        "on_conflict": "rename_date"
      }
    }
  }
}
"#;
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();

    let src = env.root.join("src");
    fs::create_dir_all(src.join("Images")).unwrap();
    fs::write(src.join("a.jpg"), "img1").unwrap();
    fs::write(src.join("Images").join("a.jpg"), "existing").unwrap();

    run_ok(
        env.cmd()
            .args(["redirect", src.to_str().unwrap(), "--format", "tsv"]),
    );

    assert!(!src.join("a.jpg").exists(), "source should be moved");

    let images = src.join("Images");
    let moved: Vec<String> = fs::read_dir(&images)
        .unwrap()
        .flatten()
        .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
        .filter(|n| n.starts_with("a (") && n.ends_with(").jpg"))
        .collect();
    assert!(
        !moved.is_empty(),
        "expected timestamp-suffixed file in Images"
    );
}

#[test]
fn redirect_glob_matches_and_rule_order_is_first_match() {
    let env = TestEnv::new();
    write_redirect_config(&env);

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();

    // Matches ext rule first even though glob also matches: should go to Images.
    fs::write(src.join("report_a.jpg"), "img").unwrap();
    // Matches glob rule only: should go to Reports.
    fs::write(src.join("report_b.txt"), "txt").unwrap();

    run_ok(
        env.cmd()
            .args(["redirect", src.to_str().unwrap(), "--format", "tsv"]),
    );

    assert!(src.join("Images").join("report_a.jpg").exists());
    assert!(src.join("Reports").join("report_b.txt").exists());
}

#[test]
fn redirect_regex_matches_file_name() {
    let env = TestEnv::new();

    let cfg = r#"
{
  "redirect": {
    "profiles": {
      "default": {
        "rules": [
          { "name": "ByRegex", "match": { "regex": "^\\d{4}-\\d{2}" }, "dest": "./ByRegex" }
        ],
        "unmatched": "skip",
        "on_conflict": "rename_new"
      }
    }
  }
}
 "#;
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("2026-02_report.txt"), "t").unwrap();
    fs::write(src.join("report_2026-02.txt"), "t").unwrap();

    run_ok(
        env.cmd()
            .args(["redirect", src.to_str().unwrap(), "--format", "tsv"]),
    );

    assert!(src.join("ByRegex").join("2026-02_report.txt").exists());
    assert!(
        src.join("report_2026-02.txt").exists(),
        "should remain unmatched"
    );
}

#[test]
fn redirect_size_matches_file_size() {
    let env = TestEnv::new();

    let cfg = r#"
{
  "redirect": {
    "profiles": {
      "default": {
        "rules": [
          { "name": "Big", "match": { "size": ">=4b" }, "dest": "./Big" }
        ],
        "unmatched": "skip",
        "on_conflict": "rename_new"
      }
    }
  }
}
"#;
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.bin"), [0u8; 4]).unwrap();
    fs::write(src.join("b.bin"), [0u8; 3]).unwrap();

    run_ok(
        env.cmd()
            .args(["redirect", src.to_str().unwrap(), "--format", "tsv"]),
    );

    assert!(src.join("Big").join("a.bin").exists());
    assert!(src.join("b.bin").exists(), "should remain unmatched");
}

#[test]
fn redirect_age_matches_file_mtime() {
    let env = TestEnv::new();

    let cfg = r#"
{
  "redirect": {
    "profiles": {
      "default": {
        "rules": [
          { "name": "Old", "match": { "age": ">=2d" }, "dest": "./Old" }
        ],
        "unmatched": "skip",
        "on_conflict": "rename_new"
      }
    }
  }
}
"#;
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();
    let old = src.join("old.txt");
    let fresh = src.join("fresh.txt");
    fs::write(&old, "o").unwrap();
    fs::write(&fresh, "f").unwrap();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    set_mtime(&old, now.saturating_sub(3 * 86400));

    run_ok(
        env.cmd()
            .args(["redirect", src.to_str().unwrap(), "--format", "tsv"]),
    );

    assert!(src.join("Old").join("old.txt").exists());
    assert!(src.join("fresh.txt").exists(), "should remain unmatched");
}

#[test]
fn redirect_unmatched_archive_moves_old_files_to_others() {
    let env = TestEnv::new();
    let cfg = r#"
{
  "redirect": {
    "profiles": {
      "default": {
        "rules": [
          { "name": "Images", "match": { "ext": ["jpg"] }, "dest": "./Images" }
        ],
        "unmatched": "archive:>=2d:./Others",
        "on_conflict": "rename_new",
        "recursive": false,
        "max_depth": 1
      }
    }
  }
}
"#;
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();
    let old = src.join("old.txt");
    let fresh = src.join("fresh.txt");
    fs::write(&old, "o").unwrap();
    fs::write(&fresh, "f").unwrap();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    set_mtime(&old, now.saturating_sub(3 * 86400));

    run_ok(
        env.cmd()
            .args(["redirect", src.to_str().unwrap(), "--format", "tsv"]),
    );

    assert!(src.join("Others").join("old.txt").exists());
    assert!(
        src.join("fresh.txt").exists(),
        "fresh unmatched should remain"
    );
}

#[test]
fn redirect_dest_template_renders_created_year_month() {
    let env = TestEnv::new();
    let cfg = r#"
{
  "redirect": {
    "profiles": {
      "default": {
        "rules": [
          { "name": "Images", "match": { "ext": ["jpg"] }, "dest": "./Images/{created.year}/{created.month}" }
        ],
        "unmatched": "skip",
        "on_conflict": "rename_new",
        "recursive": false,
        "max_depth": 1
      }
    }
  }
}
"#;
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();
    let f = src.join("a.jpg");
    fs::write(&f, "img").unwrap();

    let created = 1_700_000_000u64;
    let modified = created + 60;
    set_ctime_mtime(&f, created, modified);
    let (y, m) = year_month_from_unix_secs(created);

    run_ok(
        env.cmd()
            .args(["redirect", src.to_str().unwrap(), "--format", "tsv"]),
    );

    let dest = src
        .join("Images")
        .join(format!("{y:04}"))
        .join(format!("{m:02}"))
        .join("a.jpg");
    assert!(dest.exists(), "expected file moved into template directory");
}

#[test]
fn redirect_recursive_scan_moves_nested_files_and_respects_xunignore() {
    let env = TestEnv::new();
    let cfg = r#"
{
  "redirect": {
    "profiles": {
      "default": {
        "rules": [
          { "name": "Images", "match": { "ext": ["jpg"] }, "dest": "./Images" }
        ],
        "unmatched": "skip",
        "on_conflict": "rename_new",
        "recursive": true,
        "max_depth": 3
      }
    }
  }
}
"#;
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();

    let src = env.root.join("src");
    fs::create_dir_all(src.join("sub").join("inner")).unwrap();
    fs::create_dir_all(src.join("sub2")).unwrap();
    fs::write(src.join("sub").join("inner").join("a.jpg"), "img-a").unwrap();
    fs::write(src.join("sub2").join("b.jpg"), "img-b").unwrap();
    fs::write(src.join(".xunignore"), "sub/inner/\n").unwrap();

    run_ok(
        env.cmd()
            .args(["redirect", src.to_str().unwrap(), "--format", "tsv"]),
    );

    assert!(
        src.join("sub").join("inner").join("a.jpg").exists(),
        "ignored directory should not be scanned"
    );
    assert!(
        src.join("Images").join("b.jpg").exists(),
        "non-ignored nested file should be moved"
    );
}

#[test]
fn redirect_recursive_scan_respects_max_depth() {
    let env = TestEnv::new();
    let cfg = r#"
{
  "redirect": {
    "profiles": {
      "default": {
        "rules": [
          { "name": "Images", "match": { "ext": ["jpg"] }, "dest": "./Images" }
        ],
        "unmatched": "skip",
        "on_conflict": "rename_new",
        "recursive": true,
        "max_depth": 1
      }
    }
  }
}
"#;
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();

    let src = env.root.join("src");
    fs::create_dir_all(src.join("d1")).unwrap();
    fs::create_dir_all(src.join("d1").join("d2")).unwrap();
    fs::write(src.join("d1").join("a.jpg"), "img-a").unwrap();
    fs::write(src.join("d1").join("d2").join("b.jpg"), "img-b").unwrap();

    run_ok(
        env.cmd()
            .args(["redirect", src.to_str().unwrap(), "--format", "tsv"]),
    );

    assert!(
        src.join("Images").join("a.jpg").exists(),
        "depth=1 should be scanned"
    );
    assert!(
        src.join("d1").join("d2").join("b.jpg").exists(),
        "depth=2 should not be scanned when max_depth=1"
    );
}

#[test]
fn redirect_trash_conflict_removes_existing_then_moves_new() {
    let env = TestEnv::new();
    let cfg = r#"
{
  "redirect": {
    "profiles": {
      "default": {
        "rules": [
          { "name": "Images", "match": { "ext": ["jpg"] }, "dest": "./Images" }
        ],
        "unmatched": "skip",
        "on_conflict": "trash",
        "recursive": false,
        "max_depth": 1
      }
    }
  }
}
"#;
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();

    let src = env.root.join("src");
    fs::create_dir_all(src.join("Images")).unwrap();
    fs::write(src.join("a.jpg"), "new").unwrap();
    fs::write(src.join("Images").join("a.jpg"), "old").unwrap();

    run_ok(
        env.cmd()
            .args(["redirect", src.to_str().unwrap(), "--format", "tsv"]),
    );

    assert!(!src.join("a.jpg").exists());
    let content = fs::read_to_string(src.join("Images").join("a.jpg")).unwrap_or_default();
    assert_eq!(content, "new");
    assert!(
        !src.join("Images").join("a (1).jpg").exists(),
        "trash should not create rename backup"
    );
}

#[test]
fn redirect_profile_missing_exits_nonzero() {
    let env = TestEnv::new();
    write_redirect_config(&env);

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.jpg"), "img").unwrap();

    let out = run_err(env.cmd().args([
        "redirect",
        src.to_str().unwrap(),
        "--profile",
        "nope",
        "--format",
        "json",
    ]));
    assert_eq!(out.status.code(), Some(2));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("Redirect profile not found"));
}

#[test]
fn redirect_rules_empty_is_config_error() {
    let env = TestEnv::new();
    let cfg = r#"
{
  "redirect": {
    "profiles": {
      "default": { "rules": [], "unmatched": "skip", "on_conflict": "rename_new" }
    }
  }
}
"#;
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.jpg"), "img").unwrap();

    let out = run_err(env.cmd().args(["redirect", src.to_str().unwrap()]));
    assert_eq!(out.status.code(), Some(2));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("rules is empty"));
}

#[test]
fn redirect_tsv_output_has_stable_columns() {
    let env = TestEnv::new();
    write_redirect_config(&env);

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.jpg"), "img").unwrap();

    let out = run_ok(
        env.cmd()
            .args(["redirect", src.to_str().unwrap(), "--format", "tsv"]),
    );
    let s = String::from_utf8_lossy(&out.stdout);
    let first = s.lines().next().unwrap_or("");
    let cols: Vec<&str> = first.split('\t').collect();
    assert_eq!(cols.len(), 6, "expected 6 tsv columns: {first}");
}

#[test]
fn redirect_conflict_skip_keeps_source() {
    let env = TestEnv::new();
    let cfg = r#"
{
  "redirect": {
    "profiles": {
      "default": {
        "rules": [
          { "name": "Images", "match": { "ext": ["jpg"] }, "dest": "./Images" }
        ],
        "unmatched": "skip",
        "on_conflict": "skip"
      }
    }
  }
}
"#;
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();

    let src = env.root.join("src");
    fs::create_dir_all(src.join("Images")).unwrap();
    fs::write(src.join("a.jpg"), "img1").unwrap();
    fs::write(src.join("Images").join("a.jpg"), "existing").unwrap();

    run_ok(
        env.cmd()
            .args(["redirect", src.to_str().unwrap(), "--format", "json"]),
    );

    assert!(src.join("a.jpg").exists(), "source should remain on skip");
}

#[test]
fn redirect_overwrite_requires_yes_in_non_interactive() {
    let env = TestEnv::new();

    let src = env.root.join("src");
    fs::create_dir_all(src.join("Images")).unwrap();
    fs::write(src.join("a.jpg"), "img1").unwrap();
    fs::write(src.join("Images").join("a.jpg"), "existing").unwrap();

    let cfg = r#"
{
  "redirect": {
    "profiles": {
      "default": {
        "rules": [
          { "name": "Images", "match": { "ext": ["jpg"] }, "dest": "./Images" }
        ],
        "unmatched": "skip",
        "on_conflict": "overwrite"
      }
    }
  }
}
"#;
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();

    let out = run_err(env.cmd().args(["redirect", src.to_str().unwrap()]));
    assert_eq!(out.status.code(), Some(2));

    run_ok(env.cmd().args([
        "redirect",
        src.to_str().unwrap(),
        "--yes",
        "--format",
        "tsv",
    ]));
    assert!(src.join("Images").join("a.jpg").exists());
    let content = fs::read_to_string(src.join("Images").join("a.jpg")).unwrap_or_default();
    assert!(
        content.contains("img1"),
        "expected overwrite to replace content"
    );
}

#[test]
fn redirect_hash_dedup_move_deletes_source_when_dest_same_content() {
    let env = TestEnv::new();
    let cfg = r#"
{
  "redirect": {
    "profiles": {
      "default": {
        "rules": [
          { "name": "Images", "match": { "ext": ["jpg"] }, "dest": "./Images" }
        ],
        "unmatched": "skip",
        "on_conflict": "hash_dedup"
      }
    }
  }
}
"#;
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();

    let src = env.root.join("src");
    fs::create_dir_all(src.join("Images")).unwrap();
    fs::write(src.join("a.jpg"), "same").unwrap();
    fs::write(src.join("Images").join("a.jpg"), "same").unwrap();

    let out = run_ok(
        env.cmd()
            .args(["redirect", src.to_str().unwrap(), "--format", "json"]),
    );
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let arr = v.as_array().unwrap();
    assert!(
        arr.iter()
            .any(|x| x["action"] == "dedup" && x["reason"] == "hash_dedup_same_deleted_src"),
        "expected dedup action in json output"
    );

    assert!(
        !src.join("a.jpg").exists(),
        "dedup should delete source file"
    );
    assert!(
        src.join("Images").join("a.jpg").exists(),
        "dest should remain"
    );
    let content = fs::read_to_string(src.join("Images").join("a.jpg")).unwrap_or_default();
    assert_eq!(content, "same");
}

#[test]
fn redirect_hash_dedup_copy_skips_when_dest_same_content() {
    let env = TestEnv::new();
    let cfg = r#"
{
  "redirect": {
    "profiles": {
      "default": {
        "rules": [
          { "name": "Images", "match": { "ext": ["jpg"] }, "dest": "./Images" }
        ],
        "unmatched": "skip",
        "on_conflict": "hash_dedup"
      }
    }
  }
}
"#;
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();

    let src = env.root.join("src");
    fs::create_dir_all(src.join("Images")).unwrap();
    fs::write(src.join("a.jpg"), "same").unwrap();
    fs::write(src.join("Images").join("a.jpg"), "same").unwrap();

    let out = run_ok(env.cmd().args([
        "redirect",
        src.to_str().unwrap(),
        "--copy",
        "--format",
        "json",
    ]));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let arr = v.as_array().unwrap();
    assert!(
        arr.iter()
            .any(|x| x["action"] == "skip" && x["reason"] == "hash_dedup_same"),
        "expected skip on hash_dedup when --copy"
    );

    assert!(src.join("a.jpg").exists(), "copy should keep source file");
    assert!(
        src.join("Images").join("a.jpg").exists(),
        "dest should remain"
    );
}

#[test]
fn redirect_hash_dedup_different_content_falls_back_to_rename_new() {
    let env = TestEnv::new();
    let cfg = r#"
{
  "redirect": {
    "profiles": {
      "default": {
        "rules": [
          { "name": "Images", "match": { "ext": ["jpg"] }, "dest": "./Images" }
        ],
        "unmatched": "skip",
        "on_conflict": "hash_dedup"
      }
    }
  }
}
"#;
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();

    let src = env.root.join("src");
    fs::create_dir_all(src.join("Images")).unwrap();
    fs::write(src.join("a.jpg"), "new").unwrap();
    fs::write(src.join("Images").join("a.jpg"), "existing").unwrap();

    run_ok(
        env.cmd()
            .args(["redirect", src.to_str().unwrap(), "--format", "tsv"]),
    );

    assert!(
        src.join("Images").join("a.jpg").exists(),
        "existing dest should remain"
    );
    let existing = fs::read_to_string(src.join("Images").join("a.jpg")).unwrap_or_default();
    assert_eq!(existing, "existing");

    let renamed = src.join("Images").join("a (1).jpg");
    assert!(
        renamed.exists(),
        "expected renamed file to be created when content differs"
    );
    let moved = fs::read_to_string(renamed).unwrap_or_default();
    assert_eq!(moved, "new");
    assert!(!src.join("a.jpg").exists(), "source should be moved out");
}

#[test]
fn redirect_hash_dedup_dry_run_does_not_delete_source() {
    let env = TestEnv::new();
    let cfg = r#"
{
  "redirect": {
    "profiles": {
      "default": {
        "rules": [
          { "name": "Images", "match": { "ext": ["jpg"] }, "dest": "./Images" }
        ],
        "unmatched": "skip",
        "on_conflict": "hash_dedup"
      }
    }
  }
}
"#;
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();

    let src = env.root.join("src");
    fs::create_dir_all(src.join("Images")).unwrap();
    fs::write(src.join("a.jpg"), "same").unwrap();
    fs::write(src.join("Images").join("a.jpg"), "same").unwrap();

    let out = run_ok(env.cmd().args([
        "redirect",
        src.to_str().unwrap(),
        "--dry-run",
        "--format",
        "json",
    ]));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let arr = v.as_array().unwrap();
    assert!(
        arr.iter()
            .any(|x| x["action"] == "dedup" && x["result"] == "dry_run"),
        "expected dedup dry_run output"
    );

    assert!(
        src.join("a.jpg").exists(),
        "dry run should not delete source file"
    );
    assert!(
        src.join("Images").join("a.jpg").exists(),
        "dry run should not touch destination file"
    );
}

#[test]
fn redirect_rename_existing_conflict_renames_old_file_and_moves_new_in_place() {
    let env = TestEnv::new();
    let cfg = r#"
{
  "redirect": {
    "profiles": {
      "default": {
        "rules": [
          { "name": "Images", "match": { "ext": ["jpg"] }, "dest": "./Images" }
        ],
        "unmatched": "skip",
        "on_conflict": "rename_existing"
      }
    }
  }
}
"#;
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();

    let src = env.root.join("src");
    fs::create_dir_all(src.join("Images")).unwrap();
    fs::write(src.join("a.jpg"), "new").unwrap();
    fs::write(src.join("Images").join("a.jpg"), "old").unwrap();

    run_ok(
        env.cmd()
            .args(["redirect", src.to_str().unwrap(), "--format", "tsv"]),
    );

    assert!(!src.join("a.jpg").exists(), "source should be moved out");
    let content = fs::read_to_string(src.join("Images").join("a.jpg")).unwrap_or_default();
    assert_eq!(content, "new", "expected new file to land at a.jpg");

    let bak = src.join("Images").join("a (1).jpg");
    assert!(
        bak.exists(),
        "expected old file to be renamed out of the way"
    );
    let old = fs::read_to_string(bak).unwrap_or_default();
    assert_eq!(old, "old");
}

#[test]
fn redirect_rename_existing_conflict_works_in_copy_mode() {
    let env = TestEnv::new();
    let cfg = r#"
{
  "redirect": {
    "profiles": {
      "default": {
        "rules": [
          { "name": "Images", "match": { "ext": ["jpg"] }, "dest": "./Images" }
        ],
        "unmatched": "skip",
        "on_conflict": "rename_existing"
      }
    }
  }
}
"#;
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();

    let src = env.root.join("src");
    fs::create_dir_all(src.join("Images")).unwrap();
    fs::write(src.join("a.jpg"), "new").unwrap();
    fs::write(src.join("Images").join("a.jpg"), "old").unwrap();

    run_ok(env.cmd().args([
        "redirect",
        src.to_str().unwrap(),
        "--copy",
        "--format",
        "tsv",
    ]));

    assert!(src.join("a.jpg").exists(), "copy should keep source file");
    let content = fs::read_to_string(src.join("Images").join("a.jpg")).unwrap_or_default();
    assert_eq!(content, "new", "expected new copy at a.jpg");

    let bak = src.join("Images").join("a (1).jpg");
    assert!(bak.exists(), "expected old file renamed");
    let old = fs::read_to_string(bak).unwrap_or_default();
    assert_eq!(old, "old");
}

#[test]
fn redirect_rename_existing_dry_run_does_not_rename_destination() {
    let env = TestEnv::new();
    let cfg = r#"
{
  "redirect": {
    "profiles": {
      "default": {
        "rules": [
          { "name": "Images", "match": { "ext": ["jpg"] }, "dest": "./Images" }
        ],
        "unmatched": "skip",
        "on_conflict": "rename_existing"
      }
    }
  }
}
"#;
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();

    let src = env.root.join("src");
    fs::create_dir_all(src.join("Images")).unwrap();
    fs::write(src.join("a.jpg"), "new").unwrap();
    fs::write(src.join("Images").join("a.jpg"), "old").unwrap();

    run_ok(env.cmd().args([
        "redirect",
        src.to_str().unwrap(),
        "--dry-run",
        "--format",
        "tsv",
    ]));

    assert!(src.join("a.jpg").exists(), "dry run should not move source");
    let content = fs::read_to_string(src.join("Images").join("a.jpg")).unwrap_or_default();
    assert_eq!(
        content, "old",
        "dry run should not rename/overwrite destination"
    );
    assert!(
        !src.join("Images").join("a (1).jpg").exists(),
        "dry run should not create rename_existing backup"
    );
}
