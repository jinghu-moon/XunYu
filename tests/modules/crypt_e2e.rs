#![cfg(all(windows, feature = "crypt"))]

#[path = "../support/mod.rs"]
mod common;

use age::secrecy::ExposeSecret;
use common::*;
use serde_json::Value;
use std::fs;

#[test]
fn crypt_age_recipient_roundtrip_and_audit() {
    let env = TestEnv::new();
    let work = make_safe_work_dir("crypt-age");
    let src = work.join("plain.txt");
    let enc = work.join("plain.txt.age");
    let dec = work.join("plain.dec.txt");
    let identity_path = work.join("id.txt");
    let content = "xun-crypt-age-roundtrip";
    fs::write(&src, content).unwrap();

    let id = age::x25519::Identity::generate();
    let recipient = id.to_public().to_string();
    fs::write(
        &identity_path,
        format!("{}\n", id.to_string().expose_secret()),
    )
    .unwrap();

    run_ok(env.cmd().args([
        "encrypt",
        src.to_str().unwrap(),
        "--to",
        &recipient,
        "--out",
        enc.to_str().unwrap(),
    ]));
    assert!(enc.exists(), "encrypted output should exist");

    run_ok(env.cmd().args([
        "decrypt",
        enc.to_str().unwrap(),
        "--identity",
        identity_path.to_str().unwrap(),
        "--out",
        dec.to_str().unwrap(),
    ]));
    assert_eq!(
        fs::read_to_string(&dec).unwrap(),
        content,
        "decrypted content should match source"
    );

    let audit_path = env.audit_path();
    assert!(
        audit_path.exists(),
        "audit file should exist: {:?}",
        audit_path
    );
    let entries: Vec<Value> = fs::read_to_string(&audit_path)
        .unwrap()
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect();

    assert!(
        entries.iter().any(|e| {
            e.get("action").and_then(Value::as_str) == Some("encrypt_age")
                && e.get("result").and_then(Value::as_str) == Some("success")
        }),
        "encrypt_age success audit entry missing"
    );
    assert!(
        entries.iter().any(|e| {
            e.get("action").and_then(Value::as_str) == Some("decrypt_age")
                && e.get("result").and_then(Value::as_str) == Some("success")
        }),
        "decrypt_age success audit entry missing"
    );

    cleanup_dir(&work);
}

#[test]
fn crypt_age_invalid_recipient_fails() {
    let env = TestEnv::new();
    let work = make_safe_work_dir("crypt-age-invalid");
    let src = work.join("plain.txt");
    let enc = work.join("plain.txt.age");
    fs::write(&src, "invalid-recipient").unwrap();

    let out = run_err(env.cmd().args([
        "encrypt",
        src.to_str().unwrap(),
        "--to",
        "not-a-valid-age-recipient",
        "--out",
        enc.to_str().unwrap(),
    ]));
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(5),
        "invalid recipient should map to exit code 5; stderr={err}"
    );
    assert!(
        err.contains("Age Encryption failed"),
        "expected age error message, got: {err}"
    );
    assert!(!enc.exists(), "invalid recipient should not create output");

    cleanup_dir(&work);
}

#[test]
fn crypt_decrypt_requires_identity_or_passphrase() {
    let env = TestEnv::new();
    let work = make_safe_work_dir("crypt-age-identity-required");
    let src = work.join("plain.txt");
    let enc = work.join("plain.txt.age");
    fs::write(&src, "identity-required").unwrap();

    let id = age::x25519::Identity::generate();
    let recipient = id.to_public().to_string();
    run_ok(env.cmd().args([
        "encrypt",
        src.to_str().unwrap(),
        "--to",
        &recipient,
        "--out",
        enc.to_str().unwrap(),
    ]));

    let out = run_err(env.cmd().args(["decrypt", enc.to_str().unwrap()]));
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(2),
        "missing identity/passphrase should map to exit code 2; stderr={err}"
    );
    assert!(
        err.contains("Please specify either --efs, --passphrase, or --identity"),
        "unexpected stderr: {err}"
    );

    cleanup_dir(&work);
}

#[test]
fn crypt_encrypt_passphrase_non_interactive_aborts_fast() {
    let env = TestEnv::new();
    let work = make_safe_work_dir("crypt-passphrase-non-interactive");
    let src = work.join("plain.txt");
    fs::write(&src, "passphrase-abort").unwrap();

    let out = run_err(
        env.cmd()
            .args(["encrypt", src.to_str().unwrap(), "--passphrase"]),
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(2),
        "non-interactive passphrase should abort with code 2; stderr={err}"
    );
    assert!(
        err.contains("Encryption aborted: empty passphrase"),
        "unexpected stderr: {err}"
    );

    cleanup_dir(&work);
}

#[test]
fn crypt_efs_roundtrip_or_reports_capability_issue() {
    let env = TestEnv::new();
    let work = make_safe_work_dir("crypt-efs");
    let src = work.join("efs.txt");
    fs::write(&src, "efs-roundtrip").unwrap();

    let out = run_raw(env.cmd().args(["encrypt", src.to_str().unwrap(), "--efs"]));
    if out.status.success() {
        run_ok(env.cmd().args(["decrypt", src.to_str().unwrap(), "--efs"]));
        assert_eq!(
            fs::read_to_string(&src).unwrap(),
            "efs-roundtrip",
            "efs decrypt should keep file readable"
        );
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        assert_eq!(
            out.status.code(),
            Some(3),
            "efs failure should map to exit code 3; stderr={err}"
        );
        assert!(
            err.contains("does not support Windows EFS")
                || err.contains("EFS Encryption failed")
                || err.contains("Failed to query volume capabilities"),
            "unexpected efs error: {err}"
        );
    }

    cleanup_dir(&work);
}
