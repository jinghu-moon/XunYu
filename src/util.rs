use std::collections::HashSet;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::{fs, path::Path};

pub(crate) const EXIT_ACCESS_DENIED: i32 = 3;
#[cfg(feature = "lock")]
pub(crate) const EXIT_LOCKED_UNAUTHORIZED: i32 = 10;
#[cfg(feature = "lock")]
pub(crate) const EXIT_UNLOCK_FAILED: i32 = 11;
pub(crate) const EXIT_REBOOT_SCHEDULED: i32 = 20;

pub(crate) fn parse_tags(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for t in raw.split(',') {
        let tt = t.trim();
        if tt.is_empty() {
            continue;
        }
        let key = tt.to_lowercase();
        if seen.insert(key) {
            out.push(tt.to_string());
        }
    }
    out
}

pub(crate) fn normalize_path(raw: &str) -> String {
    let mut s = raw.trim().replace('\\', "/");
    while s.ends_with('/') {
        s.pop();
    }
    s.to_lowercase()
}

pub(crate) fn has_cmd(cmd: &str) -> bool {
    static CACHE: OnceLock<Mutex<std::collections::HashMap<String, bool>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()));

    let key = cmd.trim().to_ascii_lowercase();
    if let Ok(map) = cache.lock()
        && let Some(v) = map.get(&key)
    {
        return *v;
    }

    let ok = Command::new("where")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if let Ok(mut map) = cache.lock() {
        map.insert(key, ok);
    }
    ok
}

pub(crate) fn split_csv(values: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for v in values {
        for part in v.split(',') {
            let t = part.trim();
            if !t.is_empty() {
                out.push(t.to_string());
            }
        }
    }
    out
}

pub(crate) fn normalize_glob_path(raw: &str) -> String {
    let s = raw.trim().replace('\\', "/");
    let s = s.trim_start_matches("./");
    let s = s.trim_start_matches('/');
    s.to_lowercase()
}

#[derive(Default)]
pub(crate) struct IgnorePatterns {
    pub(crate) exclude: Vec<String>,
    pub(crate) include: Vec<String>,
}

pub(crate) fn read_ignore_file(path: &Path) -> IgnorePatterns {
    let mut out = IgnorePatterns::default();
    let Ok(content) = fs::read_to_string(path) else {
        return out;
    };
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix('!') {
            let pat = normalize_glob_path(rest);
            if !pat.is_empty() {
                out.include.push(pat);
            }
        } else {
            let pat = normalize_glob_path(trimmed);
            if !pat.is_empty() {
                out.exclude.push(pat);
            }
        }
    }
    out
}

pub(crate) fn glob_match(pattern: &str, text: &str) -> bool {
    let p = pattern.as_bytes();
    let t = text.as_bytes();
    let mut pi = 0usize;
    let mut ti = 0usize;
    let mut star = None;
    let mut match_i = 0usize;
    while ti < t.len() {
        if pi < p.len() && (p[pi] == b'?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == b'*' {
            star = Some(pi);
            match_i = ti;
            pi += 1;
        } else if let Some(star_pos) = star {
            pi = star_pos + 1;
            match_i += 1;
            ti = match_i;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == b'*' {
        pi += 1;
    }
    pi == p.len()
}

pub(crate) fn matches_patterns(
    rel: &str,
    name_lower: &str,
    patterns: &[String],
    is_dir: bool,
) -> bool {
    for pat in patterns {
        let mut p = pat.as_str();
        let dir_only = p.ends_with('/');
        if dir_only {
            p = p.trim_end_matches('/');
        }
        if p.is_empty() {
            continue;
        }
        let target = if p.contains('/') { rel } else { name_lower };
        if glob_match(p, target) {
            if dir_only && !is_dir {
                continue;
            }
            return true;
        }
        if dir_only && is_dir {
            let prefix = format!("{p}/");
            if rel == p || rel.starts_with(&prefix) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn split_csv_splits_and_trims() {
        let values = vec![
            "a,b".to_string(),
            " c ".to_string(),
            "".to_string(),
            "d, ,e".to_string(),
        ];
        assert_eq!(split_csv(&values), vec!["a", "b", "c", "d", "e"]);
    }

    #[test]
    fn split_csv_empty_returns_empty_vec() {
        let values = vec!["".to_string()];
        assert!(split_csv(&values).is_empty());
    }

    #[test]
    fn normalize_glob_path_normalizes_slashes_and_case_and_prefix() {
        assert_eq!(normalize_glob_path(".\\Foo\\Bar/"), "foo/bar/");
        assert_eq!(normalize_glob_path("/Foo/Bar"), "foo/bar");
    }

    #[test]
    fn matches_patterns_matches_file_globs() {
        let rel = "dir/file.txt";
        let name_lower = "file.txt";

        assert!(matches_patterns(
            rel,
            name_lower,
            &[String::from("*.txt")],
            false
        ));
        assert!(matches_patterns(
            rel,
            name_lower,
            &[String::from("dir/*.txt")],
            false
        ));
        assert!(!matches_patterns(
            rel,
            name_lower,
            &[String::from("*.rs")],
            false
        ));
    }

    #[test]
    fn matches_patterns_directory_suffix_matches_directories() {
        let pat = vec![String::from("build/")];

        assert!(matches_patterns("build", "build", &pat, true));
        assert!(matches_patterns("build/sub", "sub", &pat, true));
        assert!(!matches_patterns("build/file.txt", "file.txt", &pat, false));
    }

    #[test]
    fn read_ignore_file_parses_include_and_exclude() {
        let dir = tempdir().unwrap();
        let p = dir.path().join(".xunignore");
        fs::write(
            &p,
            r#"
# comment
target/
*.tmp
!target/keep.txt
! ./Allow/This.txt
"#,
        )
        .unwrap();

        let ig = read_ignore_file(&p);
        assert_eq!(ig.exclude, vec!["target/", "*.tmp"]);
        assert_eq!(ig.include, vec!["target/keep.txt", "allow/this.txt"]);
    }

    #[test]
    fn read_ignore_file_missing_returns_empty() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("missing.ignore");
        let ig = read_ignore_file(&p);
        assert!(ig.exclude.is_empty());
        assert!(ig.include.is_empty());
    }

    #[test]
    fn has_cmd_detects_existing_and_missing_commands() {
        assert!(has_cmd("where"));
        assert!(!has_cmd("xun-cmd-does-not-exist-123456"));
    }
}
