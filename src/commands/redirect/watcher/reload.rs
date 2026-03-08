use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::config::RedirectProfile;

use super::super::config;
use super::ignore::{IgnoreSet, build_ignore_set, resolve_dest_dirs};

pub(super) fn maybe_reload_profile(
    cfg_path: &Path,
    last_cfg_mtime: &mut Option<SystemTime>,
    source_abs: &Path,
    profile_name: &str,
    profile: &mut RedirectProfile,
    ignore: &mut IgnoreSet,
    dest_dirs: &mut Vec<PathBuf>,
) {
    if !should_reload_config(cfg_path, last_cfg_mtime) {
        return;
    }

    let cfg = crate::config::load_config();
    match config::get_profile(&cfg, profile_name) {
        Ok(p) => match config::validate_profile(p) {
            Ok(_) => {
                *profile = p.clone();
                *ignore = build_ignore_set(source_abs, profile);
                *dest_dirs = resolve_dest_dirs(source_abs, profile);
                ui_println!("redirect watch: profile reloaded: {}", profile_name);
            }
            Err(msg) => {
                ui_println!(
                    "redirect watch: reload ignored (invalid profile {}): {}",
                    profile_name,
                    msg
                );
            }
        },
        Err(msg) => {
            ui_println!(
                "redirect watch: reload ignored (missing profile {}): {}",
                profile_name,
                msg
            );
        }
    }
}

fn file_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

fn should_reload_config(path: &Path, last: &mut Option<SystemTime>) -> bool {
    let cur = file_mtime(path);
    if &cur != last {
        *last = cur;
        return true;
    }
    false
}
