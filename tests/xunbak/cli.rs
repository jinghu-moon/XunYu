#[path = "../support/mod.rs"]
mod common;

use common::{TestEnv, run_err, run_ok, run_raw};
use serde_json::Value;
use std::fs;
use std::io::Write;

fn incompressible_bytes(len: usize) -> Vec<u8> {
    let mut state = 0x1234_5678u32;
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        out.push((state & 0xFF) as u8);
    }
    out
}

#[test]
fn cli_backup_container_creates_xunbak_file() {
    let env = TestEnv::new();
    let root = env.root.join("proj");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));

    assert!(root.join("backup.xunbak").exists());
}

#[test]
fn cli_backup_container_supports_extended_codecs() {
    let env = TestEnv::new();
    let content = "alpha alpha alpha beta beta beta gamma gamma gamma ".repeat(512);

    for (codec_arg, expected_codec, output_name) in [
        (
            "lz4",
            xun::xunbak::constants::Codec::LZ4,
            "backup-lz4.xunbak",
        ),
        (
            "deflate",
            xun::xunbak::constants::Codec::DEFLATE,
            "backup-deflate.xunbak",
        ),
        (
            "bzip2",
            xun::xunbak::constants::Codec::BZIP2,
            "backup-bzip2.xunbak",
        ),
        (
            "ppmd",
            xun::xunbak::constants::Codec::PPMD,
            "backup-ppmd.xunbak",
        ),
        (
            "lzma2",
            xun::xunbak::constants::Codec::LZMA2,
            "backup-lzma2.xunbak",
        ),
    ] {
        let root = env.root.join(format!("proj_container_codec_{codec_arg}"));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("notes.txt"), &content).unwrap();

        run_ok(env.cmd().args([
            "backup",
            "-C",
            root.to_str().unwrap(),
            "--container",
            output_name,
            "--compression",
            codec_arg,
            "-m",
            "t",
        ]));

        let container = root.join(output_name);
        let manifest = xun::xunbak::reader::ContainerReader::open(&container)
            .unwrap()
            .load_manifest()
            .unwrap();
        assert_eq!(manifest.entries.len(), 1, "codec={codec_arg}");
        assert_eq!(
            manifest.entries[0].codec, expected_codec,
            "codec={codec_arg}"
        );

        for level in ["quick", "full", "paranoid"] {
            let verify_out =
                run_ok(
                    env.cmd()
                        .args(["verify", container.to_str().unwrap(), "--level", level]),
                );
            let stdout = String::from_utf8_lossy(&verify_out.stdout);
            assert!(
                stdout.contains("passed: true"),
                "codec={codec_arg} level={level}"
            );
        }
    }
}

#[test]
fn cli_backup_create_subcommand_format_xunbak_creates_file() {
    let env = TestEnv::new();
    let root = env.root.join("proj");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "xunbak",
        "-o",
        "created.xunbak",
        "--compression",
        "none",
    ]));

    assert!(root.join("created.xunbak").exists());
}

#[test]
fn cli_backup_create_subcommand_format_xunbak_creates_container() {
    let env = TestEnv::new();
    let root = env.root.join("proj_create_subcommand");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "xunbak",
        "-o",
        "artifact.xunbak",
    ]));

    assert!(root.join("artifact.xunbak").exists());
}

#[test]
fn cli_backup_create_subcommand_format_xunbak_defaults_to_zstd() {
    let env = TestEnv::new();
    let root = env.root.join("proj_create_default_codec");
    fs::create_dir_all(&root).unwrap();
    let content = "alpha alpha alpha beta beta beta gamma gamma gamma ".repeat(512);
    fs::write(root.join("notes.txt"), &content).unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "notes.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "xunbak",
        "-o",
        "default-codec.xunbak",
    ]));

    let manifest = xun::xunbak::reader::ContainerReader::open(&root.join("default-codec.xunbak"))
        .unwrap()
        .load_manifest()
        .unwrap();
    assert_eq!(manifest.entries.len(), 1);
    assert_eq!(
        manifest.entries[0].codec,
        xun::xunbak::constants::Codec::ZSTD
    );
}

#[test]
fn cli_backup_create_subcommand_format_xunbak_no_compress_forces_none() {
    let env = TestEnv::new();
    let root = env.root.join("proj_create_no_compress");
    fs::create_dir_all(&root).unwrap();
    let content = "alpha alpha alpha beta beta beta gamma gamma gamma ".repeat(512);
    fs::write(root.join("notes.txt"), &content).unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "notes.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "xunbak",
        "-o",
        "no-compress.xunbak",
        "--no-compress",
    ]));

    let manifest = xun::xunbak::reader::ContainerReader::open(&root.join("no-compress.xunbak"))
        .unwrap()
        .load_manifest()
        .unwrap();
    assert_eq!(manifest.entries.len(), 1);
    assert_eq!(
        manifest.entries[0].codec,
        xun::xunbak::constants::Codec::NONE
    );
}

#[test]
fn cli_backup_create_subcommand_format_xunbak_rejects_invalid_compression_with_fix_hint() {
    let env = TestEnv::new();
    let root = env.root.join("proj_create_invalid_codec");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    let out = run_err(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "xunbak",
        "-o",
        "artifact.xunbak",
        "--compression",
        "brotli",
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("invalid compression argument"));
    assert!(stderr.contains("Fix:"));
}

#[test]
fn cli_backup_create_help_lists_current_xunbak_compression_profiles() {
    let env = TestEnv::new();
    let out = run_raw(env.cmd().args(["backup", "create", "--help"]));
    assert!(out.status.success());
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(text.contains("lzma2"));
    assert!(text.contains("deflate"));
    assert!(text.contains("bzip2"));
    assert!(text.contains("ppmd"));
    assert!(text.contains("auto"));
    assert!(text.contains("zstd (default)"));
    assert!(text.contains("ppmd (text-heavy)"));
    assert!(text.contains("lzma2 (slow"));
    assert!(text.contains("archive)"));
    assert!(text.contains("deflate/bzip2 (compatibility)"));
}

#[test]
fn cli_backup_create_subcommand_format_xunbak_supports_extended_codecs() {
    let env = TestEnv::new();
    let content = "alpha alpha alpha beta beta beta gamma gamma gamma ".repeat(512);

    for (codec_arg, expected_codec, output_name) in [
        (
            "lz4",
            xun::xunbak::constants::Codec::LZ4,
            "artifact-lz4.xunbak",
        ),
        (
            "deflate",
            xun::xunbak::constants::Codec::DEFLATE,
            "artifact-deflate.xunbak",
        ),
        (
            "bzip2",
            xun::xunbak::constants::Codec::BZIP2,
            "artifact-bzip2.xunbak",
        ),
        (
            "ppmd",
            xun::xunbak::constants::Codec::PPMD,
            "artifact-ppmd.xunbak",
        ),
        (
            "lzma2",
            xun::xunbak::constants::Codec::LZMA2,
            "artifact-lzma2.xunbak",
        ),
    ] {
        let root = env.root.join(format!("proj_create_codec_{codec_arg}"));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("notes.txt"), &content).unwrap();
        let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "notes.txt" ],
  "exclude": []
}"#;
        fs::write(root.join(".xun-bak.json"), cfg).unwrap();

        run_ok(env.cmd().args([
            "backup",
            "create",
            "-C",
            root.to_str().unwrap(),
            "--format",
            "xunbak",
            "-o",
            output_name,
            "--compression",
            codec_arg,
        ]));

        let container = root.join(output_name);
        let manifest = xun::xunbak::reader::ContainerReader::open(&container)
            .unwrap()
            .load_manifest()
            .unwrap();
        assert_eq!(manifest.entries.len(), 1, "codec={codec_arg}");
        assert_eq!(
            manifest.entries[0].codec, expected_codec,
            "codec={codec_arg}"
        );

        let out_dir = root.join(format!("restored-{codec_arg}"));
        run_ok(env.cmd().args([
            "backup",
            "restore",
            container.to_str().unwrap(),
            "--to",
            out_dir.to_str().unwrap(),
            "-C",
            root.to_str().unwrap(),
            "-y",
        ]));
        assert_eq!(
            fs::read_to_string(out_dir.join("notes.txt")).unwrap(),
            content,
            "codec={codec_arg}"
        );
    }
}

#[test]
fn cli_backup_create_subcommand_format_xunbak_auto_prefers_ppmd_for_text() {
    let env = TestEnv::new();
    let root = env.root.join("proj_create_auto_text");
    fs::create_dir_all(&root).unwrap();
    let content = "alpha alpha alpha beta beta beta gamma gamma gamma ".repeat(512);
    fs::write(root.join("notes.txt"), &content).unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "notes.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "xunbak",
        "-o",
        "auto-text.xunbak",
        "--compression",
        "auto",
    ]));

    let manifest = xun::xunbak::reader::ContainerReader::open(&root.join("auto-text.xunbak"))
        .unwrap()
        .load_manifest()
        .unwrap();
    assert_eq!(manifest.entries.len(), 1);
    assert_eq!(
        manifest.entries[0].codec,
        xun::xunbak::constants::Codec::PPMD
    );
}

