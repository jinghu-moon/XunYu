#![cfg(all(windows, feature = "crypt"))]

mod common;

use common::*;
use serde_json::Value;
use std::fs;

fn parse_stdout_json(output: &std::process::Output) -> Value {
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(stdout.trim()).unwrap_or_else(|err| {
        panic!("stdout is not valid json: {err}; stdout={stdout}")
    })
}

struct VerifyExpectation<'a> {
    status: &'a str,
    header_valid: bool,
    payload_valid: bool,
    footer_present: bool,
    header_reason_contains: Option<&'a str>,
    payload_reason_contains: Option<&'a str>,
}

fn assert_verify_snapshot(json: &Value, expect: &VerifyExpectation<'_>) {
    assert_eq!(json["status"], expect.status);
    assert_eq!(json["header"]["valid"], expect.header_valid);
    assert_eq!(json["payload"]["valid"], expect.payload_valid);
    assert_eq!(json["footer"]["present"], expect.footer_present);
    if let Some(reason) = expect.header_reason_contains {
        assert!(json["header"]["reason"]
            .as_str()
            .unwrap_or_default()
            .contains(reason));
    }
    if let Some(reason) = expect.payload_reason_contains {
        assert!(json["payload"]["reason"]
            .as_str()
            .unwrap_or_default()
            .contains(reason));
    }
}

#[test]
fn filevault_verify_rejects_tampered_header() {
    let env = TestEnv::new();
    let work = make_safe_work_dir("filevault-header-tamper");
    let plain = work.join("plain.txt");
    let vault = work.join("plain.fv");
    fs::write(&plain, b"filevault-header-tamper").unwrap();

    run_ok(env.cmd().args([
        "vault",
        "enc",
        plain.to_str().unwrap(),
        "-o",
        vault.to_str().unwrap(),
        "--password",
        "hunter2",
    ]));

    let mut bytes = fs::read(&vault).unwrap();
    let tamper_at = 40usize.min(bytes.len().saturating_sub(1));
    bytes[tamper_at] ^= 0x5a;
    fs::write(&vault, bytes).unwrap();

    let verify = run_err(env.cmd().args([
        "vault",
        "verify",
        vault.to_str().unwrap(),
        "--json",
    ]));
    let json = parse_stdout_json(&verify);
    assert_eq!(json["status"], "corrupt");
    assert_eq!(json["header"]["valid"], false);
    assert!(json["header"]["reason"].as_str().unwrap_or_default().contains("mac"));

    cleanup_dir(&work);
}

#[test]
fn filevault_verify_rejects_reordered_frames() {
    let env = TestEnv::new();
    let work = make_safe_work_dir("filevault-frame-reorder");
    let plain = work.join("movie.bin");
    let vault = work.join("movie.fv");
    fs::write(&plain, vec![0x42; 256 * 1024]).unwrap();

    run_ok(env.cmd().args([
        "vault",
        "enc",
        plain.to_str().unwrap(),
        "-o",
        vault.to_str().unwrap(),
        "--password",
        "hunter2",
        "--chunk-size",
        "65536",
    ]));

    let inspect = run_ok(env.cmd().args([
        "vault",
        "inspect",
        vault.to_str().unwrap(),
        "--json",
    ]));
    let inspect_json = parse_stdout_json(&inspect);
    let first = inspect_json["layout"]["first_frame_offset"].as_u64().unwrap() as usize;
    let second = inspect_json["layout"]["second_frame_offset"].as_u64().unwrap() as usize;
    let frame_len = inspect_json["layout"]["frame_span"].as_u64().unwrap() as usize;

    let mut bytes = fs::read(&vault).unwrap();
    let frame0 = bytes[first..first + frame_len].to_vec();
    let frame1 = bytes[second..second + frame_len].to_vec();
    bytes[first..first + frame_len].copy_from_slice(&frame1);
    bytes[second..second + frame_len].copy_from_slice(&frame0);
    fs::write(&vault, bytes).unwrap();

    let verify = run_err(env.cmd().args([
        "vault",
        "verify",
        vault.to_str().unwrap(),
        "--json",
    ]));
    let json = parse_stdout_json(&verify);
    assert_eq!(json["status"], "corrupt");
    assert!(json["payload"]["reason"].as_str().unwrap_or_default().contains("frame"));

    cleanup_dir(&work);
}

