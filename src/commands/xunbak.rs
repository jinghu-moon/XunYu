use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::Local;
use serde::{Deserialize, Serialize};

#[cfg(windows)]
use winreg::RegKey;
#[cfg(windows)]
use winreg::enums::{HKEY_CLASSES_ROOT, HKEY_CURRENT_USER};

use crate::cli::{
    XunbakCmd, XunbakPluginCmd, XunbakPluginDoctorCmd, XunbakPluginInstallCmd,
    XunbakPluginSubCommand, XunbakPluginUninstallCmd, XunbakSubCommand,
};
use crate::output::{CliError, CliResult};

const DEFAULT_SEVENZIP_CANDIDATES: &[&str] = &["C:/A_Softwares/7-Zip", "C:/Program Files/7-Zip"];
const PLUGIN_DLL_NAME: &str = "xunbak.dll";
const XUNBAK_EXTENSION_SUBKEY: &str = ".xunbak";
const XUNBAK_PROG_ID: &str = "XunYu.xunbak";
const XUNBAK_OPEN_COMMAND_SUBKEY: &str = "XunYu.xunbak\\shell\\open\\command";
const USER_CLASSES_ROOT: &str = "Software\\Classes";

pub(crate) fn cmd_xunbak(args: XunbakCmd) -> CliResult {
    match args.cmd {
        XunbakSubCommand::Plugin(cmd) => cmd_plugin(cmd),
    }
}

fn cmd_plugin(args: XunbakPluginCmd) -> CliResult {
    match args.cmd {
        XunbakPluginSubCommand::Install(cmd) => cmd_plugin_install(cmd),
        XunbakPluginSubCommand::Uninstall(cmd) => cmd_plugin_uninstall(cmd),
        XunbakPluginSubCommand::Doctor(cmd) => cmd_plugin_doctor(cmd),
    }
}

fn cmd_plugin_install(args: XunbakPluginInstallCmd) -> CliResult {
    let config = parse_plugin_build_config(args.config.as_deref())?;
    let sevenzip = resolve_sevenzip_installation(args.sevenzip_home.as_deref())?;
    let plugin_dll = resolve_plugin_dll_path(config);
    if !plugin_dll.is_file() {
        return Err(CliError::with_details(
            2,
            format!("Plugin build artifact not found: {}", plugin_dll.display()),
            &[
                "Fix: Build the plugin first, or point XUN_XUNBAK_PLUGIN_BUILD_ROOT to a prepared artifact tree.",
                "Fix: Example build output path is build/xunbak-7z-plugin/<Debug|Release>/xunbak.dll.",
            ],
        ));
    }

    fs::create_dir_all(&sevenzip.formats_dir).map_err(|err| {
        CliError::with_details(
            1,
            format!(
                "Failed to create 7-Zip Formats directory {}: {err}",
                sevenzip.formats_dir.display()
            ),
            &["Fix: Ensure the selected 7-Zip directory is writable."],
        )
    })?;

    let target_dll = sevenzip.formats_dir.join(PLUGIN_DLL_NAME);
    if target_dll.exists() && args.no_overwrite {
        return Err(CliError::with_details(
            2,
            format!("Target plugin already exists: {}", target_dll.display()),
            &["Fix: Remove --no-overwrite to replace the existing DLL."],
        ));
    }

    let backup_path = if target_dll.exists() {
        Some(backup_existing_plugin(&target_dll)?)
    } else {
        None
    };

    fs::copy(&plugin_dll, &target_dll).map_err(|err| {
        CliError::with_details(
            1,
            format!(
                "Failed to copy plugin DLL {} -> {}: {err}",
                plugin_dll.display(),
                target_dll.display()
            ),
            &["Fix: Ensure the selected 7-Zip directory is writable and not locked by another process."],
        )
    })?;

    out_println!("Installed: {}", target_dll.display());
    if let Some(path) = backup_path {
        out_println!("Backup: {}", path.display());
    }
    out_println!("7-Zip: {}", sevenzip.file_manager_path.display());
    if args.associate {
        let mut store = association_store_from_env()
            .map_err(|err| map_association_store_error("associate .xunbak with 7-Zip", err))?;
        associate_xunbak_with_store(&mut *store, &sevenzip.file_manager_path)
            .map_err(|err| map_association_store_error("associate .xunbak with 7-Zip", err))?;
        out_println!(
            "Associated: .xunbak -> {}",
            sevenzip.file_manager_path.display()
        );
    }
    Ok(())
}

