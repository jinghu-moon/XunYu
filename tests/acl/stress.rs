use super::common::*;
use crate::common::*;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn next_seed(seed: &mut u32) -> u32 {
    *seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
    *seed
}

fn pick_index(seed: &mut u32, len: usize) -> usize {
    (next_seed(seed) as usize) % len.max(1)
}

fn setup_acl_stress_tree(
    env: &TestEnv,
    label: &str,
    files: usize,
    dirs: usize,
) -> (PathBuf, Vec<PathBuf>) {
    let root = env.root.join(label);
    fs::create_dir_all(&root).unwrap();
    let dir_count = dirs.max(1);
    let mut subdirs = Vec::with_capacity(dir_count);
    for i in 0..dir_count {
        let dir = root.join(format!("d{:03}", i));
        fs::create_dir_all(&dir).unwrap();
        subdirs.push(dir);
    }
    let files = files.max(1);
    let mut file_paths = Vec::with_capacity(files);
    for i in 0..files {
        let dir = &subdirs[i % dir_count];
        let file = dir.join(format!("f{:06}.txt", i));
        fs::write(&file, b"data").unwrap();
        file_paths.push(file);
    }
    (root, file_paths)
}

fn apply_random_acl_rules(
    env: &TestEnv,
    paths: &[PathBuf],
    seed: u32,
) -> BTreeMap<PathBuf, Vec<String>> {
    let principals = ["S-1-1-0", "S-1-5-32-545", "S-1-5-32-544"];
    let rights = ["Read", "Write", "Modify"];
    let mut seed = seed;
    let mut groups: BTreeMap<(usize, usize), Vec<PathBuf>> = BTreeMap::new();
    let mut expected: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();

    for path in paths {
        let rules = 1 + (next_seed(&mut seed) as usize % 3);
        let mut selected: HashSet<(usize, usize)> = HashSet::new();
        while selected.len() < rules {
            let pi = pick_index(&mut seed, principals.len());
            let ri = pick_index(&mut seed, rights.len());
            selected.insert((pi, ri));
        }
        let mut final_rights: BTreeMap<usize, usize> = BTreeMap::new();
        for (pi, ri) in selected.iter().copied() {
            let e = final_rights.entry(pi).or_insert(ri);
            if ri > *e { *e = ri; }
            groups.entry((pi, ri)).or_default().push(path.clone());
        }
        for (pi, ri) in final_rights {
            let p = principals[pi];
            let r = rights[ri];
            let mask = rights_mask_for_label(r);
            expected.entry(path.clone()).or_default().push(format!("{p}|{mask}"));
        }
    }

    let mut idx = 0usize;
    for ((pi, ri), targets) in groups {
        let list = env.root.join(format!("acl_stress_batch_{idx}.txt"));
        idx += 1;
        let content: String = targets.iter().map(|p| format!("{}
", str_path(p))).collect();
        fs::write(&list, content).unwrap();
        run_ok(acl_cmd(env).args([
            "acl", "add", "--file", &str_path(&list),
            "--principal", principals[pi],
            "--rights", rights[ri],
            "--ace-type", "Allow",
            "--inherit", "None", "-y",
        ]));
    }
    expected
}

#[test]
fn acl_stress_small_random_rules() {
    if !env_bool("XUN_TEST_ACL_STRESS", false) || !is_admin() {
        return;
    }
    let env = TestEnv::new();
    let files = env_usize("XUN_TEST_ACL_STRESS_FILES", 300);
    let dirs = env_usize("XUN_TEST_ACL_STRESS_DIRS", 12);
    let setup_start = Instant::now();
    let (root, files) = setup_acl_stress_tree(&env, "acl_stress_small", files, dirs);
    let setup_elapsed = setup_start.elapsed();
    let add_start = Instant::now();
    let _expected = apply_random_acl_rules(&env, &files, 0x1234_5678);
    let add_elapsed = add_start.elapsed();
    let start = Instant::now();
    run_ok(acl_cmd(&env).args([
        "acl", "orphans", "-p", &str_path(&root), "--action", "none",
    ]));
    let elapsed = start.elapsed();
    eprintln!(
        "perf: acl_stress_small setup_ms={} add_ms={} orphans_ms={}",
        setup_elapsed.as_millis(), add_elapsed.as_millis(), elapsed.as_millis()
    );
    assert_under_ms("acl_stress_small_orphans", elapsed, "XUN_TEST_ACL_STRESS_MAX_MS");
}

#[test]
fn acl_stress_large_random_rules() {
    if !env_bool("XUN_TEST_ACL_STRESS_LARGE", false) || !is_admin() {
        return;
    }
    let env = TestEnv::new();
    let files_count = env_usize("XUN_TEST_ACL_STRESS_LARGE_FILES", 5000);
    let dirs_count = env_usize("XUN_TEST_ACL_STRESS_LARGE_DIRS", 40);
    let total_start = Instant::now();
    let (root, files) = setup_acl_stress_tree(&env, "acl_stress_large", files_count, dirs_count);
    let setup_elapsed = total_start.elapsed();
    let pre_backup_start = Instant::now();
    let mut pre_records: Vec<(PathBuf, PathBuf, Vec<BackupEntry>)> = Vec::new();
    for (idx, path) in files.iter().enumerate() {
        let backup = env.root.join(format!("acl_stress_large_pre_{idx}.json"));
        backup_acl_to(&env, path, &backup);
        let pre_entries = backup_entries_from_file(&backup);
        pre_records.push((path.clone(), backup, pre_entries));
    }
    let pre_backup_elapsed = pre_backup_start.elapsed();
    let add_start = Instant::now();
    let expected = apply_random_acl_rules(&env, &files, 0x9E37_79B9);
    let add_elapsed = add_start.elapsed();
    let expected_principals: HashSet<&str> =
        ["S-1-1-0", "S-1-5-32-545", "S-1-5-32-544"].into_iter().collect();
    let mut post_backup_elapsed = Duration::from_millis(0);
    let mut compare_elapsed = Duration::from_millis(0);
    for (idx, (path, _backup, pre_entries)) in pre_records.iter().enumerate() {
        let post_backup = env.root.join(format!("acl_stress_large_post_{idx}.json"));
        let pb_start = Instant::now();
        backup_acl_to(&env, path, &post_backup);
        let post_entries = backup_entries_from_file(&post_backup);
        let post_keys = backup_keys_from_entries(&post_entries);
        let _ = fs::remove_file(&post_backup);
        post_backup_elapsed += pb_start.elapsed();
        let cmp_start = Instant::now();
        for entry in pre_entries {
            if entry.ace_type == "Allow" && expected_principals.contains(entry.raw_sid.as_str()) {
                continue;
            }
            let key = backup_key_for_entry(entry);
            assert!(post_keys.contains(&key), "pre ACL entry missing after write: {key}");
        }
        let expected_keys = expected.get(path).expect("missing expected keys");
        for key in expected_keys {
            let mut parts = key.split('|');
            let raw_sid = parts.next().unwrap_or_default();
            let expected_mask = normalize_rights_mask(
                parts.next().and_then(|v| v.parse::<u32>().ok()).unwrap_or(0)
            );
            let pre_mask = allow_mask_for_sid(pre_entries, raw_sid);
            if (pre_mask & expected_mask) == expected_mask { continue; }
            let post_mask = allow_mask_for_sid(&post_entries, raw_sid);
            assert!(
                (post_mask & expected_mask) == expected_mask,
                "expected ACL allow mask missing: {raw_sid}|Allow|{expected_mask} post={post_mask}"
            );
        }
        compare_elapsed += cmp_start.elapsed();
    }
    let start = Instant::now();
    run_ok(acl_cmd(&env).args([
        "acl", "orphans", "-p", &str_path(&root), "--action", "none",
    ]));
    let elapsed = start.elapsed();
    eprintln!(
        "perf: acl_stress_large setup_ms={} pre_ms={} add_ms={} post_ms={} cmp_ms={} orphans_ms={}",
        setup_elapsed.as_millis(), pre_backup_elapsed.as_millis(),
        add_elapsed.as_millis(), post_backup_elapsed.as_millis(),
        compare_elapsed.as_millis(), elapsed.as_millis()
    );
    assert_under_ms("acl_stress_large_orphans", elapsed, "XUN_TEST_ACL_STRESS_LARGE_MAX_MS");
    let restore_start = Instant::now();
    for (path, backup, _) in &pre_records {
        restore_acl(&env, path, backup);
        let _ = fs::remove_file(backup);
    }
    eprintln!(
        "perf: acl_stress_large restore_ms={} total_ms={}",
        restore_start.elapsed().as_millis(), total_start.elapsed().as_millis()
    );
}

