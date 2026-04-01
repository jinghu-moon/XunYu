use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct NormalizedPath {
    display: String,
    key: String,
}

impl NormalizedPath {
    pub(crate) fn display(&self) -> &str {
        &self.display
    }

    pub(crate) fn key(&self) -> &str {
        &self.key
    }
}

#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalizePathError {
    Empty,
    InvalidUncPath,
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BookmarkSource {
    Explicit,
    Imported,
    Learned,
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BookmarkRecordView<'a> {
    pub(crate) name: Option<&'a str>,
    pub(crate) name_norm: Option<&'a str>,
    pub(crate) path: &'a str,
    pub(crate) path_norm: &'a str,
    pub(crate) tags: &'a [String],
    pub(crate) source: BookmarkSource,
    pub(crate) pinned: bool,
    pub(crate) visit_count: Option<u32>,
    pub(crate) last_visited: Option<u64>,
    pub(crate) frecency_score: f64,
    pub(crate) workspace: Option<&'a str>,
}

#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryScope {
    Auto,
    Global,
    Child,
    BaseDir(PathBuf),
    Workspace(String),
}

#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryContext {
    pub cwd: PathBuf,
    pub workspace: Option<String>,
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScoreFactors {
    pub match_score: f64,
    pub frecency_mult: f64,
    pub scope_mult: f64,
    pub source_mult: f64,
    pub pin_mult: f64,
}

pub(crate) fn normalize_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

pub(crate) fn normalize_path(
    raw: &str,
    cwd: &Path,
    home: Option<&Path>,
) -> Result<NormalizedPath, NormalizePathError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(NormalizePathError::Empty);
    }

    let expanded = expand_tilde(raw, home);
    validate_unc(&expanded)?;
    let absolute = absolutize_path(&expanded, cwd);
    let display = normalize_display_path(&absolute);
    let key = comparison_key(&display);
    Ok(NormalizedPath { display, key })
}

pub(crate) fn compute_match_score<T: AsRef<str>>(
    tokens: &[T],
    bookmark: &BookmarkRecordView<'_>,
) -> f64 {
    if tokens.is_empty() {
        return 0.0;
    }

    let name_norm = bookmark.name_norm.as_deref().unwrap_or("");
    let basename = bookmark.path_norm.rsplit('/').next().unwrap_or("");

    let mut total = 0.0;
    for (idx, token) in tokens.iter().enumerate() {
        let token_norm = token.as_ref().to_ascii_lowercase();
        let is_last = idx + 1 == tokens.len();
        let mut best: f64 = 0.0;

        if !name_norm.is_empty() {
            if token_norm == name_norm {
                best = best.max(100.0);
            } else if name_norm.starts_with(&token_norm) {
                best = best.max(80.0);
            }
        }

        if token_norm == basename {
            best = best.max(70.0 + if is_last { 10.0 } else { 0.0 });
        } else if basename.starts_with(&token_norm) {
            best = best.max(60.0 + if is_last { 10.0 } else { 0.0 });
        }

        let ordered = score_segment_ordered(&token_norm, bookmark.path_norm);
        best = best.max(ordered);

        let fuzzy = subsequence_score(&token_norm, name_norm)
            .max(subsequence_score(&token_norm, &basename))
            .max(
                bookmark
                    .path_norm
                    .split('/')
                    .map(|seg| subsequence_score(&token_norm, seg))
                    .fold(0.0, f64::max),
            );
        best = best.max(fuzzy);

        if best <= 0.0 {
            return 0.0;
        }
        total += best;
    }

    total + compute_tag_bonus(tokens, bookmark.tags)
}

pub(crate) fn time_decay(last_visited: u64, now: u64) -> f64 {
    let elapsed = now.saturating_sub(last_visited);
    if elapsed < 3_600 {
        4.0
    } else if elapsed < 86_400 {
        2.0
    } else if elapsed < 604_800 {
        1.0
    } else if elapsed < 2_592_000 {
        0.5
    } else {
        0.2
    }
}

pub(crate) fn raw_frecency(visit_count: u32, last_visited: u64, now: u64) -> f64 {
    ((visit_count as f64) + 1.0).ln() * time_decay(last_visited, now)
}

pub(crate) fn frecency_mult(
    visit_count: Option<u32>,
    last_visited: Option<u64>,
    frecency_score: f64,
    global_max: f64,
    now: u64,
) -> f64 {
    let seed = if let (Some(v), Some(ts)) = (visit_count, last_visited) {
        raw_frecency(v, ts, now)
    } else {
        frecency_score
    };
    1.0 + normalize_to_unit(seed, global_max) * 0.25
}