#[test]
fn cli_backup_create_subcommand_format_xunbak_auto_falls_back_to_none_for_incompressible_binary() {
    let env = TestEnv::new();
    let root = env.root.join("proj_create_auto_binary");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("data.bin"), incompressible_bytes(8192)).unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "data.bin" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "xunbak",
        "-o",
        "auto-binary.xunbak",
        "--compression",
        "auto",
    ]));

    let manifest = xun::xunbak::reader::ContainerReader::open(&root.join("auto-binary.xunbak"))
        .unwrap()
        .load_manifest()
        .unwrap();
    assert_eq!(manifest.entries.len(), 1);
    assert_eq!(
        manifest.entries[0].codec,
        xun::xunbak::constants::Codec::NONE
    );
}

#[test]
fn cli_backup_create_subcommand_format_xunbak_explicit_codecs_fall_back_to_none_for_incompressible_binary()
 {
    let env = TestEnv::new();

    for codec_arg in ["lz4", "deflate", "bzip2", "ppmd", "lzma2"] {
        let root = env.root.join(format!("proj_create_fallback_{codec_arg}"));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("data.bin"), incompressible_bytes(8192)).unwrap();
        let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "data.bin" ],
  "exclude": []
}"#;
        fs::write(root.join(".xun-bak.json"), cfg).unwrap();

        let output_name = format!("{codec_arg}.xunbak");
        run_ok(env.cmd().args([
            "backup",
            "create",
            "-C",
            root.to_str().unwrap(),
            "--format",
            "xunbak",
            "-o",
            &output_name,
            "--compression",
            codec_arg,
        ]));

        let manifest = xun::xunbak::reader::ContainerReader::open(&root.join(&output_name))
            .unwrap()
            .load_manifest()
            .unwrap();
        assert_eq!(manifest.entries.len(), 1, "codec={codec_arg}");
        assert_eq!(
            manifest.entries[0].codec,
            xun::xunbak::constants::Codec::NONE,
            "codec={codec_arg}"
        );
    }
}

#[test]
fn cli_backup_create_subcommand_format_xunbak_json_respects_config_selection() {
    let env = TestEnv::new();
    let root = env.root.join("proj_create_json_selection");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src").join("main.rs"), "fn main() {}").unwrap();
    fs::write(root.join("README.md"), "readme").unwrap();
    fs::write(root.join("skip.log"), "skip").unwrap();

    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "src", "README.md" ],
  "exclude": [ "*.log" ]
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    let out = run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "xunbak",
        "-o",
        "artifact.xunbak",
        "--json",
    ]));
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["action"], "create");
    assert_eq!(value["format"], "xunbak");
    assert_eq!(value["selected"], 2);
    assert_eq!(value["written"], 2);

    let out_dir = root.join("restore_json_selection");
    run_ok(env.cmd().args([
        "backup",
        "restore",
        root.join("artifact.xunbak").to_str().unwrap(),
        "--to",
        out_dir.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));
    assert!(out_dir.join("src").join("main.rs").exists());
    assert!(out_dir.join("README.md").exists());
    assert!(!out_dir.join("skip.log").exists());
}

#[test]
fn cli_backup_create_subcommand_format_xunbak_matches_legacy_container_output() {
    let env = TestEnv::new();
    let root = env.root.join("proj_create_equivalent");
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    fs::write(root.join("nested").join("b.txt"), "bbb").unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt", "nested" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "legacy.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));
    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "xunbak",
        "-o",
        "new.xunbak",
        "--compression",
        "none",
    ]));

    let legacy_manifest = xun::xunbak::reader::ContainerReader::open(&root.join("legacy.xunbak"))
        .unwrap()
        .load_manifest()
        .unwrap();
    let new_manifest = xun::xunbak::reader::ContainerReader::open(&root.join("new.xunbak"))
        .unwrap()
        .load_manifest()
        .unwrap();

    let legacy_entries: Vec<(&str, [u8; 32])> = legacy_manifest
        .entries
        .iter()
        .filter(|entry| entry.path == "a.txt" || entry.path == "nested/b.txt")
        .map(|entry| (entry.path.as_str(), entry.content_hash))
        .collect();
    let new_entries: Vec<(&str, [u8; 32])> = new_manifest
        .entries
        .iter()
        .filter(|entry| entry.path == "a.txt" || entry.path == "nested/b.txt")
        .map(|entry| (entry.path.as_str(), entry.content_hash))
        .collect();
    assert_eq!(legacy_entries, new_entries);
}

#[test]
fn cli_backup_create_subcommand_format_xunbak_matches_legacy_zstd_output() {
    let env = TestEnv::new();
    let root = env.root.join("proj_create_equivalent_zstd");
    fs::create_dir_all(root.join("nested")).unwrap();
    let content = "alpha alpha alpha beta beta beta gamma gamma gamma ".repeat(512);
    fs::write(root.join("a.txt"), &content).unwrap();
    fs::write(root.join("nested").join("b.txt"), &content).unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt", "nested" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "legacy-zstd.xunbak",
        "--compression",
        "zstd",
        "-m",
        "t",
    ]));
    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "xunbak",
        "-o",
        "new-zstd.xunbak",
        "--compression",
        "zstd",
    ]));

    let legacy_reader =
        xun::xunbak::reader::ContainerReader::open(&root.join("legacy-zstd.xunbak")).unwrap();
    let new_reader =
        xun::xunbak::reader::ContainerReader::open(&root.join("new-zstd.xunbak")).unwrap();
    let legacy_manifest = legacy_reader.load_manifest().unwrap();
    let new_manifest = new_reader.load_manifest().unwrap();

    let legacy_entries: Vec<(&str, xun::xunbak::constants::Codec, [u8; 32])> = legacy_manifest
        .entries
        .iter()
        .filter(|entry| entry.path == "a.txt" || entry.path == "nested/b.txt")
        .map(|entry| (entry.path.as_str(), entry.codec, entry.content_hash))
        .collect();
    let new_entries: Vec<(&str, xun::xunbak::constants::Codec, [u8; 32])> = new_manifest
        .entries
        .iter()
        .filter(|entry| entry.path == "a.txt" || entry.path == "nested/b.txt")
        .map(|entry| (entry.path.as_str(), entry.codec, entry.content_hash))
        .collect();

    assert_eq!(legacy_entries, new_entries);
    assert_eq!(legacy_reader.header.header.min_reader_version, 1);
    assert_eq!(new_reader.header.header.min_reader_version, 1);
}

#[test]
fn cli_backup_create_subcommand_format_xunbak_split_creates_numbered_volumes_without_temp_artifacts()
 {
    let env = TestEnv::new();
    let root = env.root.join("proj_split_create");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "a".repeat(800)).unwrap();
    fs::write(root.join("b.txt"), "b".repeat(800)).unwrap();
    fs::write(root.join("c.txt"), "c".repeat(800)).unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt", "b.txt", "c.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "xunbak",
        "-o",
        "split.xunbak",
        "--compression",
        "none",
        "--split-size",
        "1900",
    ]));

    assert!(root.join("split.xunbak.001").exists());
    assert!(root.join("split.xunbak.002").exists());
    let temp_staged = fs::read_dir(&root)
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .any(|name| name.contains("tmp-split-xunbak") || name.contains("tmp.xunbak"));
    assert!(
        !temp_staged,
        "temporary split staging files should be cleaned up"
    );
}

#[test]
fn cli_backup_create_subcommand_format_xunbak_split_supports_extended_codecs() {
    let env = TestEnv::new();
    let content = "alpha alpha alpha beta beta beta gamma gamma gamma ".repeat(512);

    for (codec_arg, expected_codec, output_name) in [
        (
            "lz4",
            xun::xunbak::constants::Codec::LZ4,
            "split-lz4.xunbak",
        ),
        (
            "deflate",
            xun::xunbak::constants::Codec::DEFLATE,
            "split-deflate.xunbak",
        ),
        (
            "bzip2",
            xun::xunbak::constants::Codec::BZIP2,
            "split-bzip2.xunbak",
        ),
        (
            "ppmd",
            xun::xunbak::constants::Codec::PPMD,
            "split-ppmd.xunbak",
        ),
        (
            "lzma2",
            xun::xunbak::constants::Codec::LZMA2,
            "split-lzma2.xunbak",
        ),
    ] {
        let root = env
            .root
            .join(format!("proj_split_create_codec_{codec_arg}"));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("notes.txt"), &content).unwrap();
        let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "notes.txt" ],
  "exclude": []
}"#;
        fs::write(root.join(".xun-bak.json"), cfg).unwrap();

        run_ok(env.cmd().args([
            "backup",
            "create",
            "-C",
            root.to_str().unwrap(),
            "--format",
            "xunbak",
            "-o",
            output_name,
            "--compression",
            codec_arg,
            "--split-size",
            "1900",
        ]));

        let container = root.join(output_name);
        assert!(root.join(format!("{output_name}.001")).exists());
        let manifest = xun::xunbak::reader::ContainerReader::open(&container)
            .unwrap()
            .load_manifest()
            .unwrap();
        assert_eq!(manifest.entries.len(), 1, "codec={codec_arg}");
        assert_eq!(
            manifest.entries[0].codec, expected_codec,
            "codec={codec_arg}"
        );

        let out_dir = root.join(format!("restored-{codec_arg}"));
        run_ok(env.cmd().args([
            "backup",
            "restore",
            container.to_str().unwrap(),
            "--to",
            out_dir.to_str().unwrap(),
            "-C",
            root.to_str().unwrap(),
            "-y",
        ]));
        assert_eq!(
            fs::read_to_string(out_dir.join("notes.txt")).unwrap(),
            content,
            "codec={codec_arg}"
        );
    }
}

