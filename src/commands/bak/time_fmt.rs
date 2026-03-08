pub(crate) fn format_now(fmt: &str) -> String {
    use windows_sys::Win32::Foundation::SYSTEMTIME;
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;

    let st = unsafe {
        let mut st: SYSTEMTIME = std::mem::zeroed();
        GetLocalTime(&mut st);
        st
    };
    // Support .NET-style tokens used in .svconfig.json
    fmt.replace("yyyy", &format!("{:04}", st.wYear))
        .replace("MM", &format!("{:02}", st.wMonth))
        .replace("dd", &format!("{:02}", st.wDay))
        .replace("HH", &format!("{:02}", st.wHour))
        .replace("mm", &format!("{:02}", st.wMinute))
        .replace("ss", &format!("{:02}", st.wSecond))
}
