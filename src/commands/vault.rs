use std::path::{Path, PathBuf};

use crate::cli::{
    VaultCleanupCmd, VaultCmd, VaultDecCmd, VaultEncCmd, VaultInspectCmd, VaultRecoverKeyCmd,
    VaultResumeCmd, VaultRewrapCmd, VaultSubCommand, VaultVerifyCmd,
};
use crate::filevault::{
    DecryptOptions, EncryptOptions, FileVaultError, KdfKind, PayloadAlgorithm, RecoverKeyOptions,
    RewrapOptions, SlotKind, UnlockOptions, VerifyOptions, cleanup_artifacts, decrypt_file,
    encrypt_file, inspect_file, recover_key_file, resume_file, rewrap_file, verify_file,
};
use crate::output::{CliError, CliResult};

pub(crate) fn cmd_vault(args: VaultCmd) -> CliResult {
    match args.cmd {
        VaultSubCommand::Enc(cmd) => cmd_enc(cmd),
        VaultSubCommand::Dec(cmd) => cmd_dec(cmd),
        VaultSubCommand::Inspect(cmd) => cmd_inspect(cmd),
        VaultSubCommand::Verify(cmd) => cmd_verify(cmd),
        VaultSubCommand::Resume(cmd) => cmd_resume(cmd),
        VaultSubCommand::Cleanup(cmd) => cmd_cleanup(cmd),
        VaultSubCommand::Rewrap(cmd) => cmd_rewrap(cmd),
        VaultSubCommand::RecoverKey(cmd) => cmd_recover_key(cmd),
    }
}

fn cmd_enc(args: VaultEncCmd) -> CliResult {
    let options = EncryptOptions {
        input: PathBuf::from(&args.input),
        output: args
            .output
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(format!("{}.fv", args.input))),
        password: args.password,
        keyfile: args.keyfile.map(PathBuf::from),
        recovery_key: args.recovery_key,
        emit_recovery_key: args.emit_recovery_key.map(PathBuf::from),
        dpapi: args.dpapi,
        payload_algorithm: PayloadAlgorithm::from_cli(&args.algo).map_err(map_error)?,
        kdf: KdfKind::from_cli(&args.kdf).map_err(map_error)?,
        chunk_size: args.chunk_size,
    };
    let value = encrypt_file(&options).map_err(map_error)?;
    emit_result(&value, args.json, &format!("已加密到 {}", options.output.display()));
    Ok(())
}

fn cmd_dec(args: VaultDecCmd) -> CliResult {
    let input = PathBuf::from(&args.input);
    let output = args
        .output
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_decrypt_output(&input));
    let value = decrypt_file(&DecryptOptions {
        input,
        output: output.clone(),
        unlock: unlock_options(
            args.password,
            args.keyfile.map(PathBuf::from),
            args.recovery_key,
            args.dpapi,
        ),
    })
    .map_err(map_error)?;
    emit_result(&value, args.json, &format!("已解密到 {}", output.display()));
    Ok(())
}

fn cmd_inspect(args: VaultInspectCmd) -> CliResult {
    let value = inspect_file(Path::new(&args.path)).map_err(map_error)?;
    emit_result(&value, args.json, &format!("已检查 {}", args.path));
    Ok(())
}

fn cmd_verify(args: VaultVerifyCmd) -> CliResult {
    let result = verify_file(&VerifyOptions {
        input: PathBuf::from(&args.path),
        unlock: unlock_options(
            args.password,
            args.keyfile.map(PathBuf::from),
            args.recovery_key,
            args.dpapi,
        ),
    });
    match result {
        Ok(value) => {
            emit_result(&value, args.json, &format!("校验通过 {}", args.path));
            Ok(())
        }
        Err(FileVaultError::Verify(payload)) if payload.trim_start().starts_with('{') => {
            if args.json {
                out_println!("{}", payload);
            }
            Err(CliError::new(5, "vault verify failed"))
        }
        Err(err) => Err(map_error(err)),
    }
}