#[test]
fn cli_backup_container_second_run_updates_incrementally() {
    let env = TestEnv::new();
    let root = env.root.join("proj");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));
    let first_size = fs::metadata(root.join("backup.xunbak")).unwrap().len();

    std::thread::sleep(std::time::Duration::from_millis(20));
    fs::write(root.join("b.txt"), "bbb").unwrap();
    let out = run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t2",
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Updated xunbak:"));
    let second_size = fs::metadata(root.join("backup.xunbak")).unwrap().len();
    assert!(second_size > first_size);
}

#[test]
fn cli_backup_container_split_second_run_updates_incrementally() {
    let env = TestEnv::new();
    let root = env.root.join("proj_split_update");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "a".repeat(800)).unwrap();
    fs::write(root.join("b.txt"), "b".repeat(800)).unwrap();
    fs::write(root.join("c.txt"), "c".repeat(800)).unwrap();
    fs::write(root.join("d.txt"), "d".repeat(800)).unwrap();
    fs::write(root.join("e.txt"), "e".repeat(800)).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "--split-size",
        "2800",
        "-m",
        "t",
    ]));

    std::thread::sleep(std::time::Duration::from_millis(20));
    fs::write(root.join("f.txt"), "f".repeat(800)).unwrap();
    let out = run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "--split-size",
        "2800",
        "-m",
        "t2",
    ]));

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Updated xunbak:"));
    assert!(root.join("backup.xunbak.001").exists());
    assert!(root.join("backup.xunbak.002").exists());
    let temp_staged = fs::read_dir(&root)
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .any(|name| name.contains("tmp-split-xunbak") || name.contains("tmp.xunbak"));
    assert!(
        !temp_staged,
        "temporary split staging files should be cleaned up"
    );

    let out_dir = root.join("restore_after_split_update");
    run_ok(env.cmd().args([
        "backup",
        "restore",
        root.join("backup.xunbak").to_str().unwrap(),
        "--to",
        out_dir.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));
    assert_eq!(
        fs::read_to_string(out_dir.join("f.txt")).unwrap(),
        "f".repeat(800)
    );
}

#[test]
fn cli_backup_convert_corrupted_xunbak_fails_preflight_verify_by_default() {
    let env = TestEnv::new();
    let root = env.root.join("proj_corrupted_convert");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();

    let container = root.join("backup.xunbak");
    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));

    let reader = xun::xunbak::reader::ContainerReader::open(&container).unwrap();
    let corrupt_offset =
        reader.checkpoint.manifest_offset + xun::xunbak::constants::RECORD_PREFIX_SIZE as u64 + 8;
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&container)
        .unwrap();
    use std::io::{Read, Seek, SeekFrom, Write};
    file.seek(SeekFrom::Start(corrupt_offset)).unwrap();
    let mut byte = [0u8; 1];
    file.read_exact(&mut byte).unwrap();
    byte[0] ^= 0x5A;
    file.seek(SeekFrom::Start(corrupt_offset)).unwrap();
    file.write_all(&byte).unwrap();

    let out = run_err(env.cmd().args([
        "backup",
        "convert",
        container.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        root.join("out.zip").to_str().unwrap(),
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("source verify failed"));
}

#[test]
fn cli_backup_convert_corrupted_xunbak_json_reports_preflight_failed() {
    let env = TestEnv::new();
    let root = env.root.join("proj_corrupted_convert_json");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();

    let container = root.join("backup.xunbak");
    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));

    let reader = xun::xunbak::reader::ContainerReader::open(&container).unwrap();
    let corrupt_offset =
        reader.checkpoint.manifest_offset + xun::xunbak::constants::RECORD_PREFIX_SIZE as u64 + 8;
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&container)
        .unwrap();
    use std::io::{Read, Seek, SeekFrom, Write};
    file.seek(SeekFrom::Start(corrupt_offset)).unwrap();
    let mut byte = [0u8; 1];
    file.read_exact(&mut byte).unwrap();
    byte[0] ^= 0x5A;
    file.seek(SeekFrom::Start(corrupt_offset)).unwrap();
    file.write_all(&byte).unwrap();

    let out = env
        .cmd()
        .args([
            "backup",
            "convert",
            container.to_str().unwrap(),
            "--format",
            "zip",
            "-o",
            root.join("out.zip").to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["status"], "preflight_failed");
}

#[test]
fn cli_backup_convert_unselected_corrupted_blob_fails_with_verify_source_full() {
    let env = TestEnv::new();
    let root = env.root.join("proj_verify_source_full");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    fs::write(root.join("b.txt"), "bbb").unwrap();

    let container = root.join("backup.xunbak");
    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));

    let reader = xun::xunbak::reader::ContainerReader::open(&container).unwrap();
    let manifest = reader.load_manifest().unwrap();
    let bad = manifest
        .entries
        .iter()
        .find(|entry| entry.path == "b.txt")
        .unwrap();
    let corrupt_offset = bad.blob_offset
        + xun::xunbak::constants::RECORD_PREFIX_SIZE as u64
        + xun::xunbak::constants::BLOB_HEADER_SIZE as u64;
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&container)
        .unwrap();
    use std::io::{Read, Seek, SeekFrom, Write};
    file.seek(SeekFrom::Start(corrupt_offset)).unwrap();
    let mut byte = [0u8; 1];
    file.read_exact(&mut byte).unwrap();
    byte[0] ^= 0x5A;
    file.seek(SeekFrom::Start(corrupt_offset)).unwrap();
    file.write_all(&byte).unwrap();

    let out = run_err(env.cmd().args([
        "backup",
        "convert",
        container.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        root.join("out.zip").to_str().unwrap(),
        "--file",
        "a.txt",
        "--verify-source",
        "full",
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("source verify failed"));
}

#[test]
fn cli_backup_convert_unselected_corrupted_blob_succeeds_with_verify_source_off() {
    let env = TestEnv::new();
    let root = env.root.join("proj_verify_source_off");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    fs::write(root.join("b.txt"), "bbb").unwrap();

    let container = root.join("backup.xunbak");
    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));

    let reader = xun::xunbak::reader::ContainerReader::open(&container).unwrap();
    let manifest = reader.load_manifest().unwrap();
    let bad = manifest
        .entries
        .iter()
        .find(|entry| entry.path == "b.txt")
        .unwrap();
    let corrupt_offset = bad.blob_offset
        + xun::xunbak::constants::RECORD_PREFIX_SIZE as u64
        + xun::xunbak::constants::BLOB_HEADER_SIZE as u64;
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&container)
        .unwrap();
    use std::io::{Read, Seek, SeekFrom, Write};
    file.seek(SeekFrom::Start(corrupt_offset)).unwrap();
    let mut byte = [0u8; 1];
    file.read_exact(&mut byte).unwrap();
    byte[0] ^= 0x5A;
    file.seek(SeekFrom::Start(corrupt_offset)).unwrap();
    file.write_all(&byte).unwrap();

    let output = root.join("out.zip");
    run_ok(env.cmd().args([
        "backup",
        "convert",
        container.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        output.to_str().unwrap(),
        "--file",
        "a.txt",
        "--verify-source",
        "off",
    ]));
    assert!(output.exists());
}

