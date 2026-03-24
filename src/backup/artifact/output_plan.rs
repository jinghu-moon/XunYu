use std::fs;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
#[cfg(feature = "xunbak")]
use std::time::{SystemTime, UNIX_EPOCH};

use crate::backup_formats::OverwriteMode;
use crate::output::{CliError, CliResult};
#[cfg(feature = "xunbak")]
use uuid::Uuid;

/// Represents the planning result for a zip output target.
pub struct ZipOutputPlan {
    pub target: PathBuf,
    pub temp: PathBuf,
}

impl ZipOutputPlan {
    /// Build a plan for writing `target` as zip using a sibling temp path.
    pub fn prepare(target: &Path, overwrite: OverwriteMode) -> CliResult<Self> {
        let temp = target.with_extension("tmp.zip");
        prepare_temp_target(target, &temp, overwrite)?;
        Ok(Self {
            target: target.to_path_buf(),
            temp,
        })
    }

    pub fn temp_path(&self) -> &Path {
        &self.temp
    }

    pub fn finalize(self) -> CliResult<()> {
        finalize_temp_target(&self.temp, &self.target)
    }

    /// Clean any temp file produced by this plan.
    pub fn cleanup(&self) -> CliResult<()> {
        cleanup_temp_target(&self.temp)
    }
}

/// Represents the planning result for a directory output target.
pub struct DirOutputPlan {
    pub target: PathBuf,
    pub temp: PathBuf,
}

impl DirOutputPlan {
    pub fn prepare(target: &Path, overwrite: OverwriteMode) -> CliResult<Self> {
        let temp = build_temp_dir_target(target);
        ensure_parent(target)?;
        handle_existing_target(target, overwrite)?;
        cleanup_temp_dir(&temp)?;
        Ok(Self {
            target: target.to_path_buf(),
            temp,
        })
    }

    pub fn temp_path(&self) -> &Path {
        &self.temp
    }

    pub fn finalize(self) -> CliResult<()> {
        if self.target.exists() {
            remove_existing_target_path(&self.target)?;
        }
        fs::rename(&self.temp, &self.target).map_err(|err| {
            CliError::new(
                1,
                format!(
                    "Finalize directory output failed {} -> {}: {err}",
                    self.temp.display(),
                    self.target.display()
                ),
            )
        })?;
        Ok(())
    }

    pub fn cleanup(&self) -> CliResult<()> {
        cleanup_temp_dir(&self.temp)
    }
}

/// Represents the planning result for a single-file 7z output target.
pub struct SevenZOutputPlan {
    pub target: PathBuf,
    pub temp: PathBuf,
}

impl SevenZOutputPlan {
    /// Build a plan for writing `target` as 7z using a sibling temp path.
    pub fn prepare(target: &Path, overwrite: OverwriteMode) -> CliResult<Self> {
        let temp = target.with_extension("tmp.7z");
        prepare_temp_target(target, &temp, overwrite)?;
        Ok(Self {
            target: target.to_path_buf(),
            temp,
        })
    }

    pub fn temp_path(&self) -> &Path {
        &self.temp
    }

    pub fn finalize(self) -> CliResult<()> {
        finalize_temp_target(&self.temp, &self.target)
    }

    pub fn cleanup(&self) -> CliResult<()> {
        cleanup_temp_target(&self.temp)
    }
}

/// Represents the planning result for a split 7z output set.
pub struct SevenZSplitOutputPlan {
    pub target_base: PathBuf,
    pub temp_base: PathBuf,
}

impl SevenZSplitOutputPlan {
    pub fn prepare(target_base: &Path, overwrite: OverwriteMode) -> CliResult<Self> {
        ensure_parent(target_base)?;
        handle_existing_numbered_target(target_base, overwrite)?;
        let temp_base = build_temp_sevenz_base(target_base);
        cleanup_numbered_outputs(&temp_base)?;
        Ok(Self {
            target_base: target_base.to_path_buf(),
            temp_base,
        })
    }

    pub fn temp_base_path(&self) -> &Path {
        &self.temp_base
    }

    pub fn finalize(self) -> CliResult<()> {
        finalize_numbered_outputs(&self.temp_base, &self.target_base)
    }

    pub fn cleanup(&self) -> CliResult<()> {
        cleanup_numbered_outputs(&self.temp_base)
    }
}