pub(crate) fn compute_scope_mult(
    bookmark: &BookmarkRecordView<'_>,
    ctx: &QueryContext,
    scope: &QueryScope,
) -> f64 {
    let bm = Path::new(&bookmark.path_norm);
    let cwd = &ctx.cwd;

    match scope {
        QueryScope::Global => 1.0,
        QueryScope::BaseDir(base) => {
            if normalize_display_path(&base) == bookmark.path {
                return 1.0;
            }
            if is_under_base(bm, &base) {
                1.0
            } else {
                0.0
            }
        }
        QueryScope::Workspace(name) => {
            if bookmark.workspace.as_deref() == Some(name.as_str()) {
                1.3
            } else {
                0.0
            }
        }
        QueryScope::Child => {
            if paths_equal(cwd, bm) {
                2.5
            } else if is_under_base(bm, cwd) {
                3.0
            } else {
                0.5
            }
        }
        QueryScope::Auto => {
            if paths_equal(cwd, bm) {
                2.5
            } else if is_under_base(cwd, bm) {
                2.0
            } else if is_under_base(bm, cwd) {
                1.8
            } else if bookmark.workspace.is_some()
                && bookmark.workspace == ctx.workspace.as_deref()
                && bookmark.workspace.is_some()
            {
                1.3
            } else {
                1.0
            }
        }
    }
}

pub(crate) fn source_mult(source: BookmarkSource) -> f64 {
    match source {
        BookmarkSource::Explicit => 1.20,
        BookmarkSource::Imported => 1.05,
        BookmarkSource::Learned => 1.00,
    }
}

pub(crate) fn pin_mult(pinned: bool) -> f64 {
    if pinned { 1.50 } else { 1.00 }
}

pub(crate) fn compute_final_score(factors: ScoreFactors) -> f64 {
    factors.match_score
        * factors.frecency_mult
        * factors.scope_mult
        * factors.source_mult
        * factors.pin_mult
}

fn expand_tilde(raw: &str, home: Option<&Path>) -> String {
    let Some(home) = home else {
        return raw.to_string();
    };
    if raw == "~" {
        return home.to_string_lossy().into_owned();
    }
    if let Some(rest) = raw.strip_prefix("~/").or_else(|| raw.strip_prefix("~\\")) {
        return home.join(rest).to_string_lossy().into_owned();
    }
    raw.to_string()
}

fn validate_unc(raw: &str) -> Result<(), NormalizePathError> {
    let normalized = raw.replace('/', "\\");
    if let Some(rest) = normalized.strip_prefix("\\\\") {
        let mut parts = rest.split('\\').filter(|s| !s.is_empty());
        let server = parts.next();
        let share = parts.next();
        if server.is_none() || share.is_none() {
            return Err(NormalizePathError::InvalidUncPath);
        }
    }
    Ok(())
}

fn absolutize_path(raw: &str, cwd: &Path) -> PathBuf {
    let raw_path = Path::new(raw);
    if raw_path.is_absolute() || raw.starts_with("\\\\") || raw.starts_with("//") {
        return raw_path.to_path_buf();
    }
    normalize_join(cwd, raw_path)
}

fn normalize_join(base: &Path, rel: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in base.components() {
        out.push(component.as_os_str());
    }
    for component in rel.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn normalize_display_path(path: &Path) -> String {
    let mut s = path.to_string_lossy().replace('\\', "/");
    while s.ends_with('/') && !is_root_like(&s) {
        s.pop();
    }
    s
}

fn comparison_key(display: &str) -> String {
    if cfg!(windows) {
        display.to_ascii_lowercase()
    } else {
        display.to_string()
    }
}

fn is_root_like(s: &str) -> bool {
    s == "/" || (s.len() == 3 && s.as_bytes()[1] == b':' && s.ends_with('/'))
}

fn score_segment_ordered(token: &str, path_norm: &str) -> f64 {
    if path_norm.split('/').any(|segment| segment.contains(token)) {
        45.0
    } else {
        0.0
    }
}

fn subsequence_score(token: &str, text: &str) -> f64 {
    if token.is_empty() || text.is_empty() {
        return 0.0;
    }
    let mut text_iter = text.chars();
    for c in token.chars() {
        loop {
            match text_iter.next() {
                Some(current) if current == c => break,
                Some(_) => continue,
                None => return 0.0,
            }
        }
    }
    let density = token.len() as f64 / text.len().max(1) as f64;
    (10.0 + density * 25.0).clamp(10.0, 35.0)
}

fn compute_tag_bonus<T: AsRef<str>>(tokens: &[T], tags: &[String]) -> f64 {
    let mut bonus: f64 = 0.0;
    for token in tokens {
        let token_norm = token.as_ref();
        if tags
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case(token_norm))
        {
            bonus += 10.0;
        }
    }
    bonus.min(15.0)
}

fn normalize_to_unit(value: f64, max: f64) -> f64 {
    if max <= 0.0 {
        return 0.0;
    }
    (value / max).clamp(0.0, 1.0)
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    comparison_key(&normalize_display_path(a)) == comparison_key(&normalize_display_path(b))
}

