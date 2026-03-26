use std::path::PathBuf;

use crate::backup::artifact::capabilities::{
    validate_convert_capabilities, validate_create_capabilities,
};
use crate::backup::artifact::selection::SelectionSpec;
use crate::backup_formats::{
    BackupArtifactFormat, OverwriteMode, ProgressMode, VerifyOutputMode, VerifySourceMode,
};
use crate::cli::{BackupConvertCmd, BackupCreateCmd, BackupRestoreCmd};
use crate::output::CliError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackupCreateOptions {
    pub message: Option<String>,
    pub source_dir: PathBuf,
    pub format: BackupArtifactFormat,
    pub output: Option<PathBuf>,
    pub compression: Option<String>,
    pub split_size: Option<String>,
    pub solid: bool,
    pub method: Option<String>,
    pub level: Option<u32>,
    pub dry_run: bool,
    pub list: bool,
    pub no_compress: bool,
    pub retain: Option<usize>,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub incremental: bool,
    pub skip_if_unchanged: bool,
    pub progress: ProgressMode,
    pub json: bool,
    pub no_sidecar: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackupConvertOptions {
    pub artifact: PathBuf,
    pub format: BackupArtifactFormat,
    pub output: PathBuf,
    pub selection: SelectionSpec,
    pub split_size: Option<String>,
    pub solid: bool,
    pub method: Option<String>,
    pub level: Option<u32>,
    pub threads: Option<u32>,
    pub password: Option<String>,
    pub encrypt_header: bool,
    pub overwrite: OverwriteMode,
    pub dry_run: bool,
    pub list: bool,
    pub verify_source: VerifySourceMode,
    pub verify_output: VerifyOutputMode,
    pub progress: ProgressMode,
    pub json: bool,
    pub no_sidecar: bool,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackupRestoreOptions {
    pub name_or_path: String,
    pub file: Option<String>,
    pub glob: Option<String>,
    pub destination: Option<PathBuf>,
    pub snapshot: bool,
    pub project_dir: PathBuf,
    pub dry_run: bool,
    pub yes: bool,
    pub json: bool,
}

impl TryFrom<BackupCreateCmd> for BackupCreateOptions {
    type Error = CliError;

    fn try_from(value: BackupCreateCmd) -> Result<Self, Self::Error> {
        let source_dir = match value.dir {
            Some(dir) => PathBuf::from(dir),
            None => std::env::current_dir().map_err(|err| {
                CliError::new(1, format!("Failed to get current directory: {err}"))
            })?,
        };
        let format = value
            .format
            .as_deref()
            .map(str::parse::<BackupArtifactFormat>)
            .transpose()
            .map_err(|err| CliError::new(2, err))?
            .unwrap_or(BackupArtifactFormat::Dir);

        let options = Self {
            message: value.msg,
            source_dir,
            format,
            output: value.output.map(PathBuf::from),
            compression: value.compression,
            split_size: value.split_size,
            solid: value.solid,
            method: value.method,
            level: value.level,
            dry_run: value.dry_run,
            list: value.list,
            no_compress: value.no_compress,
            retain: value.retain,
            include: value.include,
            exclude: value.exclude,
            incremental: value.incremental,
            skip_if_unchanged: value.skip_if_unchanged,
            progress: value
                .progress
                .as_deref()
                .map(str::parse::<ProgressMode>)
                .transpose()
                .map_err(|err| CliError::new(2, err))?
                .unwrap_or(ProgressMode::Auto),
            json: value.json,
            no_sidecar: value.no_sidecar,
        };
        options.validate()?;
        Ok(options)
    }
}

impl BackupCreateOptions {
    fn validate(&self) -> Result<(), CliError> {
        validate_create_capabilities(
            self.format,
            self.split_size.is_some(),
            self.solid,
            self.method.as_deref(),
            self.level.is_some(),
        )?;
        Ok(())
    }
}

impl TryFrom<BackupConvertCmd> for BackupConvertOptions {
    type Error = CliError;

    fn try_from(value: BackupConvertCmd) -> Result<Self, Self::Error> {
        let options = Self {
            artifact: PathBuf::from(value.artifact),
            format: value
                .format
                .parse::<BackupArtifactFormat>()
                .map_err(|err| CliError::new(2, err))?,
            output: PathBuf::from(value.output),
            selection: SelectionSpec::from_inputs(&value.file, &value.glob, &value.patterns_from)
                .map_err(|err| CliError::new(2, err))?,
            split_size: value.split_size,
            solid: value.solid,
            method: value.method,
            level: value.level,
            threads: value.threads,
            password: value.password,
            encrypt_header: value.encrypt_header,
            overwrite: value
                .overwrite
                .as_deref()
                .map(str::parse::<OverwriteMode>)
                .transpose()
                .map_err(|err| CliError::new(2, err))?
                .unwrap_or(OverwriteMode::Ask),
            dry_run: value.dry_run,
            list: value.list,
            verify_source: value
                .verify_source
                .as_deref()
                .map(str::parse::<VerifySourceMode>)
                .transpose()
                .map_err(|err| CliError::new(2, err))?
                .unwrap_or(VerifySourceMode::Quick),
            verify_output: value
                .verify_output
                .as_deref()
                .map(str::parse::<VerifyOutputMode>)
                .transpose()
                .map_err(|err| CliError::new(2, err))?
                .unwrap_or(VerifyOutputMode::On),
            progress: value
                .progress
                .as_deref()
                .map(str::parse::<ProgressMode>)
                .transpose()
                .map_err(|err| CliError::new(2, err))?
                .unwrap_or(ProgressMode::Auto),
            json: value.json,
            no_sidecar: value.no_sidecar,
        };
        options.validate()?;
        Ok(options)
    }
}

impl BackupConvertOptions {
    fn validate(&self) -> Result<(), CliError> {
        if self.encrypt_header && self.password.is_none() {
            return Err(CliError::with_details(
                2,
                "backup convert --encrypt-header requires --password",
                &["Fix: Add `--password <secret>` or remove `--encrypt-header`."],
            ));
        }

        validate_convert_capabilities(
            self.format,
            self.split_size.is_some(),
            self.solid,
            self.method.as_deref(),
            self.level.is_some(),
            self.threads.is_some(),
            self.password.is_some(),
            self.encrypt_header,
        )?;
        Ok(())
    }
}

impl TryFrom<BackupRestoreCmd> for BackupRestoreOptions {
    type Error = CliError;

    fn try_from(value: BackupRestoreCmd) -> Result<Self, Self::Error> {
        let project_dir = match value.dir {
            Some(dir) => PathBuf::from(dir),
            None => std::env::current_dir().map_err(|err| {
                CliError::new(1, format!("Failed to get current directory: {err}"))
            })?,
        };

        Ok(Self {
            name_or_path: value.name_or_path,
            file: value.file,
            glob: value.glob,
            destination: value.to.map(PathBuf::from),
            snapshot: value.snapshot,
            project_dir,
            dry_run: value.dry_run,
            yes: value.yes,
            json: value.json,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use tempfile::tempdir;

    use crate::backup_formats::{
        BackupArtifactFormat, OverwriteMode, ProgressMode, VerifyOutputMode, VerifySourceMode,
    };
    use crate::cli::{BackupConvertCmd, BackupCreateCmd, BackupRestoreCmd};

    use super::{BackupConvertOptions, BackupCreateOptions, BackupRestoreOptions};

    #[test]
    fn backup_create_options_default_to_dir_format() {
        let options = BackupCreateOptions::try_from(BackupCreateCmd {
            msg: None,
            dir: None,
            format: None,
            output: None,
            compression: None,
            split_size: None,
            solid: false,
            method: None,
            level: None,
            dry_run: false,
            list: false,
            no_compress: false,
            retain: None,
            include: Vec::new(),
            exclude: Vec::new(),
            incremental: false,
            skip_if_unchanged: false,
            diff_mode: None,
            progress: None,
            json: false,
            no_sidecar: false,
        })
        .unwrap();

        assert_eq!(options.format, BackupArtifactFormat::Dir);
    }

    #[test]
    fn backup_convert_options_parse_defaults_and_patterns_from() {
        let dir = tempdir().unwrap();
        let patterns = dir.path().join("patterns.txt");
        fs::write(&patterns, "*.md\n").unwrap();

        let options = BackupConvertOptions::try_from(BackupConvertCmd {
            artifact: "input".to_string(),
            format: "zip".to_string(),
            output: "out.zip".to_string(),
            file: vec!["README.md".to_string()],
            glob: vec!["src/*.rs".to_string()],
            patterns_from: vec![patterns.display().to_string()],
            split_size: None,
            solid: false,
            method: None,
            level: None,
            threads: None,
            password: None,
            encrypt_header: false,
            overwrite: None,
            dry_run: false,
            list: false,
            verify_source: None,
            verify_output: None,
            progress: None,
            json: false,
            no_sidecar: false,
        })
        .unwrap();

        assert_eq!(options.format, BackupArtifactFormat::Zip);
        assert_eq!(options.overwrite, OverwriteMode::Ask);
        assert_eq!(options.verify_source, VerifySourceMode::Quick);
        assert_eq!(options.verify_output, VerifyOutputMode::On);
        assert_eq!(options.progress, ProgressMode::Auto);
        assert_eq!(options.selection.files, vec!["README.md"]);
        assert_eq!(options.selection.globs, vec!["src/*.rs", "*.md"]);
    }

    #[test]
    fn backup_convert_options_reject_split_size_for_dir() {
        let err = BackupConvertOptions::try_from(BackupConvertCmd {
            artifact: "input.zip".to_string(),
            format: "dir".to_string(),
            output: "out".to_string(),
            file: Vec::new(),
            glob: Vec::new(),
            patterns_from: Vec::new(),
            split_size: Some("2G".to_string()),
            solid: false,
            method: None,
            level: None,
            threads: None,
            password: None,
            encrypt_header: false,
            overwrite: None,
            dry_run: false,
            list: true,
            verify_source: None,
            verify_output: None,
            progress: None,
            json: false,
            no_sidecar: false,
        })
        .unwrap_err();

        assert!(err.message.contains("--split-size"));
    }

    #[test]
    fn backup_convert_options_reject_invalid_zip_method() {
        let err = BackupConvertOptions::try_from(BackupConvertCmd {
            artifact: "input.zip".to_string(),
            format: "zip".to_string(),
            output: "out.zip".to_string(),
            file: Vec::new(),
            glob: Vec::new(),
            patterns_from: Vec::new(),
            split_size: None,
            solid: false,
            method: Some("lzma2".to_string()),
            level: None,
            threads: None,
            password: None,
            encrypt_header: false,
            overwrite: None,
            dry_run: false,
            list: true,
            verify_source: None,
            verify_output: None,
            progress: None,
            json: false,
            no_sidecar: false,
        })
        .unwrap_err();

        assert!(err.message.contains("invalid for zip"));
    }

    #[test]
    fn backup_create_options_reject_xunbak_method_flag() {
        let err = BackupCreateOptions::try_from(BackupCreateCmd {
            msg: None,
            dir: None,
            format: Some("xunbak".to_string()),
            output: Some("artifact.xunbak".to_string()),
            compression: None,
            split_size: None,
            solid: false,
            method: Some("zstd".to_string()),
            level: Some(3),
            dry_run: false,
            list: false,
            no_compress: false,
            retain: None,
            include: Vec::new(),
            exclude: Vec::new(),
            incremental: false,
            skip_if_unchanged: false,
            diff_mode: None,
            progress: None,
            json: false,
            no_sidecar: false,
        })
        .unwrap_err();

        assert!(
            err.message
                .contains("backup create --method is invalid for xunbak output")
        );
    }

    #[test]
    fn backup_create_options_reject_xunbak_level_flag() {
        let err = BackupCreateOptions::try_from(BackupCreateCmd {
            msg: None,
            dir: None,
            format: Some("xunbak".to_string()),
            output: Some("artifact.xunbak".to_string()),
            compression: None,
            split_size: None,
            solid: false,
            method: None,
            level: Some(3),
            dry_run: false,
            list: false,
            no_compress: false,
            retain: None,
            include: Vec::new(),
            exclude: Vec::new(),
            incremental: false,
            skip_if_unchanged: false,
            diff_mode: None,
            progress: None,
            json: false,
            no_sidecar: false,
        })
        .unwrap_err();

        assert!(
            err.message
                .contains("backup create --level is invalid for xunbak output")
        );
    }

    #[test]
    fn backup_create_options_use_create_prefix_in_flag_errors() {
        let err = BackupCreateOptions::try_from(BackupCreateCmd {
            msg: None,
            dir: None,
            format: Some("dir".to_string()),
            output: Some("out".to_string()),
            compression: None,
            split_size: Some("2G".to_string()),
            solid: false,
            method: None,
            level: None,
            dry_run: false,
            list: false,
            no_compress: false,
            retain: None,
            include: Vec::new(),
            exclude: Vec::new(),
            incremental: false,
            skip_if_unchanged: false,
            diff_mode: None,
            progress: None,
            json: false,
            no_sidecar: false,
        })
        .unwrap_err();

        assert!(
            err.message
                .contains("backup create --split-size is invalid for dir output")
        );
    }

    #[test]
    fn backup_convert_options_reject_threads_for_zip_output() {
        let err = BackupConvertOptions::try_from(BackupConvertCmd {
            artifact: "input.zip".to_string(),
            format: "zip".to_string(),
            output: "out.zip".to_string(),
            file: Vec::new(),
            glob: Vec::new(),
            patterns_from: Vec::new(),
            split_size: None,
            solid: false,
            method: None,
            level: None,
            threads: Some(4),
            password: None,
            encrypt_header: false,
            overwrite: None,
            dry_run: false,
            list: false,
            verify_source: None,
            verify_output: None,
            progress: None,
            json: false,
            no_sidecar: false,
        })
        .unwrap_err();

        assert!(
            err.message
                .contains("backup convert --threads is invalid for zip output")
        );
    }

    #[test]
    fn backup_convert_options_require_password_for_encrypt_header() {
        let err = BackupConvertOptions::try_from(BackupConvertCmd {
            artifact: "input.zip".to_string(),
            format: "7z".to_string(),
            output: "out.7z".to_string(),
            file: Vec::new(),
            glob: Vec::new(),
            patterns_from: Vec::new(),
            split_size: None,
            solid: false,
            method: None,
            level: None,
            threads: None,
            password: None,
            encrypt_header: true,
            overwrite: None,
            dry_run: false,
            list: true,
            verify_source: None,
            verify_output: None,
            progress: None,
            json: false,
            no_sidecar: false,
        })
        .unwrap_err();

        assert!(err.message.contains("--encrypt-header requires --password"));
    }

    #[test]
    fn backup_restore_options_only_capture_restore_fields() {
        let options = BackupRestoreOptions::try_from(BackupRestoreCmd {
            name_or_path: "archive.zip".to_string(),
            file: Some("src/main.rs".to_string()),
            glob: None,
            to: Some("out".to_string()),
            snapshot: true,
            dir: Some("project".to_string()),
            dry_run: true,
            yes: false,
            json: true,
        })
        .unwrap();

        assert_eq!(options.name_or_path, "archive.zip");
        assert_eq!(options.file.as_deref(), Some("src/main.rs"));
        assert_eq!(options.destination, Some(PathBuf::from("out")));
        assert!(options.snapshot);
        assert_eq!(options.project_dir, PathBuf::from("project"));
        assert!(options.dry_run);
        assert!(options.json);
    }
}
