use std::ffi::{CStr, OsStr, c_char, c_int, c_uchar, c_ulong, c_void};
use std::os::windows::ffi::OsStrExt;
use std::sync::OnceLock;

use windows_sys::Win32::Foundation::{FreeLibrary, HMODULE};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

use crate::img::error::ImgError;

type TjHandle = *mut c_void;
type TjInitCompressFn = unsafe extern "C" fn() -> TjHandle;
type TjBufSizeFn =
    unsafe extern "C" fn(width: c_int, height: c_int, jpeg_subsamp: c_int) -> c_ulong;
type TjCompress2Fn = unsafe extern "C" fn(
    handle: TjHandle,
    src_buf: *const c_uchar,
    width: c_int,
    pitch: c_int,
    height: c_int,
    pixel_format: c_int,
    jpeg_buf: *mut *mut c_uchar,
    jpeg_size: *mut c_ulong,
    jpeg_subsamp: c_int,
    jpeg_qual: c_int,
    flags: c_int,
) -> c_int;
type TjDestroyFn = unsafe extern "C" fn(handle: TjHandle) -> c_int;
type TjFreeFn = unsafe extern "C" fn(buffer: *mut c_uchar);
type TjGetErrorStr2Fn = unsafe extern "C" fn(handle: TjHandle) -> *mut c_char;

const TJPF_RGB: c_int = 0;
const TJSAMP_420: c_int = 2;
const TJFLAG_NOREALLOC: c_int = 1024;
const TJFLAG_FASTDCT: c_int = 2048;

struct TurboJpegApi {
    tj_init_compress: TjInitCompressFn,
    tj_buf_size: TjBufSizeFn,
    tj_compress2: TjCompress2Fn,
    tj_destroy: TjDestroyFn,
    tj_free: TjFreeFn,
    tj_get_error_str2: TjGetErrorStr2Fn,
}

static TURBO_API: OnceLock<Result<TurboJpegApi, String>> = OnceLock::new();

struct CompressorGuard<'a> {
    api: &'a TurboJpegApi,
    handle: TjHandle,
}

impl Drop for CompressorGuard<'_> {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                (self.api.tj_destroy)(self.handle);
            }
        }
    }
}

fn encode_failed(msg: impl Into<String>) -> ImgError {
    ImgError::EncodeFailed {
        encoder: "turbojpeg",
        msg: msg.into(),
    }
}

fn to_wide_nul(s: &str) -> Vec<u16> {
    let mut wide: Vec<u16> = OsStr::new(s).encode_wide().collect();
    wide.push(0);
    wide
}

