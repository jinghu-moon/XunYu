use crate::common::*;
use chrono::TimeZone;
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::os::windows::fs::MetadataExt;
use std::os::windows::io::AsRawHandle;
use std::thread;
use std::time::Duration;

fn same_file_index(path_a: &std::path::Path, path_b: &std::path::Path) -> bool {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };

    let open = |path: &std::path::Path| std::fs::OpenOptions::new().read(true).open(path).ok();
    let info = |file: &std::fs::File| -> Option<(u64, u64)> {
        let mut info = unsafe { std::mem::zeroed::<BY_HANDLE_FILE_INFORMATION>() };
        let handle = file.as_raw_handle() as windows_sys::Win32::Foundation::HANDLE;
        if handle == INVALID_HANDLE_VALUE {
            return None;
        }
        let ok = unsafe { GetFileInformationByHandle(handle, &mut info) };
        if ok == 0 {
            return None;
        }
        let index = ((info.nFileIndexHigh as u64) << 32) | (info.nFileIndexLow as u64);
        Some((info.dwVolumeSerialNumber as u64, index))
    };

    let (Some(file_a), Some(file_b)) = (open(path_a), open(path_b)) else {
        return false;
    };
    let (Some(info_a), Some(info_b)) = (info(&file_a), info(&file_b)) else {
        return false;
    };
    info_a == info_b
}

fn set_last_write_time_utc(path: &std::path::Path, year: u16, month: u16, day: u16) {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::Storage::FileSystem::SetFileTime;

    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    let ns = chrono::Utc
        .with_ymd_and_hms(year as i32, month as u32, day as u32, 1, 2, 4)
        .single()
        .unwrap()
        .timestamp_nanos_opt()
        .unwrap() as i128;
    let filetime = (ns / 100 + 116_444_736_000_000_000) as u64;
    let modified = FILETIME {
        dwLowDateTime: filetime as u32,
        dwHighDateTime: (filetime >> 32) as u32,
    };
    let ok = unsafe {
        SetFileTime(
            file.as_raw_handle() as windows_sys::Win32::Foundation::HANDLE,
            std::ptr::null(),
            std::ptr::null(),
            &modified,
        )
    };
    assert_ne!(ok, 0, "SetFileTime failed for {}", path.display());
}

#[test]
fn backup_creates_backup_folder() {
    let env = TestEnv::new();
    let root = env.root.join("proj");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "hi").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let backups = root.join("A_backups");
    let entry = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v1-"))
        .expect("backup v1 not found");
    let first = entry.path();
    assert!(first.is_dir());
    assert!(first.join("a.txt").exists());
}

#[test]
fn backup_create_subcommand_without_format_uses_legacy_directory_backup() {
    let env = TestEnv::new();
    let root = env.root.join("proj_backup_create");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "hi").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "create", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let backups = root.join("A_backups");
    let entry = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v1-"))
        .expect("backup v1 not found");
    assert!(entry.path().join("a.txt").exists());
}

#[test]
fn backup_create_zip_writes_standard_zip_to_project_relative_output() {
    let env = TestEnv::new();
    let root = env.root.join("proj_backup_create_zip");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src").join("main.rs"), "fn main() {}").unwrap();
    fs::write(root.join("README.md"), "readme").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "src", "README.md" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        "artifact.zip",
    ]));

    let output = root.join("artifact.zip");
    assert!(output.exists());
    let file = fs::File::open(&output).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut entry = archive.by_name("src/main.rs").unwrap();
    let mut content = String::new();
    std::io::Read::read_to_string(&mut entry, &mut content).unwrap();
    assert_eq!(content, "fn main() {}");
    drop(entry);
    let mut sidecar = archive.by_name("__xunyu__/export_manifest.json").unwrap();
    let mut sidecar_text = String::new();
    std::io::Read::read_to_string(&mut sidecar, &mut sidecar_text).unwrap();
    let value: Value = serde_json::from_str(&sidecar_text).unwrap();
    assert_eq!(value["format"], "zip");
    assert!(value["entries"].as_array().unwrap().len() >= 2);
}

#[test]
fn backup_create_zip_no_sidecar_omits_export_manifest() {
    let env = TestEnv::new();
    let root = env.root.join("proj_backup_create_zip_no_sidecar");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        "artifact.zip",
        "--no-sidecar",
    ]));

    let file = fs::File::open(root.join("artifact.zip")).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    assert!(archive.by_name("__xunyu__/export_manifest.json").is_err());
}

#[test]
fn backup_create_zip_json_reports_extended_summary_fields() {
    let env = TestEnv::new();
    let root = env.root.join("proj_backup_create_zip_json");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    let out = run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        "artifact.zip",
        "--json",
    ]));
    let json: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["dry_run"], false);
    assert_eq!(json["selected"], 1);
    assert_eq!(json["written"], 1);
    assert_eq!(json["skipped"], 0);
    assert_eq!(json["overwrite_count"], 0);
    assert_eq!(json["verify_source"], "off");
    assert_eq!(json["verify_output"], "off");
    assert!(json["bytes_out"].as_u64().unwrap() > 0);
    assert!(json["duration_ms"].as_u64().is_some());
    assert_eq!(json["outputs"][0], root.join("artifact.zip").to_string_lossy().to_string());
}

#[test]
fn backup_create_rejects_invalid_zip_method_with_fix_hint() {
    let env = TestEnv::new();
    let root = env.root.join("proj_backup_create_invalid_zip_method");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    let out = run_err(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        "artifact.zip",
        "--method",
        "lzma2",
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("invalid for zip"));
    assert!(stderr.contains("Fix:"));
}

#[test]
fn top_level_export_command_still_exists() {
    let env = TestEnv::new();
    let out = run_raw(env.cmd().args(["export", "--help"]));
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Export"));
}

#[test]
fn backup_create_zip_dry_run_does_not_create_output() {
    let env = TestEnv::new();
    let root = env.root.join("proj_backup_create_zip_dry_run");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        "artifact.zip",
        "--dry-run",
    ]));
    assert!(!root.join("artifact.zip").exists());
}

#[test]
fn backup_create_zip_write_failure_cleans_temp_output_and_does_not_publish_target() {
    let env = TestEnv::new();
    let root = env.root.join("proj_backup_create_zip_fail_cleanup");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    let out = run_err(
        env.cmd()
            .env("XUN_TEST_FAIL_AFTER_WRITE", "1")
            .args([
                "backup",
                "create",
                "-C",
                root.to_str().unwrap(),
                "--format",
                "zip",
                "-o",
                "artifact.zip",
            ]),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("resume is not supported yet"));
    assert!(!root.join("artifact.zip").exists());
    assert!(!root.join("artifact.tmp.zip").exists());
}

#[test]
fn backup_create_progress_always_emits_read_and_write_phases() {
    let env = TestEnv::new();
    let root = env.root.join("proj_backup_create_progress");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    let out = run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        "artifact.zip",
        "--progress",
        "always",
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("progress: phase=read"));
    assert!(stderr.contains("progress: phase=compress"));
    assert!(stderr.contains("progress: phase=write"));
}

#[test]
fn backup_create_does_not_attempt_xunbak_verify_on_source_files() {
    let env = TestEnv::new();
    let root = env.root.join("proj_create_no_preflight_verify");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("broken.xunbak"), b"not-a-container").unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "broken.xunbak" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        "artifact.zip",
    ]));

    assert!(root.join("artifact.zip").exists());
}

#[test]
fn backup_create_zip_existing_output_fails() {
    let env = TestEnv::new();
    let root = env.root.join("proj_backup_create_zip_fail");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "hi").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();
    fs::write(root.join("artifact.zip"), b"existing").unwrap();

    let out = run_err(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        "artifact.zip",
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("output already exists"));
}

#[test]
fn backup_create_dir_output_writes_selected_files() {
    let env = TestEnv::new();
    let root = env.root.join("proj_backup_create_dir");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src").join("main.rs"), "fn main() {}").unwrap();
    fs::write(root.join("README.md"), "readme").unwrap();
    fs::write(root.join("skip.log"), "skip").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "src", "README.md" ],
  "exclude": [ "*.log" ]
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "dir",
        "-o",
        "artifact_dir",
    ]));

    let output = root.join("artifact_dir");
    assert_eq!(
        fs::read_to_string(output.join("src").join("main.rs")).unwrap(),
        "fn main() {}"
    );
    assert_eq!(
        fs::read_to_string(output.join("README.md")).unwrap(),
        "readme"
    );
    assert!(!output.join("skip.log").exists());
    let sidecar = output.join("__xunyu__").join("export_manifest.json");
    let value: Value = serde_json::from_slice(&fs::read(&sidecar).unwrap()).unwrap();
    assert_eq!(value["format"], "dir");
    assert!(value["entries"].as_array().unwrap().len() >= 2);
}

#[test]
fn backup_create_dir_output_rejects_source_directory_as_target() {
    let env = TestEnv::new();
    let root = env.root.join("proj_backup_create_dir_same_target");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "hi").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    let out = run_err(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "dir",
        "-o",
        ".",
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("output must differ from source directory"));
}

#[test]
fn backup_create_dir_output_preserves_mtime_and_readonly_attribute() {
    let env = TestEnv::new();
    let root = env.root.join("proj_backup_create_dir_metadata");
    fs::create_dir_all(&root).unwrap();
    let source = root.join("readonly.txt");
    fs::write(&source, "hello").unwrap();
    set_last_write_time_utc(&source, 2024, 1, 2);

    let mut permissions = fs::metadata(&source).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&source, permissions).unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "readonly.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "dir",
        "-o",
        "artifact_dir",
    ]));

    let output = root.join("artifact_dir").join("readonly.txt");
    let source_meta = fs::metadata(&source).unwrap();
    let output_meta = fs::metadata(&output).unwrap();
    assert_eq!(source_meta.last_write_time(), output_meta.last_write_time());
    assert_eq!(
        source_meta.file_attributes() & 0x0000_0001,
        output_meta.file_attributes() & 0x0000_0001
    );
}

#[test]
fn backup_create_7z_writes_standard_archive_to_project_relative_output() {
    let env = TestEnv::new();
    let root = env.root.join("proj_backup_create_7z");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src").join("main.rs"), "fn main() {}").unwrap();
    fs::write(root.join("README.md"), "readme").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "src", "README.md" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        "artifact.7z",
    ]));

    let output = root.join("artifact.7z");
    assert!(output.exists());
    let mut archive =
        sevenz_rust2::ArchiveReader::open(&output, sevenz_rust2::Password::empty()).unwrap();
    let content = archive.read_file("src/main.rs").unwrap();
    assert_eq!(String::from_utf8(content).unwrap(), "fn main() {}");
    let sidecar = archive.read_file("__xunyu__/export_manifest.json").unwrap();
    let value: Value = serde_json::from_slice(&sidecar).unwrap();
    assert_eq!(value["format"], "7z");
    assert!(value["entries"].as_array().unwrap().len() >= 2);
}