fn is_under_base(path: &Path, base: &Path) -> bool {
    let path_s = comparison_key(&normalize_display_path(path));
    let base_s = comparison_key(&normalize_display_path(base));
    path_s == base_s || path_s.starts_with(&(base_s + "/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestRecord {
        name: Option<String>,
        name_norm: Option<String>,
        path: String,
        path_norm: String,
        tags: Vec<String>,
        workspace: Option<String>,
    }

    impl TestRecord {
        fn view(&self) -> BookmarkRecordView<'_> {
            BookmarkRecordView {
                name: self.name.as_deref(),
                name_norm: self.name_norm.as_deref(),
                path: &self.path,
                path_norm: &self.path_norm,
                tags: &self.tags,
                source: BookmarkSource::Explicit,
                pinned: false,
                visit_count: Some(10),
                last_visited: Some(1_700_000_000),
                frecency_score: 20.0,
                workspace: self.workspace.as_deref(),
            }
        }
    }

    fn record(name: Option<&str>, path: &str) -> TestRecord {
        TestRecord {
            name: name.map(str::to_string),
            name_norm: name.map(normalize_name),
            path: path.to_string(),
            path_norm: comparison_key(path),
            tags: vec!["work".to_string()],
            workspace: Some("xunyu".to_string()),
        }
    }

    #[test]
    fn normalize_path_has_display_and_key() {
        let cwd = Path::new("C:/Users/Dev");
        let got = normalize_path("C:\\Users\\Dev\\Projects", cwd, None).unwrap();
        assert_eq!(got.display(), "C:/Users/Dev/Projects");
        assert_eq!(got.key(), "c:/users/dev/projects");
    }

    #[test]
    fn tilde_expansion_windows() {
        let home = Path::new("C:/Users/dev");
        let cwd = Path::new("C:/Users/dev");
        let got = normalize_path("~\\projects\\foo", cwd, Some(home)).unwrap();
        assert_eq!(got.display(), "C:/Users/dev/projects/foo");
    }

    #[test]
    fn tilde_in_middle_not_expanded() {
        let home = Path::new("C:/Users/dev");
        let cwd = Path::new("C:/Users/dev");
        let got = normalize_path("/projects/~foo", cwd, Some(home)).unwrap();
        assert_eq!(got.display(), "C:/projects/~foo");
    }

    #[test]
    fn relative_path_resolved_against_cwd() {
        let cwd = Path::new("/home/dev/projects/foo");
        let got = normalize_path("../sibling", cwd, None).unwrap();
        assert_eq!(got.display(), "/home/dev/projects/sibling");
    }

    #[test]
    fn root_trailing_slash_preserved_unix() {
        let cwd = Path::new("/");
        let got = normalize_path("/", cwd, None).unwrap();
        assert_eq!(got.display(), "/");
    }

    #[test]
    fn unc_path_missing_share_fails() {
        let cwd = Path::new("C:/");
        let err = normalize_path("\\\\server", cwd, None).unwrap_err();
        assert_eq!(err, NormalizePathError::InvalidUncPath);
    }

    #[test]
    fn same_path_different_separator_equals() {
        let a = normalize_path("C:\\Users\\Dev", Path::new("C:/"), None).unwrap();
        let b = normalize_path("C:/Users/Dev/", Path::new("C:/"), None).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn match_name_exact_scores_100() {
        let b = record(Some("my-project"), "C:/dev/my-project");
        assert_eq!(compute_match_score(&["my-project"], &b.view()), 100.0);
    }

    #[test]
    fn tag_hit_does_not_beat_name_exact() {
        let by_name = record(Some("my-project"), "C:/dev/other");
        let by_tag = record(Some("other"), "C:/dev/other");
        assert!(
            compute_match_score(&["my-project"], &by_name.view())
                > compute_match_score(&["work"], &by_tag.view())
        );
    }

    #[test]
    fn frecency_mult_for_imported_with_null_visit_count() {
        let got = frecency_mult(None, None, 50.0, 100.0, 1_700_000_100);
        assert!(got > 1.0);
    }

    #[test]
    fn global_scope_always_one() {
        let b = record(Some("my-project"), "C:/dev/my-project");
        let ctx = QueryContext {
            cwd: PathBuf::from("C:/dev"),
            workspace: Some("xunyu".to_string()),
        };
        assert_eq!(compute_scope_mult(&b.view(), &ctx, &QueryScope::Global), 1.0);
    }

    #[test]
    fn final_score_formula_correctness() {
        let factors = ScoreFactors {
            match_score: 80.0,
            frecency_mult: 1.2,
            scope_mult: 1.5,
            source_mult: 1.2,
            pin_mult: 1.0,
        };
        assert!((compute_final_score(factors) - 172.8).abs() < 0.0001);
    }
}
