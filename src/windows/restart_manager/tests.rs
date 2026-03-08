use super::*;
use std::fs::File;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn get_locking_processes_empty_paths_returns_empty() {
    let lockers = get_locking_processes(&[]).expect("ok");
    assert!(lockers.is_empty());
}

#[test]
fn lock_query_error_classification_and_guidance() {
    let e = LockQueryError::from_win32(29, LockQueryStage::StartSession, "x");
    assert!(e.is_registry_unavailable());
    assert!(e.guidance().to_lowercase().contains("registry"));

    let e = LockQueryError::from_win32(121, LockQueryStage::GetListData, "x");
    assert!(e.is_registry_mutex_timeout());
    assert!(e.guidance().to_lowercase().contains("mutex"));

    let e = LockQueryError::from_win32(5, LockQueryStage::RegisterResources, "x");
    assert!(e.is_directory_path_error());
    assert!(e.guidance().to_lowercase().contains("directory"));

    let e = LockQueryError::from_win32(5, LockQueryStage::HandleEngine, "x");
    assert!(!e.is_directory_path_error());
    assert!(e.guidance().to_lowercase().contains("access denied"));

    let e = LockQueryError::from_ntstatus(-1, LockQueryStage::HandleEnumerate, "x");
    assert!(e.guidance().to_lowercase().contains("ntstatus"));
}

#[test]
fn lock_query_error_display_formats_win32_and_ntstatus() {
    let e = LockQueryError::from_win32(29, LockQueryStage::StartSession, "oops");
    let s = format!("{e}");
    assert!(s.starts_with("OS Error 29 at rm_start_session: oops"));

    let e = LockQueryError::from_ntstatus(0xC0000004u32 as i32, LockQueryStage::HandleEngine, "x");
    let s = format!("{e}");
    assert!(s.starts_with("NTSTATUS 0xC0000004 at handle_engine: x"));
}

#[test]
fn test_restart_manager_self_lock() {
    let temp_dir = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let test_file = temp_dir.join(format!(
        "xun_test_rm_lock_{}_{}.txt",
        std::process::id(),
        nanos
    ));

    // 1. Open the file to keep a handle open
    let mut file = File::create(&test_file).expect("Failed to create test file");
    writeln!(file, "test").unwrap();

    // 2. Query lockers
    let paths = vec![test_file.as_path()];
    let lockers = match get_locking_processes(&paths) {
        Ok(lockers) => lockers,
        Err(e) if e.is_registry_unavailable() || e.is_registry_mutex_timeout() => {
            // Some Windows environments do not expose stable Restart Manager query results.
            drop(file);
            let _ = std::fs::remove_file(&test_file);
            return;
        }
        Err(e) => panic!("RmGetList failed: {}", e),
    };

    // 3. We should find our own process
    let my_pid = std::process::id();
    let found = lockers.iter().any(|l| l.pid == my_pid);

    // 4. Cleanup by dropping `file` and removing the test file
    drop(file);
    let _ = std::fs::remove_file(&test_file);

    assert!(found, "Our process should be listed by Restart Manager");
}

#[test]
fn probe_registry_access_reports_error_when_key_is_not_writable() {
    let err =
        probe_registry_access_impl(|_subkey, _flags| Err(std::io::Error::from_raw_os_error(5)))
            .unwrap_err();
    assert_eq!(err.code, 5);
    assert_eq!(err.stage, LockQueryStage::RegistryProbe);
    assert!(err.detail.contains("HKCU\\SOFTWARE"));
}