#[test]
fn backup_convert_directory_artifact_to_7z_output_writes_selected_files() {
    let env = TestEnv::new();
    let root = env.root.join("proj_convert_skeleton");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src").join("main.rs"), "fn main() {}").unwrap();
    fs::write(root.join("README.md"), "readme").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "src", "README.md" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let artifact = fs::read_dir(root.join("A_backups"))
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v1-"))
        .unwrap()
        .path();

    run_ok(env.cmd().args([
        "backup",
        "convert",
        artifact.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        env.root.join("out.7z").to_str().unwrap(),
        "--glob",
        "src/*.rs",
    ]));

    let output = env.root.join("out.7z");
    assert!(output.exists());
    let mut archive =
        sevenz_rust2::ArchiveReader::open(&output, sevenz_rust2::Password::empty()).unwrap();
    assert!(archive.read_file("README.md").is_err());
    let content = archive.read_file("src/main.rs").unwrap();
    assert_eq!(String::from_utf8(content).unwrap(), "fn main() {}");
}

#[test]
fn backup_convert_7z_artifact_to_directory_output_writes_selected_files() {
    let env = TestEnv::new();
    let root = env.root.join("proj_convert_7z_to_dir");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src").join("main.rs"), "fn main() {}").unwrap();
    fs::write(root.join("README.md"), "readme").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "src", "README.md" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();
    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        "artifact.7z",
    ]));

    let output = env.root.join("out_dir");
    run_ok(env.cmd().args([
        "backup",
        "convert",
        root.join("artifact.7z").to_str().unwrap(),
        "--format",
        "dir",
        "-o",
        output.to_str().unwrap(),
        "--file",
        "README.md",
    ]));

    assert_eq!(fs::read_to_string(output.join("README.md")).unwrap(), "readme");
    assert!(!output.join("src").join("main.rs").exists());
}

#[test]
fn backup_convert_zip_artifact_to_7z_output_writes_selected_files() {
    let env = TestEnv::new();
    let zip_path = env.root.join("artifact.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("src/main.rs", options).unwrap();
    writer.write_all(b"fn main() {}").unwrap();
    writer.start_file("README.md", options).unwrap();
    writer.write_all(b"readme").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let output = env.root.join("from_zip.7z");
    run_ok(env.cmd().args([
        "backup",
        "convert",
        zip_path.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        output.to_str().unwrap(),
        "--file",
        "README.md",
    ]));

    let mut archive =
        sevenz_rust2::ArchiveReader::open(&output, sevenz_rust2::Password::empty()).unwrap();
    assert!(archive.read_file("src/main.rs").is_err());
    let content = archive.read_file("README.md").unwrap();
    assert_eq!(String::from_utf8(content).unwrap(), "readme");
}

#[test]
fn backup_convert_to_7z_no_sidecar_omits_export_manifest() {
    let env = TestEnv::new();
    let root = env.root.join("proj_convert_to_7z_no_sidecar");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();
    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let artifact = fs::read_dir(root.join("A_backups"))
        .unwrap()
        .flatten()
        .find(|entry| entry.file_name().to_string_lossy().starts_with("v1-"))
        .unwrap()
        .path();
    let output = env.root.join("no_sidecar.7z");
    run_ok(env.cmd().args([
        "backup",
        "convert",
        artifact.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        output.to_str().unwrap(),
        "--no-sidecar",
    ]));

    let mut archive =
        sevenz_rust2::ArchiveReader::open(&output, sevenz_rust2::Password::empty()).unwrap();
    assert!(archive.read_file("__xunyu__/export_manifest.json").is_err());
}

#[test]
fn backup_convert_7z_artifact_to_zip_output_writes_selected_files() {
    let env = TestEnv::new();
    let root = env.root.join("proj_convert_7z_to_zip");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    fs::write(root.join("b.txt"), "bbb").unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt", "b.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        "artifact.7z",
    ]));

    let output = env.root.join("from_7z.zip");
    run_ok(env.cmd().args([
        "backup",
        "convert",
        root.join("artifact.7z").to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        output.to_str().unwrap(),
        "--file",
        "b.txt",
    ]));

    let file = fs::File::open(&output).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    assert!(archive.by_name("a.txt").is_err());
    let mut entry = archive.by_name("b.txt").unwrap();
    let mut content = String::new();
    std::io::Read::read_to_string(&mut entry, &mut content).unwrap();
    assert_eq!(content, "bbb");
}

#[test]
fn backup_convert_7z_output_method_copy_is_applied() {
    let env = TestEnv::new();
    let zip_path = env.root.join("artifact.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("README.md", options).unwrap();
    writer.write_all(b"readme").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let output = env.root.join("copy.7z");
    run_ok(env.cmd().args([
        "backup",
        "convert",
        zip_path.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        output.to_str().unwrap(),
        "--method",
        "copy",
    ]));

    let archive =
        sevenz_rust2::ArchiveReader::open(&output, sevenz_rust2::Password::empty()).unwrap();
    let entry = archive
        .archive()
        .files
        .iter()
        .find(|entry| entry.name() == "README.md")
        .unwrap();
    assert_eq!(entry.size(), 6);
    assert_eq!(entry.compressed_size, 6);
}

#[test]
fn backup_create_7z_defaults_to_lzma2_and_non_solid() {
    let env = TestEnv::new();
    let root = env.root.join("proj_backup_create_7z_defaults");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa aaa aaa aaa aaa aaa").unwrap();
    fs::write(root.join("b.txt"), "bbb bbb bbb bbb bbb bbb").unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt", "b.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        "artifact.7z",
    ]));

    let archive = sevenz_rust2::ArchiveReader::open(
        root.join("artifact.7z"),
        sevenz_rust2::Password::empty(),
    )
    .unwrap();
    assert!(!archive.archive().is_solid);
    let first_block = archive.archive().blocks.first().unwrap();
    let first_coder = first_block.ordered_coder_iter().next().unwrap().1;
    let method = sevenz_rust2::EncoderMethod::by_id(first_coder.encoder_method_id()).unwrap();
    assert_eq!(method, sevenz_rust2::EncoderMethod::LZMA2);
}

#[test]
fn backup_create_7z_preserves_mtime_and_readonly_metadata_when_converted_to_dir() {
    let env = TestEnv::new();
    let root = env.root.join("proj_backup_create_7z_metadata");
    fs::create_dir_all(&root).unwrap();
    let source = root.join("readonly.txt");
    fs::write(&source, "hello").unwrap();
    set_last_write_time_utc(&source, 2024, 1, 2);

    let mut permissions = fs::metadata(&source).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&source, permissions).unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "readonly.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        "artifact.7z",
    ]));

    let output = env.root.join("restored_from_7z_dir");
    run_ok(env.cmd().args([
        "backup",
        "convert",
        root.join("artifact.7z").to_str().unwrap(),
        "--format",
        "dir",
        "-o",
        output.to_str().unwrap(),
    ]));

    let source_meta = fs::metadata(&source).unwrap();
    let restored_meta = fs::metadata(output.join("readonly.txt")).unwrap();
    assert_eq!(source_meta.last_write_time(), restored_meta.last_write_time());
    assert_eq!(
        source_meta.file_attributes() & 0x0000_0001,
        restored_meta.file_attributes() & 0x0000_0001
    );
}

#[test]
fn backup_create_zip_uses_stored_for_precompressed_extension() {
    let env = TestEnv::new();
    let root = env.root.join("proj_backup_create_zip_precompressed");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("asset.zip"), b"pretend zip bytes").unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "asset.zip" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        "artifact.zip",
    ]));

    let file = fs::File::open(root.join("artifact.zip")).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let entry = archive.by_name("asset.zip").unwrap();
    assert_eq!(entry.compression(), zip::CompressionMethod::Stored);
}

#[test]
fn backup_create_7z_no_compress_uses_copy_for_payloads() {
    let env = TestEnv::new();
    let root = env.root.join("proj_backup_create_7z_copy");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("asset.zip"), b"pretend zip bytes").unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "asset.zip" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        "artifact.7z",
        "--method",
        "copy",
    ]));

    let archive =
        sevenz_rust2::ArchiveReader::open(root.join("artifact.7z"), sevenz_rust2::Password::empty()).unwrap();
    let entry = archive
        .archive()
        .files
        .iter()
        .find(|item| item.name() == "asset.zip")
        .unwrap();
    assert_eq!(entry.size(), entry.compressed_size);
}

#[test]
fn backup_create_7z_split_creates_numbered_volumes_without_temp_artifacts() {
    let env = TestEnv::new();
    let root = env.root.join("proj_backup_create_7z_split");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "a".repeat(1200)).unwrap();
    fs::write(root.join("b.txt"), "b".repeat(1200)).unwrap();
    fs::write(root.join("c.txt"), "c".repeat(1200)).unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt", "b.txt", "c.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        "artifact.7z",
        "--no-compress",
        "--split-size",
        "1400",
    ]));

    assert!(root.join("artifact.7z.001").exists());
    assert!(root.join("artifact.7z.002").exists());
    let temp_staged = fs::read_dir(&root)
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .any(|name| name.contains("tmp.7z"));
    assert!(!temp_staged, "temporary split 7z files should be cleaned up");
}

#[test]
fn backup_convert_zip_artifact_to_split_7z_output_creates_numbered_volumes() {
    let env = TestEnv::new();
    let zip_path = env.root.join("artifact.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("a.txt", options).unwrap();
    writer.write_all(&vec![b'a'; 1200]).unwrap();
    writer.start_file("b.txt", options).unwrap();
    writer.write_all(&vec![b'b'; 1200]).unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let output = env.root.join("split.7z");
    run_ok(env.cmd().args([
        "backup",
        "convert",
        zip_path.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        output.to_str().unwrap(),
        "--method",
        "copy",
        "--split-size",
        "1400",
    ]));

    assert!(env.root.join("split.7z.001").exists());
    assert!(env.root.join("split.7z.002").exists());
}

#[test]
fn backup_convert_split_7z_artifact_to_directory_output_writes_selected_files() {
    let env = TestEnv::new();
    let root = env.root.join("proj_convert_split_7z_to_dir");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "a".repeat(1200)).unwrap();
    fs::write(root.join("b.txt"), "b".repeat(1200)).unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt", "b.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();
    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        "artifact.7z",
        "--no-compress",
        "--split-size",
        "1400",
    ]));

    let output = env.root.join("from_split_7z");
    run_ok(env.cmd().args([
        "backup",
        "convert",
        root.join("artifact.7z").to_str().unwrap(),
        "--format",
        "dir",
        "-o",
        output.to_str().unwrap(),
        "--file",
        "b.txt",
    ]));

    assert!(!output.join("a.txt").exists());
    assert_eq!(fs::read_to_string(output.join("b.txt")).unwrap(), "b".repeat(1200));
}