/// Represents the planning result for a single-file xunbak output target.
#[cfg(feature = "xunbak")]
pub struct XunbakOutputPlan {
    pub target: PathBuf,
    pub temp: PathBuf,
}

#[cfg(feature = "xunbak")]
impl XunbakOutputPlan {
    pub fn prepare(target: &Path, overwrite: OverwriteMode) -> CliResult<Self> {
        let temp = target.with_extension("tmp.xunbak");
        prepare_temp_target(target, &temp, overwrite)?;
        Ok(Self {
            target: target.to_path_buf(),
            temp,
        })
    }

    pub fn temp_path(&self) -> &Path {
        &self.temp
    }

    pub fn finalize(self) -> CliResult<()> {
        finalize_temp_target(&self.temp, &self.target)
    }

    pub fn cleanup(&self) -> CliResult<()> {
        cleanup_temp_target(&self.temp)
    }
}

/// Represents the planning result for a split xunbak output set.
#[cfg(feature = "xunbak")]
pub struct XunbakSplitOutputPlan {
    pub target_base: PathBuf,
    pub temp_base: PathBuf,
}

#[cfg(feature = "xunbak")]
impl XunbakSplitOutputPlan {
    pub fn prepare(target_base: &Path, overwrite: OverwriteMode) -> CliResult<Self> {
        ensure_parent(target_base)?;
        handle_existing_split_target(target_base, overwrite)?;
        let temp_base = build_temp_xunbak_base(target_base, "tmp-split-xunbak");
        cleanup_split_outputs(&temp_base)?;
        Ok(Self {
            target_base: target_base.to_path_buf(),
            temp_base,
        })
    }

    pub fn temp_base_path(&self) -> &Path {
        &self.temp_base
    }

    pub fn finalize(self) -> CliResult<()> {
        let staged = list_split_outputs(&self.temp_base)?;
        if staged.is_empty() {
            return Err(CliError::new(
                1,
                format!(
                    "No staged split outputs found for {}",
                    self.temp_base.display()
                ),
            ));
        }
        for staged_path in staged {
            let suffix = split_suffix(&staged_path).ok_or_else(|| {
                CliError::new(
                    1,
                    format!("Invalid staged split output: {}", staged_path.display()),
                )
            })?;
            let target_path = PathBuf::from(format!("{}.{}", self.target_base.display(), suffix));
            if target_path.exists() {
                replace_file(&staged_path, &target_path)?;
            } else {
                fs::rename(&staged_path, &target_path).map_err(|err| {
                    CliError::new(
                        1,
                        format!(
                            "Finalize split output failed {} -> {}: {err}",
                            staged_path.display(),
                            target_path.display()
                        ),
                    )
                })?;
            }
        }
        Ok(())
    }

    pub fn cleanup(&self) -> CliResult<()> {
        cleanup_split_outputs(&self.temp_base)
    }
}

/// Represents the staging plan for updating a single-file xunbak container.
#[cfg(feature = "xunbak")]
pub struct XunbakSingleUpdatePlan {
    pub target: PathBuf,
    pub work: PathBuf,
    pub staged_original: PathBuf,
}

#[cfg(feature = "xunbak")]
impl XunbakSingleUpdatePlan {
    pub fn prepare(target: &Path) -> CliResult<Self> {
        let work = target.with_extension("tmp.xunbak");
        let staged_original = build_temp_xunbak_base(target, "tmp-update-orig");
        cleanup_temp_target(&work)?;
        cleanup_temp_target(&staged_original)?;
        fs::rename(target, &staged_original).map_err(|err| {
            CliError::new(
                1,
                format!(
                    "Failed to stage existing xunbak for update {} -> {}: {err}",
                    target.display(),
                    staged_original.display()
                ),
            )
        })?;
        fs::copy(&staged_original, &work).map_err(|err| {
            CliError::new(
                1,
                format!(
                    "Failed to copy staged xunbak {} -> {}: {err}",
                    staged_original.display(),
                    work.display()
                ),
            )
        })?;
        Ok(Self {
            target: target.to_path_buf(),
            work,
            staged_original,
        })
    }

    pub fn work_path(&self) -> &Path {
        &self.work
    }