#[test]
fn cli_backup_convert_selected_corrupted_blob_still_fails_with_verify_source_off() {
    let env = TestEnv::new();
    let root = env.root.join("proj_verify_selected_blob_off");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    fs::write(root.join("b.txt"), "bbb").unwrap();

    let container = root.join("backup.xunbak");
    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));

    let reader = xun::xunbak::reader::ContainerReader::open(&container).unwrap();
    let manifest = reader.load_manifest().unwrap();
    let bad = manifest
        .entries
        .iter()
        .find(|entry| entry.path == "b.txt")
        .unwrap();
    let corrupt_offset = bad.blob_offset
        + xun::xunbak::constants::RECORD_PREFIX_SIZE as u64
        + xun::xunbak::constants::BLOB_HEADER_SIZE as u64;
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&container)
        .unwrap();
    use std::io::{Read, Seek, SeekFrom, Write};
    file.seek(SeekFrom::Start(corrupt_offset)).unwrap();
    let mut byte = [0u8; 1];
    file.read_exact(&mut byte).unwrap();
    byte[0] ^= 0x5A;
    file.seek(SeekFrom::Start(corrupt_offset)).unwrap();
    file.write_all(&byte).unwrap();

    let out = run_err(env.cmd().args([
        "backup",
        "convert",
        container.to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        root.join("out.zip").to_str().unwrap(),
        "--file",
        "b.txt",
        "--verify-source",
        "off",
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Error:") || stderr.contains("hash"));
}

#[test]
fn cli_backup_convert_xunbak_output_verify_on_detects_corrupted_postwrite_output() {
    let env = TestEnv::new();
    let root = env.root.join("proj_verify_output_xunbak_on");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();

    let out = run_err(
        env.cmd()
            .env("XUN_TEST_CORRUPT_OUTPUT_AFTER_WRITE", "truncate")
            .args([
                "backup",
                "convert",
                root.to_str().unwrap(),
                "--format",
                "xunbak",
                "-o",
                root.join("out.xunbak").to_str().unwrap(),
            ]),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("output verify failed"));
}

#[test]
fn cli_backup_convert_xunbak_output_verify_on_json_reports_verify_failed() {
    let env = TestEnv::new();
    let root = env.root.join("proj_verify_output_xunbak_json");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();

    let out = env
        .cmd()
        .env("XUN_TEST_CORRUPT_OUTPUT_AFTER_WRITE", "truncate")
        .args([
            "backup",
            "convert",
            root.to_str().unwrap(),
            "--format",
            "xunbak",
            "-o",
            root.join("out.xunbak").to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["status"], "verify_failed");
}

#[test]
fn cli_backup_convert_xunbak_output_verify_off_skips_corrupted_postwrite_output_check() {
    let env = TestEnv::new();
    let root = env.root.join("proj_verify_output_xunbak_off");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();

    let output = root.join("out.xunbak");
    run_ok(
        env.cmd()
            .env("XUN_TEST_CORRUPT_OUTPUT_AFTER_WRITE", "truncate")
            .args([
                "backup",
                "convert",
                root.to_str().unwrap(),
                "--format",
                "xunbak",
                "-o",
                output.to_str().unwrap(),
                "--verify-output",
                "off",
            ]),
    );
    assert!(output.exists());
}

#[test]
fn cli_backup_container_split_update_failure_restores_original_volumes() {
    let env = TestEnv::new();
    let root = env.root.join("proj_split_update_rollback");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "a".repeat(800)).unwrap();
    fs::write(root.join("b.txt"), "b".repeat(800)).unwrap();
    fs::write(root.join("c.txt"), "c".repeat(800)).unwrap();
    fs::write(root.join("d.txt"), "d".repeat(800)).unwrap();
    fs::write(root.join("e.txt"), "e".repeat(800)).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "--split-size",
        "2800",
        "-m",
        "t",
    ]));

    let before_001 = fs::read(root.join("backup.xunbak.001")).unwrap();
    let before_002 = fs::read(root.join("backup.xunbak.002")).unwrap();

    fs::write(root.join("f.txt"), "f".repeat(800)).unwrap();
    let out = common::run_err(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "--split-size",
        "1900",
        "-m",
        "t2",
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("SplitSizeMismatch") || stderr.contains("split size mismatch"));

    assert_eq!(
        fs::read(root.join("backup.xunbak.001")).unwrap(),
        before_001
    );
    assert_eq!(
        fs::read(root.join("backup.xunbak.002")).unwrap(),
        before_002
    );

    let out_dir = root.join("restore_after_failed_update");
    run_ok(env.cmd().args([
        "backup",
        "restore",
        root.join("backup.xunbak").to_str().unwrap(),
        "--to",
        out_dir.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));
    assert!(!out_dir.join("f.txt").exists());
}

#[test]
fn cli_restore_container_restores_to_target_dir() {
    let env = TestEnv::new();
    let root = env.root.join("proj");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));

    let out_dir = root.join("restore");
    run_ok(env.cmd().args([
        "backup",
        "restore",
        root.join("backup.xunbak").to_str().unwrap(),
        "--to",
        out_dir.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));

    assert_eq!(fs::read_to_string(out_dir.join("a.txt")).unwrap(), "aaa");
}

#[test]
fn cli_backup_restore_subcommand_restores_xunbak_to_target_dir() {
    let env = TestEnv::new();
    let root = env.root.join("proj_restore_subcommand");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "xunbak",
        "-o",
        "artifact.xunbak",
    ]));

    let out_dir = root.join("restore");
    run_ok(env.cmd().args([
        "backup",
        "restore",
        root.join("artifact.xunbak").to_str().unwrap(),
        "--to",
        out_dir.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));

    assert_eq!(fs::read_to_string(out_dir.join("a.txt")).unwrap(), "aaa");
}

#[test]
fn cli_backup_restore_xunbak_dry_run_does_not_write_files() {
    let env = TestEnv::new();
    let root = env.root.join("proj_restore_dry_run");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "xunbak",
        "-o",
        "artifact.xunbak",
    ]));

    let out_dir = root.join("restore-dry-run");
    let out = run_ok(env.cmd().args([
        "backup",
        "restore",
        root.join("artifact.xunbak").to_str().unwrap(),
        "--to",
        out_dir.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "--dry-run",
        "-y",
    ]));

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("DRY RUN: would restore a.txt"),
        "stderr should contain dry-run restore preview, got: {stderr}"
    );
    assert!(
        !out_dir.exists(),
        "dry-run should not create restore target directory"
    );
}

#[test]
fn cli_backup_restore_xunbak_dry_run_json_reports_planned_count() {
    let env = TestEnv::new();
    let root = env.root.join("proj_restore_dry_run_json");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    fs::write(root.join("nested").join("b.txt"), "bbb").unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt", "nested" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "xunbak",
        "-o",
        "artifact.xunbak",
    ]));

    let out_dir = root.join("restore-dry-run-json");
    let out = run_ok(env.cmd().args([
        "backup",
        "restore",
        root.join("artifact.xunbak").to_str().unwrap(),
        "--to",
        out_dir.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "--dry-run",
        "--json",
        "-y",
    ]));

    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["action"], "preview");
    assert_eq!(value["mode"], "all");
    assert_eq!(value["dry_run"], true);
    assert_eq!(value["restored"], 2);
    assert_eq!(value["failed"], 0);
    assert!(!out_dir.exists());
}

#[test]
fn cli_restore_container_restores_single_file() {
    let env = TestEnv::new();
    let root = env.root.join("proj");
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    fs::write(root.join("nested").join("b.txt"), "bbb").unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));

    let out_dir = root.join("restore");
    run_ok(env.cmd().args([
        "backup",
        "restore",
        root.join("backup.xunbak").to_str().unwrap(),
        "--file",
        "nested/b.txt",
        "--to",
        out_dir.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));

    assert!(!out_dir.join("a.txt").exists());
    assert_eq!(
        fs::read_to_string(out_dir.join("nested").join("b.txt")).unwrap(),
        "bbb"
    );
}

#[test]
fn cli_restore_container_restores_glob_selection() {
    let env = TestEnv::new();
    let root = env.root.join("proj");
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(root.join("a.rs"), "aaa").unwrap();
    fs::write(root.join("nested").join("b.rs"), "bbb").unwrap();
    fs::write(root.join("c.txt"), "ccc").unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));

    let out_dir = root.join("restore");
    run_ok(env.cmd().args([
        "backup",
        "restore",
        root.join("backup.xunbak").to_str().unwrap(),
        "--glob",
        "nested/**/*.rs",
        "--to",
        out_dir.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));

    assert!(!out_dir.join("a.rs").exists());
    assert_eq!(
        fs::read_to_string(out_dir.join("nested").join("b.rs")).unwrap(),
        "bbb"
    );
    assert!(!out_dir.join("c.txt").exists());
}

