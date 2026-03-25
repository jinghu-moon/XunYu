use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

#[cfg(test)]
fn hash_bytes(bytes: &[u8]) -> [u8; 32] {
    *blake3::hash(bytes).as_bytes()
}

pub(crate) fn compute_file_content_hash(path: &Path) -> io::Result<[u8; 32]> {
    let mut input = File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 16 * 1024 * 1024];
    loop {
        let n = input.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(*hasher.finalize().as_bytes())
}

pub(crate) fn encode_hash_hex(hash: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in hash {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

pub(crate) fn decode_hash_hex(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64 {
        return Err(format!(
            "expected 64 hex chars for blake3 hash, got {}",
            value.len()
        ));
    }
    let mut out = [0u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = decode_hex_nibble(pair[0])?;
        let low = decode_hex_nibble(pair[1])?;
        out[index] = (high << 4) | low;
    }
    Ok(out)
}

fn decode_hex_nibble(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(format!("invalid hex digit: {}", byte as char)),
    }
}

#[allow(dead_code)]
pub(crate) fn build_path_index<T, F>(items: &[T], mut path_of: F) -> HashMap<&str, &T>
where
    F: FnMut(&T) -> &str,
{
    items.iter().map(|item| (path_of(item), item)).collect()
}

pub(crate) fn normalize_path_lookup_key(path: &str) -> String {
    path.replace('\\', "/").to_lowercase()
}

pub(crate) fn build_lookup_path_index<T, F>(items: &[T], mut path_of: F) -> HashMap<String, &T>
where
    F: FnMut(&T) -> &str,
{
    let mut index = HashMap::with_capacity(items.len());
    for item in items {
        index.insert(normalize_path_lookup_key(path_of(item)), item);
    }
    index
}

pub(crate) fn build_content_hash_groups<'a, T, F>(
    items: &'a [T],
    mut hash_of: F,
) -> HashMap<[u8; 32], Vec<&'a T>>
where
    F: FnMut(&T) -> [u8; 32],
{
    let mut groups: HashMap<[u8; 32], Vec<&'a T>> = HashMap::new();
    for item in items {
        groups.entry(hash_of(item)).or_default().push(item);
    }
    groups
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{
        build_content_hash_groups, build_lookup_path_index, build_path_index,
        compute_file_content_hash, decode_hash_hex, encode_hash_hex, hash_bytes,
        normalize_path_lookup_key,
    };

    #[derive(Debug, PartialEq, Eq)]
    struct TestEntry {
        path: &'static str,
        hash: [u8; 32],
    }

    #[test]
    fn compute_file_content_hash_matches_one_shot_blake3() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sample.bin");
        let content = b"shared hash module";
        std::fs::write(&path, content).unwrap();

        assert_eq!(
            compute_file_content_hash(&path).unwrap(),
            hash_bytes(content)
        );
    }

    #[test]
    fn compute_file_content_hash_matches_one_shot_blake3_for_10mb_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sample-10mb.bin");
        let content = vec![0x5au8; 10 * 1024 * 1024];
        std::fs::write(&path, &content).unwrap();

        assert_eq!(
            compute_file_content_hash(&path).unwrap(),
            hash_bytes(&content)
        );
    }

    #[test]
    fn compute_file_content_hash_matches_empty_file_hash() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty.bin");
        std::fs::write(&path, []).unwrap();

        assert_eq!(compute_file_content_hash(&path).unwrap(), hash_bytes(b""));
    }

    #[test]
    fn build_path_index_returns_path_to_entry_map() {
        let entries = vec![
            TestEntry {
                path: "a.txt",
                hash: [1; 32],
            },
            TestEntry {
                path: "b.txt",
                hash: [2; 32],
            },
        ];

        let index = build_path_index(&entries, |entry| entry.path);
        assert_eq!(index["a.txt"].hash, [1; 32]);
        assert_eq!(index["b.txt"].hash, [2; 32]);
    }

    #[test]
    fn build_content_hash_groups_keeps_multiple_entries_per_hash() {
        let entries = vec![
            TestEntry {
                path: "a.txt",
                hash: [7; 32],
            },
            TestEntry {
                path: "b.txt",
                hash: [7; 32],
            },
            TestEntry {
                path: "c.txt",
                hash: [9; 32],
            },
        ];

        let groups = build_content_hash_groups(&entries, |entry| entry.hash);
        assert_eq!(groups[&[7; 32]].len(), 2);
        assert_eq!(groups[&[9; 32]].len(), 1);
    }

    #[test]
    fn normalize_path_lookup_key_normalizes_separators_and_case() {
        assert_eq!(
            normalize_path_lookup_key("Src\\Nested/ReadMe.TXT"),
            "src/nested/readme.txt"
        );
    }

    #[test]
    fn build_lookup_path_index_uses_windows_style_case_insensitive_keys() {
        let entries = vec![TestEntry {
            path: "Src\\Main.RS",
            hash: [3; 32],
        }];

        let index = build_lookup_path_index(&entries, |entry| entry.path);
        assert_eq!(index["src/main.rs"].hash, [3; 32]);
    }

    #[test]
    fn encode_hash_hex_returns_64_lowercase_chars() {
        let hex = encode_hash_hex(&[0xab; 32]);
        assert_eq!(hex.len(), 64);
        assert!(
            hex.chars()
                .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase())
        );
    }

    #[test]
    fn decode_hash_hex_roundtrips_encoded_hash() {
        let hash = [0xcd; 32];
        let encoded = encode_hash_hex(&hash);
        assert_eq!(decode_hash_hex(&encoded).unwrap(), hash);
    }
}