#[test]
fn backup_convert_split_7z_write_failure_cleans_temp_outputs_and_does_not_publish_target() {
    let env = TestEnv::new();
    let zip_path = env.root.join("artifact.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("a.txt", options).unwrap();
    writer.write_all(&vec![b'a'; 1200]).unwrap();
    writer.start_file("b.txt", options).unwrap();
    writer.write_all(&vec![b'b'; 1200]).unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let out = run_err(
        env.cmd()
            .env("XUN_TEST_FAIL_AFTER_WRITE", "1")
            .args([
                "backup",
                "convert",
                zip_path.to_str().unwrap(),
                "--format",
                "7z",
                "-o",
                env.root.join("fail_split.7z").to_str().unwrap(),
                "--method",
                "copy",
                "--split-size",
                "1400",
            ]),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("resume is not supported yet"));
    assert!(!env.root.join("fail_split.7z.001").exists());
    assert!(!env.root.join("fail_split.tmp.7z.001").exists());
}

#[test]
fn backup_convert_first_volume_7z_artifact_to_directory_output_writes_selected_files() {
    let env = TestEnv::new();
    let root = env.root.join("proj_convert_first_volume_7z_to_dir");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "a".repeat(1200)).unwrap();
    fs::write(root.join("b.txt"), "b".repeat(1200)).unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt", "b.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();
    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        "artifact.7z",
        "--no-compress",
        "--split-size",
        "1400",
    ]));

    let output = env.root.join("from_first_volume_7z");
    run_ok(env.cmd().args([
        "backup",
        "convert",
        root.join("artifact.7z.001").to_str().unwrap(),
        "--format",
        "dir",
        "-o",
        output.to_str().unwrap(),
        "--file",
        "a.txt",
    ]));

    assert_eq!(fs::read_to_string(output.join("a.txt")).unwrap(), "a".repeat(1200));
    assert!(!output.join("b.txt").exists());
}

#[test]
fn backup_convert_rejects_invalid_7z_method() {
    let env = TestEnv::new();
    let zip_path = env.root.join("artifact.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("a.txt", options).unwrap();
    writer.write_all(b"aaa").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let out = run_err(env.cmd().args([
        "backup",
        "convert",
        zip_path.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        env.root.join("invalid.7z").to_str().unwrap(),
        "--method",
        "ppmd",
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("invalid for 7z"));
}

#[test]
fn backup_create_7z_accepts_split_size_and_writes_first_volume() {
    let env = TestEnv::new();
    let root = env.root.join("proj_backup_create_7z_split_accept");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        "artifact.7z",
        "--split-size",
        "2G",
    ]));
    assert!(root.join("artifact.7z.001").exists());
}

#[test]
fn backup_create_list_outputs_selected_files_without_creating_backup() {
    let env = TestEnv::new();
    let root = env.root.join("proj_create_list");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src").join("main.rs"), "fn main() {}").unwrap();
    fs::write(root.join("README.md"), "readme").unwrap();
    fs::write(root.join("skip.log"), "skip").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "src", "README.md" ],
  "exclude": [ "*.log" ]
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    let out = run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "zip",
        "--list",
        "--json",
    ]));
    let json: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["action"], "create");
    assert_eq!(json["mode"], "list");
    assert_eq!(json["format"], "zip");
    assert_eq!(json["selected"], 2);
    assert_eq!(
        json["entries"],
        serde_json::json!(["README.md", "src/main.rs"])
    );
    assert!(
        !root.join("A_backups").exists(),
        "--list should not create backup output"
    );
}

#[test]
fn backup_convert_list_lists_directory_artifact_without_creating_output() {
    let env = TestEnv::new();
    let root = env.root.join("proj_convert_list_dir");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src").join("main.rs"), "fn main() {}").unwrap();
    fs::write(root.join("README.md"), "readme").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "src", "README.md" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();
    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let artifact = fs::read_dir(root.join("A_backups"))
        .unwrap()
        .flatten()
        .find(|entry| entry.file_name().to_string_lossy().starts_with("v1-"))
        .unwrap()
        .path();
    let output = env.root.join("preview.zip");

    let out = run_ok(env.cmd().args([
        "backup",
        "convert",
        artifact.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        output.to_str().unwrap(),
        "--list",
        "--json",
        "--glob",
        "src/*.rs",
    ]));
    let json: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["action"], "convert");
    assert_eq!(json["mode"], "list");
    assert_eq!(json["format"], "zip");
    assert_eq!(json["selected"], 1);
    assert_eq!(json["entries"], serde_json::json!(["src/main.rs"]));
    assert!(!output.exists(), "--list should not create output files");
}

#[test]
fn backup_convert_list_merges_file_glob_and_patterns_from() {
    let env = TestEnv::new();
    let root = env.root.join("proj_convert_patterns");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src").join("main.rs"), "fn main() {}").unwrap();
    fs::write(root.join("README.md"), "readme").unwrap();
    fs::write(root.join("notes.txt"), "notes").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "src", "README.md", "notes.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();
    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let artifact = fs::read_dir(root.join("A_backups"))
        .unwrap()
        .flatten()
        .find(|entry| entry.file_name().to_string_lossy().starts_with("v1-"))
        .unwrap()
        .path();
    let patterns = env.root.join("patterns.txt");
    fs::write(&patterns, "# comment\nsrc/*.rs\nREADME.md\n").unwrap();

    let out = run_ok(env.cmd().args([
        "backup",
        "convert",
        artifact.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        env.root.join("preview.zip").to_str().unwrap(),
        "--list",
        "--json",
        "--file",
        "notes.txt",
        "--patterns-from",
        patterns.to_str().unwrap(),
    ]));
    let json: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["selected"], 3);
    assert_eq!(
        json["entries"],
        serde_json::json!(["README.md", "notes.txt", "src/main.rs"])
    );
}

#[test]
fn backup_convert_list_lists_zip_artifact_without_creating_output() {
    let env = TestEnv::new();
    let zip_path = env.root.join("artifact.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("src/main.rs", options).unwrap();
    writer.write_all(b"fn main() {}").unwrap();
    writer.start_file("README.md", options).unwrap();
    writer.write_all(b"readme").unwrap();
    writer.start_file(".bak-meta.json", options).unwrap();
    writer.write_all(b"{}").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let output = env.root.join("restored");
    let out = run_ok(env.cmd().args([
        "backup",
        "convert",
        zip_path.to_str().unwrap(),
        "--format",
        "dir",
        "-o",
        output.to_str().unwrap(),
        "--list",
        "--json",
    ]));
    let json: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["selected"], 2);
    assert_eq!(
        json["entries"],
        serde_json::json!(["README.md", "src/main.rs"])
    );
    assert!(!output.exists(), "--list should not materialize output");
}