#[test]
fn cli_restore_container_selective_modes_support_extended_codecs() {
    let env = TestEnv::new();
    let content = "alpha alpha alpha beta beta beta gamma gamma gamma ".repeat(512);

    for (codec_arg, output_name) in [
        ("lz4", "selective-lz4.xunbak"),
        ("deflate", "selective-deflate.xunbak"),
        ("bzip2", "selective-bzip2.xunbak"),
        ("ppmd", "selective-ppmd.xunbak"),
        ("lzma2", "selective-lzma2.xunbak"),
    ] {
        let root = env.root.join(format!("proj_selective_codec_{codec_arg}"));
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("a.txt"), &content).unwrap();
        fs::write(root.join("nested").join("b.txt"), &content).unwrap();
        let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "a.txt", "nested" ],
  "exclude": []
}"#;
        fs::write(root.join(".xun-bak.json"), cfg).unwrap();

        run_ok(env.cmd().args([
            "backup",
            "create",
            "-C",
            root.to_str().unwrap(),
            "--format",
            "xunbak",
            "-o",
            output_name,
            "--compression",
            codec_arg,
        ]));

        let container = root.join(output_name);

        let file_out = root.join("restore-file");
        run_ok(env.cmd().args([
            "backup",
            "restore",
            container.to_str().unwrap(),
            "--file",
            "nested/b.txt",
            "--to",
            file_out.to_str().unwrap(),
            "-C",
            root.to_str().unwrap(),
            "-y",
        ]));
        assert!(!file_out.join("a.txt").exists(), "codec={codec_arg} file");
        assert_eq!(
            fs::read_to_string(file_out.join("nested").join("b.txt")).unwrap(),
            content,
            "codec={codec_arg} file"
        );

        let glob_out = root.join("restore-glob");
        run_ok(env.cmd().args([
            "backup",
            "restore",
            container.to_str().unwrap(),
            "--glob",
            "nested/**/*.txt",
            "--to",
            glob_out.to_str().unwrap(),
            "-C",
            root.to_str().unwrap(),
            "-y",
        ]));
        assert!(!glob_out.join("a.txt").exists(), "codec={codec_arg} glob");
        assert_eq!(
            fs::read_to_string(glob_out.join("nested").join("b.txt")).unwrap(),
            content,
            "codec={codec_arg} glob"
        );
    }
}

#[test]
fn cli_backup_convert_xunbak_to_dir_output_writes_selected_file() {
    let env = TestEnv::new();
    let root = env.root.join("proj");
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    fs::write(root.join("nested").join("b.txt"), "bbb").unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));

    let out_dir = root.join("converted");
    run_ok(env.cmd().args([
        "backup",
        "convert",
        root.join("backup.xunbak").to_str().unwrap(),
        "--format",
        "dir",
        "-o",
        out_dir.to_str().unwrap(),
        "--file",
        "nested/b.txt",
    ]));

    assert!(!out_dir.join("a.txt").exists());
    assert_eq!(
        fs::read_to_string(out_dir.join("nested").join("b.txt")).unwrap(),
        "bbb"
    );
}

#[test]
fn cli_backup_convert_xunbak_to_zip_output_writes_selected_file() {
    let env = TestEnv::new();
    let root = env.root.join("proj");
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    fs::write(root.join("nested").join("b.txt"), "bbb").unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));

    let zip_path = root.join("converted.zip");
    run_ok(env.cmd().args([
        "backup",
        "convert",
        root.join("backup.xunbak").to_str().unwrap(),
        "--format",
        "zip",
        "-o",
        zip_path.to_str().unwrap(),
        "--file",
        "nested/b.txt",
    ]));

    let file = fs::File::open(&zip_path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    assert!(archive.by_name("a.txt").is_err());
    let mut entry = archive.by_name("nested/b.txt").unwrap();
    let mut content = String::new();
    std::io::Read::read_to_string(&mut entry, &mut content).unwrap();
    assert_eq!(content, "bbb");
}

#[test]
fn cli_backup_convert_xunbak_to_7z_output_writes_selected_file() {
    let env = TestEnv::new();
    let root = env.root.join("proj_xunbak_to_7z");
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();
    fs::write(root.join("nested").join("b.txt"), "bbb").unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));

    let archive_path = root.join("converted.7z");
    run_ok(env.cmd().args([
        "backup",
        "convert",
        root.join("backup.xunbak").to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        archive_path.to_str().unwrap(),
        "--file",
        "nested/b.txt",
    ]));

    let mut archive =
        sevenz_rust2::ArchiveReader::open(&archive_path, sevenz_rust2::Password::empty()).unwrap();
    assert!(archive.read_file("a.txt").is_err());
    let content = archive.read_file("nested/b.txt").unwrap();
    assert_eq!(String::from_utf8(content).unwrap(), "bbb");
}

#[test]
fn cli_backup_convert_xunbak_to_standard_outputs_supports_extended_codecs() {
    let env = TestEnv::new();
    let content = "alpha alpha alpha beta beta beta gamma gamma gamma ".repeat(512);

    for (codec_arg, output_stub) in [
        ("lz4", "lz4"),
        ("deflate", "deflate"),
        ("bzip2", "bzip2"),
        ("ppmd", "ppmd"),
        ("lzma2", "lzma2"),
    ] {
        let root = env
            .root
            .join(format!("proj_xunbak_to_outputs_{output_stub}"));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("notes.txt"), &content).unwrap();
        let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "notes.txt" ],
  "exclude": []
}"#;
        fs::write(root.join(".xun-bak.json"), cfg).unwrap();

        let container = root.join("source.xunbak");
        run_ok(env.cmd().args([
            "backup",
            "create",
            "-C",
            root.to_str().unwrap(),
            "--format",
            "xunbak",
            "-o",
            container.to_str().unwrap(),
            "--compression",
            codec_arg,
        ]));

        let dir_output = root.join("out-dir");
        run_ok(env.cmd().args([
            "backup",
            "convert",
            container.to_str().unwrap(),
            "--format",
            "dir",
            "-o",
            dir_output.to_str().unwrap(),
        ]));
        assert_eq!(
            fs::read_to_string(dir_output.join("notes.txt")).unwrap(),
            content,
            "codec={codec_arg} dir"
        );

        let zip_output = root.join("out.zip");
        run_ok(env.cmd().args([
            "backup",
            "convert",
            container.to_str().unwrap(),
            "--format",
            "zip",
            "-o",
            zip_output.to_str().unwrap(),
        ]));
        let file = fs::File::open(&zip_output).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut entry = archive.by_name("notes.txt").unwrap();
        let mut zip_text = String::new();
        std::io::Read::read_to_string(&mut entry, &mut zip_text).unwrap();
        assert_eq!(zip_text, content, "codec={codec_arg} zip");

        let sevenz_output = root.join("out.7z");
        run_ok(env.cmd().args([
            "backup",
            "convert",
            container.to_str().unwrap(),
            "--format",
            "7z",
            "-o",
            sevenz_output.to_str().unwrap(),
        ]));
        let mut archive =
            sevenz_rust2::ArchiveReader::open(&sevenz_output, sevenz_rust2::Password::empty())
                .unwrap();
        let bytes = archive.read_file("notes.txt").unwrap();
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            content,
            "codec={codec_arg} 7z"
        );
    }
}