    pub fn commit(self) -> CliResult<()> {
        finalize_temp_target(&self.work, &self.target)?;
        cleanup_temp_target(&self.staged_original)
    }

    pub fn rollback(self) -> CliResult<()> {
        cleanup_temp_target(&self.work)?;
        if !self.target.exists() {
            fs::rename(&self.staged_original, &self.target).map_err(|err| {
                CliError::new(
                    1,
                    format!(
                        "Failed to restore staged xunbak {} -> {}: {err}",
                        self.staged_original.display(),
                        self.target.display()
                    ),
                )
            })?;
        }
        Ok(())
    }
}

/// Represents the staging plan for updating a split xunbak container set.
#[cfg(feature = "xunbak")]
pub struct XunbakSplitUpdatePlan {
    pub target_base: PathBuf,
    pub work_base: PathBuf,
}

#[cfg(feature = "xunbak")]
impl XunbakSplitUpdatePlan {
    pub fn prepare(target_base: &Path) -> CliResult<Self> {
        let work_base = build_temp_xunbak_base(target_base, "tmp-update-split");
        cleanup_split_outputs(&work_base)?;
        move_split_outputs(target_base, &work_base)?;
        Ok(Self {
            target_base: target_base.to_path_buf(),
            work_base,
        })
    }

    pub fn work_base_path(&self) -> &Path {
        &self.work_base
    }

    pub fn commit(self) -> CliResult<()> {
        move_split_outputs(&self.work_base, &self.target_base)
    }

    pub fn rollback(self) -> CliResult<()> {
        move_split_outputs(&self.work_base, &self.target_base)
    }
}

fn ensure_parent(target: &Path) -> CliResult<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            CliError::new(
                1,
                format!("Create output directory failed {}: {err}", parent.display()),
            )
        })?;
    }
    Ok(())
}

fn handle_existing_target(target: &Path, overwrite: OverwriteMode) -> CliResult<()> {
    if !target.exists() {
        return Ok(());
    }

    match overwrite {
        OverwriteMode::Fail => Err(CliError::with_details(
            2,
            format!("backup convert output already exists: {}", target.display()),
            &["Fix: Remove the destination or use `--overwrite replace`."],
        )),
        OverwriteMode::Replace => Ok(()),
        OverwriteMode::Ask => Err(CliError::with_details(
            2,
            format!(
                "backup convert cannot prompt to replace output in current context: {}",
                target.display()
            ),
            &["Fix: Pass `--overwrite replace` or `--overwrite fail`."],
        )),
    }
}

fn prepare_temp_target(target: &Path, temp: &Path, overwrite: OverwriteMode) -> CliResult<()> {
    ensure_parent(target)?;
    handle_existing_target(target, overwrite)?;
    cleanup_temp_target(temp)?;
    Ok(())
}

fn finalize_temp_target(temp: &Path, target: &Path) -> CliResult<()> {
    if target.exists() {
        replace_file(temp, target)?;
    } else {
        fs::rename(temp, target).map_err(|err| {
            CliError::new(
                1,
                format!(
                    "Finalize output failed {} -> {}: {err}",
                    temp.display(),
                    target.display()
                ),
            )
        })?;
    }
    Ok(())
}

fn cleanup_temp_target(temp: &Path) -> CliResult<()> {
    if temp.exists() {
        fs::remove_file(temp).map_err(|err| {
            CliError::new(
                1,
                format!("Cleanup temp file failed {}: {err}", temp.display()),
            )
        })?;
    }
    Ok(())
}

fn cleanup_temp_dir(temp: &Path) -> CliResult<()> {
    if temp.exists() {
        fs::remove_dir_all(temp).map_err(|err| {
            CliError::new(
                1,
                format!("Cleanup temp directory failed {}: {err}", temp.display()),
            )
        })?;
    }
    Ok(())
}

fn build_temp_dir_target(target: &Path) -> PathBuf {
    let file_name = target
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("dir");
    target.with_file_name(format!("{file_name}.tmp-dir"))
}

fn build_temp_sevenz_base(target: &Path) -> PathBuf {
    target.with_extension("tmp.7z")
}

