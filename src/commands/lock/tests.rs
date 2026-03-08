use super::*;

#[test]
fn unlock_max_retries_is_three() {
    assert_eq!(UNLOCK_MAX_RETRIES, 3);
}

#[test]
fn critical_process_name_detection_is_case_insensitive() {
    assert!(is_critical_process_name("lsass.exe"));
    assert!(is_critical_process_name("LSASS.EXE"));
    assert!(!is_critical_process_name("notepad.exe"));
}
