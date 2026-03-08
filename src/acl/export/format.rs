use std::path::Path;

use anyhow::{Context, Result};

use crate::acl::orphan::OrphanEntry;
use crate::acl::types::DiffResult;
use crate::acl::types::{AclSnapshot, RepairStats};

/// Export a `DiffResult` as CSV. Returns the number of data rows written.
pub(super) fn export_diff_csv(diff: &DiffResult, dest: &Path) -> Result<usize> {
    let mut wtr = csv::Writer::from_path(dest)
        .with_context(|| format!("export_diff_csv: cannot open {}", dest.display()))?;
    wtr.write_record(["差异方向", "主体", "权限", "访问类型", "来源", "SID"])?;
    let mut count = 0usize;
    for e in &diff.only_in_a {
        let rights = e.rights_display();
        let ace_type = e.ace_type.to_string();
        let src = if e.is_inherited { "继承" } else { "显式" };
        wtr.write_record([
            "仅在A",
            e.principal.as_str(),
            rights.as_str(),
            ace_type.as_str(),
            src,
            e.raw_sid.as_str(),
        ])?;
        count += 1;
    }
    for e in &diff.only_in_b {
        let rights = e.rights_display();
        let ace_type = e.ace_type.to_string();
        let src = if e.is_inherited { "继承" } else { "显式" };
        wtr.write_record([
            "仅在B",
            e.principal.as_str(),
            rights.as_str(),
            ace_type.as_str(),
            src,
            e.raw_sid.as_str(),
        ])?;
        count += 1;
    }
    wtr.flush()?;
    Ok(count)
}

/// Export a list of orphaned SID entries as CSV.
pub(super) fn export_orphans_csv(orphans: &[OrphanEntry], dest: &Path) -> Result<usize> {
    let mut wtr = csv::Writer::from_path(dest)
        .with_context(|| format!("export_orphans_csv: cannot open {}", dest.display()))?;
    wtr.write_record(["路径", "孤儿SID", "访问类型", "权限", "权限掩码"])?;
    for o in orphans {
        let path = o.path.to_string_lossy().into_owned();
        let ace_type = o.ace.ace_type.to_string();
        let rights = o.ace.rights_display();
        let mask = format!("{:#010x}", o.ace.rights_mask);
        wtr.write_record([
            path.as_str(),
            o.ace.raw_sid.as_str(),
            ace_type.as_str(),
            rights.as_str(),
            mask.as_str(),
        ])?;
    }
    wtr.flush()?;
    Ok(orphans.len())
}

/// Export failed paths from a `RepairStats` as CSV.
pub(super) fn export_repair_errors_csv(stats: &RepairStats, dest: &Path) -> Result<usize> {
    let mut wtr = csv::Writer::from_path(dest)
        .with_context(|| format!("export_repair_errors_csv: cannot open {}", dest.display()))?;
    wtr.write_record(["阶段", "路径", "错误信息"])?;
    let mut count = 0usize;
    for (p, e) in &stats.owner_fail {
        wtr.write_record(["夺权", &p.to_string_lossy(), e.as_str()])?;
        count += 1;
    }
    for (p, e) in &stats.acl_fail {
        wtr.write_record(["赋权", &p.to_string_lossy(), e.as_str()])?;
        count += 1;
    }
    wtr.flush()?;
    Ok(count)
}

/// Export all ACE entries from a snapshot as CSV.
pub(super) fn export_acl_csv(snapshot: &AclSnapshot, dest: &Path) -> Result<usize> {
    let mut wtr = csv::Writer::from_path(dest)
        .with_context(|| format!("export_acl_csv: cannot open {}", dest.display()))?;
    wtr.write_record([
        "访问类型",
        "来源",
        "主体",
        "权限名",
        "权限掩码",
        "继承标志",
        "传播标志",
        "是否孤儿",
        "SID",
    ])?;
    for e in &snapshot.entries {
        let ace_type = e.ace_type.to_string();
        let rights = e.rights_display();
        let mask = format!("{:#010x}", e.rights_mask);
        let inheritance = e.inheritance.to_string();
        let propagation = e.propagation.to_string();
        let src = if e.is_inherited { "继承" } else { "显式" };
        let orphan = if e.is_orphan { "是" } else { "否" };
        wtr.write_record([
            ace_type.as_str(),
            src,
            e.principal.as_str(),
            rights.as_str(),
            mask.as_str(),
            inheritance.as_str(),
            propagation.as_str(),
            orphan,
            e.raw_sid.as_str(),
        ])?;
    }
    wtr.flush()?;
    Ok(snapshot.entries.len())
}
