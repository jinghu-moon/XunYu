use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use windows::Win32::Foundation::{HLOCAL, LocalFree, WIN32_ERROR};
use windows::Win32::Security::Authorization::{
    GetNamedSecurityInfoW, SE_FILE_OBJECT, SetNamedSecurityInfoW,
};
use windows::Win32::Security::{
    ACL, ACL_SIZE_INFORMATION, AclSizeInformation, AddAce, DACL_SECURITY_INFORMATION,
    GetAclInformation, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID,
};
use windows::core::PCWSTR;

use crate::acl::error::AclError;
use crate::acl::privilege::enable_repair_privileges;
use crate::acl::reader::list_children;
use crate::acl::types::RepairStats;
use crate::acl::writer::lookup_account_sid;
use crate::config::AclConfig;

fn check_win32(status: WIN32_ERROR, context: impl Into<String>) -> Result<()> {
    if status.0 != 0 {
        let err = anyhow::Error::new(AclError::from_win32(status.0));
        return Err(err).context(context.into());
    }
    Ok(())
}

/// SID string for `BUILTIN\Administrators` (S-1-5-32-544).
const ADMINS_SID_STR: &str = "S-1-5-32-544";

/// `FileSystemRights::FullControl`
const FULL_CONTROL: u32 = 0x1F01FF;

// ACE type / flags
const OBJECT_INHERIT_ACE: u8 = 0x01;
const CONTAINER_INHERIT_ACE: u8 = 0x02;

// ── Public entry-point ────────────────────────────────────────────────────────