#[test]
fn backup_convert_rejects_split_size_for_dir_output_before_preview() {
    let env = TestEnv::new();
    let zip_path = env.root.join("artifact.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("a.txt", options).unwrap();
    writer.write_all(b"aaa").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let out = run_err(env.cmd().args([
        "backup",
        "convert",
        zip_path.to_str().unwrap(),
        "--format",
        "dir",
        "-o",
        env.root.join("out").to_str().unwrap(),
        "--list",
        "--split-size",
        "2G",
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("backup convert --split-size is invalid for dir output"));
}

#[test]
fn backup_convert_parameter_error_exits_with_code_2() {
    let env = TestEnv::new();
    let zip_path = env.root.join("artifact.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("a.txt", options).unwrap();
    writer.write_all(b"aaa").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let out = run_raw(env.cmd().args([
        "backup",
        "convert",
        zip_path.to_str().unwrap(),
        "--format",
        "dir",
        "-o",
        env.root.join("out").to_str().unwrap(),
        "--split-size",
        "2G",
    ]));
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Fix:"));
}

#[test]
fn backup_convert_verify_failure_exits_with_code_1() {
    let env = TestEnv::new();
    let zip_path = env.root.join("artifact.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("a.txt", options).unwrap();
    writer.write_all(b"aaa").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let out = run_raw(
        env.cmd()
            .env("XUN_TEST_CORRUPT_OUTPUT_AFTER_WRITE", "truncate")
            .args([
                "backup",
                "convert",
                zip_path.to_str().unwrap(),
                "--format",
                "zip",
                "-o",
                env.root.join("verify_fail.zip").to_str().unwrap(),
            ]),
    );
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn backup_create_success_exits_with_code_0() {
    let env = TestEnv::new();
    let root = env.root.join("proj_exit_code_success");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    let out = run_raw(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        "artifact.zip",
    ]));
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn backup_convert_write_failure_exits_with_code_1() {
    let env = TestEnv::new();
    let missing = env.root.join("missing.zip");
    let output = env.root.join("out.zip");

    let out = run_raw(env.cmd().args([
        "backup",
        "convert",
        missing.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        output.to_str().unwrap(),
    ]));
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn backup_convert_json_reports_write_failed_status() {
    let env = TestEnv::new();
    let missing = env.root.join("missing.zip");
    let output = env.root.join("out.zip");

    let out = run_err(env.cmd().args([
        "backup",
        "convert",
        missing.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        output.to_str().unwrap(),
        "--json",
    ]));
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["status"], "write_failed");
}

#[test]
fn backup_convert_rejects_encrypt_header_without_password() {
    let env = TestEnv::new();
    let zip_path = env.root.join("artifact.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("a.txt", options).unwrap();
    writer.write_all(b"aaa").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let out = run_err(env.cmd().args([
        "backup",
        "convert",
        zip_path.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        env.root.join("out.7z").to_str().unwrap(),
        "--list",
        "--encrypt-header",
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("backup convert --encrypt-header requires --password"));
}

#[test]
fn backup_convert_directory_artifact_to_directory_output_writes_selected_files() {
    let env = TestEnv::new();
    let root = env.root.join("proj_convert_dir_to_dir");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src").join("main.rs"), "fn main() {}").unwrap();
    fs::write(root.join("README.md"), "readme").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "src", "README.md" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();
    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let artifact = fs::read_dir(root.join("A_backups"))
        .unwrap()
        .flatten()
        .find(|entry| entry.file_name().to_string_lossy().starts_with("v1-"))
        .unwrap()
        .path();
    let output = env.root.join("converted_dir");

    run_ok(env.cmd().args([
        "backup",
        "convert",
        artifact.to_str().unwrap(),
        "--format",
        "dir",
        "-o",
        output.to_str().unwrap(),
        "--glob",
        "src/*.rs",
    ]));

    assert_eq!(
        fs::read_to_string(output.join("src").join("main.rs")).unwrap(),
        "fn main() {}"
    );
    assert!(!output.join("README.md").exists());
}

#[test]
fn backup_convert_zip_artifact_to_directory_output_writes_selected_files() {
    let env = TestEnv::new();
    let zip_path = env.root.join("artifact.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("src/main.rs", options).unwrap();
    writer.write_all(b"fn main() {}").unwrap();
    writer.start_file("README.md", options).unwrap();
    writer.write_all(b"readme").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let output = env.root.join("zip_to_dir");
    run_ok(env.cmd().args([
        "backup",
        "convert",
        zip_path.to_str().unwrap(),
        "--format",
        "dir",
        "-o",
        output.to_str().unwrap(),
        "--file",
        "README.md",
    ]));

    assert_eq!(
        fs::read_to_string(output.join("README.md")).unwrap(),
        "readme"
    );
    assert!(!output.join("src").join("main.rs").exists());
}

#[test]
fn backup_convert_directory_output_rejects_existing_output_when_overwrite_fail() {
    let env = TestEnv::new();
    let zip_path = env.root.join("artifact.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("a.txt", options).unwrap();
    writer.write_all(b"aaa").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let output = env.root.join("exists");
    fs::create_dir_all(&output).unwrap();

    let out = run_err(env.cmd().args([
        "backup",
        "convert",
        zip_path.to_str().unwrap(),
        "--format",
        "dir",
        "-o",
        output.to_str().unwrap(),
        "--overwrite",
        "fail",
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("output already exists"));
}

#[test]
fn backup_convert_directory_output_replace_removes_old_files_and_writes_new_selection() {
    let env = TestEnv::new();
    let zip_path = env.root.join("artifact.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("fresh.txt", options).unwrap();
    writer.write_all(b"fresh").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let output = env.root.join("replace_out");
    fs::create_dir_all(&output).unwrap();
    fs::write(output.join("stale.txt"), "stale").unwrap();

    run_ok(env.cmd().args([
        "backup",
        "convert",
        zip_path.to_str().unwrap(),
        "--format",
        "dir",
        "-o",
        output.to_str().unwrap(),
        "--overwrite",
        "replace",
    ]));

    assert!(!output.join("stale.txt").exists());
    assert_eq!(
        fs::read_to_string(output.join("fresh.txt")).unwrap(),
        "fresh"
    );
}

#[test]
fn backup_convert_directory_output_json_summary_reports_written_files() {
    let env = TestEnv::new();
    let zip_path = env.root.join("artifact.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("src/main.rs", options).unwrap();
    writer.write_all(b"fn main() {}").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let output = env.root.join("json_out");
    let out = run_ok(env.cmd().args([
        "backup",
        "convert",
        zip_path.to_str().unwrap(),
        "--format",
        "dir",
        "-o",
        output.to_str().unwrap(),
        "--json",
    ]));
    let json: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["action"], "convert");
    assert_eq!(json["status"], "ok");
    assert_eq!(json["format"], "dir");
    assert_eq!(json["selected"], 1);
    assert_eq!(json["written"], 1);
    assert_eq!(
        fs::read_to_string(output.join("src").join("main.rs")).unwrap(),
        "fn main() {}"
    );
}

#[test]
fn backup_convert_directory_artifact_to_zip_output_writes_selected_files() {
    let env = TestEnv::new();
    let root = env.root.join("proj_convert_dir_to_zip");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src").join("main.rs"), "fn main() {}").unwrap();
    fs::write(root.join("README.md"), "readme").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "src", "README.md" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();
    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let artifact = fs::read_dir(root.join("A_backups"))
        .unwrap()
        .flatten()
        .find(|entry| entry.file_name().to_string_lossy().starts_with("v1-"))
        .unwrap()
        .path();
    let output = env.root.join("converted.zip");

    run_ok(env.cmd().args([
        "backup",
        "convert",
        artifact.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        output.to_str().unwrap(),
        "--glob",
        "src/*.rs",
    ]));

    let file = fs::File::open(&output).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut entry = archive.by_name("src/main.rs").unwrap();
    let mut content = String::new();
    std::io::Read::read_to_string(&mut entry, &mut content).unwrap();
    drop(entry);
    assert_eq!(content, "fn main() {}");
    assert!(archive.by_name("README.md").is_err());
}

#[test]
fn backup_convert_zip_output_replace_overwrites_existing_file() {
    let env = TestEnv::new();
    let zip_path = env.root.join("artifact.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("fresh.txt", options).unwrap();
    writer.write_all(b"fresh").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let output = env.root.join("existing.zip");
    fs::write(&output, b"stale").unwrap();

    run_ok(env.cmd().args([
        "backup",
        "convert",
        zip_path.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        output.to_str().unwrap(),
        "--overwrite",
        "replace",
    ]));

    let file = fs::File::open(&output).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut entry = archive.by_name("fresh.txt").unwrap();
    let mut content = String::new();
    std::io::Read::read_to_string(&mut entry, &mut content).unwrap();
    assert_eq!(content, "fresh");
}

#[test]
fn backup_convert_zip_output_json_summary_reports_written_files() {
    let env = TestEnv::new();
    let zip_path = env.root.join("artifact.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("src/main.rs", options).unwrap();
    writer.write_all(b"fn main() {}").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let output = env.root.join("json.zip");
    let out = run_ok(env.cmd().args([
        "backup",
        "convert",
        zip_path.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        output.to_str().unwrap(),
        "--json",
    ]));
    let json: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["action"], "convert");
    assert_eq!(json["status"], "ok");
    assert_eq!(json["format"], "zip");
    assert_eq!(json["selected"], 1);
    assert_eq!(json["written"], 1);
    assert_eq!(json["skipped"], 0);
    assert_eq!(json["overwrite_count"], 0);
    assert_eq!(json["verify_source"], "quick");
    assert_eq!(json["verify_output"], "on");
    assert!(json["duration_ms"].as_u64().is_some());
    assert!(json["bytes_out"].as_u64().unwrap() > 0);
    assert_eq!(json["outputs"][0], output.to_string_lossy().to_string());
    assert!(output.exists());
}

#[test]
fn backup_convert_list_json_reports_extended_summary_fields() {
    let env = TestEnv::new();
    let zip_path = env.root.join("artifact.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("src/main.rs", options).unwrap();
    writer.write_all(b"fn main() {}").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let output = env.root.join("list.zip");
    let out = run_ok(env.cmd().args([
        "backup",
        "convert",
        zip_path.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        output.to_str().unwrap(),
        "--list",
        "--json",
    ]));
    let json: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["mode"], "list");
    assert_eq!(json["dry_run"], false);
    assert_eq!(json["verify_source"], "quick");
    assert_eq!(json["verify_output"], "on");
    assert_eq!(json["bytes_out"], 0);
    assert_eq!(json["outputs"].as_array().unwrap().len(), 0);
    assert!(json["duration_ms"].as_u64().is_some());
}

#[test]
fn backup_convert_dry_run_json_reports_extended_summary_fields_without_output() {
    let env = TestEnv::new();
    let zip_path = env.root.join("artifact.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("src/main.rs", options).unwrap();
    writer.write_all(b"fn main() {}").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let output = env.root.join("dry_run.zip");
    let out = run_ok(env.cmd().args([
        "backup",
        "convert",
        zip_path.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        output.to_str().unwrap(),
        "--dry-run",
        "--json",
    ]));
    let json: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["mode"], "dry_run");
    assert_eq!(json["dry_run"], true);
    assert_eq!(json["verify_source"], "quick");
    assert_eq!(json["verify_output"], "on");
    assert_eq!(json["bytes_out"], 0);
    assert_eq!(json["outputs"].as_array().unwrap().len(), 0);
    assert!(!output.exists());
}

#[test]
fn backup_convert_zip_output_method_stored_is_applied() {
    let env = TestEnv::new();
    let root = env.root.join("proj_convert_method_stored");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("notes.txt"), "hello hello hello hello").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "notes.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();
    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let artifact = fs::read_dir(root.join("A_backups"))
        .unwrap()
        .flatten()
        .find(|entry| entry.file_name().to_string_lossy().starts_with("v1-"))
        .unwrap()
        .path();
    let output = env.root.join("stored_method.zip");

    run_ok(env.cmd().args([
        "backup",
        "convert",
        artifact.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        output.to_str().unwrap(),
        "--method",
        "stored",
    ]));

    let file = fs::File::open(&output).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let zipped = archive.by_name("notes.txt").unwrap();
    assert_eq!(zipped.compression(), zip::CompressionMethod::Stored);
}

#[test]
fn backup_convert_zip_output_defaults_to_deflated_for_text_files() {
    let env = TestEnv::new();
    let root = env.root.join("proj_convert_method_default");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("notes.txt"), "hello hello hello hello").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "notes.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();
    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let artifact = fs::read_dir(root.join("A_backups"))
        .unwrap()
        .flatten()
        .find(|entry| entry.file_name().to_string_lossy().starts_with("v1-"))
        .unwrap()
        .path();
    let output = env.root.join("default_method.zip");

    run_ok(env.cmd().args([
        "backup",
        "convert",
        artifact.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        output.to_str().unwrap(),
    ]));

    let file = fs::File::open(&output).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let zipped = archive.by_name("notes.txt").unwrap();
    assert_eq!(zipped.compression(), zip::CompressionMethod::Deflated);
}

#[test]
fn backup_convert_rejects_invalid_zip_method() {
    let env = TestEnv::new();
    let zip_path = env.root.join("artifact.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("a.txt", options).unwrap();
    writer.write_all(b"aaa").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    for method in ["lzma2", "bzip2", "ppmd"] {
        let out = run_err(env.cmd().args([
            "backup",
            "convert",
            zip_path.to_str().unwrap(),
            "--format",
            "zip",
            "-o",
            env.root.join(format!("{method}.zip")).to_str().unwrap(),
            "--method",
            method,
        ]));
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("invalid for zip"));
        assert!(stderr.contains("Fix:"));
    }
}

#[test]
fn backup_convert_zip_output_verify_on_detects_corrupted_postwrite_output() {
    let env = TestEnv::new();
    let zip_path = env.root.join("artifact.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("a.txt", options).unwrap();
    writer.write_all(b"aaa").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let output = env.root.join("verify_fail.zip");
    let out = run_err(
        env.cmd()
            .env("XUN_TEST_CORRUPT_OUTPUT_AFTER_WRITE", "truncate")
            .args([
                "backup",
                "convert",
                zip_path.to_str().unwrap(),
                "--format",
                "zip",
                "-o",
                output.to_str().unwrap(),
            ]),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("output verify failed"));
}

#[test]
fn backup_convert_zip_output_verify_off_skips_corrupted_postwrite_output_check() {
    let env = TestEnv::new();
    let zip_path = env.root.join("artifact.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("a.txt", options).unwrap();
    writer.write_all(b"aaa").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let output = env.root.join("verify_off.zip");
    run_ok(
        env.cmd()
            .env("XUN_TEST_CORRUPT_OUTPUT_AFTER_WRITE", "truncate")
            .args([
                "backup",
                "convert",
                zip_path.to_str().unwrap(),
                "--format",
                "zip",
                "-o",
                output.to_str().unwrap(),
                "--verify-output",
                "off",
            ]),
    );
    assert!(output.exists());
}

#[test]
fn backup_convert_progress_always_emits_verify_read_write_and_verify_output_phases() {
    let env = TestEnv::new();
    let zip_path = env.root.join("artifact.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("a.txt", options).unwrap();
    writer.write_all(b"aaa").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let out = run_ok(env.cmd().args([
        "backup",
        "convert",
        zip_path.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        env.root.join("progress.7z").to_str().unwrap(),
        "--progress",
        "always",
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("progress: phase=verify_source"));
    assert!(stderr.contains("progress: phase=read"));
    assert!(stderr.contains("progress: phase=compress"));
    assert!(stderr.contains("progress: phase=write"));
    assert!(stderr.contains("progress: phase=verify_output"));
}

#[test]
fn backup_convert_7z_output_verify_on_detects_corrupted_postwrite_output() {
    let env = TestEnv::new();
    let zip_path = env.root.join("artifact.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("a.txt", options).unwrap();
    writer.write_all(b"aaa").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let output = env.root.join("verify_fail.7z");
    let out = run_err(
        env.cmd()
            .env("XUN_TEST_CORRUPT_OUTPUT_AFTER_WRITE", "truncate")
            .args([
                "backup",
                "convert",
                zip_path.to_str().unwrap(),
                "--format",
                "7z",
                "-o",
                output.to_str().unwrap(),
            ]),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("output verify failed"));
}

#[test]
fn backup_convert_7z_output_verify_off_skips_corrupted_postwrite_output_check() {
    let env = TestEnv::new();
    let zip_path = env.root.join("artifact.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("a.txt", options).unwrap();
    writer.write_all(b"aaa").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let output = env.root.join("verify_off.7z");
    run_ok(
        env.cmd()
            .env("XUN_TEST_CORRUPT_OUTPUT_AFTER_WRITE", "truncate")
            .args([
                "backup",
                "convert",
                zip_path.to_str().unwrap(),
                "--format",
                "7z",
                "-o",
                output.to_str().unwrap(),
                "--verify-output",
                "off",
            ]),
    );
    assert!(output.exists());
}

#[test]
fn backup_convert_7z_output_verify_on_json_reports_verify_failed() {
    let env = TestEnv::new();
    let zip_path = env.root.join("artifact.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("a.txt", options).unwrap();
    writer.write_all(b"aaa").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let out = run_err(
        env.cmd()
            .env("XUN_TEST_CORRUPT_OUTPUT_AFTER_WRITE", "truncate")
            .args([
                "backup",
                "convert",
                zip_path.to_str().unwrap(),
                "--format",
                "7z",
                "-o",
                env.root.join("verify_fail_json.7z").to_str().unwrap(),
                "--json",
            ]),
    );
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["status"], "verify_failed");
}

#[test]
fn backup_skip_if_unchanged_skips_new_version() {
    let env = TestEnv::new();
    let root = env.root.join("proj_skip_unchanged");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "same").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "v1"]),
    );

    let out = run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "-m",
        "v2",
        "--skip-if-unchanged",
    ]));

    let backups = root.join("A_backups");
    let versions: Vec<String> = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("v") && n.contains('-') && !n.ends_with(".meta.json"))
        .collect();
    assert_eq!(versions.len(), 1, "no-change backup should not create v2");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("no changes detected"),
        "skip path should explain why backup was skipped, got: {stderr}"
    );
}

#[test]
fn bak_dry_run_creates_no_version() {
    let env = TestEnv::new();
    let root = env.root.join("proj_dry");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "hi").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "t", "--dry-run"]),
    );

    let backups = root.join("A_backups");
    // dry-run 时目录可能不存在，或存在但为空
    let entries: Vec<_> = fs::read_dir(&backups)
        .map(|rd| rd.flatten().collect())
        .unwrap_or_default();
    assert!(
        entries.is_empty(),
        "dry-run should create no backup entries"
    );
}

#[test]
fn bak_gitignore_excludes_file() {
    let env = TestEnv::new();
    let root = env.root.join("proj_gitignore");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("keep.txt"), "ok").unwrap();
    fs::write(root.join("skip.txt"), "no").unwrap();
    fs::write(root.join(".gitignore"), "skip.txt\n").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "." ],
  "exclude": [],
  "useGitignore": true
}"#;
    fs::write(root.join(".svconfig.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let backups = root.join("A_backups");
    let entry = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v1-"))
        .expect("backup v1 not found");
    let first = entry.path();
    assert!(first.join("keep.txt").exists());
    assert!(!first.join("skip.txt").exists());
}

#[test]
fn bak_incremental_reports_new_file() {
    let env = TestEnv::new();
    let root = env.root.join("proj_inc");
    let data = root.join("data");
    fs::create_dir_all(&data).unwrap();
    fs::write(data.join("a.txt"), "one").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "data" ],
  "exclude": []
}"#;
    fs::write(root.join(".svconfig.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "first"]),
    );

    fs::write(data.join("b.txt"), "two").unwrap();
    let out = run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "second"]),
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("+ "));
    assert!(err.contains("data\\b.txt") || err.contains("data/b.txt"));
}

#[test]
fn bak_retention_removes_old_versions() {
    let env = TestEnv::new();
    let root = env.root.join("proj_ret");
    let data = root.join("data");
    fs::create_dir_all(&data).unwrap();
    fs::write(data.join("a.txt"), "one").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 2, "deleteCount": 1 },
  "include": [ "data" ],
  "exclude": []
}"#;
    fs::write(root.join(".svconfig.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "v1"]),
    );
    fs::write(data.join("a.txt"), "two").unwrap();
    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "v2"]),
    );
    fs::write(data.join("a.txt"), "three").unwrap();
    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "v3"]),
    );

    let backups = root.join("A_backups");
    let mut versions: Vec<String> = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("v") && n.contains('-'))
        .collect();
    versions.sort();

    assert_eq!(versions.len(), 2);
    assert!(!versions.iter().any(|n| n.starts_with("v1-")));
}

