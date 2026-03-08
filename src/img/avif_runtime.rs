use std::ffi::{OsStr, c_void};
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use std::sync::OnceLock;

use image::{DynamicImage, ImageBuffer, Rgba};
use windows_sys::Win32::Foundation::{FreeLibrary, GetLastError, HMODULE};
use windows_sys::Win32::System::LibraryLoader::{
    GetProcAddress, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
    LoadLibraryExW, LoadLibraryW,
};

type DecodeFn = unsafe extern "C" fn(data: *const u8, len: usize, out: *mut XunAvifImage) -> i32;
type FreeFn = unsafe extern "C" fn(p: *mut c_void);

#[repr(C)]
#[derive(Default)]
struct XunAvifImage {
    pixels: *mut u8,
    width: u32,
    height: u32,
    stride: u32,
}

struct AvifApi {
    decode_rgba8: DecodeFn,
    free_fn: FreeFn,
}

static AVIF_API: OnceLock<Option<AvifApi>> = OnceLock::new();

fn avif_debug_enabled() -> bool {
    std::env::var_os("XUN_AVIF_DEBUG").is_some()
}

fn to_wide_nul(s: &str) -> Vec<u16> {
    let mut wide: Vec<u16> = OsStr::new(s).encode_wide().collect();
    wide.push(0);
    wide
}

unsafe fn load_symbol<T>(module: HMODULE, symbol: &'static [u8]) -> Result<T, ()> {
    let proc = unsafe { GetProcAddress(module, symbol.as_ptr()) };
    let Some(proc) = proc else {
        return Err(());
    };
    Ok(unsafe { std::mem::transmute_copy(&proc) })
}

fn try_load_library(path: &PathBuf) -> Option<HMODULE> {
    let normalized = path.to_string_lossy().replace('/', "\\");
    let wide = to_wide_nul(&normalized);
    let mut last_error = 0u32;
    let module = unsafe {
        let module = LoadLibraryExW(
            wide.as_ptr(),
            std::ptr::null_mut(),
            LOAD_LIBRARY_SEARCH_DEFAULT_DIRS | LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
        );
        if module.is_null() {
            last_error = GetLastError();
            let fallback = LoadLibraryW(wide.as_ptr());
            if fallback.is_null() {
                last_error = GetLastError();
            }
            fallback
        } else {
            module
        }
    };
    if module.is_null() {
        if avif_debug_enabled() {
            let msg = std::io::Error::from_raw_os_error(last_error as i32);
            eprintln!(
                "[img-avif] LoadLibrary failed: {} (GetLastError={last_error}, msg={msg})",
                path.display(),
            );
        }
        None
    } else {
        if avif_debug_enabled() {
            eprintln!("[img-avif] LoadLibraryW ok: {}", path.display());
        }
        Some(module)
    }
}

fn avif_dll_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::<PathBuf>::new();

    if let Ok(custom) = std::env::var("XUN_AVIF_DLL") {
        if !custom.trim().is_empty() {
            candidates.push(PathBuf::from(custom));
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("xun_avif.dll"));
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("xun_avif.dll"));
    }

    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("third_party")
            .join("avif")
            .join("install")
            .join("bin")
            .join("xun_avif.dll"),
    );

    candidates.push(PathBuf::from("xun_avif.dll"));
    candidates
}

fn load_avif_api() -> Option<AvifApi> {
    for dll_path in avif_dll_candidates() {
        let Some(module) = try_load_library(&dll_path) else {
            continue;
        };

        let loaded = || -> Result<AvifApi, ()> {
            Ok(AvifApi {
                decode_rgba8: unsafe { load_symbol(module, b"xun_avif_decode_rgba8\0") }?,
                free_fn: unsafe { load_symbol(module, b"xun_avif_free\0") }?,
            })
        };

        match loaded() {
            Ok(api) => return Some(api),
            Err(()) => {
                if avif_debug_enabled() {
                    eprintln!("[img-avif] missing symbol from: {}", dll_path.display());
                }
                unsafe {
                    FreeLibrary(module);
                }
            }
        }
    }

    if avif_debug_enabled() {
        eprintln!("[img-avif] no available xun_avif.dll backend, fallback to image crate");
    }
    None
}

fn avif_api() -> Option<&'static AvifApi> {
    AVIF_API.get_or_init(load_avif_api).as_ref()
}

pub fn try_decode_avif_via_dll(bytes: &[u8]) -> Option<DynamicImage> {
    let api = avif_api()?;

    let mut out = XunAvifImage::default();
    let rc = unsafe { (api.decode_rgba8)(bytes.as_ptr(), bytes.len(), &mut out) };
    if rc != 0 || out.pixels.is_null() || out.width == 0 || out.height == 0 {
        if avif_debug_enabled() {
            eprintln!(
                "[img-avif] xun_avif_decode_rgba8 failed: rc={rc}, ptr_null={}, width={}, height={}",
                out.pixels.is_null(),
                out.width,
                out.height
            );
        }
        return None;
    }

    let width = out.width as usize;
    let height = out.height as usize;
    let stride = out.stride as usize;
    let row_bytes = width.checked_mul(4)?;
    let total_bytes = row_bytes.checked_mul(height)?;

    let mut rgba = vec![0u8; total_bytes];

    unsafe {
        if stride == row_bytes {
            std::ptr::copy_nonoverlapping(out.pixels, rgba.as_mut_ptr(), total_bytes);
        } else {
            for y in 0..height {
                let src = out.pixels.add(y * stride);
                let dst = rgba.as_mut_ptr().add(y * row_bytes);
                std::ptr::copy_nonoverlapping(src, dst, row_bytes);
            }
        }
        (api.free_fn)(out.pixels.cast());
    }

    let img = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_vec(out.width, out.height, rgba)?;
    Some(DynamicImage::ImageRgba8(img))
}
