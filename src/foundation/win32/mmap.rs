#![allow(dead_code)]

use std::fs::File;
use std::io;
use std::marker::PhantomData;
use std::os::windows::io::FromRawHandle;
use std::path::Path;
use std::ptr;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_SHARE_READ, OPEN_EXISTING};
use windows_sys::Win32::System::Memory::{CreateFileMappingW, FILE_MAP_READ, MEMORY_MAPPED_VIEW_ADDRESS, MapViewOfFile, UnmapViewOfFile};

use super::to_wide;

/// A read-only memory-mapped file view.
///
/// The mapping is created with `PAGE_READONLY` and viewed with `FILE_MAP_READ`.
/// On drop, the view is unmapped and the mapping handle is closed.
///
/// # Safety
/// The `data` pointer is valid for `len` bytes as long as this struct exists.
/// The mapping handle is separate from the file handle — closing the file does
/// not invalidate the mapping.
pub(crate) struct MmapView {
    view: MEMORY_MAPPED_VIEW_ADDRESS,
    len: usize,
    mapping: HANDLE,
    _file: File,
    // PhantomData to make MmapView !Send and !Sync.
    _marker: PhantomData<*const ()>,
}

impl MmapView {
    /// Open a file as a read-only memory mapping.
    ///
    /// Returns `Ok(None)` if the file does not exist or is empty.
    pub(crate) fn open(path: &Path) -> io::Result<Option<Self>> {
        // Open the file for reading with shared access.
        let wide = to_wide(path.as_os_str());
        let file_handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                FILE_GENERIC_READ,
                FILE_SHARE_READ,
                ptr::null(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                ptr::null_mut(),
            )
        };
        if file_handle == INVALID_HANDLE_VALUE {
            let err = io::Error::last_os_error();
            // Treat NotFound and PermissionDenied as "no cache" for graceful fallback.
            // PermissionDenied can occur when the path is a directory.
            if err.kind() == io::ErrorKind::NotFound || err.kind() == io::ErrorKind::PermissionDenied {
                return Ok(None);
            }
            return Err(err);
        }

        // Wrap in File for RAII cleanup on error paths.
        let file = unsafe { File::from_raw_handle(file_handle as _) };
        let metadata = file.metadata()?;
        let len = metadata.len() as usize;
        if len == 0 {
            return Ok(None);
        }

        // Create a read-only file mapping.
        // CreateFileMappingW does not take ownership of file_handle.
        let mapping = unsafe {
            CreateFileMappingW(
                file_handle,
                ptr::null(),
                0x02, // PAGE_READONLY
                0,
                0,
                ptr::null(),
            )
        };
        if mapping.is_null() {
            return Err(io::Error::last_os_error());
        }

        // Map the entire file into memory.
        let view = unsafe {
            MapViewOfFile(
                mapping,
                FILE_MAP_READ,
                0,
                0,
                len,
            )
        };
        if view.Value.is_null() {
            let err = io::Error::last_os_error();
            unsafe { CloseHandle(mapping); }
            return Err(err);
        }

        Ok(Some(Self {
            view,
            len,
            mapping,
            _file: file,
            _marker: PhantomData,
        }))
    }

    /// Get the mapped data as a byte slice.
    pub(crate) fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.view.Value as *const u8, self.len) }
    }

    /// Get the length of the mapped region.
    pub(crate) fn len(&self) -> usize {
        self.len
    }
}

impl Drop for MmapView {
    fn drop(&mut self) {
        unsafe {
            UnmapViewOfFile(self.view);
            CloseHandle(self.mapping);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn mmap_open_returns_none_for_missing_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.bin");
        let result = MmapView::open(&path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn mmap_open_returns_none_for_empty_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty.bin");
        fs::write(&path, b"").unwrap();
        let result = MmapView::open(&path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn mmap_load_returns_correct_data() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("data.bin");
        let data = b"Hello, mmap world!";
        fs::write(&path, data).unwrap();

        let mmap = MmapView::open(&path).unwrap().unwrap();
        assert_eq!(mmap.len(), data.len());
        assert_eq!(mmap.as_slice(), data);
    }

    #[test]
    fn mmap_handles_large_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("large.bin");
        // 1MB of data
        let data: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();
        fs::write(&path, &data).unwrap();

        let mmap = MmapView::open(&path).unwrap().unwrap();
        assert_eq!(mmap.len(), data.len());
        assert_eq!(mmap.as_slice(), &data[..]);
    }
}