#[test]
fn filevault_verify_reports_missing_footer() {
    let env = TestEnv::new();
    let work = make_safe_work_dir("filevault-footer-missing");
    let plain = work.join("db.sqlite");
    let vault = work.join("db.sqlite.fv");
    fs::write(&plain, vec![0x24; 96 * 1024]).unwrap();

    run_ok(env.cmd().args([
        "vault",
        "enc",
        plain.to_str().unwrap(),
        "-o",
        vault.to_str().unwrap(),
        "--password",
        "hunter2",
    ]));

    let mut bytes = fs::read(&vault).unwrap();
    bytes.truncate(bytes.len().saturating_sub(80));
    fs::write(&vault, bytes).unwrap();

    let verify = run_err(env.cmd().args([
        "vault",
        "verify",
        vault.to_str().unwrap(),
        "--json",
    ]));
    let json = parse_stdout_json(&verify);
    assert_eq!(json["status"], "incomplete");
    assert_eq!(json["footer"]["present"], false);

    cleanup_dir(&work);
}

#[test]
fn filevault_resume_finishes_interrupted_write_without_polluting_target() {
    let env = TestEnv::new();
    let work = make_safe_work_dir("filevault-resume");
    let plain = work.join("archive.tar");
    let vault = work.join("archive.tar.fv");
    let recovered = work.join("archive.recovered.tar");
    fs::write(&plain, vec![0x7b; 320 * 1024]).unwrap();
    let original_plain = fs::read(&plain).unwrap();

    let crash = run_err(
        env.cmd()
            .env("XUN_FILEVAULT_FAIL_AFTER_FRAMES", "2")
            .args([
                "vault",
                "enc",
                plain.to_str().unwrap(),
                "-o",
                vault.to_str().unwrap(),
                "--password",
                "hunter2",
                "--chunk-size",
                "65536",
            ]),
    );
    let stderr = String::from_utf8_lossy(&crash.stderr);
    assert!(stderr.contains("simulated crash") || stderr.contains("interrupted"));
    assert!(!vault.exists(), "final target must remain absent before commit rename");
    assert_eq!(fs::read(&plain).unwrap(), original_plain, "source file must remain unpolluted");
    assert!(vault.with_extension("fv.fvjournal").exists() || work.join("archive.tar.fv.fvjournal").exists());

    run_ok(env.cmd().args([
        "vault",
        "resume",
        vault.to_str().unwrap(),
        "--password",
        "hunter2",
    ]));
    assert!(vault.exists(), "resume should materialize final ciphertext");

    run_ok(env.cmd().args([
        "vault",
        "dec",
        vault.to_str().unwrap(),
        "-o",
        recovered.to_str().unwrap(),
        "--password",
        "hunter2",
    ]));
    assert_eq!(fs::read(&recovered).unwrap(), original_plain);

    cleanup_dir(&work);
}

#[test]
fn filevault_cleanup_removes_orphan_artifacts_idempotently() {
    let env = TestEnv::new();
    let work = make_safe_work_dir("filevault-cleanup");
    let plain = work.join("cleanup.txt");
    let vault = work.join("cleanup.txt.fv");
    fs::write(&plain, vec![0x39; 128 * 1024]).unwrap();

    let _ = run_err(
        env.cmd()
            .env("XUN_FILEVAULT_FAIL_AFTER_FRAMES", "1")
            .args([
                "vault",
                "enc",
                plain.to_str().unwrap(),
                "-o",
                vault.to_str().unwrap(),
                "--password",
                "hunter2",
            ]),
    );

    run_ok(env.cmd().args(["vault", "cleanup", vault.to_str().unwrap()]));
    run_ok(env.cmd().args(["vault", "cleanup", vault.to_str().unwrap()]));
    assert!(!vault.exists());
    assert!(!work.join("cleanup.txt.fv.fvtmp").exists());
    assert!(!work.join("cleanup.txt.fv.fvjournal").exists());

    cleanup_dir(&work);
}

