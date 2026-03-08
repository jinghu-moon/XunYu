use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn target_profile_dir(out_dir: &Path) -> Option<PathBuf> {
    out_dir.ancestors().nth(3).map(Path::to_path_buf)
}

fn copy_if_exists(src: &Path, dst_dir: &Path) -> std::io::Result<()> {
    if !src.exists() {
        return Ok(());
    }
    fs::create_dir_all(dst_dir)?;
    let dst = dst_dir.join(src.file_name().unwrap_or_default());
    fs::copy(src, dst)?;
    Ok(())
}

fn turbojpeg_bin_dir_from_env() -> Option<PathBuf> {
    println!("cargo:rerun-if-env-changed=XUN_TURBOJPEG_DLL_DIR");
    if let Ok(dir) = env::var("XUN_TURBOJPEG_DLL_DIR") {
        let p = PathBuf::from(dir);
        if p.join("turbojpeg.dll").exists() {
            return Some(p);
        }
    }

    // 兼容旧构建链（若外部仍提供 DEP_TURBOJPEG_ROOT）。
    if let Ok(root) = env::var("DEP_TURBOJPEG_ROOT") {
        let p = Path::new(&root).join("bin");
        if p.join("turbojpeg.dll").exists() {
            return Some(p);
        }
    }

    None
}

fn alias_shim_candidates(manifest_dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(v) = env::var("XUN_ALIAS_SHIM_TEMPLATE") {
        let p = PathBuf::from(v);
        if !p.as_os_str().is_empty() {
            out.push(p);
        }
    }
    out.push(
        manifest_dir
            .join("target")
            .join("release-shim")
            .join("alias-shim.exe"),
    );
    out.push(
        manifest_dir
            .join("target")
            .join("release-shim")
            .join("deps")
            .join("alias_shim.exe"),
    );
    out
}

fn write_alias_shim_blob(out_dir: &Path, manifest_dir: &Path) {
    println!("cargo:rerun-if-env-changed=XUN_ALIAS_SHIM_TEMPLATE");
    let candidates = alias_shim_candidates(manifest_dir);
    for candidate in &candidates {
        println!("cargo:rerun-if-changed={}", candidate.display());
    }

    let mut payload = Vec::new();
    for candidate in candidates {
        let Ok(meta) = fs::metadata(&candidate) else {
            continue;
        };
        if !meta.is_file() || meta.len() == 0 {
            continue;
        }
        if let Ok(bytes) = fs::read(&candidate) {
            payload = bytes;
            break;
        }
    }
    let blob_path = out_dir.join("alias_shim_template.bin");
    if payload.is_empty()
        && fs::metadata(&blob_path)
            .map(|m| m.is_file() && m.len() > 0)
            .unwrap_or(false)
    {
        return;
    }
    if let Err(err) = fs::write(&blob_path, &payload) {
        println!(
            "cargo:warning=failed to write alias shim blob {}: {}",
            blob_path.display(),
            err
        );
    }
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let out_dir = match env::var("OUT_DIR") {
        Ok(v) => PathBuf::from(v),
        Err(_) => return,
    };
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    write_alias_shim_blob(&out_dir, &manifest_dir);

    if env::var("CARGO_FEATURE_IMG_TURBO").is_err() {
        return;
    }

    let Some(profile_dir) = target_profile_dir(&out_dir) else {
        return;
    };

    let Some(bin_dir) = turbojpeg_bin_dir_from_env() else {
        return;
    };

    let dlls = ["turbojpeg.dll", "jpeg62.dll"];

    for dll in dlls {
        let src = bin_dir.join(dll);
        if let Err(e) = copy_if_exists(&src, &profile_dir) {
            println!("cargo:warning=复制 {} 失败: {}", src.display(), e);
        }
    }
}