fn cmd_plugin_uninstall(args: XunbakPluginUninstallCmd) -> CliResult {
    let sevenzip = resolve_sevenzip_installation(args.sevenzip_home.as_deref())?;
    let target_dll = sevenzip.formats_dir.join(PLUGIN_DLL_NAME);
    if target_dll.exists() {
        fs::remove_file(&target_dll).map_err(|err| {
            CliError::with_details(
                1,
                format!(
                    "Failed to remove plugin DLL {}: {err}",
                    target_dll.display()
                ),
                &["Fix: Ensure the DLL is not locked by 7-Zip and the directory is writable."],
            )
        })?;
        out_println!("Removed: {}", target_dll.display());
    } else {
        out_println!("Plugin not installed: {}", target_dll.display());
    }
    if args.remove_association {
        let mut store = association_store_from_env()
            .map_err(|err| map_association_store_error("remove .xunbak association", err))?;
        let outcome =
            remove_xunbak_association_with_store(&mut *store, Some(&sevenzip.file_manager_path))
                .map_err(|err| map_association_store_error("remove .xunbak association", err))?;
        match outcome {
            RemoveAssociationOutcome::Removed => out_println!("Association removed: .xunbak"),
            RemoveAssociationOutcome::AlreadyAbsent => {
                out_println!("Association not present: .xunbak")
            }
            RemoveAssociationOutcome::SkippedThirdParty => {
                out_println!("Association kept: current .xunbak binding is not 7-Zip")
            }
            RemoveAssociationOutcome::SkippedUnknown => {
                out_println!("Association kept: current .xunbak binding is unresolved")
            }
            RemoveAssociationOutcome::SkippedUnmanaged => {
                out_println!("Association kept: current .xunbak binding is not managed in HKCU")
            }
        }
    }
    Ok(())
}

fn cmd_plugin_doctor(args: XunbakPluginDoctorCmd) -> CliResult {
    let report = build_doctor_report(args.sevenzip_home.as_deref());
    out_println!("{}", render_doctor_report(&report));
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PluginBuildConfig {
    Debug,
    Release,
}

impl PluginBuildConfig {
    fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "Debug",
            Self::Release => "Release",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SevenZipInstallation {
    home: PathBuf,
    formats_dir: PathBuf,
    file_manager_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AssociationStatus {
    Unassociated,
    AssociatedWithSevenZip { command: String },
    AssociatedWithOther { prog_id: String, command: String },
    Unknown(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SevenZZstdCodecStatus {
    Supported,
    NotDetected,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DoctorReport {
    explicit_home: Option<PathBuf>,
    sevenzip: Option<SevenZipInstallation>,
    sevenzip_version: Option<String>,
    sevenz_zstd_codec: SevenZZstdCodecStatus,
    plugin_dll_path: Option<PathBuf>,
    plugin_installed: bool,
    association: AssociationStatus,
    suggestions: Vec<String>,
    issue: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AssociationEntry {
    user_prog_id: Option<String>,
    effective_prog_id: String,
    command: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemoveAssociationOutcome {
    Removed,
    AlreadyAbsent,
    SkippedThirdParty,
    SkippedUnknown,
    SkippedUnmanaged,
}

trait AssociationStore {
    fn read_user_default(&self, subkey: &str) -> io::Result<Option<String>>;
    fn read_classes_default(&self, subkey: &str) -> io::Result<Option<String>>;
    fn set_user_default(&mut self, subkey: &str, value: &str) -> io::Result<()>;
    fn delete_user_tree(&mut self, subkey: &str) -> io::Result<bool>;
}

#[cfg(windows)]
struct WindowsAssociationStore;

#[cfg(windows)]
impl AssociationStore for WindowsAssociationStore {
    fn read_user_default(&self, subkey: &str) -> io::Result<Option<String>> {
        read_registry_default_user(subkey)
    }

    fn read_classes_default(&self, subkey: &str) -> io::Result<Option<String>> {
        read_registry_default_hkcr(subkey)
    }

    fn set_user_default(&mut self, subkey: &str, value: &str) -> io::Result<()> {
        write_registry_default_user(subkey, value)
    }

    fn delete_user_tree(&mut self, subkey: &str) -> io::Result<bool> {
        delete_registry_tree_user(subkey)
    }
}

#[derive(Debug, Clone)]
struct FileAssociationStore {
    path: PathBuf,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
struct FileAssociationStoreData {
    user: BTreeMap<String, String>,
    classes: BTreeMap<String, String>,
}

impl FileAssociationStore {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn load(&self) -> io::Result<FileAssociationStoreData> {
        if !self.path.exists() {
            return Ok(FileAssociationStoreData::default());
        }
        let content = fs::read_to_string(&self.path)?;
        if content.trim().is_empty() {
            return Ok(FileAssociationStoreData::default());
        }
        serde_json::from_str(&content).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "invalid fake association store {}: {err}",
                    self.path.display()
                ),
            )
        })
    }

    fn save(&self, data: &FileAssociationStoreData) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(data).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("serialize fake association store failed: {err}"),
            )
        })?;
        fs::write(&self.path, content)
    }
}

impl AssociationStore for FileAssociationStore {
    fn read_user_default(&self, subkey: &str) -> io::Result<Option<String>> {
        Ok(self.load()?.user.get(subkey).cloned())
    }

    fn read_classes_default(&self, subkey: &str) -> io::Result<Option<String>> {
        Ok(self.load()?.classes.get(subkey).cloned())
    }

    fn set_user_default(&mut self, subkey: &str, value: &str) -> io::Result<()> {
        let mut data = self.load()?;
        data.user.insert(subkey.to_string(), value.to_string());
        self.save(&data)
    }

    fn delete_user_tree(&mut self, subkey: &str) -> io::Result<bool> {
        let mut data = self.load()?;
        let exact = subkey.to_string();
        let prefix = format!("{subkey}\\");
        let before = data.user.len();
        data.user
            .retain(|key, _| key != &exact && !key.starts_with(&prefix));
        let removed = data.user.len() != before;
        if removed {
            self.save(&data)?;
        }
        Ok(removed)
    }
}

fn association_store_from_env() -> io::Result<Box<dyn AssociationStore>> {
    if let Some(path) = env::var_os("XUN_XUNBAK_PLUGIN_TEST_ASSOC_FILE") {
        return Ok(Box::new(FileAssociationStore::new(PathBuf::from(path))));
    }

    #[cfg(windows)]
    {
        Ok(Box::new(WindowsAssociationStore))
    }

    #[cfg(not(windows))]
    {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "xunbak association management is only supported on Windows",
        ))
    }
}