/// Perform a full forced ACL repair on `root` and all its children.
///
/// # Phases
/// 1. Enable repair privileges (`SeRestore`, `SeBackup`, `SeTakeOwnership`).
/// 2. Enumerate all targets (root + recursive children, excluding protected paths).
/// 3. **Phase 1** (parallel): take ownership → `BUILTIN\Administrators`.
/// 4. **Phase 2** (parallel): grant FullControl + reset inheritance in one write.
///
/// Progress is displayed via two [`ProgressBar`]s if stdout is a TTY, or
/// silently if `quiet = true`.
pub fn force_repair(root: &Path, config: &AclConfig, quiet: bool) -> Result<RepairStats> {
    // Phase 0: privileges
    enable_repair_privileges().context("force_repair: privilege activation")?;

    // Collect targets: root itself + all recursive children
    let mut targets: Vec<PathBuf> = vec![root.to_path_buf()];
    match list_children(root, true) {
        Ok(children) => targets.extend(children),
        Err(e) => eprintln!("[warn] 枚举子对象失败（部分路径可能跳过）: {e:#}"),
    }
    let total = targets.len();

    if total == 0 {
        return Ok(RepairStats::default());
    }

    // Resolve Administrators SID once
    let admins_sid = lookup_account_sid(ADMINS_SID_STR)
        .context("force_repair: cannot resolve BUILTIN\\Administrators SID")?;

    // Set rayon thread pool limit from config
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(config.throttle_limit)
        .build()
        .context("force_repair: failed to create thread pool")?;

    // Progress bars
    let mp = MultiProgress::new();
    let bar_style = ProgressStyle::with_template(
        "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>6}/{len:6}  {msg}",
    )
    .unwrap_or_else(|_| ProgressStyle::default_bar());

    let bar1 = if quiet {
        ProgressBar::hidden()
    } else {
        mp.add(ProgressBar::new(total as u64).with_style(bar_style.clone()))
    };
    bar1.set_message("阶段 1/2：夺取所有权");

    let bar2 = if quiet {
        ProgressBar::hidden()
    } else {
        mp.add(ProgressBar::new(total as u64).with_style(bar_style))
    };
    bar2.set_message("阶段 2/2：赋权+重置继承");

    // Shared error collectors
    let owner_fail: Arc<Mutex<Vec<(PathBuf, String)>>> = Arc::new(Mutex::new(Vec::new()));
    let owner_ok_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let acl_fail: Arc<Mutex<Vec<(PathBuf, String)>>> = Arc::new(Mutex::new(Vec::new()));
    let acl_ok_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    // ── Phase 1: take ownership ───────────────────────────────────────────────
    {
        let admins = admins_sid.clone();
        let bar = bar1.clone();
        let fail = owner_fail.clone();
        let ok_count = owner_ok_count.clone();

        pool.install(|| {
            targets.par_iter().for_each(|path| {
                match set_owner_to_admins(path, &admins) {
                    Ok(()) => {
                        ok_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                    Err(e) => {
                        let mut g = fail.lock().unwrap();
                        g.push((path.clone(), format!("{e:#}")));
                    }
                }
                bar.inc(1);
            });
        });
        bar1.finish_with_message(format!(
            "夺权完成  ✓ {} / ✗ {}",
            ok_count.load(std::sync::atomic::Ordering::Relaxed),
            owner_fail.lock().unwrap().len()
        ));
    }

    // ── Phase 2: grant FullControl + reset inheritance ────────────────────────
    {
        let admins = admins_sid.clone();
        let bar = bar2.clone();
        let fail = acl_fail.clone();
        let ok_count = acl_ok_count.clone();

        pool.install(|| {
            targets.par_iter().for_each(|path| {
                match set_full_control_reset_inherit(path, &admins) {
                    Ok(()) => {
                        ok_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                    Err(e) => {
                        let mut g = fail.lock().unwrap();
                        g.push((path.clone(), format!("{e:#}")));
                    }
                }
                bar.inc(1);
            });
        });
        bar2.finish_with_message(format!(
            "赋权完成  ✓ {} / ✗ {}",
            acl_ok_count.load(std::sync::atomic::Ordering::Relaxed),
            acl_fail.lock().unwrap().len()
        ));
    }

    let stats = RepairStats {
        total,
        owner_ok: owner_ok_count.load(std::sync::atomic::Ordering::Relaxed),
        owner_fail: Arc::try_unwrap(owner_fail).unwrap().into_inner().unwrap(),
        acl_ok: acl_ok_count.load(std::sync::atomic::Ordering::Relaxed),
        acl_fail: Arc::try_unwrap(acl_fail).unwrap().into_inner().unwrap(),
    };
    Ok(stats)
}

// ── Phase helpers ─────────────────────────────────────────────────────────────

/// Phase 1 — take ownership of `path`, setting owner to `admins_sid`.
pub fn set_owner_to_admins(path: &Path, admins_sid: &[u8]) -> Result<()> {
    use crate::acl::privilege::enable_privilege;
    let _ = enable_privilege("SeTakeOwnershipPrivilege");
    let _ = enable_privilege("SeRestorePrivilege");

    let sid = PSID(admins_sid.as_ptr() as *mut _);
    let pw: Vec<u16> = path
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let status = SetNamedSecurityInfoW(
            PCWSTR(pw.as_ptr()),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            sid,
            PSID::default(),
            None,
            None,
        );
        check_win32(
            status,
            format!("set_owner_to_admins: failed for {}", path.display()),
        )?;
    }
    Ok(())
}

/// Phase 2 — grant FullControl to `admins_sid` and enable DACL inheritance.
///
/// Both changes are written in a **single** `SetNamedSecurityInfoW` call.
pub fn set_full_control_reset_inherit(path: &Path, admins_sid: &[u8]) -> Result<()> {
    use crate::acl::privilege::enable_privilege;
    use windows::Win32::Security::{ACE_FLAGS, ACL_REVISION, AddAccessAllowedAceEx, InitializeAcl};

    let _ = enable_privilege("SeBackupPrivilege");
    let _ = enable_privilege("SeRestorePrivilege");

    let sid = PSID(admins_sid.as_ptr() as *mut _);
    let ace_flags = ACE_FLAGS(u32::from(OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE));

    let pw: Vec<u16> = path
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        // --- Read existing DACL ---
        let mut p_old_dacl: *mut ACL = std::ptr::null_mut();
        let mut p_sd: PSECURITY_DESCRIPTOR = PSECURITY_DESCRIPTOR::default();

        let status = GetNamedSecurityInfoW(
            PCWSTR(pw.as_ptr()),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            None,
            None,
            Some(&mut p_old_dacl),
            None,
            &mut p_sd,
        );
        check_win32(
            status,
            format!("Phase2: GetNamedSecurityInfoW for {}", path.display()),
        )?;

        // --- Compute required ACL size ---
        let sid_len = windows::Win32::Security::GetLengthSid(sid);
        let ace_size = std::mem::size_of::<windows::Win32::Security::ACCESS_ALLOWED_ACE>() as u32
            + sid_len
            - 4; // sizeof(DWORD) for SidStart

        // Get existing ACL size
        let old_size = if p_old_dacl.is_null() {
            0u32
        } else {
            let mut info = ACL_SIZE_INFORMATION::default();
            if GetAclInformation(
                p_old_dacl,
                &mut info as *mut _ as *mut _,
                std::mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
                AclSizeInformation,
            )
            .is_ok()
            {
                info.AclBytesFree + info.AclBytesInUse
            } else {
                512
            }
        };

        let new_acl_size =
            std::mem::size_of::<ACL>() as u32 + old_size + ace_size + 8 /* padding */;

        // --- Build new ACL ---
        let mut acl_buf = vec![0u8; new_acl_size as usize];
        let new_acl = acl_buf.as_mut_ptr() as *mut ACL;

        InitializeAcl(new_acl, new_acl_size, ACL_REVISION)
            .map_err(|_| AclError::last_win32())
            .context("Phase2: InitializeAcl failed")?;

        // Copy existing ACEs from old DACL
        if !p_old_dacl.is_null() {
            let mut old_info = ACL_SIZE_INFORMATION::default();
            if GetAclInformation(
                p_old_dacl,
                &mut old_info as *mut _ as *mut _,
                std::mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
                AclSizeInformation,
            )
            .is_ok()
            {
                for i in 0..old_info.AceCount {
                    let mut ace_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
                    if windows::Win32::Security::GetAce(p_old_dacl, i, &mut ace_ptr).is_ok()
                        && !ace_ptr.is_null()
                    {
                        let header = &*(ace_ptr as *const crate::acl::reader::AceHeaderPublic);
                        let _ = AddAce(
                            new_acl,
                            ACL_REVISION,
                            u32::MAX,
                            ace_ptr,
                            header.ace_size as u32,
                        );
                    }
                }
            }
        }

        // Add the FullControl Allow ACE for Administrators
        AddAccessAllowedAceEx(new_acl, ACL_REVISION, ace_flags, FULL_CONTROL, sid)
            .map_err(|_| AclError::last_win32())
            .context("Phase2: AddAccessAllowedAceEx failed")?;

        LocalFree(HLOCAL(p_sd.0 as *mut _));

        // --- Write DACL back (with UNPROTECTED flag to re-enable inheritance) ---
        // 0x20000000 = UNPROTECTED_DACL_SECURITY_INFORMATION
        let si = windows::Win32::Security::OBJECT_SECURITY_INFORMATION(
            DACL_SECURITY_INFORMATION.0 | 0x2000_0000,
        );

        let status = SetNamedSecurityInfoW(
            PCWSTR(pw.as_ptr()),
            SE_FILE_OBJECT,
            si,
            PSID::default(),
            PSID::default(),
            Some(new_acl),
            None,
        );
        check_win32(
            status,
            format!("Phase2: SetNamedSecurityInfoW for {}", path.display()),
        )?;
    }
    Ok(())
}