#[test]
fn cli_backup_convert_dir_to_xunbak_output_writes_selected_file() {
    let env = TestEnv::new();
    let root = env.root.join("proj");
    fs::create_dir_all(root.join("artifact").join("nested")).unwrap();
    fs::write(root.join("artifact").join("a.txt"), "aaa").unwrap();
    fs::write(root.join("artifact").join("nested").join("b.txt"), "bbb").unwrap();

    let output = root.join("from_dir.xunbak");
    run_ok(env.cmd().args([
        "backup",
        "convert",
        root.join("artifact").to_str().unwrap(),
        "--format",
        "xunbak",
        "-o",
        output.to_str().unwrap(),
        "--file",
        "nested/b.txt",
    ]));

    let out_dir = root.join("converted_from_dir_restore");
    run_ok(env.cmd().args([
        "backup",
        "restore",
        output.to_str().unwrap(),
        "--to",
        out_dir.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));

    assert!(!out_dir.join("a.txt").exists());
    assert_eq!(
        fs::read_to_string(out_dir.join("nested").join("b.txt")).unwrap(),
        "bbb"
    );
}

#[test]
fn cli_backup_convert_to_xunbak_rejects_invalid_method_with_fix_hint() {
    let env = TestEnv::new();
    let root = env.root.join("proj_convert_invalid_codec");
    fs::create_dir_all(root.join("artifact")).unwrap();
    fs::write(root.join("artifact").join("a.txt"), "aaa").unwrap();

    let output = root.join("invalid.xunbak");
    let out = run_err(env.cmd().args([
        "backup",
        "convert",
        root.join("artifact").to_str().unwrap(),
        "--format",
        "xunbak",
        "-o",
        output.to_str().unwrap(),
        "--method",
        "brotli",
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("invalid compression argument"));
    assert!(stderr.contains("Fix:"));
}

#[test]
fn cli_backup_convert_dir_to_xunbak_supports_extended_codecs() {
    let env = TestEnv::new();
    let content = "alpha alpha alpha beta beta beta gamma gamma gamma ".repeat(512);

    for (method_arg, expected_codec, output_name) in [
        ("lz4", xun::xunbak::constants::Codec::LZ4, "dir-lz4.xunbak"),
        (
            "deflate",
            xun::xunbak::constants::Codec::DEFLATE,
            "dir-deflate.xunbak",
        ),
        (
            "bzip2",
            xun::xunbak::constants::Codec::BZIP2,
            "dir-bzip2.xunbak",
        ),
        (
            "ppmd",
            xun::xunbak::constants::Codec::PPMD,
            "dir-ppmd.xunbak",
        ),
        (
            "lzma2",
            xun::xunbak::constants::Codec::LZMA2,
            "dir-lzma2.xunbak",
        ),
    ] {
        let root = env.root.join(format!("proj_dir_to_xunbak_{method_arg}"));
        fs::create_dir_all(root.join("artifact")).unwrap();
        fs::write(root.join("artifact").join("notes.txt"), &content).unwrap();

        let output = root.join(output_name);
        run_ok(env.cmd().args([
            "backup",
            "convert",
            root.join("artifact").to_str().unwrap(),
            "--format",
            "xunbak",
            "-o",
            output.to_str().unwrap(),
            "--method",
            method_arg,
        ]));

        let manifest = xun::xunbak::reader::ContainerReader::open(&output)
            .unwrap()
            .load_manifest()
            .unwrap();
        assert_eq!(manifest.entries.len(), 1, "method={method_arg}");
        assert_eq!(
            manifest.entries[0].codec, expected_codec,
            "method={method_arg}"
        );

        let out_dir = root.join(format!("restored-{method_arg}"));
        run_ok(env.cmd().args([
            "backup",
            "restore",
            output.to_str().unwrap(),
            "--to",
            out_dir.to_str().unwrap(),
            "-C",
            root.to_str().unwrap(),
            "-y",
        ]));
        assert_eq!(
            fs::read_to_string(out_dir.join("notes.txt")).unwrap(),
            content,
            "method={method_arg}"
        );
    }
}

#[test]
fn cli_backup_convert_zip_to_xunbak_output_writes_selected_file() {
    let env = TestEnv::new();
    let root = env.root.join("proj_zip_to_xunbak");
    fs::create_dir_all(&root).unwrap();

    let zip_path = root.join("source.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("a.txt", options).unwrap();
    writer.write_all(b"aaa").unwrap();
    writer.start_file("nested/b.txt", options).unwrap();
    writer.write_all(b"bbb").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let container = root.join("converted.xunbak");
    run_ok(env.cmd().args([
        "backup",
        "convert",
        zip_path.to_str().unwrap(),
        "--format",
        "xunbak",
        "-o",
        container.to_str().unwrap(),
        "--file",
        "nested/b.txt",
    ]));

    let out_dir = root.join("restored");
    run_ok(env.cmd().args([
        "backup",
        "restore",
        container.to_str().unwrap(),
        "--to",
        out_dir.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));

    assert!(!out_dir.join("a.txt").exists());
    assert_eq!(
        fs::read_to_string(out_dir.join("nested").join("b.txt")).unwrap(),
        "bbb"
    );
}

#[test]
fn cli_backup_convert_zip_to_xunbak_does_not_materialize_staging_directory() {
    let env = TestEnv::new();
    let root = env.root.join("proj_zip_to_xunbak_no_staging");
    fs::create_dir_all(&root).unwrap();

    let zip_path = root.join("source.zip");
    let file = fs::File::create(&zip_path).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("notes.txt", options).unwrap();
    writer
        .write_all(
            "alpha alpha alpha beta beta beta gamma gamma gamma "
                .repeat(1024)
                .as_bytes(),
        )
        .unwrap();
    writer.finish().unwrap();

    let temp_root = env.root.join("isolated-temp");
    fs::create_dir_all(&temp_root).unwrap();
    let container = root.join("converted.xunbak");

    run_ok(
        env.cmd()
            .env("TEMP", &temp_root)
            .env("TMP", &temp_root)
            .args([
                "backup",
                "convert",
                zip_path.to_str().unwrap(),
                "--format",
                "xunbak",
                "-o",
                container.to_str().unwrap(),
            ]),
    );

    let temp_entries: Vec<String> = fs::read_dir(&temp_root)
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        temp_entries.is_empty(),
        "expected no staging/temp files, got: {temp_entries:?}"
    );
}

#[test]
fn cli_backup_convert_zip_to_xunbak_supports_extended_codecs() {
    let env = TestEnv::new();
    let content = "alpha alpha alpha beta beta beta gamma gamma gamma ".repeat(512);

    for (method_arg, expected_codec, output_name) in [
        (
            "lz4",
            xun::xunbak::constants::Codec::LZ4,
            "converted-lz4.xunbak",
        ),
        (
            "deflate",
            xun::xunbak::constants::Codec::DEFLATE,
            "converted-deflate.xunbak",
        ),
        (
            "bzip2",
            xun::xunbak::constants::Codec::BZIP2,
            "converted-bzip2.xunbak",
        ),
        (
            "ppmd",
            xun::xunbak::constants::Codec::PPMD,
            "converted-ppmd.xunbak",
        ),
        (
            "lzma2",
            xun::xunbak::constants::Codec::LZMA2,
            "converted-lzma2.xunbak",
        ),
    ] {
        let root = env.root.join(format!("proj_zip_to_xunbak_{method_arg}"));
        fs::create_dir_all(&root).unwrap();

        let zip_path = root.join("source.zip");
        let cursor = std::io::Cursor::new(Vec::<u8>::new());
        let mut writer = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        writer.start_file("notes.txt", options).unwrap();
        writer.write_all(content.as_bytes()).unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        fs::write(&zip_path, bytes).unwrap();

        let container = root.join(output_name);
        run_ok(env.cmd().args([
            "backup",
            "convert",
            zip_path.to_str().unwrap(),
            "--format",
            "xunbak",
            "-o",
            container.to_str().unwrap(),
            "--method",
            method_arg,
        ]));

        let manifest = xun::xunbak::reader::ContainerReader::open(&container)
            .unwrap()
            .load_manifest()
            .unwrap();
        assert_eq!(manifest.entries.len(), 1, "method={method_arg}");
        assert_eq!(
            manifest.entries[0].codec, expected_codec,
            "method={method_arg}"
        );

        let out_dir = root.join(format!("restored-{method_arg}"));
        run_ok(env.cmd().args([
            "backup",
            "restore",
            container.to_str().unwrap(),
            "--to",
            out_dir.to_str().unwrap(),
            "-C",
            root.to_str().unwrap(),
            "-y",
        ]));
        assert_eq!(
            fs::read_to_string(out_dir.join("notes.txt")).unwrap(),
            content,
            "method={method_arg}"
        );
    }
}

#[test]
fn cli_backup_convert_zip_to_xunbak_auto_prefers_ppmd_for_text() {
    let env = TestEnv::new();
    let root = env.root.join("proj_convert_auto_text");
    fs::create_dir_all(&root).unwrap();
    let content = "alpha alpha alpha beta beta beta gamma gamma gamma ".repeat(512);

    let zip_path = root.join("source.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("notes.txt", options).unwrap();
    writer.write_all(content.as_bytes()).unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let container = root.join("auto-text.xunbak");
    run_ok(env.cmd().args([
        "backup",
        "convert",
        zip_path.to_str().unwrap(),
        "--format",
        "xunbak",
        "-o",
        container.to_str().unwrap(),
        "--method",
        "auto",
    ]));

    let manifest = xun::xunbak::reader::ContainerReader::open(&container)
        .unwrap()
        .load_manifest()
        .unwrap();
    assert_eq!(manifest.entries.len(), 1);
    assert_eq!(
        manifest.entries[0].codec,
        xun::xunbak::constants::Codec::PPMD
    );
}

#[test]
fn cli_backup_convert_7z_to_xunbak_output_writes_selected_file() {
    let env = TestEnv::new();
    let root = env.root.join("proj_7z_to_xunbak");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src").join("main.rs"), "fn main() {}").unwrap();
    fs::write(root.join("README.md"), "readme").unwrap();
    let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "src", "README.md" ],
  "exclude": []
}"#;
    fs::write(root.join(".xun-bak.json"), cfg).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "create",
        "-C",
        root.to_str().unwrap(),
        "--format",
        "7z",
        "-o",
        "source.7z",
    ]));

    let container = root.join("converted.xunbak");
    run_ok(env.cmd().args([
        "backup",
        "convert",
        root.join("source.7z").to_str().unwrap(),
        "--format",
        "xunbak",
        "-o",
        container.to_str().unwrap(),
        "--file",
        "README.md",
    ]));

    let out_dir = root.join("restored_from_7z");
    run_ok(env.cmd().args([
        "backup",
        "restore",
        container.to_str().unwrap(),
        "--to",
        out_dir.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));

    assert!(!out_dir.join("src").join("main.rs").exists());
    assert_eq!(
        fs::read_to_string(out_dir.join("README.md")).unwrap(),
        "readme"
    );
}

#[test]
fn cli_backup_convert_7z_to_xunbak_supports_extended_codecs() {
    let env = TestEnv::new();
    let content = "alpha alpha alpha beta beta beta gamma gamma gamma ".repeat(512);

    for (method_arg, expected_codec, output_name) in [
        ("lz4", xun::xunbak::constants::Codec::LZ4, "7z-lz4.xunbak"),
        (
            "deflate",
            xun::xunbak::constants::Codec::DEFLATE,
            "7z-deflate.xunbak",
        ),
        (
            "bzip2",
            xun::xunbak::constants::Codec::BZIP2,
            "7z-bzip2.xunbak",
        ),
        (
            "ppmd",
            xun::xunbak::constants::Codec::PPMD,
            "7z-ppmd.xunbak",
        ),
        (
            "lzma2",
            xun::xunbak::constants::Codec::LZMA2,
            "7z-lzma2.xunbak",
        ),
    ] {
        let root = env.root.join(format!("proj_7z_to_xunbak_{method_arg}"));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("notes.txt"), &content).unwrap();
        let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "notes.txt" ],
  "exclude": []
}"#;
        fs::write(root.join(".xun-bak.json"), cfg).unwrap();

        run_ok(env.cmd().args([
            "backup",
            "create",
            "-C",
            root.to_str().unwrap(),
            "--format",
            "7z",
            "-o",
            "source.7z",
        ]));

        let container = root.join(output_name);
        run_ok(env.cmd().args([
            "backup",
            "convert",
            root.join("source.7z").to_str().unwrap(),
            "--format",
            "xunbak",
            "-o",
            container.to_str().unwrap(),
            "--method",
            method_arg,
            "--file",
            "notes.txt",
        ]));

        let manifest = xun::xunbak::reader::ContainerReader::open(&container)
            .unwrap()
            .load_manifest()
            .unwrap();
        assert_eq!(manifest.entries.len(), 1, "method={method_arg}");
        assert_eq!(
            manifest.entries[0].codec, expected_codec,
            "method={method_arg}"
        );

        let out_dir = root.join(format!("restored-{method_arg}"));
        run_ok(env.cmd().args([
            "backup",
            "restore",
            container.to_str().unwrap(),
            "--to",
            out_dir.to_str().unwrap(),
            "-C",
            root.to_str().unwrap(),
            "-y",
        ]));
        assert_eq!(
            fs::read_to_string(out_dir.join("notes.txt")).unwrap(),
            content,
            "method={method_arg}"
        );
    }
}