fn map_association_store_error(action: &str, err: io::Error) -> CliError {
    CliError::with_details(
        1,
        format!("Failed to {action}: {err}"),
        &[
            "Fix: Ensure the current user can access HKCU\\Software\\Classes, or use the test association backend in automated tests.",
            "Rollback: Run `xun xunbak plugin uninstall --remove-association` after fixing the issue if you need to clean up a partial user-level binding.",
        ],
    )
}

fn build_xunbak_open_command(sevenzip_fm_path: &Path) -> String {
    format!("\"{}\" \"%1\"", sevenzip_fm_path.display())
}

fn read_association_entry_with_store(
    store: &dyn AssociationStore,
) -> io::Result<Option<AssociationEntry>> {
    let user_prog_id = store
        .read_user_default(XUNBAK_EXTENSION_SUBKEY)?
        .filter(|value| !value.trim().is_empty());
    let effective_prog_id = match user_prog_id.clone() {
        Some(value) => value,
        None => {
            let Some(value) = store
                .read_classes_default(XUNBAK_EXTENSION_SUBKEY)?
                .filter(|value| !value.trim().is_empty())
            else {
                return Ok(None);
            };
            value
        }
    };
    let command_subkey = format!("{effective_prog_id}\\shell\\open\\command");
    let command = store
        .read_user_default(&command_subkey)?
        .or_else(|| store.read_classes_default(&command_subkey).ok().flatten())
        .filter(|value| !value.trim().is_empty());

    Ok(Some(AssociationEntry {
        user_prog_id,
        effective_prog_id,
        command,
    }))
}

fn classify_association_entry(
    entry: &AssociationEntry,
    sevenzip_fm_path: Option<&Path>,
) -> AssociationStatus {
    let Some(command) = entry.command.clone() else {
        return AssociationStatus::Unknown(format!(
            "missing open command for prog_id `{}`",
            entry.effective_prog_id
        ));
    };
    if command_matches_sevenzip(&command, sevenzip_fm_path) {
        AssociationStatus::AssociatedWithSevenZip { command }
    } else {
        AssociationStatus::AssociatedWithOther {
            prog_id: entry.effective_prog_id.clone(),
            command,
        }
    }
}

fn detect_xunbak_association_with_store(
    store: &dyn AssociationStore,
    sevenzip_fm_path: Option<&Path>,
) -> io::Result<AssociationStatus> {
    let Some(entry) = read_association_entry_with_store(store)? else {
        return Ok(AssociationStatus::Unassociated);
    };
    Ok(classify_association_entry(&entry, sevenzip_fm_path))
}

fn associate_xunbak_with_store(
    store: &mut dyn AssociationStore,
    sevenzip_fm_path: &Path,
) -> io::Result<()> {
    let command = build_xunbak_open_command(sevenzip_fm_path);
    store.set_user_default(XUNBAK_EXTENSION_SUBKEY, XUNBAK_PROG_ID)?;
    store.set_user_default(XUNBAK_PROG_ID, "XunYu xunbak Archive")?;
    store.set_user_default(XUNBAK_OPEN_COMMAND_SUBKEY, &command)?;
    Ok(())
}

