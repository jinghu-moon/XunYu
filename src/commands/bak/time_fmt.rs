use windows_sys::Win32::Foundation::{FILETIME, SYSTEMTIME};
use windows_sys::Win32::System::SystemInformation::GetLocalTime;
use windows_sys::Win32::System::Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime};

/// 将 Unix 时间戳（秒）格式化为 "yyyy-MM-dd HH:mm" 本地时间字符串
pub(crate) fn fmt_unix_ts(secs: u64) -> String {
    // Unix epoch → FILETIME（100ns 间隔，起点 1601-01-01）
    let ft_val = secs
        .saturating_mul(10_000_000)
        .saturating_add(11_644_473_600u64 * 10_000_000);
    let ft = FILETIME {
        dwLowDateTime: ft_val as u32,
        dwHighDateTime: (ft_val >> 32) as u32,
    };
    let mut utc: SYSTEMTIME = unsafe { std::mem::zeroed() };
    let mut local: SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe {
        FileTimeToSystemTime(&ft, &mut utc);
        SystemTimeToTzSpecificLocalTime(std::ptr::null(), &utc, &mut local);
    }
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        local.wYear, local.wMonth, local.wDay, local.wHour, local.wMinute
    )
}

/// 格式化当前本地时间，支持 .NET 风格 token
pub(crate) fn format_now(fmt: &str) -> String {
    let st = unsafe {
        let mut st: SYSTEMTIME = std::mem::zeroed();
        GetLocalTime(&mut st);
        st
    };
    fmt.replace("yyyy", &format!("{:04}", st.wYear))
        .replace("MM", &format!("{:02}", st.wMonth))
        .replace("dd", &format!("{:02}", st.wDay))
        .replace("HH", &format!("{:02}", st.wHour))
        .replace("mm", &format!("{:02}", st.wMinute))
        .replace("ss", &format!("{:02}", st.wSecond))
}