#[test]
fn cli_backup_convert_zip_to_split_xunbak_output_creates_numbered_volumes_without_temp_artifacts() {
    let env = TestEnv::new();
    let root = env.root.join("proj_zip_to_split_xunbak");
    fs::create_dir_all(&root).unwrap();

    let zip_path = root.join("source.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("a.txt", options).unwrap();
    writer.write_all(&vec![b'a'; 800]).unwrap();
    writer.start_file("b.txt", options).unwrap();
    writer.write_all(&vec![b'b'; 800]).unwrap();
    writer.start_file("c.txt", options).unwrap();
    writer.write_all(&vec![b'c'; 800]).unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let container = root.join("converted.xunbak");
    run_ok(env.cmd().args([
        "backup",
        "convert",
        zip_path.to_str().unwrap(),
        "--format",
        "xunbak",
        "-o",
        container.to_str().unwrap(),
        "--method",
        "none",
        "--split-size",
        "1900",
    ]));

    assert!(root.join("converted.xunbak.001").exists());
    assert!(root.join("converted.xunbak.002").exists());
    let temp_staged = fs::read_dir(&root)
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .any(|name| name.contains("tmp-split-xunbak") || name.contains("tmp.xunbak"));
    assert!(
        !temp_staged,
        "temporary split staging files should be cleaned up"
    );
}

#[test]
fn cli_backup_convert_zip_to_split_xunbak_json_reports_volume_outputs_and_bytes() {
    let env = TestEnv::new();
    let root = env.root.join("proj_zip_to_split_xunbak_json");
    fs::create_dir_all(&root).unwrap();

    let zip_path = root.join("source.zip");
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("a.txt", options).unwrap();
    writer.write_all(&vec![b'a'; 800]).unwrap();
    writer.start_file("b.txt", options).unwrap();
    writer.write_all(&vec![b'b'; 800]).unwrap();
    writer.start_file("c.txt", options).unwrap();
    writer.write_all(&vec![b'c'; 800]).unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    fs::write(&zip_path, bytes).unwrap();

    let container = root.join("converted.xunbak");
    let out = run_ok(env.cmd().args([
        "backup",
        "convert",
        zip_path.to_str().unwrap(),
        "--format",
        "xunbak",
        "-o",
        container.to_str().unwrap(),
        "--method",
        "none",
        "--split-size",
        "1900",
        "--json",
    ]));

    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    let outputs: Vec<String> = value["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect();
    assert_eq!(value["format"], "xunbak");
    assert!(value["bytes_out"].as_u64().unwrap() > 0);
    assert!(outputs.len() >= 2);
    assert!(
        outputs.iter().all(|path| path.contains("converted.xunbak.")
            && path.starts_with(&root.display().to_string()))
    );
    assert!(outputs.iter().any(|path| path.ends_with(".001")));
    assert!(outputs.iter().any(|path| path.ends_with(".002")));
}

#[test]
fn cli_backup_convert_zip_to_split_xunbak_supports_extended_codecs() {
    let env = TestEnv::new();
    let content = "alpha alpha alpha beta beta beta gamma gamma gamma ".repeat(512);

    for (method_arg, expected_codec, output_name) in [
        (
            "lz4",
            xun::xunbak::constants::Codec::LZ4,
            "split-lz4.xunbak",
        ),
        (
            "deflate",
            xun::xunbak::constants::Codec::DEFLATE,
            "split-deflate.xunbak",
        ),
        (
            "bzip2",
            xun::xunbak::constants::Codec::BZIP2,
            "split-bzip2.xunbak",
        ),
        (
            "ppmd",
            xun::xunbak::constants::Codec::PPMD,
            "split-ppmd.xunbak",
        ),
        (
            "lzma2",
            xun::xunbak::constants::Codec::LZMA2,
            "split-lzma2.xunbak",
        ),
    ] {
        let root = env
            .root
            .join(format!("proj_zip_to_split_xunbak_{method_arg}"));
        fs::create_dir_all(&root).unwrap();

        let zip_path = root.join("source.zip");
        let cursor = std::io::Cursor::new(Vec::<u8>::new());
        let mut writer = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        writer.start_file("notes.txt", options).unwrap();
        writer.write_all(content.as_bytes()).unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        fs::write(&zip_path, bytes).unwrap();

        let container = root.join(output_name);
        run_ok(env.cmd().args([
            "backup",
            "convert",
            zip_path.to_str().unwrap(),
            "--format",
            "xunbak",
            "-o",
            container.to_str().unwrap(),
            "--method",
            method_arg,
            "--split-size",
            "1900",
        ]));

        assert!(root.join(format!("{output_name}.001")).exists());
        let manifest = xun::xunbak::reader::ContainerReader::open(&container)
            .unwrap()
            .load_manifest()
            .unwrap();
        assert_eq!(manifest.entries.len(), 1, "method={method_arg}");
        assert_eq!(
            manifest.entries[0].codec, expected_codec,
            "method={method_arg}"
        );

        let out_dir = root.join(format!("restored-{method_arg}"));
        run_ok(env.cmd().args([
            "backup",
            "restore",
            container.to_str().unwrap(),
            "--to",
            out_dir.to_str().unwrap(),
            "-C",
            root.to_str().unwrap(),
            "-y",
        ]));
        assert_eq!(
            fs::read_to_string(out_dir.join("notes.txt")).unwrap(),
            content,
            "method={method_arg}"
        );
    }
}

#[test]
fn cli_backup_convert_xunbak_to_xunbak_supports_extended_codecs() {
    let env = TestEnv::new();
    let content = "alpha alpha alpha beta beta beta gamma gamma gamma ".repeat(512);

    for (method_arg, expected_codec, output_name) in [
        ("lz4", xun::xunbak::constants::Codec::LZ4, "x2x-lz4.xunbak"),
        (
            "deflate",
            xun::xunbak::constants::Codec::DEFLATE,
            "x2x-deflate.xunbak",
        ),
        (
            "bzip2",
            xun::xunbak::constants::Codec::BZIP2,
            "x2x-bzip2.xunbak",
        ),
        (
            "ppmd",
            xun::xunbak::constants::Codec::PPMD,
            "x2x-ppmd.xunbak",
        ),
        (
            "lzma2",
            xun::xunbak::constants::Codec::LZMA2,
            "x2x-lzma2.xunbak",
        ),
    ] {
        let root = env.root.join(format!("proj_xunbak_to_xunbak_{method_arg}"));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("notes.txt"), &content).unwrap();
        let cfg = r#"{
  "storage": { "backupsDir": "A_backups", "compress": false },
  "naming": { "prefix": "v", "dateFormat": "yyyy-MM-dd_HHmm", "defaultDesc": "backup" },
  "retention": { "maxBackups": 5, "deleteCount": 1 },
  "include": [ "notes.txt" ],
  "exclude": []
}"#;
        fs::write(root.join(".xun-bak.json"), cfg).unwrap();

        let source_container = root.join("source.xunbak");
        run_ok(env.cmd().args([
            "backup",
            "create",
            "-C",
            root.to_str().unwrap(),
            "--format",
            "xunbak",
            "-o",
            source_container.to_str().unwrap(),
            "--compression",
            "none",
        ]));

        let output = root.join(output_name);
        run_ok(env.cmd().args([
            "backup",
            "convert",
            source_container.to_str().unwrap(),
            "--format",
            "xunbak",
            "-o",
            output.to_str().unwrap(),
            "--method",
            method_arg,
        ]));

        let manifest = xun::xunbak::reader::ContainerReader::open(&output)
            .unwrap()
            .load_manifest()
            .unwrap();
        assert_eq!(manifest.entries.len(), 1, "method={method_arg}");
        assert_eq!(
            manifest.entries[0].codec, expected_codec,
            "method={method_arg}"
        );

        let out_dir = root.join(format!("restored-{method_arg}"));
        run_ok(env.cmd().args([
            "backup",
            "restore",
            output.to_str().unwrap(),
            "--to",
            out_dir.to_str().unwrap(),
            "-C",
            root.to_str().unwrap(),
            "-y",
        ]));
        assert_eq!(
            fs::read_to_string(out_dir.join("notes.txt")).unwrap(),
            content,
            "method={method_arg}"
        );
    }
}