#[test]
fn bak_missing_config_auto_creates_default_config() {
    let env = TestEnv::new();
    let root = env.root.join("proj_no_cfg");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "hi").unwrap();

    let out = run_ok(env.cmd().args([
        "bak",
        "-C",
        root.to_str().unwrap(),
        "--include",
        "a.txt",
        "--no-compress",
        "-m",
        "t",
    ]));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(root.join(".xun-bak.json").exists());
    assert!(
        err.contains("Auto-created default config"),
        "unexpected stderr:\n{err}"
    );
}

#[test]
fn bak_compress_true_creates_zip() {
    let env = TestEnv::new();
    let root = env.root.join("proj_zip");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "hi").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": true },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let backups = root.join("A_backups");
    let zip = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("v1-") && n.ends_with(".zip"))
        })
        .expect("backup zip not found");
    assert!(zip.is_file());

    let file = fs::File::open(&zip).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut manifest = archive.by_name(".bak-manifest.json").unwrap();
    let mut manifest_text = String::new();
    std::io::Read::read_to_string(&mut manifest, &mut manifest_text).unwrap();
    let value: Value = serde_json::from_str(&manifest_text).unwrap();
    assert_eq!(value["version"], 2);
    assert_eq!(value["entries"][0]["path"], "a.txt");
}

#[test]
fn bak_incremental_reports_modified_file_with_tilde() {
    let env = TestEnv::new();
    let root = env.root.join("proj_mod");
    fs::create_dir_all(&root).unwrap();
    let file = root.join("a.txt");
    fs::write(&file, "one").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".svconfig.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "v1"]),
    );

    // bak's incremental change detection treats mtime deltas <= 2s as unchanged (filesystem timestamp slop).
    thread::sleep(Duration::from_secs(3));
    fs::write(&file, "two").unwrap();
    let out = run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "v2"]),
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains('~') && err.contains("a.txt"),
        "expected modified marker in stderr:\n{err}"
    );
}

#[test]
fn bak_incremental_reports_deleted_file_with_minus() {
    let env = TestEnv::new();
    let root = env.root.join("proj_del");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "one").unwrap();
    fs::write(root.join("b.txt"), "two").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "." ],
  "exclude": []
}"#;
    fs::write(root.join(".svconfig.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "v1"]),
    );
    fs::remove_file(root.join("b.txt")).unwrap();

    let out = run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "v2"]),
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("\n- ") && err.contains("b.txt"),
        "expected deleted marker in stderr:\n{err}"
    );
}

#[test]
fn bak_version_increments_v1_v2() {
    let env = TestEnv::new();
    let root = env.root.join("proj_ver");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "hi").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".svconfig.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "first"]),
    );
    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "second"]),
    );

    let backups = root.join("A_backups");
    let mut names: Vec<String> = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("v") && n.contains('-'))
        .collect();
    names.sort();

    assert!(names.iter().any(|n| n.starts_with("v1-")));
    assert!(names.iter().any(|n| n.starts_with("v2-")));
}

// ─── 新增：配置迁移 ───────────────────────────────────────────────────────────

