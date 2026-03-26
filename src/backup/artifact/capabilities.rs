use crate::backup::artifact::sevenz::parse_sevenz_method_for_cli;
use crate::backup::artifact::zip::parse_zip_method_for_cli;
use crate::backup_formats::BackupArtifactFormat;
use crate::output::CliError;

#[derive(Clone, Copy)]
pub(crate) enum CommandValidationKind {
    Create,
    Convert,
}

impl CommandValidationKind {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Create => "backup create",
            Self::Convert => "backup convert",
        }
    }
}

#[derive(Clone, Copy)]
enum MethodValidationKind {
    Unsupported,
    Zip,
    SevenZ,
    #[cfg(feature = "xunbak")]
    XunbakCompression,
    #[cfg(not(feature = "xunbak"))]
    Unchecked,
}

#[derive(Clone, Copy)]
struct FormatCapabilities {
    split_fix: Option<&'static str>,
    solid_fix: Option<&'static str>,
    method_validation: MethodValidationKind,
    method_fix: Option<&'static str>,
    level_fix: Option<&'static str>,
    threads_fix: Option<&'static str>,
    password_fix: Option<&'static str>,
    encrypt_header_fix: Option<&'static str>,
}

const fn create_capabilities(format: BackupArtifactFormat) -> FormatCapabilities {
    match format {
        BackupArtifactFormat::Dir => FormatCapabilities {
            split_fix: Some("Remove `--split-size`; directory output does not support volumes."),
            solid_fix: Some("Remove `--solid`; directory output is not a compressed container."),
            method_validation: MethodValidationKind::Unsupported,
            method_fix: Some(
                "Remove `--method`; directory output does not have a compression method.",
            ),
            level_fix: Some("Remove `--level`; directory output does not compress data."),
            threads_fix: None,
            password_fix: None,
            encrypt_header_fix: None,
        },
        BackupArtifactFormat::Zip => FormatCapabilities {
            split_fix: Some("Remove `--split-size`; ZIP multi-disk output is not supported."),
            solid_fix: Some("Remove `--solid`; solid blocks are a 7z-only concept."),
            method_validation: MethodValidationKind::Zip,
            method_fix: None,
            level_fix: None,
            threads_fix: None,
            password_fix: None,
            encrypt_header_fix: None,
        },
        BackupArtifactFormat::SevenZ => FormatCapabilities {
            split_fix: None,
            solid_fix: None,
            method_validation: MethodValidationKind::SevenZ,
            method_fix: None,
            level_fix: None,
            threads_fix: None,
            password_fix: None,
            encrypt_header_fix: None,
        },
        BackupArtifactFormat::Xunbak => FormatCapabilities {
            split_fix: None,
            solid_fix: Some("Remove `--solid`; xunbak does not use 7z solid blocks."),
            method_validation: MethodValidationKind::Unsupported,
            method_fix: Some(
                "Remove `--method`; xunbak create uses `--compression`, not `--method`.",
            ),
            level_fix: Some("Remove `--level`; xunbak create uses `--compression zstd:N` instead."),
            threads_fix: None,
            password_fix: None,
            encrypt_header_fix: None,
        },
    }
}

const fn convert_capabilities(format: BackupArtifactFormat) -> FormatCapabilities {
    match format {
        BackupArtifactFormat::Dir => FormatCapabilities {
            split_fix: Some("Remove `--split-size`; directory output does not support volumes."),
            solid_fix: Some("Remove `--solid`; directory output is not a compressed container."),
            method_validation: MethodValidationKind::Unsupported,
            method_fix: Some(
                "Remove `--method`; directory output does not have a compression method.",
            ),
            level_fix: Some("Remove `--level`; directory output does not compress data."),
            threads_fix: Some("Remove `--threads`; directory output does not perform compression."),
            password_fix: Some(
                "Remove `--password`; directory output does not support encryption.",
            ),
            encrypt_header_fix: Some(
                "Remove `--encrypt-header`; directory output does not support encrypted headers.",
            ),
        },
        BackupArtifactFormat::Zip => FormatCapabilities {
            split_fix: Some("Remove `--split-size`; ZIP multi-disk output is not supported."),
            solid_fix: Some("Remove `--solid`; solid blocks are a 7z-only concept."),
            method_validation: MethodValidationKind::Zip,
            method_fix: None,
            level_fix: None,
            threads_fix: Some(
                "Remove `--threads`; ZIP export does not expose custom thread control.",
            ),
            password_fix: Some(
                "Remove `--password`; ZIP encryption is out of scope for the current implementation.",
            ),
            encrypt_header_fix: Some(
                "Remove `--encrypt-header`; ZIP header encryption is not supported.",
            ),
        },
        BackupArtifactFormat::SevenZ => FormatCapabilities {
            split_fix: None,
            solid_fix: None,
            method_validation: MethodValidationKind::SevenZ,
            method_fix: None,
            level_fix: None,
            threads_fix: Some(
                "Remove `--threads`; custom 7z thread control is not implemented yet.",
            ),
            password_fix: Some("Remove `--password`; 7z encryption is not implemented yet."),
            encrypt_header_fix: Some(
                "Remove `--encrypt-header`; 7z header encryption is not implemented yet.",
            ),
        },
        BackupArtifactFormat::Xunbak => FormatCapabilities {
            split_fix: None,
            solid_fix: Some("Remove `--solid`; xunbak does not use 7z solid blocks."),
            #[cfg(feature = "xunbak")]
            method_validation: MethodValidationKind::XunbakCompression,
            #[cfg(not(feature = "xunbak"))]
            method_validation: MethodValidationKind::Unchecked,
            method_fix: None,
            level_fix: None,
            threads_fix: Some(
                "Remove `--threads`; xunbak export thread control is not implemented.",
            ),
            password_fix: Some(
                "Remove `--password`; xunbak encryption is not part of the current export path.",
            ),
            encrypt_header_fix: Some(
                "Remove `--encrypt-header`; xunbak header encryption is not implemented.",
            ),
        },
    }
}