fn remove_existing_target_path(path: &Path) -> CliResult<()> {
    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|err| {
            CliError::new(
                1,
                format!("Remove output directory failed {}: {err}", path.display()),
            )
        })?;
    } else if path.exists() {
        fs::remove_file(path).map_err(|err| {
            CliError::new(
                1,
                format!("Remove output file failed {}: {err}", path.display()),
            )
        })?;
    }
    Ok(())
}

fn handle_existing_numbered_target(target_base: &Path, overwrite: OverwriteMode) -> CliResult<()> {
    let existing = list_numbered_outputs(target_base)?;
    if existing.is_empty() {
        return Ok(());
    }
    match overwrite {
        OverwriteMode::Fail => Err(CliError::with_details(
            2,
            format!(
                "backup convert output already exists: {}",
                target_base.display()
            ),
            &["Fix: Remove the destination or use `--overwrite replace`."],
        )),
        OverwriteMode::Replace => {
            for path in existing {
                fs::remove_file(&path).map_err(|err| {
                    CliError::new(
                        1,
                        format!("Remove split output failed {}: {err}", path.display()),
                    )
                })?;
            }
            Ok(())
        }
        OverwriteMode::Ask => Err(CliError::with_details(
            2,
            format!(
                "backup convert cannot prompt to replace output in current context: {}",
                target_base.display()
            ),
            &["Fix: Pass `--overwrite replace` or `--overwrite fail`."],
        )),
    }
}

fn cleanup_numbered_outputs(base: &Path) -> CliResult<()> {
    for path in list_numbered_outputs(base)? {
        fs::remove_file(&path).map_err(|err| {
            CliError::new(
                1,
                format!("Cleanup split output failed {}: {err}", path.display()),
            )
        })?;
    }
    Ok(())
}

fn finalize_numbered_outputs(temp_base: &Path, target_base: &Path) -> CliResult<()> {
    let staged = list_numbered_outputs(temp_base)?;
    if staged.is_empty() {
        return Err(CliError::new(
            1,
            format!("No staged split outputs found for {}", temp_base.display()),
        ));
    }
    for staged_path in staged {
        let suffix = split_suffix(&staged_path).ok_or_else(|| {
            CliError::new(
                1,
                format!("Invalid staged split output: {}", staged_path.display()),
            )
        })?;
        let target_path = PathBuf::from(format!("{}.{}", target_base.display(), suffix));
        if target_path.exists() {
            replace_file(&staged_path, &target_path)?;
        } else {
            fs::rename(&staged_path, &target_path).map_err(|err| {
                CliError::new(
                    1,
                    format!(
                        "Finalize split output failed {} -> {}: {err}",
                        staged_path.display(),
                        target_path.display()
                    ),
                )
            })?;
        }
    }
    Ok(())
}

fn list_numbered_outputs(base: &Path) -> CliResult<Vec<PathBuf>> {
    let mut outputs = Vec::new();
    let Some(parent) = base.parent() else {
        return Ok(outputs);
    };
    let Some(prefix) = base.file_name().and_then(|name| name.to_str()) else {
        return Ok(outputs);
    };
    let read_dir = fs::read_dir(parent).map_err(|err| {
        CliError::new(
            1,
            format!(
                "Read split output directory failed {}: {err}",
                parent.display()
            ),
        )
    })?;
    for entry in read_dir.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(&format!("{prefix}."))
            && name.len() == prefix.len() + 4
            && name[prefix.len() + 1..]
                .chars()
                .all(|ch| ch.is_ascii_digit())
        {
            outputs.push(entry.path());
        }
    }
    outputs.sort();
    Ok(outputs)
}

fn split_suffix(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    let suffix = &name[name.len().checked_sub(3)?..];
    if suffix.chars().all(|ch| ch.is_ascii_digit()) {
        Some(suffix.to_string())
    } else {
        None
    }
}

#[cfg(feature = "xunbak")]
fn handle_existing_split_target(target_base: &Path, overwrite: OverwriteMode) -> CliResult<()> {
    let existing = list_split_outputs(target_base)?;
    if existing.is_empty() {
        return Ok(());
    }
    match overwrite {
        OverwriteMode::Fail => Err(CliError::with_details(
            2,
            format!(
                "backup convert output already exists: {}",
                target_base.display()
            ),
            &["Fix: Remove the destination or use `--overwrite replace`."],
        )),
        OverwriteMode::Replace => {
            for path in existing {
                fs::remove_file(&path).map_err(|err| {
                    CliError::new(
                        1,
                        format!("Remove split output failed {}: {err}", path.display()),
                    )
                })?;
            }
            Ok(())
        }
        OverwriteMode::Ask => Err(CliError::with_details(
            2,
            format!(
                "backup convert cannot prompt to replace output in current context: {}",
                target_base.display()
            ),
            &["Fix: Pass `--overwrite replace` or `--overwrite fail`."],
        )),
    }
}

