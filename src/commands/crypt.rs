use std::path::PathBuf;

use crate::cli::{DecryptCmd, EncryptCmd};
use crate::output::{CliError, CliResult};

pub(crate) fn cmd_encrypt(args: EncryptCmd) -> CliResult {
    let mut input_policy = crate::path_guard::PathPolicy::for_read();
    input_policy.allow_relative = true;
    let input_validation =
        crate::path_guard::validate_paths(vec![args.path.clone()], &input_policy);
    if !input_validation.issues.is_empty() {
        let mut details: Vec<String> = input_validation
            .issues
            .iter()
            .map(|issue| format!("Invalid input path: {} ({})", issue.raw, issue.detail))
            .collect();
        details.push("Hint: Check the path exists before encrypting.".to_string());
        return Err(CliError::with_details(
            2,
            "Invalid input path.".to_string(),
            &details,
        ));
    }
    let path = input_validation
        .ok
        .into_iter()
        .next()
        .map(|p| PathBuf::from(p))
        .unwrap_or_else(|| PathBuf::from(&args.path));

    if args.efs {
        match crate::windows::volume::is_volume_efs_capable(&path) {
            Ok(true) => match crate::windows::efs::encrypt_file(&path) {
                Ok(_) => {
                    ui_println!("Successfully encrypted {:?} using Windows EFS.", path);
                    crate::security::audit::audit_log(
                        "encrypt_efs",
                        &args.path,
                        "cli",
                        "",
                        "success",
                        "",
                    );
                }
                Err(crate::windows::efs::EfsError::SharingViolation) => {
                    return Err(CliError::with_details(
                        crate::util::EXIT_ACCESS_DENIED,
                        "EFS Encryption failed: file is in use by another process.",
                        &[format!(
                            "Hint: Try `xun lock who {}` to find the blocking process.",
                            path.display()
                        )],
                    ));
                }
                Err(e) => {
                    return Err(CliError::new(
                        crate::util::EXIT_ACCESS_DENIED,
                        format!("EFS Encryption failed: {e}"),
                    ));
                }
            },
            Ok(false) => {
                return Err(CliError::new(
                    crate::util::EXIT_ACCESS_DENIED,
                    format!(
                        "The volume where {} resides does not support Windows EFS.",
                        path.display()
                    ),
                ));
            }
            Err(e) => {
                return Err(CliError::new(
                    crate::util::EXIT_ACCESS_DENIED,
                    format!("Failed to query volume capabilities: OS Error {e}"),
                ));
            }
        }
    } else {
        let out_path = if let Some(out) = &args.out {
            PathBuf::from(out)
        } else {
            let mut p = path.to_path_buf();
            let mut f = p.file_name().unwrap_or_default().to_os_string();
            f.push(".age");
            p.set_file_name(f);
            p
        };
        let mut out_policy = crate::path_guard::PathPolicy::for_output();
        out_policy.allow_relative = true;
        let out_validation = crate::path_guard::validate_paths(
            vec![out_path.to_string_lossy().to_string()],
            &out_policy,
        );
        if !out_validation.issues.is_empty() {
            let mut details: Vec<String> = out_validation
                .issues
                .iter()
                .map(|issue| format!("Invalid output path: {} ({})", issue.raw, issue.detail))
                .collect();
            details.push("Fix: Use a valid output file path.".to_string());
            return Err(CliError::with_details(
                2,
                "Invalid output path.".to_string(),
                &details,
            ));
        }
        let out_path = out_validation.ok.into_iter().next().unwrap_or(out_path);

        if args.passphrase {
            let pass = dialoguer::Password::new()
                .with_prompt("Enter passphrase for encryption")
                .with_confirmation("Confirm passphrase", "Passphrases mismatching")
                .interact()
                .unwrap_or_default();

            if pass.is_empty() {
                return Err(CliError::new(2, "Encryption aborted: empty passphrase."));
            }

            let secret = age::secrecy::SecretString::from(pass);
            match crate::age_wrapper::encrypt_with_passphrase(&path, &out_path, secret) {
                Ok(_) => {
                    ui_println!(
                        "Successfully encrypted to {:?} using age (passphrase).",
                        out_path
                    );
                    crate::security::audit::audit_log(
                        "encrypt_age",
                        &args.path,
                        "cli",
                        "",
                        "success",
                        "passphrase",
                    );
                }
                Err(e) => {
                    return Err(CliError::new(5, format!("Age Encryption failed: {e}")));
                }
            }
        } else if !args.to.is_empty() {
            match crate::age_wrapper::encrypt_to_recipients(&path, &out_path, args.to) {
                Ok(_) => {
                    ui_println!(
                        "Successfully encrypted to {:?} using age (recipients).",
                        out_path
                    );
                    crate::security::audit::audit_log(
                        "encrypt_age",
                        &args.path,
                        "cli",
                        "",
                        "success",
                        "recipients",
                    );
                }
                Err(e) => {
                    return Err(CliError::new(5, format!("Age Encryption failed: {e}")));
                }
            }
        } else {
            return Err(CliError::with_details(
                2,
                "Please specify either --efs, --passphrase, or --to <recipients> for encryption."
                    .to_string(),
                &["Fix: Add --efs, --passphrase, or --to <recipient>."],
            ));
        }
    }

    Ok(())
}