fn last_tj_error(api: &TurboJpegApi, handle: TjHandle) -> String {
    unsafe {
        let ptr = (api.tj_get_error_str2)(handle);
        if ptr.is_null() {
            return "unknown error".to_string();
        }
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

unsafe fn load_symbol<T>(module: HMODULE, symbol: &'static [u8]) -> Result<T, String> {
    let proc = unsafe { GetProcAddress(module, symbol.as_ptr()) };
    let Some(proc) = proc else {
        let name = std::str::from_utf8(&symbol[..symbol.len().saturating_sub(1)]).unwrap_or("?");
        return Err(format!("GetProcAddress({name}) failed"));
    };
    Ok(unsafe { std::mem::transmute_copy(&proc) })
}

fn load_turbojpeg_api() -> Result<TurboJpegApi, String> {
    let candidates = ["turbojpeg.dll", "libturbojpeg.dll"];
    let module = candidates
        .iter()
        .find_map(|dll| {
            let wide = to_wide_nul(dll);
            let handle = unsafe { LoadLibraryW(wide.as_ptr()) };
            if handle.is_null() { None } else { Some(handle) }
        })
        .ok_or_else(|| {
            format!(
                "未找到 TurboJPEG 运行时 DLL（尝试: {}）",
                candidates.join(", ")
            )
        })?;

    let load = || -> Result<TurboJpegApi, String> {
        Ok(TurboJpegApi {
            tj_init_compress: unsafe { load_symbol(module, b"tjInitCompress\0") }?,
            tj_buf_size: unsafe { load_symbol(module, b"tjBufSize\0") }?,
            tj_compress2: unsafe { load_symbol(module, b"tjCompress2\0") }?,
            tj_destroy: unsafe { load_symbol(module, b"tjDestroy\0") }?,
            tj_free: unsafe { load_symbol(module, b"tjFree\0") }?,
            tj_get_error_str2: unsafe { load_symbol(module, b"tjGetErrorStr2\0") }?,
        })
    };

    match load() {
        Ok(api) => Ok(api),
        Err(err) => {
            unsafe {
                FreeLibrary(module);
            }
            Err(err)
        }
    }
}

fn turbo_api() -> Result<&'static TurboJpegApi, ImgError> {
    match TURBO_API.get_or_init(load_turbojpeg_api) {
        Ok(api) => Ok(api),
        Err(err) => Err(encode_failed(err.clone())),
    }
}

pub fn encode_jpeg_turbo_runtime(
    rgb: &[u8],
    width: u32,
    height: u32,
    quality: u8,
) -> Result<Vec<u8>, ImgError> {
    let api = turbo_api()?;

    let width_i = c_int::try_from(width).map_err(|_| encode_failed("width 超过 c_int 范围"))?;
    let height_i = c_int::try_from(height).map_err(|_| encode_failed("height 超过 c_int 范围"))?;
    let pitch_i = width_i
        .checked_mul(3)
        .ok_or_else(|| encode_failed("pitch 计算溢出"))?;
    let quality_i = c_int::from(quality);

    let min_len = width as usize * height as usize * 3;
    if rgb.len() < min_len {
        return Err(encode_failed("输入 RGB 缓冲区长度不足"));
    }

    let handle = unsafe { (api.tj_init_compress)() };
    if handle.is_null() {
        return Err(encode_failed(format!(
            "tjInitCompress failed: {}",
            last_tj_error(api, std::ptr::null_mut())
        )));
    }
    let guard = CompressorGuard { api, handle };

    let max_size = unsafe { (api.tj_buf_size)(width_i, height_i, TJSAMP_420) } as usize;
    if max_size == 0 {
        return Err(encode_failed(format!(
            "tjBufSize failed: {}",
            last_tj_error(api, guard.handle)
        )));
    }

    let mut out = vec![0u8; max_size];
    let mut out_ptr = out.as_mut_ptr();
    let mut out_size: c_ulong = 0;
    let flags = TJFLAG_FASTDCT | TJFLAG_NOREALLOC;

    let rc = unsafe {
        (api.tj_compress2)(
            guard.handle,
            rgb.as_ptr(),
            width_i,
            pitch_i,
            height_i,
            TJPF_RGB,
            &mut out_ptr,
            &mut out_size,
            TJSAMP_420,
            quality_i,
            flags,
        )
    };

    if rc != 0 {
        let err = last_tj_error(api, guard.handle);
        if !out_ptr.is_null() && out_ptr != out.as_mut_ptr() {
            unsafe {
                (api.tj_free)(out_ptr);
            }
        }
        return Err(encode_failed(format!("tjCompress2 failed: {err}")));
    }

    if out_ptr.is_null() {
        return Err(encode_failed("tjCompress2 returned NULL output buffer"));
    }

    let encoded_len = usize::try_from(out_size).map_err(|_| encode_failed("输出长度转换失败"))?;

    if out_ptr != out.as_mut_ptr() {
        let mut copied = vec![0u8; encoded_len];
        unsafe {
            std::ptr::copy_nonoverlapping(out_ptr, copied.as_mut_ptr(), encoded_len);
            (api.tj_free)(out_ptr);
        }
        return Ok(copied);
    }

    if encoded_len > out.len() {
        return Err(encode_failed("tjCompress2 returned invalid output length"));
    }
    out.truncate(encoded_len);
    Ok(out)
}