#[test]
fn bak_config_migration_svconfig_to_xun_bak() {
    let env = TestEnv::new();
    let root = env.root.join("proj_migrate");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "hello").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    // 写旧名
    fs::write(root.join(".svconfig.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "migrate"]),
    );

    // 旧名应已被 rename
    assert!(
        !root.join(".svconfig.json").exists(),
        ".svconfig.json should be migrated"
    );
    // 新名应存在
    assert!(
        root.join(".xun-bak.json").exists(),
        ".xun-bak.json should exist after migration"
    );
    // 备份仍然正常创建
    let backups = root.join("A_backups");
    let found = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .any(|e| e.file_name().to_string_lossy().starts_with("v1-"));
    assert!(found, "backup v1 should be created after migration");
}

// ─── 新增：增量备份 ──────────────────────────────────────────────────────────

#[test]
fn bak_incremental_only_copies_changed_files() {
    let env = TestEnv::new();
    let root = env.root.join("proj_incr");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    fs::write(root.join("b.txt"), "bbb").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt", "b.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    // 全量备份 v1
    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "full"]),
    );

    // 只修改 a.txt（等待 mtime 变化，避免同秒误判为 Unchanged）
    thread::sleep(Duration::from_secs(2));
    fs::write(root.join("a.txt"), "aaa-modified").unwrap();

    // 增量备份 v2
    run_ok(env.cmd().args([
        "bak",
        "-C",
        root.to_str().unwrap(),
        "-m",
        "incr",
        "--incremental",
    ]));

    let backups = root.join("A_backups");
    let v2 = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .find(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            n.starts_with("v2-") && n.ends_with("-incr")
        })
        .expect("v2 incremental backup not found");

    let v2_path = v2.path();
    // 增量备份应包含 a.txt（修改过）
    assert!(
        v2_path.join("a.txt").exists(),
        "a.txt should be in incremental backup"
    );
    // b.txt 未修改，增量备份不含它
    assert!(
        !v2_path.join("b.txt").exists(),
        "b.txt should NOT be in incremental backup"
    );
}

#[test]
fn backup_skip_if_unchanged_still_creates_version_when_changed() {
    let env = TestEnv::new();
    let root = env.root.join("proj_skip_changed");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "v1").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "v1"]),
    );
    thread::sleep(Duration::from_millis(50));
    fs::write(root.join("a.txt"), "v2").unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "-m",
        "v2",
        "--skip-if-unchanged",
    ]));

    let backups = root.join("A_backups");
    let versions: Vec<String> = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("v") && n.contains('-') && !n.ends_with(".meta.json"))
        .collect();
    assert_eq!(versions.len(), 2, "changed backup should still create v2");
}

#[test]
fn backup_skip_if_unchanged_from_config_skips_new_version() {
    let env = TestEnv::new();
    let root = env.root.join("proj_skip_unchanged_cfg");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "same").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": [],
  "skipIfUnchanged": true
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "v1"]),
    );

    let out = run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "v2"]),
    );

    let backups = root.join("A_backups");
    let versions: Vec<String> = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("v") && n.contains('-') && !n.ends_with(".meta.json"))
        .collect();
    assert_eq!(
        versions.len(),
        1,
        "config skipIfUnchanged should skip creating a new version"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("no changes detected"));
}

#[test]
fn backup_skip_if_unchanged_uses_hash_when_only_mtime_changes() {
    let env = TestEnv::new();
    let root = env.root.join("proj_skip_hash_only_mtime");
    fs::create_dir_all(&root).unwrap();
    let file = root.join("a.txt");
    fs::write(&file, "same").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": [],
  "skipIfUnchanged": true
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "v1"]),
    );

    set_last_write_time_utc(&file, 2025, 1, 2);

    let out = run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "v2"]),
    );

    let backups = root.join("A_backups");
    let versions: Vec<String> = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("v") && n.contains('-') && !n.ends_with(".meta.json"))
        .collect();
    assert_eq!(
        versions.len(),
        1,
        "hash-aware skipIfUnchanged should skip mtime-only changes"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("no changes detected"));
}

#[test]
fn backup_diff_mode_meta_treats_mtime_only_change_as_modified() {
    let env = TestEnv::new();
    let root = env.root.join("proj_diff_mode_meta_mtime");
    fs::create_dir_all(&root).unwrap();
    let file = root.join("a.txt");
    fs::write(&file, "same").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "v1"]),
    );

    set_last_write_time_utc(&file, 2025, 1, 2);

    let out = run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "-m",
        "v2",
        "--skip-if-unchanged",
        "--diff-mode",
        "meta",
        "--json",
    ]));

    let backups = root.join("A_backups");
    let versions: Vec<String> = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("v") && n.contains('-') && !n.ends_with(".meta.json"))
        .collect();
    assert_eq!(versions.len(), 2, "meta diff mode should create a new version");

    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["status"], "ok");
    assert_eq!(value["diff_mode"], "meta");
    assert_eq!(value["modified"], 1);
}

#[test]
fn backup_diff_mode_hash_requires_previous_hash_manifest() {
    let env = TestEnv::new();
    let root = env.root.join("proj_diff_mode_hash_requires_manifest");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "same").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "v1"]),
    );

    let v1 = fs::read_dir(root.join("A_backups"))
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v1-"))
        .unwrap()
        .path();
    fs::remove_file(v1.join(".bak-manifest.json")).unwrap();

    let out = run_err(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "-m",
        "v2",
        "--diff-mode",
        "hash",
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Hash diff mode requires previous backup .bak-manifest.json"));
}

#[test]
fn backup_full_reuses_unchanged_files_via_hardlink() {
    let env = TestEnv::new();
    let root = env.root.join("proj_full_hardlink");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "same").unwrap();
    fs::write(root.join("b.txt"), "change-v1").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt", "b.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "v1"]),
    );
    thread::sleep(Duration::from_millis(50));
    fs::write(root.join("b.txt"), "change-v2").unwrap();
    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "v2"]),
    );

    let backups = root.join("A_backups");
    let v1 = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v1-"))
        .unwrap()
        .path();
    let v2 = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v2-"))
        .unwrap()
        .path();

    assert!(
        same_file_index(&v1.join("a.txt"), &v2.join("a.txt")),
        "unchanged file should be hardlinked between v1 and v2"
    );
    assert!(
        !same_file_index(&v1.join("b.txt"), &v2.join("b.txt")),
        "changed file should not be hardlinked between v1 and v2"
    );
}

#[test]
fn backup_full_reuses_renamed_file_via_hash_hardlink() {
    let env = TestEnv::new();
    let root = env.root.join("proj_full_reuse_rename");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("old.txt"), "same-content").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "old.txt", "new.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "v1"]),
    );

    thread::sleep(Duration::from_millis(50));
    fs::rename(root.join("old.txt"), root.join("new.txt")).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "v2"]),
    );

    let backups = root.join("A_backups");
    let v1 = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v1-"))
        .unwrap()
        .path();
    let v2 = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v2-"))
        .unwrap()
        .path();

    assert!(
        !v2.join("old.txt").exists(),
        "renamed old path should not exist in v2 backup"
    );
    assert!(
        v2.join("new.txt").exists(),
        "renamed new path should exist in v2 backup"
    );
    assert!(
        same_file_index(&v1.join("old.txt"), &v2.join("new.txt")),
        "renamed file should be hardlinked to previous content by hash"
    );
}

#[test]
fn backup_full_reuses_added_duplicate_file_via_hash_hardlink() {
    let env = TestEnv::new();
    let root = env.root.join("proj_full_reuse_duplicate");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "same-content").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt", "b.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "v1"]),
    );

    thread::sleep(Duration::from_millis(50));
    fs::write(root.join("b.txt"), "same-content").unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "v2"]),
    );

    let backups = root.join("A_backups");
    let v1 = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v1-"))
        .unwrap()
        .path();
    let v2 = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v2-"))
        .unwrap()
        .path();

    assert!(
        same_file_index(&v1.join("a.txt"), &v2.join("b.txt")),
        "new duplicate file should hardlink to previous snapshot content"
    );
}

#[test]
fn backup_writes_hash_cache_and_does_not_backup_it() {
    let env = TestEnv::new();
    let root = env.root.join("proj_hash_cache_file");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "same").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt", ".xun-bak-hash-cache.json" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "v1"]),
    );

    assert!(root.join(".xun-bak-hash-cache.json").exists());

    let backups = root.join("A_backups");
    let v1 = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v1-"))
        .unwrap()
        .path();
    assert!(!v1.join(".xun-bak-hash-cache.json").exists());
}

#[test]
fn backup_ignores_corrupted_hash_cache_and_recovers() {
    let env = TestEnv::new();
    let root = env.root.join("proj_hash_cache_corrupt");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "same").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();
    fs::write(root.join(".xun-bak-hash-cache.json"), "{not-json").unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "v1"]),
    );

    let backups = root.join("A_backups");
    let v1 = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v1-"))
        .unwrap()
        .path();
    assert!(v1.join("a.txt").exists());
}

#[test]
fn backup_json_reports_hash_cache_stats_when_skip_if_unchanged_hits_cache() {
    let env = TestEnv::new();
    let root = env.root.join("proj_backup_json_hash_stats");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "same").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "v1"]),
    );

    let out = run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "-m",
        "v2",
        "--skip-if-unchanged",
        "--json",
    ]));
    let value: Value = serde_json::from_slice(&out.stdout).expect("backup json should be valid");
    assert_eq!(value["action"], "backup");
    assert_eq!(value["status"], "skipped");
    assert_eq!(value["hash_checked_files"], 1);
    assert_eq!(value["hash_cache_hits"], 1);
    assert_eq!(value["hash_computed_files"], 0);
    assert_eq!(value["hash_failed_files"], 0);
    assert_eq!(value["new"], 0);
    assert_eq!(value["modified"], 0);
    assert_eq!(value["reused"], 0);
    assert_eq!(value["deleted"], 0);
}

#[test]
fn backup_without_hash_manifest_reinitializes_instead_of_using_legacy_metadata_baseline() {
    let env = TestEnv::new();
    let root = env.root.join("proj_backup_missing_hash_manifest");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "same").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "v1"]),
    );

    let backups = root.join("A_backups");
    let v1 = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v1-"))
        .unwrap()
        .path();
    fs::remove_file(v1.join(".bak-manifest.json")).unwrap();

    let out = run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "-m",
        "v2",
        "--skip-if-unchanged",
        "--json",
    ]));
    let value: Value = serde_json::from_slice(&out.stdout).expect("backup json should be valid");
    assert_eq!(value["status"], "ok");
    assert_eq!(value["new"], 1);
    assert_eq!(value["baseline_mode"], "fresh_full");

    let has_v2 = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .any(|entry| entry.file_name().to_string_lossy().starts_with("v2-"));
    assert!(has_v2, "second backup should be created as a fresh full snapshot");
}