pub(crate) fn cmd_decrypt(args: DecryptCmd) -> CliResult {
    let mut input_policy = crate::path_guard::PathPolicy::for_read();
    input_policy.allow_relative = true;
    let input_validation =
        crate::path_guard::validate_paths(vec![args.path.clone()], &input_policy);
    if !input_validation.issues.is_empty() {
        let mut details: Vec<String> = input_validation
            .issues
            .iter()
            .map(|issue| format!("Invalid input path: {} ({})", issue.raw, issue.detail))
            .collect();
        details.push("Hint: Check the path exists before decrypting.".to_string());
        return Err(CliError::with_details(
            2,
            "Invalid input path.".to_string(),
            &details,
        ));
    }
    let path = input_validation
        .ok
        .into_iter()
        .next()
        .map(|p| PathBuf::from(p))
        .unwrap_or_else(|| PathBuf::from(&args.path));

    if args.efs {
        match crate::windows::efs::decrypt_file(&path) {
            Ok(_) => {
                ui_println!("Successfully decrypted {:?} using Windows EFS.", path);
                crate::security::audit::audit_log(
                    "decrypt_efs",
                    &args.path,
                    "cli",
                    "",
                    "success",
                    "",
                );
            }
            Err(crate::windows::efs::EfsError::SharingViolation) => {
                return Err(CliError::with_details(
                    crate::util::EXIT_ACCESS_DENIED,
                    "EFS Decryption failed: file is in use by another process.",
                    &[format!(
                        "Hint: Try `xun lock who {}` to find the blocking process.",
                        path.display()
                    )],
                ));
            }
            Err(e) => {
                return Err(CliError::new(
                    crate::util::EXIT_ACCESS_DENIED,
                    format!("EFS Decryption failed: {e}"),
                ));
            }
        }
    } else {
        let out_path = if let Some(out) = &args.out {
            PathBuf::from(out)
        } else {
            let mut p = path.to_path_buf();
            if let Some(ext) = p.extension() {
                if ext == "age" {
                    p.set_extension("");
                } else {
                    let mut f = p.file_name().unwrap_or_default().to_os_string();
                    f.push(".decrypted");
                    p.set_file_name(f);
                }
            } else {
                let mut f = p.file_name().unwrap_or_default().to_os_string();
                f.push(".decrypted");
                p.set_file_name(f);
            }
            p
        };
        let mut out_policy = crate::path_guard::PathPolicy::for_output();
        out_policy.allow_relative = true;
        let out_validation = crate::path_guard::validate_paths(
            vec![out_path.to_string_lossy().to_string()],
            &out_policy,
        );
        if !out_validation.issues.is_empty() {
            let mut details: Vec<String> = out_validation
                .issues
                .iter()
                .map(|issue| format!("Invalid output path: {} ({})", issue.raw, issue.detail))
                .collect();
            details.push("Fix: Use a valid output file path.".to_string());
            return Err(CliError::with_details(
                2,
                "Invalid output path.".to_string(),
                &details,
            ));
        }
        let out_path = out_validation.ok.into_iter().next().unwrap_or(out_path);

        if args.passphrase || !args.identity.is_empty() {
            let pass = if args.passphrase {
                let p = dialoguer::Password::new()
                    .with_prompt("Enter passphrase for decryption")
                    .interact()
                    .unwrap_or_default();
                if p.is_empty() {
                    return Err(CliError::new(2, "Decryption aborted: empty passphrase."));
                }
                Some(age::secrecy::SecretString::from(p))
            } else {
                None
            };

            match crate::age_wrapper::decrypt_file(&path, &out_path, pass, args.identity) {
                Ok(_) => {
                    ui_println!("Successfully decrypted to {:?} using age.", out_path);
                    crate::security::audit::audit_log(
                        "decrypt_age",
                        &args.path,
                        "cli",
                        "",
                        "success",
                        "",
                    );
                }
                Err(e) => {
                    return Err(CliError::new(5, format!("Age Decryption failed: {e}")));
                }
            }
        } else {
            return Err(CliError::with_details(
                2,
                "Please specify either --efs, --passphrase, or --identity <file> for decryption."
                    .to_string(),
                &["Fix: Add --efs, --passphrase, or --identity <file>."],
            ));
        }
    }

    Ok(())
}