pub(crate) fn validate_create_capabilities(
    format: BackupArtifactFormat,
    split_size_present: bool,
    solid_present: bool,
    method: Option<&str>,
    level_present: bool,
) -> Result<(), CliError> {
    let caps = create_capabilities(format);
    reject_with_capability(
        CommandValidationKind::Create,
        format,
        split_size_present,
        "--split-size",
        caps.split_fix,
    )?;
    reject_with_capability(
        CommandValidationKind::Create,
        format,
        solid_present,
        "--solid",
        caps.solid_fix,
    )?;
    validate_method_with_capability(CommandValidationKind::Create, format, method, caps)?;
    reject_with_capability(
        CommandValidationKind::Create,
        format,
        level_present,
        "--level",
        caps.level_fix,
    )?;
    Ok(())
}

pub(crate) fn validate_convert_capabilities(
    format: BackupArtifactFormat,
    split_size_present: bool,
    solid_present: bool,
    method: Option<&str>,
    level_present: bool,
    threads_present: bool,
    password_present: bool,
    encrypt_header_present: bool,
) -> Result<(), CliError> {
    let caps = convert_capabilities(format);
    reject_with_capability(
        CommandValidationKind::Convert,
        format,
        split_size_present,
        "--split-size",
        caps.split_fix,
    )?;
    reject_with_capability(
        CommandValidationKind::Convert,
        format,
        solid_present,
        "--solid",
        caps.solid_fix,
    )?;
    validate_method_with_capability(CommandValidationKind::Convert, format, method, caps)?;
    reject_with_capability(
        CommandValidationKind::Convert,
        format,
        level_present,
        "--level",
        caps.level_fix,
    )?;
    reject_with_capability(
        CommandValidationKind::Convert,
        format,
        threads_present,
        "--threads",
        caps.threads_fix,
    )?;
    reject_with_capability(
        CommandValidationKind::Convert,
        format,
        password_present,
        "--password",
        caps.password_fix,
    )?;
    reject_with_capability(
        CommandValidationKind::Convert,
        format,
        encrypt_header_present,
        "--encrypt-header",
        caps.encrypt_header_fix,
    )?;
    Ok(())
}

fn reject_with_capability(
    command: CommandValidationKind,
    format: BackupArtifactFormat,
    present: bool,
    flag: &str,
    fix: Option<&'static str>,
) -> Result<(), CliError> {
    if !present {
        return Ok(());
    }
    let Some(fix) = fix else {
        return Ok(());
    };
    let fix = if fix.trim_start().starts_with("Fix:") {
        fix.to_string()
    } else {
        format!("Fix: {fix}")
    };
    Err(CliError::with_details(
        2,
        format!(
            "{} {flag} is invalid for {} output",
            command.label(),
            format.as_str()
        ),
        &[fix],
    ))
}

fn validate_method_with_capability(
    command: CommandValidationKind,
    format: BackupArtifactFormat,
    method: Option<&str>,
    caps: FormatCapabilities,
) -> Result<(), CliError> {
    let Some(method) = method else {
        return Ok(());
    };
    match caps.method_validation {
        MethodValidationKind::Unsupported => {
            reject_with_capability(command, format, true, "--method", caps.method_fix)
        }
        MethodValidationKind::Zip => {
            parse_zip_method_for_cli(command.label(), Some(method)).map(|_| ())
        }
        MethodValidationKind::SevenZ => {
            parse_sevenz_method_for_cli(command.label(), Some(method)).map(|_| ())
        }
        #[cfg(feature = "xunbak")]
        MethodValidationKind::XunbakCompression => validate_xunbak_compression(command, method),
        #[cfg(not(feature = "xunbak"))]
        MethodValidationKind::Unchecked => Ok(()),
    }
}

#[cfg(feature = "xunbak")]
fn validate_xunbak_compression(
    _command: CommandValidationKind,
    method: &str,
) -> Result<(), CliError> {
    use crate::xunbak::codec::{XUNBAK_COMPRESSION_PROFILE_FIX_HINT, parse_compression_arg};

    parse_compression_arg(method).map_err(|err| {
        CliError::with_details(
            2,
            err.to_string(),
            &[String::from(XUNBAK_COMPRESSION_PROFILE_FIX_HINT)],
        )
    })?;
    Ok(())
}