#[test]
fn filevault_rewrap_replaces_slots_without_reencrypting_payload() {
    let env = TestEnv::new();
    let work = make_safe_work_dir("filevault-rewrap");
    let plain = work.join("secrets.db");
    let vault = work.join("secrets.db.fv");
    let keyfile = work.join("slot.key");
    let recovery_file = work.join("recovery.txt");
    fs::write(&plain, vec![0x55; 192 * 1024]).unwrap();
    fs::write(&keyfile, b"keyfile-material-v1").unwrap();

    run_ok(env.cmd().args([
        "vault",
        "enc",
        plain.to_str().unwrap(),
        "-o",
        vault.to_str().unwrap(),
        "--password",
        "old-secret",
        "--emit-recovery-key",
        recovery_file.to_str().unwrap(),
    ]));
    let before = parse_stdout_json(&run_ok(env.cmd().args([
        "vault",
        "inspect",
        vault.to_str().unwrap(),
        "--json",
    ])));
    let before_digest = before["footer"]["payload_digest"].as_str().unwrap().to_string();

    run_ok(env.cmd().args([
        "vault",
        "rewrap",
        vault.to_str().unwrap(),
        "--unlock-password",
        "old-secret",
        "--add-password",
        "new-secret",
        "--add-keyfile",
        keyfile.to_str().unwrap(),
        "--remove-slot",
        "password",
    ]));

    let after = parse_stdout_json(&run_ok(env.cmd().args([
        "vault",
        "inspect",
        vault.to_str().unwrap(),
        "--json",
    ])));
    let after_digest = after["footer"]["payload_digest"].as_str().unwrap().to_string();
    assert_eq!(before_digest, after_digest, "rewrap must not re-encrypt payload ciphertext");

    let old_password = run_err(env.cmd().args([
        "vault",
        "dec",
        vault.to_str().unwrap(),
        "-o",
        work.join("old-password.out").to_str().unwrap(),
        "--password",
        "old-secret",
    ]));
    assert_eq!(old_password.status.code(), Some(5));

    run_ok(env.cmd().args([
        "vault",
        "dec",
        vault.to_str().unwrap(),
        "-o",
        work.join("new-password.out").to_str().unwrap(),
        "--password",
        "new-secret",
    ]));
    run_ok(env.cmd().args([
        "vault",
        "dec",
        vault.to_str().unwrap(),
        "-o",
        work.join("keyfile.out").to_str().unwrap(),
        "--keyfile",
        keyfile.to_str().unwrap(),
    ]));

    cleanup_dir(&work);
}

#[test]
fn filevault_recover_key_rebuilds_recovery_slot_from_remaining_legal_entry() {
    let env = TestEnv::new();
    let work = make_safe_work_dir("filevault-recover-key");
    let plain = work.join("contracts.pdf");
    let vault = work.join("contracts.pdf.fv");
    let keyfile = work.join("dp.key");
    let recovery_file = work.join("recovered-key.txt");
    fs::write(&plain, vec![0x21; 160 * 1024]).unwrap();
    fs::write(&keyfile, b"recover-key-material").unwrap();

    run_ok(env.cmd().args([
        "vault",
        "enc",
        plain.to_str().unwrap(),
        "-o",
        vault.to_str().unwrap(),
        "--password",
        "boot-secret",
        "--keyfile",
        keyfile.to_str().unwrap(),
    ]));

    run_ok(env.cmd().args([
        "vault",
        "rewrap",
        vault.to_str().unwrap(),
        "--unlock-password",
        "boot-secret",
        "--add-keyfile",
        keyfile.to_str().unwrap(),
        "--remove-slot",
        "password",
    ]));

    run_ok(env.cmd().args([
        "vault",
        "recover-key",
        vault.to_str().unwrap(),
        "--unlock-keyfile",
        keyfile.to_str().unwrap(),
        "--output",
        recovery_file.to_str().unwrap(),
    ]));
    let recovery = fs::read_to_string(&recovery_file).unwrap();
    assert!(!recovery.trim().is_empty());

    run_ok(env.cmd().args([
        "vault",
        "dec",
        vault.to_str().unwrap(),
        "-o",
        work.join("recovery.out").to_str().unwrap(),
        "--recovery-key",
        recovery.trim(),
    ]));

    cleanup_dir(&work);
}

#[test]
fn filevault_supports_dpapi_slot_on_same_windows_profile() {
    let env = TestEnv::new();
    let work = make_safe_work_dir("filevault-dpapi");
    let plain = work.join("notes.txt");
    let vault = work.join("notes.txt.fv");
    let out = work.join("notes.out.txt");
    fs::write(&plain, b"same-profile-dpapi").unwrap();

    run_ok(env.cmd().args([
        "vault",
        "enc",
        plain.to_str().unwrap(),
        "-o",
        vault.to_str().unwrap(),
        "--password",
        "fallback-secret",
        "--dpapi",
    ]));

    run_ok(env.cmd().args([
        "vault",
        "dec",
        vault.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--dpapi",
    ]));
    assert_eq!(fs::read(&out).unwrap(), b"same-profile-dpapi");

    cleanup_dir(&work);
}