// ── Clean reset v3 (TreeSetNamedSecurityInfoW + per-object error capture) ────

/// Clean-reset v3: combines V2 kernel-side bulk performance with V1's
/// per-object error reporting via the `FN_PROGRESS` callback.
///
/// # Strategy
/// 1. Take ownership of root.
/// 2. Write root DACL: PROTECTED + Administrators + SYSTEM + extras.
/// 3. `TreeSetNamedSecurityInfoW(TREE_SEC_INFO_RESET)` with `FN_PROGRESS`:
///    - `status == 0` → count as success
///    - `status != 0` → record `(path, win32_error)` in shared error list
pub fn force_reset_clean_v3(
    root: &Path,
    _config: &AclConfig,
    quiet: bool,
    extra_principals: &[String],
) -> Result<RepairStats> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Instant;
    use windows::Win32::Security::Authorization::{
        ProgressInvokeEveryObject, TREE_SEC_INFO_RESET, TreeSetNamedSecurityInfoW,
    };
    use windows::Win32::Security::{ACE_FLAGS, ACL_REVISION, AddAccessAllowedAceEx, InitializeAcl};

    let t0 = Instant::now();

    enable_repair_privileges().context("force_reset_clean_v3: privilege")?;
    let t_priv = t0.elapsed();

    let admins_sid =
        lookup_account_sid(ADMINS_SID_STR).context("force_reset_clean_v3: Administrators SID")?;
    let system_sid = lookup_account_sid("S-1-5-18").context("force_reset_clean_v3: SYSTEM SID")?;

    let extra_sids: Vec<Vec<u8>> = extra_principals
        .iter()
        .filter_map(|p| {
            let name = p.trim();
            if name.is_empty() {
                return None;
            }
            match lookup_account_sid(name) {
                Ok(sid) => Some(sid),
                Err(e) => {
                    eprintln!("[warn] 无法解析账户 '{name}': {e:#}");
                    None
                }
            }
        })
        .collect();
    let t_sid = t0.elapsed();

    let pw: Vec<u16> = root
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    // Shared state passed to FN_PROGRESS callback.
    // Safety: TreeSetNamedSecurityInfoW invokes the callback synchronously on
    // the calling thread — no cross-thread aliasing.
    struct CallbackState {
        ok_count: *const AtomicUsize,
        fail_list: *const Mutex<Vec<(PathBuf, String)>>,
        bar: *const ProgressBar,
    }
    unsafe impl Send for CallbackState {}
    unsafe impl Sync for CallbackState {}

    let ok_count = AtomicUsize::new(0);
    let fail_list: Mutex<Vec<(PathBuf, String)>> = Mutex::new(Vec::new());
    let bar = if quiet {
        ProgressBar::hidden()
    } else {
        let b = ProgressBar::new_spinner();
        b.set_style(
            ProgressStyle::with_template(
                "{spinner:.cyan} [{elapsed_precise}] {pos} 对象已处理  {msg}",
            )
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
        );
        b.enable_steady_tick(std::time::Duration::from_millis(100));
        b.set_message("重置继承中…");
        b
    };

    let cb_state = CallbackState {
        ok_count: &ok_count as *const _,
        fail_list: &fail_list as *const _,
        bar: &bar as *const _,
    };

    unsafe extern "system" fn progress_cb(
        name: windows::core::PCWSTR,
        status: u32,
        _invoke: *mut windows::Win32::Security::Authorization::PROG_INVOKE_SETTING,
        args: *const std::ffi::c_void,
        security_set: windows::Win32::Foundation::BOOL,
    ) {
        // Only process post-set callbacks to avoid double-counting.
        if args.is_null() || !security_set.as_bool() {
            return;
        }
        unsafe {
            let state = &*(args as *const CallbackState);
            if status == 0 {
                let n = (*state.ok_count).fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                (*state.bar).set_position(n as u64);
            } else {
                // Capture path + win32 error message
                let path = {
                    let mut len = 0usize;
                    let ptr = name.0;
                    while !ptr.add(len).is_null() && *ptr.add(len) != 0 {
                        len += 1;
                    }
                    PathBuf::from(String::from_utf16_lossy(std::slice::from_raw_parts(
                        ptr, len,
                    )))
                };
                let err_msg = format!("{}", AclError::from_win32(status));
                if let Ok(mut g) = (*state.fail_list).lock() {
                    g.push((path, err_msg));
                }
            }
        }
    }

    unsafe {
        let admins = PSID(admins_sid.as_ptr() as *mut _);
        let system = PSID(system_sid.as_ptr() as *mut _);
        let ace_flags = ACE_FLAGS(u32::from(OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE));

        // ── Step 1: take ownership of root ────────────────────────────────────
        let status = SetNamedSecurityInfoW(
            PCWSTR(pw.as_ptr()),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            admins,
            PSID::default(),
            None,
            None,
        );
        check_win32(status, "v3: set root owner")?;
        let t_owner = t0.elapsed();

        // ── Step 2: build and write clean root DACL ───────────────────────────
        let sid_len = |sid: PSID| windows::Win32::Security::GetLengthSid(sid);
        let ace_size = |sl: u32| {
            std::mem::size_of::<windows::Win32::Security::ACCESS_ALLOWED_ACE>() as u32 + sl - 4
        };
        let extra_size: u32 = extra_sids
            .iter()
            .map(|s| ace_size(sid_len(PSID(s.as_ptr() as *mut _))))
            .sum();
        let acl_size = std::mem::size_of::<ACL>() as u32
            + ace_size(sid_len(admins))
            + ace_size(sid_len(system))
            + extra_size
            + 16;

        let mut acl_buf = vec![0u8; acl_size as usize];
        let new_acl = acl_buf.as_mut_ptr() as *mut ACL;
        InitializeAcl(new_acl, acl_size, ACL_REVISION)
            .map_err(|_| AclError::last_win32())
            .context("v3: InitializeAcl")?;
        AddAccessAllowedAceEx(new_acl, ACL_REVISION, ace_flags, FULL_CONTROL, admins)
            .map_err(|_| AclError::last_win32())
            .context("v3: AddAce Administrators")?;
        AddAccessAllowedAceEx(new_acl, ACL_REVISION, ace_flags, FULL_CONTROL, system)
            .map_err(|_| AclError::last_win32())
            .context("v3: AddAce SYSTEM")?;
        for (i, s) in extra_sids.iter().enumerate() {
            let esid = PSID(s.as_ptr() as *mut _);
            AddAccessAllowedAceEx(new_acl, ACL_REVISION, ace_flags, FULL_CONTROL, esid)
                .map_err(|_| AclError::last_win32())
                .with_context(|| format!("v3: AddAce extra[{i}]"))?;
        }

        let si_root = windows::Win32::Security::OBJECT_SECURITY_INFORMATION(
            DACL_SECURITY_INFORMATION.0 | OWNER_SECURITY_INFORMATION.0 | 0x8000_0000,
        );
        let status = SetNamedSecurityInfoW(
            PCWSTR(pw.as_ptr()),
            SE_FILE_OBJECT,
            si_root,
            admins,
            PSID::default(),
            Some(new_acl),
            None,
        );
        check_win32(status, "v3: write root DACL")?;
        let t_root = t0.elapsed();

        // ── Step 3: kernel bulk-reset children with per-object error capture ──
        let tree_status = TreeSetNamedSecurityInfoW(
            PCWSTR(pw.as_ptr()),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            admins,
            PSID::default(),
            None,
            None,
            TREE_SEC_INFO_RESET,
            Some(progress_cb),
            ProgressInvokeEveryObject,
            Some(&cb_state as *const _ as *const std::ffi::c_void),
        );
        let t_tree = t0.elapsed();

        let ok = ok_count.load(Ordering::Relaxed);
        let failures = fail_list.lock().unwrap();
        bar.finish_with_message(format!("完成  ✓ {} / ✗ {}", ok, failures.len()));

        eprintln!("[timing-v3] privilege:  {:>8.2?}", t_priv);
        eprintln!(
            "[timing-v3] sid_resolve:{:>8.2?}  (+{:.2?})",
            t_sid,
            t_sid - t_priv
        );
        eprintln!(
            "[timing-v3] root_owner: {:>8.2?}  (+{:.2?})",
            t_owner,
            t_owner - t_sid
        );
        eprintln!(
            "[timing-v3] root_dacl:  {:>8.2?}  (+{:.2?})",
            t_root,
            t_root - t_owner
        );
        eprintln!(
            "[timing-v3] tree_reset: {:>8.2?}  (+{:.2?})",
            t_tree,
            t_tree - t_root
        );
        eprintln!("[timing-v3] total_wall: {:>8.2?}", t_tree);

        // Check overall status after collecting per-object errors
        check_win32(tree_status, "v3: TreeSetNamedSecurityInfoW")?;

        let total = ok + failures.len();
        Ok(RepairStats {
            total,
            owner_ok: ok,
            owner_fail: vec![],
            acl_ok: ok,
            acl_fail: failures
                .iter()
                .map(|(p, e)| (p.clone(), e.clone()))
                .collect(),
        })
    }
}