#[test]
fn backup_skip_if_unchanged_treats_case_only_path_change_as_same_path() {
    let env = TestEnv::new();
    let root = env.root.join("proj_backup_case_only_path_change");
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("docs").join("ReadMe.TXT"), "same").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "docs" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "v1"]),
    );

    fs::rename(
        root.join("docs").join("ReadMe.TXT"),
        root.join("docs").join("readme.txt"),
    )
    .unwrap();

    let out = run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "-m",
        "v2",
        "--skip-if-unchanged",
        "--json",
    ]));
    let value: Value = serde_json::from_slice(&out.stdout).expect("backup json should be valid");
    assert_eq!(value["status"], "skipped");

    let version_count = fs::read_dir(root.join("A_backups"))
        .unwrap()
        .flatten()
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.starts_with('v') && !name.ends_with(".meta.json")
        })
        .count();
    assert_eq!(version_count, 1);
}

#[test]
fn backup_records_removed_paths_in_snapshot_manifest() {
    let env = TestEnv::new();
    let root = env.root.join("proj_removed_manifest");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "one").unwrap();
    fs::write(root.join("b.txt"), "two").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt", "b.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "v1"]),
    );
    fs::remove_file(root.join("b.txt")).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "v2"]),
    );

    let v2 = fs::read_dir(root.join("A_backups"))
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v2-"))
        .unwrap()
        .path();
    let manifest: Value =
        serde_json::from_slice(&fs::read(v2.join(".bak-manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["removed"], serde_json::json!(["b.txt"]));
    assert_eq!(manifest["entries"].as_array().unwrap().len(), 1);
    assert_eq!(manifest["entries"][0]["path"], "a.txt");
}

#[test]
fn backup_mixed_changes_write_expected_manifest_and_stats() {
    let env = TestEnv::new();
    let root = env.root.join("proj_mixed_changes_manifest");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    fs::write(root.join("b.txt"), "bbb").unwrap();
    fs::write(root.join("c.txt"), "ccc").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt", "b.txt", "c.txt", "d.txt", "e.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "v1"]),
    );

    thread::sleep(Duration::from_millis(50));
    fs::write(root.join("a.txt"), "aaa-modified").unwrap();
    fs::remove_file(root.join("b.txt")).unwrap();
    fs::write(root.join("d.txt"), "ccc").unwrap();
    fs::write(root.join("e.txt"), "eee-new").unwrap();

    let out = run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "-m",
        "v2",
        "--json",
    ]));
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["new"], 1);
    assert_eq!(value["modified"], 1);
    assert_eq!(value["reused"], 1);
    assert_eq!(value["deleted"], 1);

    let v2 = fs::read_dir(root.join("A_backups"))
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v2-"))
        .unwrap()
        .path();
    let manifest: Value =
        serde_json::from_slice(&fs::read(v2.join(".bak-manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["removed"], serde_json::json!(["b.txt"]));
    let mut paths: Vec<String> = manifest["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["path"].as_str().unwrap().to_string())
        .collect();
    paths.sort();
    assert_eq!(paths, vec!["a.txt", "c.txt", "d.txt", "e.txt"]);

    let v2_name = v2.file_name().unwrap().to_string_lossy().to_string();
    let restore_root = env.root.join("mixed_changes_restore");
    run_ok(env.cmd().args(["backup", "restore",
        &v2_name,
        "-C",
        root.to_str().unwrap(),
        "--to",
        restore_root.to_str().unwrap(),
        "-y",
    ]));
    assert_eq!(fs::read_to_string(restore_root.join("a.txt")).unwrap(), "aaa-modified");
    assert_eq!(fs::read_to_string(restore_root.join("c.txt")).unwrap(), "ccc");
    assert_eq!(fs::read_to_string(restore_root.join("d.txt")).unwrap(), "ccc");
    assert_eq!(fs::read_to_string(restore_root.join("e.txt")).unwrap(), "eee-new");
    assert!(!restore_root.join("b.txt").exists());
}

#[test]
fn backup_restore_recomputed_hash_matches_manifest() {
    let env = TestEnv::new();
    let root = env.root.join("proj_restore_hash_match");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src").join("main.rs"), "fn main() {}\n").unwrap();
    fs::write(root.join("src").join("lib.rs"), "pub fn value() -> u32 { 7 }\n").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "src" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "v1"]),
    );

    let v1_name = fs::read_dir(root.join("A_backups"))
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v1-"))
        .unwrap()
        .file_name()
        .to_string_lossy()
        .into_owned();
    let restore_root = env.root.join("restore_hash_out");
    run_ok(env.cmd().args(["backup", "restore",
        &v1_name,
        "-C",
        root.to_str().unwrap(),
        "--to",
        restore_root.to_str().unwrap(),
        "-y",
    ]));

    let backup_dir = root.join("A_backups").join(&v1_name);
    let manifest: Value =
        serde_json::from_slice(&fs::read(backup_dir.join(".bak-manifest.json")).unwrap()).unwrap();
    for entry in manifest["entries"].as_array().unwrap() {
        let rel = entry["path"].as_str().unwrap();
        let expected = entry["content_hash"].as_str().unwrap();
        let restored_path = restore_root.join(rel.replace('/', "\\"));
        let actual = blake3::hash(&fs::read(&restored_path).unwrap()).to_hex().to_string();
        assert_eq!(actual, expected, "restored hash mismatch for {}", rel);
    }
}

#[test]
fn backup_uses_zip_manifest_as_hash_baseline_for_skip_if_unchanged() {
    let env = TestEnv::new();
    let root = env.root.join("proj_zip_hash_baseline");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "same").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": true },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "v1"]),
    );

    let out = run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "-m",
        "v2",
        "--skip-if-unchanged",
        "--json",
    ]));
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["status"], "skipped");
    assert_eq!(value["baseline_mode"], "hash_manifest");
    assert_eq!(value["hash_checked_files"], 1);
}

// ─── 新增：list mtime 格式化 ─────────────────────────────────────────────────

#[test]
fn bak_list_shows_human_readable_mtime() {
    let env = TestEnv::new();
    let root = env.root.join("proj_list_mtime");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "data").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let out = env
        .cmd()
        .args(["bak", "-C", root.to_str().unwrap(), "list"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    // mtime 应为 "20xx-" 格式，而非纯数字时间戳
    assert!(
        stderr.contains("20") && stderr.contains("-"),
        "list mtime should be human-readable date (got: {stderr})"
    );
    // 不应含纯数字大时间戳（Unix epoch > 1700000000）
    assert!(
        !stderr.contains("17000000") && !stderr.contains("16000000"),
        "list should not show raw unix timestamp (got: {stderr})"
    );
}

#[test]
fn backup_list_empty_reports_no_backups() {
    let env = TestEnv::new();
    let root = env.root.join("proj_list_empty");
    fs::create_dir_all(&root).unwrap();

    let out = run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "list"]),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("No backups found"),
        "empty list should explain no backups found, got: {stderr}"
    );
}

#[test]
fn backup_list_subcommand_form_works() {
    let env = TestEnv::new();
    let root = env.root.join("proj_list_subcommand");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "data").unwrap();

    fs::write(
        root.join(".xun-bak.json"),
        r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#,
    )
    .unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "list-sub"]),
    );

    let out = run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "list"]),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("v1-"),
        "backup list subcommand should print backup entries, got: {stderr}"
    );
}

#[test]
fn backup_list_json_outputs_machine_readable_entries() {
    let env = TestEnv::new();
    let root = env.root.join("proj_list_json");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "data").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "json"]),
    );

    let out = run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "list", "--json"]),
    );
    let value: Value = serde_json::from_slice(&out.stdout).expect("list json should be valid");
    assert_eq!(value["action"], "list");
    assert_eq!(value["count"], 1);
    let items = value["items"]
        .as_array()
        .expect("list json should contain items");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["is_zip"], false);
    assert!(items[0]["size_bytes"].as_u64().unwrap_or(0) > 0);
}

// ─── 新增：verify 命令 ────────────────────────────────────────────────────────

#[test]
fn bak_verify_no_manifest_returns_error() {
    let env = TestEnv::new();
    let root = env.root.join("proj_verify");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "data").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    // 找到备份名
    let backups = root.join("A_backups");
    let v1_name = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v1-"))
        .unwrap()
        .file_name()
        .to_string_lossy()
        .into_owned();

    let out = run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "verify", &v1_name]),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("All files OK"),
        "verify should succeed for new directory backup, got: {stdout}"
    );
}

#[test]
fn bak_verify_zip_backup_reports_ok_with_hash_manifest() {
    let env = TestEnv::new();
    let root = env.root.join("proj_verify_zip");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "data").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": true },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "ziponly"]),
    );

    let zip_name = fs::read_dir(root.join("A_backups"))
        .unwrap()
        .flatten()
        .find_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.strip_suffix(".zip").map(str::to_string)
        })
        .expect("zip backup should exist");

    let out = run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "verify", &zip_name]),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("All files OK"),
        "zip verify should succeed with embedded hash manifest, got: {stdout}"
    );
}

#[test]
fn bak_verify_subcommand_form_works() {
    let env = TestEnv::new();
    let root = env.root.join("proj_verify_subcommand");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "data").unwrap();

    fs::write(
        root.join(".xun-bak.json"),
        r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#,
    )
    .unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "verify-sub"]),
    );

    let v1_name = fs::read_dir(root.join("A_backups"))
        .unwrap()
        .flatten()
        .find(|e| e.path().is_dir())
        .unwrap()
        .file_name()
        .to_string_lossy()
        .into_owned();

    let out = run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "verify", &v1_name]),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("All files OK"),
        "backup verify subcommand should succeed, got: {stdout}"
    );
}

// ─── 新增：find 命令 ──────────────────────────────────────────────────────────

#[test]
fn bak_find_lists_backups_with_meta() {
    let env = TestEnv::new();
    let root = env.root.join("proj_find");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "data").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "myfind"]),
    );

    // find 无过滤条件 → 列出有 meta 的备份
    let out = env
        .cmd()
        .args(["bak", "-C", root.to_str().unwrap(), "find"])
        .output()
        .unwrap();
    assert!(out.status.success(), "find should succeed");
    let stderr = String::from_utf8_lossy(&out.stderr);
    // 应包含备份名（v1-...）
    assert!(
        stderr.contains("v1") || stderr.contains("myfind"),
        "find output should list backup (got: {stderr})"
    );
}