#[test]
fn filevault_algorithm_kdf_slot_matrix_roundtrip() {
    let env = TestEnv::new();
    let work = make_safe_work_dir("filevault-algo-kdf-matrix");
    let plain = work.join("matrix.bin");
    let payload = vec![0x6d; 128 * 1024];
    fs::write(&plain, &payload).unwrap();
    let keyfile = work.join("matrix.key");
    fs::write(&keyfile, b"matrix-keyfile-material").unwrap();

    let algos = ["aes256-gcm", "xchacha20-poly1305"];
    let kdfs = ["argon2id", "pbkdf2-sha256"];
    let slots = ["password", "keyfile", "recovery-key", "dpapi"];

    for algo in algos {
        for kdf in kdfs {
            for slot in slots {
                let slot_id = match slot {
                    "password" => "password",
                    "keyfile" => "keyfile",
                    "recovery-key" => "recovery",
                    "dpapi" => "dpapi",
                    _ => "unknown",
                };
                let vault = work.join(format!("matrix-{algo}-{kdf}-{slot_id}.fv"));
                let output = work.join(format!("matrix-{algo}-{kdf}-{slot_id}.out"));
                let recovery_path = work.join(format!("recovery-{algo}-{kdf}-{slot_id}.txt"));

                let mut enc = env.cmd();
                enc.args([
                    "vault",
                    "enc",
                    plain.to_str().unwrap(),
                    "-o",
                    vault.to_str().unwrap(),
                    "--algo",
                    algo,
                    "--kdf",
                    kdf,
                ]);
                match slot {
                    "password" => {
                        enc.args(["--password", "hunter2"]);
                    }
                    "keyfile" => {
                        enc.args(["--keyfile", keyfile.to_str().unwrap()]);
                    }
                    "recovery-key" => {
                        enc.args(["--emit-recovery-key", recovery_path.to_str().unwrap()]);
                    }
                    "dpapi" => {
                        enc.arg("--dpapi");
                    }
                    _ => {}
                }
                run_ok(&mut enc);

                let mut dec = env.cmd();
                dec.args([
                    "vault",
                    "dec",
                    vault.to_str().unwrap(),
                    "-o",
                    output.to_str().unwrap(),
                ]);
                match slot {
                    "password" => {
                        dec.args(["--password", "hunter2"]);
                    }
                    "keyfile" => {
                        dec.args(["--keyfile", keyfile.to_str().unwrap()]);
                    }
                    "recovery-key" => {
                        let recovery = fs::read_to_string(&recovery_path).unwrap();
                        dec.args(["--recovery-key", recovery.trim()]);
                    }
                    "dpapi" => {
                        dec.arg("--dpapi");
                    }
                    _ => {}
                }
                run_ok(&mut dec);
                assert_eq!(fs::read(&output).unwrap(), payload);
            }
        }
    }

    cleanup_dir(&work);
}

#[test]
fn filevault_resume_handles_multiple_breakpoints() {
    let env = TestEnv::new();
    let work = make_safe_work_dir("filevault-multi-breakpoints");
    let plain = work.join("payload.bin");
    let payload = vec![0x5a; 8 * 65_536];
    fs::write(&plain, &payload).unwrap();
    let original_plain = fs::read(&plain).unwrap();

    let breakpoints = [1u32, 3, 8];
    for (index, fail_after) in breakpoints.iter().enumerate() {
        let vault = work.join(format!("payload-{index}.fv"));
        let output = work.join(format!("payload-{index}.out"));
        let journal = std::path::PathBuf::from(format!("{}.fvjournal", vault.to_string_lossy()));
        let temp = std::path::PathBuf::from(format!("{}.fvtmp", vault.to_string_lossy()));

        let crash = run_err(
            env.cmd()
                .env("XUN_FILEVAULT_FAIL_AFTER_FRAMES", fail_after.to_string())
                .args([
                    "vault",
                    "enc",
                    plain.to_str().unwrap(),
                    "-o",
                    vault.to_str().unwrap(),
                    "--password",
                    "hunter2",
                    "--chunk-size",
                    "65536",
                ]),
        );
        let stderr = String::from_utf8_lossy(&crash.stderr);
        assert!(stderr.contains("simulated crash") || stderr.contains("interrupted"));
        assert!(!vault.exists(), "final target must remain absent before commit rename");
        assert_eq!(
            fs::read(&plain).unwrap(),
            original_plain,
            "source file must remain unpolluted"
        );
        assert!(journal.exists(), "journal should exist for resume");
        assert!(temp.exists(), "temp ciphertext should exist for resume");

        run_ok(env.cmd().args([
            "vault",
            "resume",
            vault.to_str().unwrap(),
            "--password",
            "hunter2",
        ]));
        assert!(vault.exists(), "resume should materialize final ciphertext");

        run_ok(env.cmd().args([
            "vault",
            "dec",
            vault.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--password",
            "hunter2",
        ]));
        assert_eq!(fs::read(&output).unwrap(), original_plain);
    }

    cleanup_dir(&work);
}

