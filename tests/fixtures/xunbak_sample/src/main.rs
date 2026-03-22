// Sample Rust project entry point for xunbak backup testing.
// This file tests: source code compression, UTF-8 encoding, typical size.

use std::collections::HashMap;
use std::path::PathBuf;

fn main() {
    let mut registry: HashMap<String, PathBuf> = HashMap::new();
    registry.insert("config".into(), PathBuf::from("config/settings.toml"));
    registry.insert("data".into(), PathBuf::from("data/store.db"));

    for (key, path) in &registry {
        println!("[{key}] => {}", path.display());
    }

    let result = compute_checksum(b"hello world");
    println!("checksum = {result:#x}");
}

fn compute_checksum(data: &[u8]) -> u32 {
    let mut hash: u32 = 0;
    for &byte in data {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_deterministic() {
        let a = compute_checksum(b"test");
        let b = compute_checksum(b"test");
        assert_eq!(a, b);
    }

    #[test]
    fn test_checksum_different_inputs() {
        let a = compute_checksum(b"foo");
        let b = compute_checksum(b"bar");
        assert_ne!(a, b);
    }
}