#[test]
fn cli_verify_container_outputs_json() {
    let env = TestEnv::new();
    let root = env.root.join("proj");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));

    let out = run_ok(env.cmd().args([
        "verify",
        root.join("backup.xunbak").to_str().unwrap(),
        "--json",
    ]));
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value.get("passed").and_then(Value::as_bool), Some(true));
    assert_eq!(value.get("level").and_then(Value::as_str), Some("quick"));
}

#[test]
fn cli_verify_full_and_paranoid_levels_succeed() {
    let env = TestEnv::new();
    let root = env.root.join("proj");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));

    let full = run_ok(env.cmd().args([
        "verify",
        root.join("backup.xunbak").to_str().unwrap(),
        "--level",
        "full",
        "--json",
    ]));
    let full_value: Value = serde_json::from_slice(&full.stdout).unwrap();
    assert_eq!(
        full_value.get("passed").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        full_value.get("level").and_then(Value::as_str),
        Some("full")
    );

    let paranoid = run_ok(env.cmd().args([
        "verify",
        root.join("backup.xunbak").to_str().unwrap(),
        "--level",
        "paranoid",
        "--json",
    ]));
    let paranoid_value: Value = serde_json::from_slice(&paranoid.stdout).unwrap();
    assert_eq!(
        paranoid_value.get("passed").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        paranoid_value.get("level").and_then(Value::as_str),
        Some("paranoid")
    );
}

#[test]
fn cli_verify_manifest_only_and_existence_only_levels_succeed() {
    let env = TestEnv::new();
    let root = env.root.join("proj_verify_new_levels");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "a".repeat(1200)).unwrap();
    fs::write(root.join("b.txt"), "b".repeat(1200)).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "--split-size",
        "1900",
        "-m",
        "t",
    ]));

    let manifest_only = run_ok(env.cmd().args([
        "verify",
        root.join("backup.xunbak").to_str().unwrap(),
        "--level",
        "manifest-only",
        "--json",
    ]));
    let manifest_only_value: Value = serde_json::from_slice(&manifest_only.stdout).unwrap();
    assert_eq!(
        manifest_only_value.get("passed").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        manifest_only_value.get("level").and_then(Value::as_str),
        Some("manifest-only")
    );

    let existence_only = run_ok(env.cmd().args([
        "verify",
        root.join("backup.xunbak").to_str().unwrap(),
        "--level",
        "existence-only",
        "--json",
    ]));
    let existence_only_value: Value = serde_json::from_slice(&existence_only.stdout).unwrap();
    assert_eq!(
        existence_only_value.get("passed").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        existence_only_value.get("level").and_then(Value::as_str),
        Some("existence-only")
    );
}

#[test]
fn cli_verify_existence_only_reports_missing_split_volume_context() {
    let env = TestEnv::new();
    let root = env.root.join("proj_verify_existence_missing_volume");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "a".repeat(1200)).unwrap();
    fs::write(root.join("b.txt"), "b".repeat(1200)).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "--split-size",
        "1900",
        "-m",
        "t",
    ]));
    let last_volume = fs::read_dir(&root)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("backup.xunbak."))
        })
        .max()
        .unwrap();
    fs::remove_file(last_volume).unwrap();

    let out = run_err(env.cmd().args([
        "verify",
        root.join("backup.xunbak").to_str().unwrap(),
        "--level",
        "existence-only",
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("First error:"));
    assert!(stderr.contains("Volume:"));
    assert!(stderr.contains("Path:"));
}

#[test]
fn cli_verify_full_reports_first_failing_path_context() {
    let env = TestEnv::new();
    let root = env.root.join("proj_verify_full_path_context");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "aaa").unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));

    let container = root.join("backup.xunbak");
    let reader = xun::xunbak::reader::ContainerReader::open(&container).unwrap();
    let manifest = reader.load_manifest().unwrap();
    let entry = &manifest.entries[0];
    let mut bytes = fs::read(&container).unwrap();
    let data_start = entry.blob_offset as usize
        + xun::xunbak::constants::RECORD_PREFIX_SIZE
        + xun::xunbak::constants::BLOB_HEADER_SIZE;
    bytes[data_start] ^= 0xFF;
    fs::write(&container, bytes).unwrap();

    let out = run_err(
        env.cmd()
            .args(["verify", container.to_str().unwrap(), "--level", "full"]),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("First error:"));
    assert!(stderr.contains("Path: a.txt"));
}

#[test]
fn cli_backup_container_prints_progress_for_long_runs() {
    let env = TestEnv::new();
    let root = env.root.join("proj");
    fs::create_dir_all(&root).unwrap();
    for i in 0..120 {
        fs::write(root.join(format!("f{i:03}.txt")), "x").unwrap();
    }

    let out = run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "-m",
        "t",
    ]));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("xunbak progress:"));
    assert!(stderr.contains("files="));
    assert!(stderr.contains("bytes="));
}

#[test]
fn cli_backup_restore_verify_split_container() {
    let env = TestEnv::new();
    let root = env.root.join("proj");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "a".repeat(80)).unwrap();
    fs::write(root.join("b.txt"), "b".repeat(80)).unwrap();
    fs::write(root.join("c.txt"), "c".repeat(80)).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "--split-size",
        "1900",
        "-m",
        "t",
    ]));

    assert!(root.join("backup.xunbak.001").exists());
    assert!(root.join("backup.xunbak.002").exists());

    let restore_dir = root.join("restore");
    run_ok(env.cmd().args([
        "backup",
        "restore",
        root.join("backup.xunbak").to_str().unwrap(),
        "--to",
        restore_dir.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));
    assert_eq!(
        fs::read_to_string(restore_dir.join("a.txt")).unwrap(),
        "a".repeat(80)
    );

    let verify = run_ok(env.cmd().args([
        "verify",
        root.join("backup.xunbak").to_str().unwrap(),
        "--level",
        "paranoid",
        "--json",
    ]));
    let value: Value = serde_json::from_slice(&verify.stdout).unwrap();
    assert_eq!(value.get("passed").and_then(Value::as_bool), Some(true));
}

#[test]
fn cli_backup_restore_accepts_split_first_volume_path() {
    let env = TestEnv::new();
    let root = env.root.join("proj_split_first");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), "a".repeat(80)).unwrap();
    fs::write(root.join("b.txt"), "b".repeat(80)).unwrap();
    fs::write(root.join("c.txt"), "c".repeat(80)).unwrap();

    run_ok(env.cmd().args([
        "backup",
        "-C",
        root.to_str().unwrap(),
        "--container",
        "backup.xunbak",
        "--compression",
        "none",
        "--split-size",
        "1900",
        "-m",
        "t",
    ]));

    let restore_dir = root.join("restore_first");
    run_ok(env.cmd().args([
        "backup",
        "restore",
        root.join("backup.xunbak.001").to_str().unwrap(),
        "--to",
        restore_dir.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        "-y",
    ]));

    assert_eq!(
        fs::read_to_string(restore_dir.join("c.txt")).unwrap(),
        "c".repeat(80)
    );
}