#[test]
fn filevault_rejects_wrong_unlock_materials() {
    let env = TestEnv::new();
    let work = make_safe_work_dir("filevault-wrong-unlock");
    let plain = work.join("plain.bin");
    let payload = vec![0x33; 96 * 1024];
    fs::write(&plain, &payload).unwrap();

    let password_vault = work.join("password.fv");
    run_ok(env.cmd().args([
        "vault",
        "enc",
        plain.to_str().unwrap(),
        "-o",
        password_vault.to_str().unwrap(),
        "--password",
        "correct-password",
    ]));
    let wrong_password = run_err(env.cmd().args([
        "vault",
        "dec",
        password_vault.to_str().unwrap(),
        "-o",
        work.join("wrong-password.out").to_str().unwrap(),
        "--password",
        "wrong-password",
    ]));
    assert_eq!(wrong_password.status.code(), Some(5));

    let keyfile = work.join("right.key");
    let wrong_keyfile = work.join("wrong.key");
    fs::write(&keyfile, b"right-keyfile-material").unwrap();
    fs::write(&wrong_keyfile, b"wrong-keyfile-material").unwrap();
    let keyfile_vault = work.join("keyfile.fv");
    run_ok(env.cmd().args([
        "vault",
        "enc",
        plain.to_str().unwrap(),
        "-o",
        keyfile_vault.to_str().unwrap(),
        "--keyfile",
        keyfile.to_str().unwrap(),
    ]));
    let wrong_keyfile_out = run_err(env.cmd().args([
        "vault",
        "dec",
        keyfile_vault.to_str().unwrap(),
        "-o",
        work.join("wrong-keyfile.out").to_str().unwrap(),
        "--keyfile",
        wrong_keyfile.to_str().unwrap(),
    ]));
    assert_eq!(wrong_keyfile_out.status.code(), Some(5));

    let recovery_vault = work.join("recovery.fv");
    let recovery_path = work.join("recovery.key");
    run_ok(env.cmd().args([
        "vault",
        "enc",
        plain.to_str().unwrap(),
        "-o",
        recovery_vault.to_str().unwrap(),
        "--emit-recovery-key",
        recovery_path.to_str().unwrap(),
    ]));
    let recovery = fs::read_to_string(&recovery_path).unwrap();
    let mut wrong_recovery = recovery.trim().to_string();
    let mut bytes = wrong_recovery.as_bytes().to_vec();
    if let Some(index) = bytes.iter().rposition(|value| *value != b'=') {
        bytes[index] = if bytes[index] == b'A' { b'B' } else { b'A' };
    }
    wrong_recovery = String::from_utf8(bytes).unwrap();
    let wrong_recovery_out = run_err(env.cmd().args([
        "vault",
        "dec",
        recovery_vault.to_str().unwrap(),
        "-o",
        work.join("wrong-recovery.out").to_str().unwrap(),
        "--recovery-key",
        wrong_recovery.trim(),
    ]));
    assert_eq!(wrong_recovery_out.status.code(), Some(5));

    let dpapi_vault = work.join("dpapi.fv");
    run_ok(env.cmd().args([
        "vault",
        "enc",
        plain.to_str().unwrap(),
        "-o",
        dpapi_vault.to_str().unwrap(),
        "--dpapi",
    ]));
    let wrong_dpapi_out = run_err(env.cmd().args([
        "vault",
        "dec",
        dpapi_vault.to_str().unwrap(),
        "-o",
        work.join("wrong-dpapi.out").to_str().unwrap(),
        "--password",
        "not-used",
    ]));
    assert_eq!(wrong_dpapi_out.status.code(), Some(5));

    cleanup_dir(&work);
}

