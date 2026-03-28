use std::fs;
use std::os::windows::fs::MetadataExt;
use std::os::windows::io::AsRawHandle;
use std::path::Path;

use tempfile::tempdir;
use xun::xunbak::reader::ContainerReader;
use xun::xunbak::writer::{BackupOptions, ContainerWriter};

fn set_windows_metadata(path: &Path, attrs: u32, created: u64, modified: u64) {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Storage::FileSystem::{SetFileAttributesW, SetFileTime};

    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    let created_ft = FILETIME {
        dwLowDateTime: created as u32,
        dwHighDateTime: (created >> 32) as u32,
    };
    let modified_ft = FILETIME {
        dwLowDateTime: modified as u32,
        dwHighDateTime: (modified >> 32) as u32,
    };
    let ok = unsafe {
        SetFileTime(
            file.as_raw_handle() as HANDLE,
            &created_ft,
            std::ptr::null(),
            &modified_ft,
        )
    };
    assert_ne!(ok, 0);

    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    let ok = unsafe { SetFileAttributesW(wide.as_ptr(), attrs) };
    assert_ne!(ok, 0);
}

#[test]
fn restore_all_restores_contents_and_sizes() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(source.join("nested")).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    fs::write(source.join("nested").join("b.txt"), "bbb").unwrap();
    fs::write(source.join("c.txt"), "").unwrap();
    let container = dir.path().join("backup.xunbak");

    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();
    let target = dir.path().join("restore");
    let reader = ContainerReader::open(&container).unwrap();
    let result = reader.restore_all(&target).unwrap();
    assert_eq!(result.restored_files, 3);
    assert_eq!(fs::read_to_string(target.join("a.txt")).unwrap(), "aaa");
    assert_eq!(
        fs::read_to_string(target.join("nested").join("b.txt")).unwrap(),
        "bbb"
    );
    assert_eq!(fs::metadata(target.join("c.txt")).unwrap().len(), 0);
}

#[test]
fn restore_all_restores_windows_attributes_and_times() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    let source_file = source.join("meta.txt");
    fs::write(&source_file, "meta").unwrap();
    let attrs = 0x01 | 0x02;
    let created = 134_116_992_000_000_000u64;
    let modified = 134_116_992_500_000_000u64;
    set_windows_metadata(&source_file, attrs, created, modified);

    let container = dir.path().join("backup.xunbak");
    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();
    let target = dir.path().join("restore");
    let reader = ContainerReader::open(&container).unwrap();
    reader.restore_all(&target).unwrap();

    let restored = fs::metadata(target.join("meta.txt")).unwrap();
    assert_eq!(restored.file_attributes() & attrs, attrs);
    assert_eq!(restored.creation_time(), created);
    assert_eq!(restored.last_write_time(), modified);
}

#[test]
fn restore_all_on_empty_container_creates_empty_target() {
    let dir = tempdir().unwrap();
    let container = dir.path().join("empty.xunbak");
    ContainerWriter::create(&container).unwrap();
    let target = dir.path().join("restore");
    let reader = ContainerReader::open(&container).unwrap();
    let result = reader.restore_all(&target).unwrap();
    assert_eq!(result.restored_files, 0);
    assert!(target.exists());
    assert!(fs::read_dir(&target).unwrap().next().is_none());
}

#[test]
fn restore_all_skips_unchanged_existing_files() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(source.join("nested")).unwrap();
    fs::write(source.join("a.txt"), "aaa").unwrap();
    fs::write(source.join("nested").join("b.txt"), "bbb").unwrap();
    let container = dir.path().join("backup.xunbak");

    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();
    let target = dir.path().join("restore");
    let reader = ContainerReader::open(&container).unwrap();
    let first = reader.restore_all(&target).unwrap();
    assert_eq!(first.restored_files, 2);
    assert_eq!(first.skipped_unchanged, 0);

    let reader = ContainerReader::open(&container).unwrap();
    let second = reader.restore_all(&target).unwrap();
    assert_eq!(second.restored_files, 0);
    assert_eq!(second.skipped_unchanged, 2);
}

#[test]
fn restore_all_rewrites_same_size_changed_file() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "aaaa").unwrap();
    let container = dir.path().join("backup.xunbak");

    ContainerWriter::backup(&container, &source, &BackupOptions::default()).unwrap();
    let target = dir.path().join("restore");
    let reader = ContainerReader::open(&container).unwrap();
    reader.restore_all(&target).unwrap();

    fs::write(target.join("a.txt"), "zzzz").unwrap();
    let reader = ContainerReader::open(&container).unwrap();
    let result = reader.restore_all(&target).unwrap();
    assert_eq!(result.restored_files, 1);
    assert_eq!(result.skipped_unchanged, 0);
    assert_eq!(fs::read_to_string(target.join("a.txt")).unwrap(), "aaaa");
}