fn remove_xunbak_association_with_store(
    store: &mut dyn AssociationStore,
    sevenzip_fm_path: Option<&Path>,
) -> io::Result<RemoveAssociationOutcome> {
    let Some(entry) = read_association_entry_with_store(store)? else {
        return Ok(RemoveAssociationOutcome::AlreadyAbsent);
    };
    let status = classify_association_entry(&entry, sevenzip_fm_path);
    if entry.user_prog_id.is_none() {
        return Ok(RemoveAssociationOutcome::SkippedUnmanaged);
    }
    match status {
        AssociationStatus::AssociatedWithOther { .. } => {
            Ok(RemoveAssociationOutcome::SkippedThirdParty)
        }
        AssociationStatus::Unknown(_) => Ok(RemoveAssociationOutcome::SkippedUnknown),
        AssociationStatus::Unassociated => Ok(RemoveAssociationOutcome::AlreadyAbsent),
        AssociationStatus::AssociatedWithSevenZip { .. } => {
            let removed_extension = store.delete_user_tree(XUNBAK_EXTENSION_SUBKEY)?;
            let removed_progid = if entry.effective_prog_id.eq_ignore_ascii_case(XUNBAK_PROG_ID) {
                store.delete_user_tree(XUNBAK_PROG_ID)?
            } else {
                false
            };
            if removed_extension || removed_progid {
                Ok(RemoveAssociationOutcome::Removed)
            } else {
                Ok(RemoveAssociationOutcome::AlreadyAbsent)
            }
        }
    }
}

fn parse_plugin_build_config(value: Option<&str>) -> Result<PluginBuildConfig, CliError> {
    match value
        .unwrap_or("debug")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "debug" => Ok(PluginBuildConfig::Debug),
        "release" => Ok(PluginBuildConfig::Release),
        other => Err(CliError::with_details(
            2,
            format!("Invalid plugin --config: {other}"),
            &["Fix: Use `debug` or `release`."],
        )),
    }
}

fn resolve_sevenzip_installation(explicit: Option<&str>) -> Result<SevenZipInstallation, CliError> {
    if let Some(raw) = explicit {
        let home = PathBuf::from(raw);
        return inspect_sevenzip_home(&home).map_err(|_| {
            CliError::with_details(
                2,
                format!("Invalid --sevenzip-home: {}", home.display()),
                &[
                    "Fix: Pass an installed 7-Zip home containing 7zFM.exe.",
                    "Fix: Supported defaults are C:/A_Softwares/7-Zip and C:/Program Files/7-Zip.",
                ],
            )
        });
    }

    let candidates = default_sevenzip_candidates();
    if let Some(home) = find_first_valid_sevenzip_home(&candidates, inspect_sevenzip_home) {
        return Ok(home);
    }

    Err(CliError::with_details(
        2,
        "7-Zip installation not found",
        &[
            "Fix: Install 7-Zip first; XunYu only supports an existing 7-Zip installation and does not distribute 7-Zip itself.",
            "Fix: Or pass --sevenzip-home explicitly.",
        ],
    ))
}

fn inspect_sevenzip_home(home: &Path) -> Result<SevenZipInstallation, io::Error> {
    let file_manager_path = home.join("7zFM.exe");
    if !file_manager_path.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("7zFM.exe not found under {}", home.display()),
        ));
    }

    Ok(SevenZipInstallation {
        home: home.to_path_buf(),
        formats_dir: home.join("Formats"),
        file_manager_path,
    })
}

fn default_sevenzip_candidates() -> Vec<PathBuf> {
    DEFAULT_SEVENZIP_CANDIDATES
        .iter()
        .map(PathBuf::from)
        .collect()
}

fn find_first_valid_sevenzip_home<F>(
    candidates: &[PathBuf],
    mut inspect: F,
) -> Option<SevenZipInstallation>
where
    F: FnMut(&Path) -> Result<SevenZipInstallation, io::Error>,
{
    for candidate in candidates {
        if let Ok(value) = inspect(candidate) {
            return Some(value);
        }
    }
    None
}

