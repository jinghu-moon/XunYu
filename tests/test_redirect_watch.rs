#![cfg(all(windows, feature = "redirect"))]

mod common;

use common::*;
use std::fs;
use std::process::Stdio;
use std::thread;
use std::time::{Duration, Instant};

fn write_redirect_config(env: &TestEnv) {
    let cfg = r#"
{
  "redirect": {
    "profiles": {
      "default": {
        "rules": [
          { "name": "Images", "match": { "ext": ["jpg"] }, "dest": "./Images" }
        ],
        "unmatched": "skip",
        "on_conflict": "rename_new"
      }
    }
  }
}
"#;
    fs::write(env.root.join(".xun.config.json"), cfg).unwrap();
}

fn wait_for<F: Fn() -> bool>(timeout: Duration, f: F) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if f() {
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
    false
}

struct ChildGuard {
    child: std::process::Child,
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[test]
fn watch_moves_new_file_into_dest() {
    let env = TestEnv::new();
    write_redirect_config(&env);

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();

    let mut cmd = env.cmd();
    cmd.args([
        "redirect",
        src.to_str().unwrap(),
        "--watch",
        "--format",
        "tsv",
    ])
    .env("XUN_REDIRECT_WATCH_MAX_BATCHES", "1")
    .env("XUN_REDIRECT_WATCH_DEBOUNCE_MS", "200")
    .env("XUN_REDIRECT_WATCH_SETTLE_MS", "50")
    .stdout(Stdio::null())
    .stderr(Stdio::null());

    let _guard = ChildGuard {
        child: cmd.spawn().unwrap(),
    };

    thread::sleep(Duration::from_millis(200));
    fs::write(src.join("a.jpg"), "img").unwrap();

    assert!(
        wait_for(Duration::from_secs(8), || src
            .join("Images")
            .join("a.jpg")
            .exists()),
        "expected watcher to move file"
    );
}

#[test]
fn watch_retries_locked_file_until_ready() {
    let env = TestEnv::new();
    write_redirect_config(&env);

    let src = env.root.join("src");
    fs::create_dir_all(&src).unwrap();

    let locked = src.join("a.jpg");
    fs::write(&locked, "img").unwrap();
    let holder = start_lock_holder(&locked);
    assert!(wait_until_locked(&locked, Duration::from_secs(3)));

    let mut cmd = env.cmd();
    cmd.args([
        "redirect",
        src.to_str().unwrap(),
        "--watch",
        "--format",
        "tsv",
    ])
    .env("XUN_REDIRECT_WATCH_MAX_BATCHES", "1")
    .env("XUN_REDIRECT_WATCH_DEBOUNCE_MS", "200")
    .env("XUN_REDIRECT_WATCH_RETRY_MS", "200")
    .env("XUN_REDIRECT_WATCH_SETTLE_MS", "50")
    .stdout(Stdio::null())
    .stderr(Stdio::null());

    let _guard = ChildGuard {
        child: cmd.spawn().unwrap(),
    };

    thread::sleep(Duration::from_millis(400));
    drop(holder);

    assert!(
        wait_for(Duration::from_secs(10), || src
            .join("Images")
            .join("a.jpg")
            .exists()),
        "expected watcher to move file after lock released"
    );
}

#[test]
fn watch_sweeps_empty_parent_dirs_after_move() {
    let env = TestEnv::new();
    write_redirect_config(&env);

    let src = env.root.join("src");
    let nested = src.join("incoming").join("sub");
    fs::create_dir_all(&nested).unwrap();

    let mut cmd = env.cmd();
    cmd.args([
        "redirect",
        src.to_str().unwrap(),
        "--watch",
        "--format",
        "tsv",
    ])
    .env("XUN_REDIRECT_WATCH_MAX_BATCHES", "1")
    .env("XUN_REDIRECT_WATCH_DEBOUNCE_MS", "200")
    .env("XUN_REDIRECT_WATCH_SETTLE_MS", "50")
    .env("XUN_REDIRECT_WATCH_MAX_SWEEP_DIRS", "32")
    .env("XUN_REDIRECT_WATCH_SWEEP_MAX_DEPTH", "16")
    .stdout(Stdio::null())
    .stderr(Stdio::null());

    let _guard = ChildGuard {
        child: cmd.spawn().unwrap(),
    };

    thread::sleep(Duration::from_millis(200));
    fs::write(nested.join("a.jpg"), "img").unwrap();

    assert!(
        wait_for(Duration::from_secs(8), || {
            src.join("Images").join("a.jpg").exists()
                && !src.join("incoming").join("sub").exists()
                && !src.join("incoming").exists()
        }),
        "expected watcher to sweep empty dirs after move"
    );
}
