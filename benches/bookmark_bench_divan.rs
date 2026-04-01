use divan::{Bencher, black_box};
use std::path::{Path, PathBuf};

use xun::bookmark_core::{QueryContext, QueryScope};
use xun::bookmark_query::{query, BookmarkQuerySpec, QueryAction, QueryFormat};
use xun::bookmark_state::Store;
use tempfile::tempdir;

fn build_store(n: usize) -> Store {
    let mut store = Store::new();
    let cwd = Path::new("C:/work");
    for i in 0..n {
        let name = format!("client-{i:05}");
        let path = format!("C:/work/projects/{name}");
        store.set(&name, &path, cwd, None, 1_700_000_000).unwrap();
    }
    store
}

fn build_mixed_store(n: usize) -> Store {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("bookmark.json");
    let bookmarks: Vec<serde_json::Value> = (0..n)
        .map(|idx| {
            let name = format!("client-{idx:05}");
            let path = format!("C:/work/projects/{name}");
            serde_json::json!({
                "id": format!("{idx}"),
                "name": name,
                "name_norm": format!("client-{idx:05}"),
                "path": path,
                "path_norm": path.to_ascii_lowercase(),
                "source": "Explicit",
                "pinned": idx % 10 == 0,
                "tags": if idx % 2 == 0 { vec!["work"] } else { vec!["bench"] },
                "desc": "",
                "workspace": if idx % 2 == 0 { Some("xunyu") } else { Some("other") },
                "created_at": 1,
                "last_visited": 1_700_000_000 + idx as u64,
                "visit_count": 1 + (idx % 50) as u32,
                "frecency_score": 1.0 + (idx % 100) as f64
            })
        })
        .collect();
    let body = serde_json::json!({
        "schema_version": 1,
        "bookmarks": bookmarks
    });
    std::fs::write(&path, serde_json::to_vec(&body).unwrap()).unwrap();
    Store::load(&path).expect("load mixed store")
}

fn build_store_file(n: usize) -> (tempfile::TempDir, PathBuf) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("bookmark.json");
    let bookmarks: Vec<serde_json::Value> = (0..n)
        .map(|idx| {
            let name = format!("client-{idx:05}");
            let path = format!("C:/work/projects/{name}");
            serde_json::json!({
                "id": format!("{idx}"),
                "name": name,
                "name_norm": format!("client-{idx:05}"),
                "path": path,
                "path_norm": path.to_ascii_lowercase(),
                "source": "Explicit",
                "pinned": idx % 10 == 0,
                "tags": if idx % 2 == 0 { vec!["work"] } else { vec!["bench"] },
                "desc": "",
                "workspace": if idx % 2 == 0 { Some("xunyu") } else { Some("other") },
                "created_at": 1,
                "last_visited": 1_700_000_000 + idx as u64,
                "visit_count": 1 + (idx % 50) as u32,
                "frecency_score": 1.0 + (idx % 100) as f64
            })
        })
        .collect();
    let body = serde_json::json!({
        "schema_version": 1,
        "bookmarks": bookmarks
    });
    std::fs::write(&path, serde_json::to_vec(&body).unwrap()).unwrap();
    (dir, path)
}

fn build_ctx() -> QueryContext {
    QueryContext {
        cwd: PathBuf::from("C:/work/projects"),
        workspace: None,
    }
}

#[divan::bench(args = [1_000usize, 5_000, 10_000], sample_count = 30)]
fn bookmark_query_list(bencher: Bencher, size: usize) {
    let store = build_store(size);
    let ctx = build_ctx();
    let spec = BookmarkQuerySpec {
        keywords: vec!["client".to_string(), "0123".to_string()],
        action: QueryAction::List,
        limit: Some(20),
        explain: false,
        why: false,
        preview: false,
        output_fmt: QueryFormat::Text,
        ..BookmarkQuerySpec::default()
    };
    bencher.bench_local(|| {
        black_box(query(
            black_box(&spec),
            black_box(&store),
            black_box(&ctx),
            black_box(1_700_000_100),
        ))
    });
}

#[divan::bench(args = [1_000usize, 5_000, 10_000], sample_count = 30)]
fn bookmark_completion(bencher: Bencher, size: usize) {
    let store = build_store(size);
    let ctx = build_ctx();
    let spec = BookmarkQuerySpec {
        keywords: vec!["client".to_string()],
        action: QueryAction::Complete,
        limit: Some(20),
        explain: false,
        why: false,
        preview: false,
        output_fmt: QueryFormat::Tsv,
        ..BookmarkQuerySpec::default()
    };
    bencher.bench_local(|| {
        black_box(query(
            black_box(&spec),
            black_box(&store),
            black_box(&ctx),
            black_box(1_700_000_100),
        ))
    });
}

#[divan::bench(args = [1_000usize, 5_000], sample_count = 30)]
fn bookmark_query_explain(bencher: Bencher, size: usize) {
    let store = build_mixed_store(size);
    let ctx = build_ctx();
    let spec = BookmarkQuerySpec {
        keywords: vec!["client".to_string(), "0001".to_string()],
        action: QueryAction::List,
        limit: Some(20),
        explain: true,
        why: true,
        preview: false,
        output_fmt: QueryFormat::Text,
        ..BookmarkQuerySpec::default()
    };
    bencher.bench_local(|| {
        black_box(query(
            black_box(&spec),
            black_box(&store),
            black_box(&ctx),
            black_box(1_700_000_100),
        ))
    });
}

#[divan::bench(args = [1_000usize, 5_000], sample_count = 30)]
fn bookmark_query_workspace_scope(bencher: Bencher, size: usize) {
    let store = build_mixed_store(size);
    let ctx = QueryContext {
        cwd: PathBuf::from("C:/work/projects"),
        workspace: Some("xunyu".to_string()),
    };
    let spec = BookmarkQuerySpec {
        keywords: vec!["client".to_string()],
        scope: QueryScope::Workspace("xunyu".to_string()),
        action: QueryAction::List,
        limit: Some(20),
        explain: false,
        why: false,
        preview: false,
        output_fmt: QueryFormat::Tsv,
        ..BookmarkQuerySpec::default()
    };
    bencher.bench_local(|| {
        black_box(query(
            black_box(&spec),
            black_box(&store),
            black_box(&ctx),
            black_box(1_700_000_100),
        ))
    });
}

#[divan::bench(args = [1_000usize, 5_000, 20_000, 50_000], sample_count = 20)]
fn bookmark_store_load(bencher: Bencher, size: usize) {
    let (_dir, path) = build_store_file(size);
    bencher.bench_local(|| {
        let store = Store::load(black_box(&path)).expect("load store");
        black_box(store.bookmarks.len())
    });
}

fn main() {
    divan::main();
}