fn resolve_plugin_dll_path(config: PluginBuildConfig) -> PathBuf {
    if let Some(root) = env::var_os("XUN_XUNBAK_PLUGIN_BUILD_ROOT") {
        return PathBuf::from(root)
            .join(config.as_str())
            .join(PLUGIN_DLL_NAME);
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("build")
        .join("xunbak-7z-plugin")
        .join(config.as_str())
        .join(PLUGIN_DLL_NAME)
}

fn backup_existing_plugin(target_dll: &Path) -> Result<PathBuf, CliError> {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let backup_name = format!("{PLUGIN_DLL_NAME}.bak.{timestamp}");
    let backup_path = target_dll.with_file_name(backup_name);
    fs::copy(target_dll, &backup_path).map_err(|err| {
        CliError::with_details(
            1,
            format!(
                "Failed to create plugin backup {}: {err}",
                backup_path.display()
            ),
            &["Fix: Ensure the existing plugin DLL is readable and the directory is writable."],
        )
    })?;
    Ok(backup_path)
}

fn build_doctor_report(explicit: Option<&str>) -> DoctorReport {
    let explicit_home = explicit.map(PathBuf::from);
    let sevenzip = match explicit_home.as_deref() {
        Some(home) => inspect_sevenzip_home(home).ok(),
        None => {
            find_first_valid_sevenzip_home(&default_sevenzip_candidates(), inspect_sevenzip_home)
        }
    };

    let issue = match (&explicit_home, &sevenzip) {
        (Some(home), None) => Some(format!(
            "Explicit 7-Zip home is invalid or missing 7zFM.exe: {}",
            home.display()
        )),
        (None, None) => Some(
            "7-Zip installation not found. Only existing 7-Zip installations are supported."
                .to_string(),
        ),
        _ => None,
    };

    let sevenzip_version = sevenzip
        .as_ref()
        .and_then(|value| read_file_version(&value.file_manager_path));
    let sevenz_zstd_codec = sevenzip
        .as_ref()
        .map(|value| detect_sevenz_zstd_codec(&value.home))
        .unwrap_or(SevenZZstdCodecStatus::Unknown);
    let plugin_dll_path = sevenzip
        .as_ref()
        .map(|value| value.formats_dir.join(PLUGIN_DLL_NAME));
    let plugin_installed = plugin_dll_path.as_ref().is_some_and(|path| path.is_file());
    let association = sevenzip
        .as_ref()
        .map(|value| detect_xunbak_association(Some(&value.file_manager_path)))
        .unwrap_or_else(|| detect_xunbak_association(None));
    let suggestions =
        build_doctor_suggestions(&issue, plugin_installed, &association, sevenz_zstd_codec);

    DoctorReport {
        explicit_home,
        sevenzip,
        sevenzip_version,
        sevenz_zstd_codec,
        plugin_dll_path,
        plugin_installed,
        association,
        suggestions,
        issue,
    }
}

fn build_doctor_suggestions(
    issue: &Option<String>,
    plugin_installed: bool,
    association: &AssociationStatus,
    sevenz_zstd_codec: SevenZZstdCodecStatus,
) -> Vec<String> {
    let mut suggestions = Vec::new();
    if issue.is_some() {
        suggestions.push(
            "Install 7-Zip yourself or pass --sevenzip-home to an existing 7-Zip directory."
                .to_string(),
        );
    }
    if !plugin_installed {
        suggestions.push("Run `xun xunbak plugin install` after building xunbak.dll.".to_string());
    }
    match association {
        AssociationStatus::AssociatedWithSevenZip { .. } => {}
        AssociationStatus::Unassociated => {
            suggestions
                .push("Associate .xunbak with 7zFM.exe for double-click open support.".to_string());
        }
        AssociationStatus::AssociatedWithOther { .. } => {
            suggestions.push(
                "Move .xunbak association back to 7zFM.exe if you want 7-Zip to open it."
                    .to_string(),
            );
        }
        AssociationStatus::Unknown(_) => {
            suggestions.push(
                "Inspect the current .xunbak open command before changing file association."
                    .to_string(),
            );
        }
    }
    match sevenz_zstd_codec {
        SevenZZstdCodecStatus::Supported => suggestions.push(
            "This 7-Zip installation reports a ZSTD codec, so 7z `--method zstd` should be readable here."
                .to_string(),
        ),
        SevenZZstdCodecStatus::NotDetected => suggestions.push(
            "7z `--method zstd` needs extraction-side external codec support; current 7-Zip did not report ZSTD codec support."
                .to_string(),
        ),
        SevenZZstdCodecStatus::Unknown => suggestions.push(
            "Could not determine whether this 7-Zip installation exposes a ZSTD codec for 7z archives."
                .to_string(),
        ),
    }
    if suggestions.is_empty() {
        suggestions.push("Environment looks ready.".to_string());
    }
    suggestions
}

fn render_doctor_report(report: &DoctorReport) -> String {
    let mut lines = Vec::new();
    lines.push("xunbak plugin doctor".to_string());
    match &report.sevenzip {
        Some(value) => {
            lines.push(format!("7-Zip Home: {}", value.home.display()));
            match &report.sevenzip_version {
                Some(version) => lines.push(format!(
                    "7-Zip FM: {} (version {})",
                    value.file_manager_path.display(),
                    version
                )),
                None => lines.push(format!("7-Zip FM: {}", value.file_manager_path.display())),
            }
        }
        None => {
            if let Some(home) = &report.explicit_home {
                lines.push(format!("7-Zip Home: {}", home.display()));
            } else {
                lines.push("7-Zip Home: not found".to_string());
            }
            lines.push("7-Zip FM: missing".to_string());
        }
    }

    match &report.plugin_dll_path {
        Some(path) if report.plugin_installed => {
            lines.push(format!("Plugin DLL: {} (installed)", path.display()));
        }
        Some(path) => {
            lines.push(format!("Plugin DLL: {} (missing)", path.display()));
        }
        None => lines.push("Plugin DLL: unresolved".to_string()),
    }

    lines.push(format!(
        "Association: {}",
        describe_association_status(&report.association)
    ));
    lines.push(format!(
        "7z ZSTD Codec: {}",
        describe_sevenz_zstd_codec(report.sevenz_zstd_codec)
    ));

    if let Some(issue) = &report.issue {
        lines.push(format!("Issue: {issue}"));
    }

    lines.push("Suggestions:".to_string());
    for suggestion in &report.suggestions {
        lines.push(format!("- {suggestion}"));
    }

    lines.join("\n")
}

fn describe_association_status(status: &AssociationStatus) -> String {
    match status {
        AssociationStatus::Unassociated => "unassociated".to_string(),
        AssociationStatus::AssociatedWithSevenZip { command } => format!("7-Zip ({command})"),
        AssociationStatus::AssociatedWithOther { prog_id, command } => {
            format!("other ({prog_id}: {command})")
        }
        AssociationStatus::Unknown(reason) => format!("unknown ({reason})"),
    }
}

fn describe_sevenz_zstd_codec(status: SevenZZstdCodecStatus) -> &'static str {
    match status {
        SevenZZstdCodecStatus::Supported => "supported",
        SevenZZstdCodecStatus::NotDetected => "not-detected",
        SevenZZstdCodecStatus::Unknown => "unknown",
    }
}

