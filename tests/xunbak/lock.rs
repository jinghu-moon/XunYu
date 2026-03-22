use tempfile::tempdir;
use xun::xunbak::lock::{LockError, LockFile, read_lock_info};

#[test]
fn acquire_write_lock_creates_json_lockfile() {
    let dir = tempdir().unwrap();
    let container = dir.path().join("sample.xunbak");
    std::fs::write(&container, b"").unwrap();

    let lock = LockFile::acquire_write_lock(&container, "backup", 64).unwrap();
    assert!(lock.path().exists());
    let info = read_lock_info(lock.path()).unwrap();
    assert_eq!(info.command, "backup");
    assert_eq!(info.write_start_offset, 64);
    assert!(!info.tool_version.is_empty());
}

#[test]
fn second_write_lock_is_rejected() {
    let dir = tempdir().unwrap();
    let container = dir.path().join("sample.xunbak");
    std::fs::write(&container, b"").unwrap();
    let first = LockFile::acquire_write_lock(&container, "backup", 0).unwrap();

    let second = LockFile::acquire_write_lock(&container, "backup", 0);
    assert!(matches!(second, Err(LockError::ContainerLocked(_))));

    first.release().unwrap();
}

#[test]
fn release_deletes_lockfile() {
    let dir = tempdir().unwrap();
    let container = dir.path().join("sample.xunbak");
    std::fs::write(&container, b"").unwrap();
    let lock = LockFile::acquire_write_lock(&container, "backup", 0).unwrap();
    let lock_path = lock.path().to_path_buf();
    lock.release().unwrap();
    assert!(!lock_path.exists());
}

#[test]
fn lock_path_uses_container_path_with_lock_suffix() {
    let dir = tempdir().unwrap();
    let container = dir.path().join("sample.xunbak");
    std::fs::write(&container, b"").unwrap();
    let lock = LockFile::acquire_write_lock(&container, "backup", 0).unwrap();
    assert!(lock.path().ends_with("sample.xunbak.lock"));
    lock.release().unwrap();
}