#[test]
fn filevault_verify_rejects_missing_frame() {
    let env = TestEnv::new();
    let work = make_safe_work_dir("filevault-missing-frame");
    let plain = work.join("plain.bin");
    let vault = work.join("plain.fv");
    fs::write(&plain, vec![0x2a; 3 * 65_536]).unwrap();

    run_ok(env.cmd().args([
        "vault",
        "enc",
        plain.to_str().unwrap(),
        "-o",
        vault.to_str().unwrap(),
        "--password",
        "hunter2",
        "--chunk-size",
        "65536",
    ]));

    let inspect = run_ok(env.cmd().args(["vault", "inspect", vault.to_str().unwrap(), "--json"]));
    let inspect_json = parse_stdout_json(&inspect);
    let first = inspect_json["layout"]["first_frame_offset"].as_u64().unwrap() as usize;
    let frame_span = inspect_json["layout"]["frame_span"].as_u64().unwrap() as usize;
    let footer_offset = inspect_json["layout"]["footer_offset"].as_u64().unwrap() as usize;
    let second = first + frame_span;
    let third = second + frame_span;

    let bytes = fs::read(&vault).unwrap();
    let footer = bytes[footer_offset..].to_vec();
    let mut mutated = Vec::with_capacity(bytes.len().saturating_sub(frame_span));
    mutated.extend_from_slice(&bytes[..second]);
    mutated.extend_from_slice(&bytes[third..footer_offset]);
    mutated.extend_from_slice(&footer);
    fs::write(&vault, mutated).unwrap();

    let verify = run_err(env.cmd().args(["vault", "verify", vault.to_str().unwrap(), "--json"]));
    let json = parse_stdout_json(&verify);
    assert_eq!(json["status"], "corrupt");
    assert!(json["payload"]["reason"].as_str().unwrap_or_default().contains("frame"));

    cleanup_dir(&work);
}

#[test]
fn filevault_verify_rejects_truncated_frame_payload() {
    let env = TestEnv::new();
    let work = make_safe_work_dir("filevault-truncated-frame");
    let plain = work.join("plain.bin");
    let vault = work.join("plain.fv");
    fs::write(&plain, vec![0x19; 2 * 65_536]).unwrap();

    run_ok(env.cmd().args([
        "vault",
        "enc",
        plain.to_str().unwrap(),
        "-o",
        vault.to_str().unwrap(),
        "--password",
        "hunter2",
        "--chunk-size",
        "65536",
    ]));

    let inspect = run_ok(env.cmd().args(["vault", "inspect", vault.to_str().unwrap(), "--json"]));
    let inspect_json = parse_stdout_json(&inspect);
    let footer_offset = inspect_json["layout"]["footer_offset"].as_u64().unwrap() as usize;
    let bytes = fs::read(&vault).unwrap();
    let footer = bytes[footer_offset..].to_vec();
    let cut_len = (footer.len() + 16).min(footer_offset);
    let mut mutated = Vec::with_capacity(bytes.len().saturating_sub(cut_len));
    mutated.extend_from_slice(&bytes[..footer_offset.saturating_sub(cut_len)]);
    mutated.extend_from_slice(&footer);
    fs::write(&vault, mutated).unwrap();

    let verify = run_err(env.cmd().args(["vault", "verify", vault.to_str().unwrap(), "--json"]));
    let json = parse_stdout_json(&verify);
    assert_eq!(json["status"], "corrupt");
    assert!(json["payload"]["reason"]
        .as_str()
        .unwrap_or_default()
        .contains("truncated"));

    cleanup_dir(&work);
}