fn detect_sevenz_zstd_codec(home: &Path) -> SevenZZstdCodecStatus {
    if let Ok(output) = env::var("XUN_XUNBAK_PLUGIN_TEST_7ZI_OUTPUT") {
        return parse_sevenz_zstd_codec_status(&output);
    }

    let cli = home.join("7z.exe");
    if !cli.is_file() {
        return SevenZZstdCodecStatus::Unknown;
    }
    let output = match Command::new(cli).arg("i").output() {
        Ok(output) if output.status.success() => output,
        _ => return SevenZZstdCodecStatus::Unknown,
    };
    parse_sevenz_zstd_codec_status(&String::from_utf8_lossy(&output.stdout))
}

fn parse_sevenz_zstd_codec_status(output: &str) -> SevenZZstdCodecStatus {
    let mut in_codecs = false;
    let mut saw_codecs = false;
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("Codecs:") {
            in_codecs = true;
            saw_codecs = true;
            continue;
        }
        if !in_codecs {
            continue;
        }
        if trimmed.is_empty() {
            break;
        }
        let upper = trimmed.to_ascii_uppercase();
        if upper.ends_with(" ZSTD")
            || upper.contains(" ZSTD ")
            || upper.ends_with(" ZSTANDARD")
            || upper.contains(" ZSTANDARD ")
        {
            return SevenZZstdCodecStatus::Supported;
        }
    }
    if saw_codecs {
        SevenZZstdCodecStatus::NotDetected
    } else {
        SevenZZstdCodecStatus::Unknown
    }
}

fn read_file_version(path: &Path) -> Option<String> {
    if let Ok(value) = env::var("XUN_XUNBAK_PLUGIN_TEST_FILE_VERSION") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    let literal = powershell_literal(path.as_os_str().to_string_lossy().as_ref());
    let script = format!(
        "$v = (Get-Item -LiteralPath '{literal}').VersionInfo.FileVersion; if ($v) {{ $v }}"
    );
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = stdout.trim();
    if version.is_empty() {
        None
    } else {
        Some(version.to_string())
    }
}

fn powershell_literal(raw: &str) -> String {
    raw.replace('\'', "''")
}

fn detect_xunbak_association(sevenzip_fm_path: Option<&Path>) -> AssociationStatus {
    let Ok(store) = association_store_from_env() else {
        return AssociationStatus::Unknown(
            "file association backend is unavailable on this platform".to_string(),
        );
    };
    match detect_xunbak_association_with_store(&*store, sevenzip_fm_path) {
        Ok(status) => status,
        Err(err) => AssociationStatus::Unknown(format!("failed to inspect association: {err}")),
    }
}

#[cfg(windows)]
fn read_registry_default_user(subkey: &str) -> io::Result<Option<String>> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let full_subkey = format!("{USER_CLASSES_ROOT}\\{subkey}");
    match hkcu.open_subkey(&full_subkey) {
        Ok(key) => key.get_value("").map(Some),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

#[cfg(windows)]
fn read_registry_default_hkcr(subkey: &str) -> io::Result<Option<String>> {
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    match hkcr.open_subkey(subkey) {
        Ok(key) => key.get_value("").map(Some),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

#[cfg(windows)]
fn write_registry_default_user(subkey: &str, value: &str) -> io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let full_subkey = format!("{USER_CLASSES_ROOT}\\{subkey}");
    let (key, _) = hkcu.create_subkey(&full_subkey)?;
    key.set_value("", &value)
}

#[cfg(windows)]
fn delete_registry_tree_user(subkey: &str) -> io::Result<bool> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let full_subkey = format!("{USER_CLASSES_ROOT}\\{subkey}");
    match hkcu.delete_subkey_all(&full_subkey) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err),
    }
}