fn cmd_resume(args: VaultResumeCmd) -> CliResult {
    let _resume_unlock = unlock_options(
        args.password,
        args.keyfile.map(PathBuf::from),
        args.recovery_key,
        args.dpapi,
    );
    let value = resume_file(Path::new(&args.path)).map_err(map_error)?;
    emit_result(&value, args.json, &format!("已恢复写入 {}", args.path));
    Ok(())
}

fn cmd_cleanup(args: VaultCleanupCmd) -> CliResult {
    let value = cleanup_artifacts(Path::new(&args.path)).map_err(map_error)?;
    emit_result(&value, args.json, &format!("已清理 {} 的临时文件", args.path));
    Ok(())
}

fn cmd_rewrap(args: VaultRewrapCmd) -> CliResult {
    let remove_slots = args
        .remove_slot
        .iter()
        .map(|value| SlotKind::from_cli(value))
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_error)?;
    let path = PathBuf::from(&args.path);
    let value = rewrap_file(&RewrapOptions {
        path: path.clone(),
        unlock: unlock_options(
            args.unlock_password,
            args.unlock_keyfile.map(PathBuf::from),
            args.unlock_recovery_key,
            args.unlock_dpapi,
        ),
        add_password: args.add_password,
        add_keyfile: args.add_keyfile.map(PathBuf::from),
        add_recovery_key: args.add_recovery_key,
        emit_recovery_key: args.emit_recovery_key.map(PathBuf::from),
        add_dpapi: args.add_dpapi,
        remove_slots,
        kdf: KdfKind::from_cli(&args.kdf).map_err(map_error)?,
    })
    .map_err(map_error)?;
    emit_result(&value, args.json, &format!("已更新 {} 的 slots", path.display()));
    Ok(())
}

fn cmd_recover_key(args: VaultRecoverKeyCmd) -> CliResult {
    let output = PathBuf::from(&args.output);
    let value = recover_key_file(&RecoverKeyOptions {
        path: PathBuf::from(&args.path),
        unlock: unlock_options(
            args.unlock_password,
            args.unlock_keyfile.map(PathBuf::from),
            args.unlock_recovery_key,
            args.unlock_dpapi,
        ),
        output: output.clone(),
    })
    .map_err(map_error)?;
    emit_result(&value, args.json, &format!("已导出 recovery key 到 {}", output.display()));
    Ok(())
}

fn unlock_options(
    password: Option<String>,
    keyfile: Option<PathBuf>,
    recovery_key: Option<String>,
    dpapi: bool,
) -> UnlockOptions {
    UnlockOptions {
        password,
        keyfile,
        recovery_key,
        dpapi,
    }
}

fn default_decrypt_output(input: &Path) -> PathBuf {
    if input.extension().and_then(|value| value.to_str()) == Some("fv") {
        let mut output = input.to_path_buf();
        output.set_extension("");
        output
    } else {
        PathBuf::from(format!("{}.decrypted", input.to_string_lossy()))
    }
}

fn emit_result(value: &serde_json::Value, json_mode: bool, human: &str) {
    if json_mode {
        out_println!("{}", value);
    } else {
        ui_println!("{human}");
    }
}

fn map_error(err: FileVaultError) -> CliError {
    match err {
        FileVaultError::InvalidArgument(message)
        | FileVaultError::InvalidFormat(message)
        | FileVaultError::Unsupported(message) => CliError::new(2, message),
        FileVaultError::Slot(message)
        | FileVaultError::Verify(message)
        | FileVaultError::Resume(message) => CliError::new(5, message),
        FileVaultError::Crypto(message) => CliError::new(5, message),
        FileVaultError::Io(err) => CliError::new(1, err.to_string()),
        FileVaultError::Json(err) => CliError::new(1, err.to_string()),
    }
}
