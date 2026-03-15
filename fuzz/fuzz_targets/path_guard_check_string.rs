#![no_main]

use libfuzzer_sys::fuzz_target;
use xun::path_guard::{PathPolicy, validate_paths};

fuzz_target!(|data: &[u8]| {
    let raw = String::from_utf8_lossy(data);
    let policy = PathPolicy::for_output();
    let _ = validate_paths(vec![raw.to_string()], &policy);
});