fn command_matches_sevenzip(command: &str, sevenzip_fm_path: Option<&Path>) -> bool {
    let lower = command.to_ascii_lowercase();
    if lower.contains("7zfm.exe") {
        return true;
    }

    let Some(path) = sevenzip_fm_path else {
        return false;
    };
    let needle = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    lower.replace('\\', "/").contains(&needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct MemoryAssociationStore {
        user: BTreeMap<String, String>,
        classes: BTreeMap<String, String>,
    }

    impl AssociationStore for MemoryAssociationStore {
        fn read_user_default(&self, subkey: &str) -> io::Result<Option<String>> {
            Ok(self.user.get(subkey).cloned())
        }

        fn read_classes_default(&self, subkey: &str) -> io::Result<Option<String>> {
            Ok(self.classes.get(subkey).cloned())
        }

        fn set_user_default(&mut self, subkey: &str, value: &str) -> io::Result<()> {
            self.user.insert(subkey.to_string(), value.to_string());
            Ok(())
        }

        fn delete_user_tree(&mut self, subkey: &str) -> io::Result<bool> {
            let exact = subkey.to_string();
            let prefix = format!("{subkey}\\");
            let before = self.user.len();
            self.user
                .retain(|key, _| key != &exact && !key.starts_with(&prefix));
            Ok(self.user.len() != before)
        }
    }

    #[test]
    fn parse_plugin_build_config_accepts_default_and_release() {
        assert_eq!(
            parse_plugin_build_config(None).unwrap(),
            PluginBuildConfig::Debug
        );
        assert_eq!(
            parse_plugin_build_config(Some("release")).unwrap(),
            PluginBuildConfig::Release
        );
    }

    #[test]
    fn parse_plugin_build_config_rejects_unknown_value() {
        let err = parse_plugin_build_config(Some("nightly")).unwrap_err();
        assert!(err.message.contains("Invalid plugin --config"));
    }

    #[test]
    fn find_first_valid_sevenzip_home_prefers_first_candidate() {
        let candidates = vec![
            PathBuf::from("C:/A_Softwares/7-Zip"),
            PathBuf::from("C:/Program Files/7-Zip"),
        ];
        let resolved = find_first_valid_sevenzip_home(&candidates, |path| {
            if path == Path::new("C:/A_Softwares/7-Zip") {
                Ok(SevenZipInstallation {
                    home: path.to_path_buf(),
                    formats_dir: path.join("Formats"),
                    file_manager_path: path.join("7zFM.exe"),
                })
            } else {
                Err(io::Error::new(io::ErrorKind::NotFound, "missing"))
            }
        })
        .unwrap();
        assert_eq!(resolved.home, PathBuf::from("C:/A_Softwares/7-Zip"));
    }

    #[test]
    fn find_first_valid_sevenzip_home_falls_back_to_program_files() {
        let candidates = vec![
            PathBuf::from("C:/A_Softwares/7-Zip"),
            PathBuf::from("C:/Program Files/7-Zip"),
        ];
        let resolved = find_first_valid_sevenzip_home(&candidates, |path| {
            if path == Path::new("C:/Program Files/7-Zip") {
                Ok(SevenZipInstallation {
                    home: path.to_path_buf(),
                    formats_dir: path.join("Formats"),
                    file_manager_path: path.join("7zFM.exe"),
                })
            } else {
                Err(io::Error::new(io::ErrorKind::NotFound, "missing"))
            }
        })
        .unwrap();
        assert_eq!(resolved.home, PathBuf::from("C:/Program Files/7-Zip"));
    }

    #[test]
    fn find_first_valid_sevenzip_home_returns_none_when_all_candidates_miss() {
        let candidates = vec![
            PathBuf::from("C:/A_Softwares/7-Zip"),
            PathBuf::from("C:/Program Files/7-Zip"),
        ];
        let resolved = find_first_valid_sevenzip_home(&candidates, |_path| {
            Err(io::Error::new(io::ErrorKind::NotFound, "missing"))
        });
        assert!(resolved.is_none());
    }

    #[test]
    fn command_matches_sevenzip_accepts_known_7zfm_command() {
        let command = "\"C:\\Program Files\\7-Zip\\7zFM.exe\" \"%1\"";
        assert!(command_matches_sevenzip(
            command,
            Some(Path::new("C:/Program Files/7-Zip/7zFM.exe"))
        ));
    }

    #[test]
    fn detect_xunbak_association_reports_unassociated_when_store_is_empty() {
        let store = MemoryAssociationStore::default();
        let status = detect_xunbak_association_with_store(
            &store,
            Some(Path::new("C:/Program Files/7-Zip/7zFM.exe")),
        )
        .unwrap();
        assert_eq!(status, AssociationStatus::Unassociated);
    }

    #[test]
    fn detect_xunbak_association_reports_sevenzip_binding() {
        let mut store = MemoryAssociationStore::default();
        store.user.insert(
            XUNBAK_EXTENSION_SUBKEY.to_string(),
            XUNBAK_PROG_ID.to_string(),
        );
        store.user.insert(
            XUNBAK_OPEN_COMMAND_SUBKEY.to_string(),
            "\"C:\\Program Files\\7-Zip\\7zFM.exe\" \"%1\"".to_string(),
        );
        let status = detect_xunbak_association_with_store(
            &store,
            Some(Path::new("C:/Program Files/7-Zip/7zFM.exe")),
        )
        .unwrap();
        assert!(matches!(
            status,
            AssociationStatus::AssociatedWithSevenZip { .. }
        ));
    }

    #[test]
    fn detect_xunbak_association_reports_third_party_binding() {
        let mut store = MemoryAssociationStore::default();
        store.user.insert(
            XUNBAK_EXTENSION_SUBKEY.to_string(),
            "ThirdParty.xunbak".to_string(),
        );
        store.user.insert(
            "ThirdParty.xunbak\\shell\\open\\command".to_string(),
            "\"C:\\Tools\\OtherApp.exe\" \"%1\"".to_string(),
        );
        let status = detect_xunbak_association_with_store(
            &store,
            Some(Path::new("C:/Program Files/7-Zip/7zFM.exe")),
        )
        .unwrap();
        assert!(matches!(
            status,
            AssociationStatus::AssociatedWithOther { .. }
        ));
    }

    #[test]
    fn associate_xunbak_with_store_writes_expected_keys() {
        let mut store = MemoryAssociationStore::default();
        associate_xunbak_with_store(&mut store, Path::new("C:/Program Files/7-Zip/7zFM.exe"))
            .unwrap();
        assert_eq!(
            store.user.get(XUNBAK_EXTENSION_SUBKEY).map(String::as_str),
            Some(XUNBAK_PROG_ID)
        );
        assert_eq!(
            store
                .user
                .get(XUNBAK_OPEN_COMMAND_SUBKEY)
                .map(String::as_str),
            Some("\"C:/Program Files/7-Zip/7zFM.exe\" \"%1\"")
        );
    }

    #[test]
    fn remove_xunbak_association_with_store_removes_managed_keys() {
        let mut store = MemoryAssociationStore::default();
        associate_xunbak_with_store(&mut store, Path::new("C:/Program Files/7-Zip/7zFM.exe"))
            .unwrap();
        let outcome = remove_xunbak_association_with_store(
            &mut store,
            Some(Path::new("C:/Program Files/7-Zip/7zFM.exe")),
        )
        .unwrap();
        assert_eq!(outcome, RemoveAssociationOutcome::Removed);
        assert!(!store.user.contains_key(XUNBAK_EXTENSION_SUBKEY));
        assert!(!store.user.contains_key(XUNBAK_OPEN_COMMAND_SUBKEY));
    }

    #[test]
    fn remove_xunbak_association_with_store_preserves_third_party_binding() {
        let mut store = MemoryAssociationStore::default();
        store.user.insert(
            XUNBAK_EXTENSION_SUBKEY.to_string(),
            "ThirdParty.xunbak".to_string(),
        );
        store.user.insert(
            "ThirdParty.xunbak\\shell\\open\\command".to_string(),
            "\"C:\\Tools\\OtherApp.exe\" \"%1\"".to_string(),
        );
        let outcome = remove_xunbak_association_with_store(
            &mut store,
            Some(Path::new("C:/Program Files/7-Zip/7zFM.exe")),
        )
        .unwrap();
        assert_eq!(outcome, RemoveAssociationOutcome::SkippedThirdParty);
        assert!(store.user.contains_key(XUNBAK_EXTENSION_SUBKEY));
    }

    #[test]
    fn parse_sevenz_zstd_codec_status_detects_supported_codec() {
        let status =
            parse_sevenz_zstd_codec_status("Codecs:\n 0 ED     40202 BZip2\n 0 ED  4F71101 ZSTD\n");
        assert_eq!(status, SevenZZstdCodecStatus::Supported);
    }

    #[test]
    fn parse_sevenz_zstd_codec_status_reports_missing_when_codecs_section_lacks_zstd() {
        let status = parse_sevenz_zstd_codec_status(
            "Formats:\n 0 ... zstd\n\nCodecs:\n 0 ED     40202 BZip2\n 0 ED         0 Copy\n",
        );
        assert_eq!(status, SevenZZstdCodecStatus::NotDetected);
    }
}