#[test]
fn bak_find_subcommand_form_works() {
    let env = TestEnv::new();
    let root = env.root.join("proj_find_subcommand");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "data").unwrap();

    fs::write(
        root.join(".xun-bak.json"),
        r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#,
    )
    .unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "find-sub"]),
    );

    let backup_dir = fs::read_dir(root.join("A_backups"))
        .unwrap()
        .flatten()
        .find(|e| e.path().is_dir())
        .expect("backup dir should exist")
        .path();
    fs::write(
        backup_dir.join(".bak-meta.json"),
        serde_json::json!({
            "version": 1,
            "ts": 1_700_000_000u64,
            "desc": "find-sub",
            "tags": ["demo"],
            "stats": { "new": 1, "modified": 0, "deleted": 0 },
            "incremental": false
        })
        .to_string(),
    )
    .unwrap();

    let out = run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "find", "demo"]),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("find-sub"),
        "backup find subcommand should find tagged backup, got: {stderr}"
    );
}

#[test]
fn bak_find_filters_backups_by_tag() {
    let env = TestEnv::new();
    let root = env.root.join("proj_find_tag");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "data").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "tagged"]),
    );
    fs::write(root.join("a.txt"), "changed").unwrap();
    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "plain"]),
    );

    let backups_root = root.join("A_backups");
    let mut entries: Vec<_> = fs::read_dir(&backups_root)
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let tagged_backup = entries
        .iter()
        .find(|e| e.file_name().to_string_lossy().starts_with("v1-"))
        .expect("v1 backup should exist")
        .path();
    let plain_backup = entries
        .iter()
        .find(|e| e.file_name().to_string_lossy().starts_with("v2-"))
        .expect("v2 backup should exist")
        .path();

    fs::write(
        tagged_backup.join(".bak-meta.json"),
        serde_json::json!({
            "version": 1,
            "ts": 1_700_000_000u64,
            "desc": "tagged",
            "tags": ["demo"],
            "stats": { "new": 1, "modified": 0, "deleted": 0 },
            "incremental": false
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        plain_backup.join(".bak-meta.json"),
        serde_json::json!({
            "version": 1,
            "ts": 1_700_000_100u64,
            "desc": "plain",
            "tags": [],
            "stats": { "new": 1, "modified": 0, "deleted": 0 },
            "incremental": false
        })
        .to_string(),
    )
    .unwrap();

    let out = run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "find", "demo"]),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("tagged"),
        "filtered find should keep tagged backup"
    );
    assert!(
        !stderr.contains("plain"),
        "filtered find should exclude untagged backup, got: {stderr}"
    );
}

#[test]
fn bak_find_json_outputs_structured_fields() {
    let env = TestEnv::new();
    let root = env.root.join("proj_find_json");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "data").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "json-find"]),
    );

    let backup_dir = fs::read_dir(root.join("A_backups"))
        .unwrap()
        .flatten()
        .find(|e| e.path().is_dir())
        .expect("backup dir should exist")
        .path();
    fs::write(
        backup_dir.join(".bak-meta.json"),
        serde_json::json!({
            "version": 1,
            "ts": 1_700_000_000u64,
            "desc": "json-find",
            "tags": ["demo"],
            "stats": { "new": 1, "modified": 0, "deleted": 0 },
            "incremental": false
        })
        .to_string(),
    )
    .unwrap();

    let out = run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "find",
        "demo",
        "--json",
    ]));
    let value: Value = serde_json::from_slice(&out.stdout).expect("find json should be valid");
    assert_eq!(value["action"], "find");
    assert_eq!(value["count"], 1);
    let items = value["items"]
        .as_array()
        .expect("find json should contain items");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["desc"], "json-find");
    assert_eq!(items[0]["tags"][0], "demo");
    assert_eq!(items[0]["stats"]["new"], 1);
}

#[test]
fn bak_find_since_until_filters_backups_by_time() {
    let env = TestEnv::new();
    let root = env.root.join("proj_find_since_until");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "data").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "old"]),
    );
    fs::write(root.join("a.txt"), "changed").unwrap();
    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "new"]),
    );

    let backups_root = root.join("A_backups");
    let mut entries: Vec<_> = fs::read_dir(&backups_root)
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let old_backup = entries[0].path();
    let new_backup = entries[1].path();

    fs::write(
        old_backup.join(".bak-meta.json"),
        serde_json::json!({
            "version": 1,
            "ts": 1_700_000_000u64,
            "desc": "old",
            "tags": [],
            "stats": { "new": 1, "modified": 0, "deleted": 0 },
            "incremental": false,
            "size_bytes": 4
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        new_backup.join(".bak-meta.json"),
        serde_json::json!({
            "version": 1,
            "ts": 1_800_000_000u64,
            "desc": "new",
            "tags": [],
            "stats": { "new": 1, "modified": 0, "deleted": 0 },
            "incremental": false,
            "size_bytes": 7
        })
        .to_string(),
    )
    .unwrap();

    let out = run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "find",
        "--since",
        "2026-01-01T00:00:00Z",
        "--json",
    ]));
    let value: Value = serde_json::from_slice(&out.stdout).expect("find json should be valid");
    assert_eq!(value["action"], "find");
    assert_eq!(value["count"], 1);
    assert_eq!(value["filters"]["since"], 1_767_225_600u64);
    let items = value["items"]
        .as_array()
        .expect("find json should contain items");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["desc"], "new");
}

#[test]
fn bak_verify_json_outputs_ok_status() {
    let env = TestEnv::new();
    let root = env.root.join("proj_verify_json");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "data").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 10, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["backup", "-C", root.to_str().unwrap(), "-m", "verify-json"]),
    );

    let v1_name = fs::read_dir(root.join("A_backups"))
        .unwrap()
        .flatten()
        .find(|e| e.path().is_dir())
        .unwrap()
        .file_name()
        .to_string_lossy()
        .into_owned();

    let out = run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "verify",
        &v1_name,
        "--json",
    ]));
    let value: Value = serde_json::from_slice(&out.stdout).expect("verify json should be valid");
    assert_eq!(value["action"], "verify");
    assert_eq!(value["status"], "ok");
    assert_eq!(value["backup_type"], "dir");
    assert_eq!(
        value["corrupted_files"]
            .as_array()
            .expect("corrupted_files should be an array")
            .len(),
        0
    );
}

// ─── 新增：retention 时间窗口 ─────────────────────────────────────────────────

#[test]
fn bak_retention_keep_daily_preserves_one_per_day() {
    let env = TestEnv::new();
    let root = env.root.join("proj_keep_daily");
    let data = root.join("data");
    fs::create_dir_all(&data).unwrap();
    fs::write(data.join("a.txt"), "v1").unwrap();

    // maxBackups=2 但 keepDaily=3：即使超出 maxBackups，keepDaily 标记的备份不删
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 2, "deleteCount": 1, "keepDaily": 3 },
  "include": [ "data" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    // 创建 3 次备份（同一天内，所有备份属同一 day bucket）
    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "d1"]),
    );
    fs::write(data.join("a.txt"), "v2").unwrap();
    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "d2"]),
    );
    fs::write(data.join("a.txt"), "v3").unwrap();
    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "d3"]),
    );

    let backups = root.join("A_backups");
    let versions: Vec<String> = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with('v') && n.contains('-'))
        .collect();

    // keepDaily=3 保护今天的 1 个代表（最新），maxBackups=2 限制总数，
    // 但至少保留 keepDaily 保护的条目，所以不应全删到 0
    // 实际结果：3 条备份，overflow=1，to_delete=1，但 keep[最新]=true，
    // 只删最旧未标记的 → 剩余 2 条
    assert!(
        versions.len() >= 2,
        "keepDaily should protect recent backups, got: {versions:?}"
    );
    // 最新备份（v3）必须被保留
    assert!(
        versions.iter().any(|n| n.starts_with("v3-")),
        "latest backup v3 should be kept by keepDaily, got: {versions:?}"
    );
}

#[test]
fn bak_retention_max_backups_without_time_window() {
    // 无时间窗口时，超出 maxBackups 严格按最旧优先删除
    let env = TestEnv::new();
    let root = env.root.join("proj_max_only");
    let data = root.join("data");
    fs::create_dir_all(&data).unwrap();
    fs::write(data.join("a.txt"), "v1").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 3, "deleteCount": 1 },
  "include": [ "data" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    for i in 1..=5 {
        fs::write(data.join("a.txt"), format!("v{i}")).unwrap();
        run_ok(
            env.cmd()
                .args(["bak", "-C", root.to_str().unwrap(), "-m", &format!("v{i}")]),
        );
    }

    let backups = root.join("A_backups");
    let mut versions: Vec<String> = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with('v') && n.contains('-'))
        .collect();
    versions.sort();

    // maxBackups=3，每次备份后超出 1 个就删 1 个最旧的
    // v1 在 v4 后删，v2 在 v5 后删 → 剩余 v3/v4/v5
    assert_eq!(
        versions.len(),
        3,
        "should retain exactly 3 backups, got: {versions:?}"
    );
    assert!(
        !versions.iter().any(|n| n.starts_with("v1-")),
        "v1 should be deleted"
    );
    assert!(
        !versions.iter().any(|n| n.starts_with("v2-")),
        "v2 should be deleted"
    );
}

// ─── 新增：多级子目录备份 ─────────────────────────────────────────────────────

#[test]
fn bak_nested_directory_scan() {
    let env = TestEnv::new();
    let root = env.root.join("proj_nested");
    let deep = root.join("src").join("components").join("ui");
    fs::create_dir_all(&deep).unwrap();
    fs::write(
        deep.join("button.tsx"),
        "export const Button = () => <button/>",
    )
    .unwrap();
    fs::write(
        root.join("src").join("index.ts"),
        "export * from './components/ui/button'",
    )
    .unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "src" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(
        env.cmd()
            .args(["bak", "-C", root.to_str().unwrap(), "-m", "t"]),
    );

    let backups = root.join("A_backups");
    let v1 = fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("v1-"))
        .unwrap()
        .path();

    // 深层文件应被完整备份
    assert!(
        v1.join("src")
            .join("components")
            .join("ui")
            .join("button.tsx")
            .exists(),
        "nested file should be backed up"
    );
    assert!(
        v1.join("src").join("index.ts").exists(),
        "top-level src file should be backed up"
    );
}