#[test]
fn filevault_verify_rejects_duplicate_frame() {
    let env = TestEnv::new();
    let work = make_safe_work_dir("filevault-duplicate-frame");
    let plain = work.join("plain.bin");
    let vault = work.join("plain.fv");
    fs::write(&plain, vec![0x44; 2 * 65_536]).unwrap();

    run_ok(env.cmd().args([
        "vault",
        "enc",
        plain.to_str().unwrap(),
        "-o",
        vault.to_str().unwrap(),
        "--password",
        "hunter2",
        "--chunk-size",
        "65536",
    ]));

    let inspect = run_ok(env.cmd().args(["vault", "inspect", vault.to_str().unwrap(), "--json"]));
    let inspect_json = parse_stdout_json(&inspect);
    let first = inspect_json["layout"]["first_frame_offset"].as_u64().unwrap() as usize;
    let frame_span = inspect_json["layout"]["frame_span"].as_u64().unwrap() as usize;
    let second = first + frame_span;

    let mut bytes = fs::read(&vault).unwrap();
    let frame0 = bytes[first..first + frame_span].to_vec();
    bytes[second..second + frame_span].copy_from_slice(&frame0);
    fs::write(&vault, bytes).unwrap();

    let verify = run_err(env.cmd().args(["vault", "verify", vault.to_str().unwrap(), "--json"]));
    let json = parse_stdout_json(&verify);
    assert_eq!(json["status"], "corrupt");
    assert!(json["payload"]["reason"]
        .as_str()
        .unwrap_or_default()
        .contains("sequence"));

    cleanup_dir(&work);
}

#[test]
fn filevault_verify_reason_snapshots() {
    let env = TestEnv::new();
    let work = make_safe_work_dir("filevault-verify-reasons");
    let plain = work.join("plain.bin");
    let vault = work.join("plain.fv");
    fs::write(&plain, vec![0x1f; 2 * 65_536]).unwrap();

    run_ok(env.cmd().args([
        "vault",
        "enc",
        plain.to_str().unwrap(),
        "-o",
        vault.to_str().unwrap(),
        "--password",
        "hunter2",
        "--chunk-size",
        "65536",
    ]));

    let inspect = run_ok(env.cmd().args(["vault", "inspect", vault.to_str().unwrap(), "--json"]));
    let inspect_json = parse_stdout_json(&inspect);
    let footer_offset = inspect_json["layout"]["footer_offset"].as_u64().unwrap() as usize;
    let original = fs::read(&vault).unwrap();

    let run_case = |name: &str, bytes: Vec<u8>, expect: VerifyExpectation<'_>| {
        let path = work.join(format!("{name}.fv"));
        fs::write(&path, bytes).unwrap();
        let verify = run_err(env.cmd().args(["vault", "verify", path.to_str().unwrap(), "--json"]));
        let json = parse_stdout_json(&verify);
        assert_verify_snapshot(&json, &expect);
    };

    let mut header_tamper = original.clone();
    let tamper_at = 40usize.min(header_tamper.len().saturating_sub(1));
    header_tamper[tamper_at] ^= 0x3c;
    run_case(
        "header-mac-mismatch",
        header_tamper,
        VerifyExpectation {
            status: "corrupt",
            header_valid: false,
            payload_valid: true,
            footer_present: true,
            header_reason_contains: Some("header mac mismatch"),
            payload_reason_contains: None,
        },
    );

    let mut footer_missing = original.clone();
    footer_missing.truncate(footer_missing.len().saturating_sub(80));
    run_case(
        "footer-missing",
        footer_missing,
        VerifyExpectation {
            status: "incomplete",
            header_valid: false,
            payload_valid: false,
            footer_present: false,
            header_reason_contains: Some("header mac unavailable without footer"),
            payload_reason_contains: Some("payload digest unavailable without footer"),
        },
    );

    let mut digest_mismatch = original.clone();
    let digest_offset = footer_offset + 8;
    if digest_offset < digest_mismatch.len() {
        digest_mismatch[digest_offset] ^= 0x7f;
    }
    run_case(
        "payload-digest-mismatch",
        digest_mismatch,
        VerifyExpectation {
            status: "corrupt",
            header_valid: false,
            payload_valid: false,
            footer_present: true,
            header_reason_contains: Some("header mac not trusted because payload digest changed"),
            payload_reason_contains: Some("frame digest mismatch"),
        },
    );

    let layout_mismatch = original.clone();
    let footer = layout_mismatch[footer_offset..].to_vec();
    let mut mutated = Vec::with_capacity(layout_mismatch.len() + 16);
    mutated.extend_from_slice(&layout_mismatch[..footer_offset]);
    mutated.extend_from_slice(&vec![0u8; 16]);
    mutated.extend_from_slice(&footer);
    run_case(
        "frame-layout-mismatch",
        mutated,
        VerifyExpectation {
            status: "corrupt",
            header_valid: false,
            payload_valid: false,
            footer_present: true,
            header_reason_contains: Some("frame layout length mismatch"),
            payload_reason_contains: Some("frame layout length mismatch"),
        },
    );

    cleanup_dir(&work);
}
