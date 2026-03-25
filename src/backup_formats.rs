use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupAction {
    Create,
    Restore,
    Convert,
}

impl BackupAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Restore => "restore",
            Self::Convert => "convert",
        }
    }
}

impl fmt::Display for BackupAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupArtifactFormat {
    #[serde(rename = "dir")]
    Dir,
    #[serde(rename = "xunbak")]
    Xunbak,
    #[serde(rename = "zip")]
    Zip,
    #[serde(rename = "7z")]
    SevenZ,
}

impl BackupArtifactFormat {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dir => "dir",
            Self::Xunbak => "xunbak",
            Self::Zip => "zip",
            Self::SevenZ => "7z",
        }
    }
}

impl fmt::Display for BackupArtifactFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for BackupArtifactFormat {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "dir" => Ok(Self::Dir),
            "xunbak" => Ok(Self::Xunbak),
            "zip" => Ok(Self::Zip),
            "7z" => Ok(Self::SevenZ),
            other => Err(format!("invalid backup artifact format: {other}")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverwriteMode {
    #[serde(rename = "ask")]
    Ask,
    #[serde(rename = "replace")]
    Replace,
    #[serde(rename = "fail")]
    Fail,
}

impl OverwriteMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ask => "ask",
            Self::Replace => "replace",
            Self::Fail => "fail",
        }
    }
}

impl fmt::Display for OverwriteMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for OverwriteMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "ask" => Ok(Self::Ask),
            "replace" => Ok(Self::Replace),
            "fail" => Ok(Self::Fail),
            other => Err(format!("invalid overwrite mode: {other}")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerifySourceMode {
    #[serde(rename = "quick")]
    Quick,
    #[serde(rename = "full")]
    Full,
    #[serde(rename = "manifest-only")]
    ManifestOnly,
    #[serde(rename = "existence-only")]
    ExistenceOnly,
    #[serde(rename = "paranoid")]
    Paranoid,
    #[serde(rename = "off")]
    Off,
}

impl VerifySourceMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Quick => "quick",
            Self::Full => "full",
            Self::ManifestOnly => "manifest-only",
            Self::ExistenceOnly => "existence-only",
            Self::Paranoid => "paranoid",
            Self::Off => "off",
        }
    }
}

impl fmt::Display for VerifySourceMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for VerifySourceMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "quick" => Ok(Self::Quick),
            "full" => Ok(Self::Full),
            "manifest-only" | "manifest_only" => Ok(Self::ManifestOnly),
            "existence-only" | "existence_only" => Ok(Self::ExistenceOnly),
            "paranoid" => Ok(Self::Paranoid),
            "off" => Ok(Self::Off),
            other => Err(format!("invalid verify source mode: {other}")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerifyOutputMode {
    #[serde(rename = "on")]
    On,
    #[serde(rename = "off")]
    Off,
}

impl VerifyOutputMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::On => "on",
            Self::Off => "off",
        }
    }
}

impl fmt::Display for VerifyOutputMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for VerifyOutputMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "on" => Ok(Self::On),
            "off" => Ok(Self::Off),
            other => Err(format!("invalid verify output mode: {other}")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgressMode {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "always")]
    Always,
    #[serde(rename = "off")]
    Off,
}

impl ProgressMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Always => "always",
            Self::Off => "off",
        }
    }
}

impl fmt::Display for ProgressMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ProgressMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "always" => Ok(Self::Always),
            "off" => Ok(Self::Off),
            other => Err(format!("invalid progress mode: {other}")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportStatus {
    #[serde(rename = "ok")]
    Ok,
    #[serde(rename = "write_failed")]
    WriteFailed,
    #[serde(rename = "verify_failed")]
    VerifyFailed,
    #[serde(rename = "preflight_failed")]
    PreflightFailed,
}

impl ExportStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::WriteFailed => "write_failed",
            Self::VerifyFailed => "verify_failed",
            Self::PreflightFailed => "preflight_failed",
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    #[allow(dead_code)]
    pub const fn exit_code(self) -> i32 {
        match self {
            Self::Ok => 0,
            Self::WriteFailed | Self::VerifyFailed => 1,
            Self::PreflightFailed => 2,
        }
    }
}

impl fmt::Display for ExportStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_artifact_format_parses_expected_values() {
        assert_eq!(
            "dir".parse::<BackupArtifactFormat>(),
            Ok(BackupArtifactFormat::Dir)
        );
        assert_eq!(
            "xunbak".parse::<BackupArtifactFormat>(),
            Ok(BackupArtifactFormat::Xunbak)
        );
        assert_eq!(
            "zip".parse::<BackupArtifactFormat>(),
            Ok(BackupArtifactFormat::Zip)
        );
        assert_eq!(
            "7z".parse::<BackupArtifactFormat>(),
            Ok(BackupArtifactFormat::SevenZ)
        );
        assert_eq!(
            "unknown".parse::<BackupArtifactFormat>(),
            Err("invalid backup artifact format: unknown".to_string())
        );
    }

    #[test]
    fn backup_format_display_and_json_are_stable() {
        assert_eq!(BackupArtifactFormat::SevenZ.to_string(), "7z");
        assert_eq!(
            serde_json::to_string(&BackupArtifactFormat::SevenZ).unwrap(),
            "\"7z\""
        );
    }

    #[test]
    fn overwrite_verify_and_progress_modes_serialize_stably() {
        assert_eq!(
            serde_json::to_string(&OverwriteMode::Replace).unwrap(),
            "\"replace\""
        );
        assert_eq!(
            serde_json::to_string(&VerifySourceMode::Paranoid).unwrap(),
            "\"paranoid\""
        );
        assert_eq!(
            serde_json::to_string(&VerifyOutputMode::Off).unwrap(),
            "\"off\""
        );
        assert_eq!(
            serde_json::to_string(&ProgressMode::Always).unwrap(),
            "\"always\""
        );
        assert_eq!(
            "manifest-only".parse::<VerifySourceMode>(),
            Ok(VerifySourceMode::ManifestOnly)
        );
        assert_eq!(
            "existence-only".parse::<VerifySourceMode>(),
            Ok(VerifySourceMode::ExistenceOnly)
        );
    }

    #[test]
    fn export_status_exit_codes_match_contract() {
        assert_eq!(ExportStatus::Ok.exit_code(), 0);
        assert_eq!(ExportStatus::WriteFailed.exit_code(), 1);
        assert_eq!(ExportStatus::VerifyFailed.exit_code(), 1);
        assert_eq!(ExportStatus::PreflightFailed.exit_code(), 2);
    }
}