#[cfg(feature = "xunbak")]
fn cleanup_split_outputs(base: &Path) -> CliResult<()> {
    for path in list_split_outputs(base)? {
        fs::remove_file(&path).map_err(|err| {
            CliError::new(
                1,
                format!("Cleanup split output failed {}: {err}", path.display()),
            )
        })?;
    }
    Ok(())
}

#[cfg(feature = "xunbak")]
fn list_split_outputs(base: &Path) -> CliResult<Vec<PathBuf>> {
    list_numbered_outputs(base)
}

#[cfg(feature = "xunbak")]
fn move_split_outputs(from_base: &Path, to_base: &Path) -> CliResult<()> {
    let from_paths = list_split_outputs(from_base)?;
    let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();

    for from_path in from_paths {
        let suffix = split_suffix(&from_path).ok_or_else(|| {
            CliError::new(1, format!("Invalid split path: {}", from_path.display()))
        })?;
        let to_path = PathBuf::from(format!("{}.{}", to_base.display(), suffix));
        if let Some(parent) = to_path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                CliError::new(
                    1,
                    format!(
                        "Create split output directory failed {}: {err}",
                        parent.display()
                    ),
                )
            })?;
        }
        if to_path.exists() {
            fs::remove_file(&to_path).map_err(|err| {
                CliError::new(
                    1,
                    format!("Remove split output failed {}: {err}", to_path.display()),
                )
            })?;
        }
        if let Err(err) = fs::rename(&from_path, &to_path) {
            for (moved_from, moved_to) in moved.into_iter().rev() {
                let _ = fs::rename(&moved_to, &moved_from);
            }
            return Err(CliError::new(
                1,
                format!(
                    "Move split output failed {} -> {}: {err}",
                    from_path.display(),
                    to_path.display()
                ),
            ));
        }
        moved.push((from_path, to_path));
    }

    Ok(())
}

#[cfg(feature = "xunbak")]
fn build_temp_xunbak_base(target: &Path, tag: &str) -> PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or(0);
    let stem = target
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("xunbak");
    let ext = target
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("xunbak");
    let file_name = format!("{stem}.{tag}-{}-{millis}.{ext}", Uuid::new_v4());
    target.with_file_name(file_name)
}

#[cfg(windows)]
fn replace_file(from: &Path, to: &Path) -> CliResult<()> {
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let mut from_w: Vec<u16> = from.as_os_str().encode_wide().collect();
    from_w.push(0);
    let mut to_w: Vec<u16> = to.as_os_str().encode_wide().collect();
    to_w.push(0);

    let ok = unsafe {
        MoveFileExW(
            from_w.as_ptr(),
            to_w.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if ok == 0 {
        return Err(CliError::new(
            1,
            format!(
                "Finalize zip output failed {} -> {}: {}",
                from.display(),
                to.display(),
                std::io::Error::last_os_error()
            ),
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
fn replace_file(from: &Path, to: &Path) -> CliResult<()> {
    fs::rename(from, to).map_err(|err| {
        CliError::new(
            1,
            format!(
                "Finalize zip output failed {} -> {}: {err}",
                from.display(),
                to.display()
            ),
        )
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::tempdir;

    use crate::backup_formats::OverwriteMode;

    use super::{DirOutputPlan, SevenZOutputPlan, SevenZSplitOutputPlan, ZipOutputPlan};
    #[cfg(feature = "xunbak")]
    use super::{
        XunbakOutputPlan, XunbakSingleUpdatePlan, XunbakSplitOutputPlan, XunbakSplitUpdatePlan,
    };

    #[test]
    fn zip_output_plan_creates_temp_path_and_finalizes() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("project.zip");
        let plan = ZipOutputPlan::prepare(&target, OverwriteMode::Replace).unwrap();
        fs::write(plan.temp_path(), "temp").unwrap();
        plan.finalize().unwrap();
        assert!(target.exists());
    }

    #[test]
    fn zip_output_plan_clean_removes_temp() {
        let dir = tempdir().unwrap();
        let temp = dir.path().join("test.tmp.zip");
        fs::write(&temp, "temp").unwrap();
        let plan = ZipOutputPlan {
            target: dir.path().join("dummy.zip"),
            temp,
        };
        plan.cleanup().unwrap();
        assert!(!plan.temp_path().exists());
    }

    #[test]
    fn sevenz_output_plan_creates_temp_path_and_finalizes() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("project.7z");
        let plan = SevenZOutputPlan::prepare(&target, OverwriteMode::Replace).unwrap();
        fs::write(plan.temp_path(), "temp").unwrap();
        plan.finalize().unwrap();
        assert!(target.exists());
    }

    #[test]
    fn sevenz_split_output_plan_finalizes_staged_volumes() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("project.7z");
        let plan = SevenZSplitOutputPlan::prepare(&target, OverwriteMode::Replace).unwrap();
        fs::write(format!("{}.001", plan.temp_base_path().display()), "one").unwrap();
        fs::write(format!("{}.002", plan.temp_base_path().display()), "two").unwrap();
        plan.finalize().unwrap();
        assert!(dir.path().join("project.7z.001").exists());
        assert!(dir.path().join("project.7z.002").exists());
    }

    #[test]
    fn dir_output_plan_creates_temp_path_and_finalizes() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("project_dir");
        let plan = DirOutputPlan::prepare(&target, OverwriteMode::Replace).unwrap();
        fs::create_dir_all(plan.temp_path()).unwrap();
        fs::write(plan.temp_path().join("a.txt"), "temp").unwrap();
        plan.finalize().unwrap();
        assert!(target.join("a.txt").exists());
    }

    #[cfg(feature = "xunbak")]
    #[test]
    fn xunbak_output_plan_creates_temp_path_and_finalizes() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("project.xunbak");
        let plan = XunbakOutputPlan::prepare(&target, OverwriteMode::Replace).unwrap();
        fs::write(plan.temp_path(), "temp").unwrap();
        plan.finalize().unwrap();
        assert!(target.exists());
    }

    #[cfg(feature = "xunbak")]
    #[test]
    fn xunbak_split_output_plan_finalizes_staged_volumes() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("project.xunbak");
        let plan = XunbakSplitOutputPlan::prepare(&target, OverwriteMode::Replace).unwrap();
        fs::write(format!("{}.001", plan.temp_base_path().display()), "one").unwrap();
        fs::write(format!("{}.002", plan.temp_base_path().display()), "two").unwrap();
        plan.finalize().unwrap();
        assert!(dir.path().join("project.xunbak.001").exists());
        assert!(dir.path().join("project.xunbak.002").exists());
    }

    #[cfg(feature = "xunbak")]
    #[test]
    fn xunbak_single_update_plan_rolls_back_original_file() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("project.xunbak");
        fs::write(&target, "old").unwrap();

        let plan = XunbakSingleUpdatePlan::prepare(&target).unwrap();
        assert!(!target.exists());
        fs::write(plan.work_path(), "new").unwrap();
        plan.rollback().unwrap();

        assert_eq!(fs::read_to_string(&target).unwrap(), "old");
    }

    #[cfg(feature = "xunbak")]
    #[test]
    fn xunbak_split_update_plan_rolls_back_original_volumes() {
        let dir = tempdir().unwrap();
        let base = dir.path().join("project.xunbak");
        fs::write(dir.path().join("project.xunbak.001"), "one").unwrap();
        fs::write(dir.path().join("project.xunbak.002"), "two").unwrap();

        let plan = XunbakSplitUpdatePlan::prepare(&base).unwrap();
        assert!(!dir.path().join("project.xunbak.001").exists());
        fs::write(
            format!("{}.001", plan.work_base_path().display()),
            "updated-one",
        )
        .unwrap();
        plan.rollback().unwrap();

        assert_eq!(
            fs::read_to_string(dir.path().join("project.xunbak.001")).unwrap(),
            "one"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("project.xunbak.002")).unwrap(),
            "two"
        );
    }
}
