use std::path::PathBuf;

use serde::{Deserialize, Serialize};

// ── Rights display table ──────────────────────────────────────────────────────

/// Mapping from raw `u32` FileSystemRights mask → `(short_name, description)`.
pub const RIGHTS_TABLE: &[(u32, &str, &str)] = &[
    (
        2_032_127,
        "FullControl",
        "完全控制（读/写/执行/删除/改ACL/取所有权）",
    ),
    (1_245_631, "Modify", "修改（读/写/执行/删除，不含改ACL）"),
    (
        1_179_817,
        "ReadAndExecute",
        "读取并执行（查看内容和运行程序，不能写入）",
    ),
    (
        1_180_086,
        "Read+Write",
        "读取+写入（查看并修改内容，不能运行或删除）",
    ),
    (1_179_785, "Read", "只读（仅能查看文件和目录内容）"),
    (278, "Write", "写入（可创建/修改文件，不含读取）"),
    (
        1_180_095,
        "ReadExec+Write",
        "读取并执行+写入（可读/写/运行，不能删除）",
    ),
];

/// Look up the short display name for a rights mask.
pub fn rights_short(mask: u32) -> String {
    // Strip Synchronize (0x00100000) before lookup
    let m = mask & !0x00100000;
    for &(k, short, _) in RIGHTS_TABLE {
        if m == (k & !0x00100000) {
            return short.to_string();
        }
    }
    format!("{mask:#010x}")
}

/// Look up the long description for a rights mask.
#[allow(dead_code)]
pub fn rights_desc(mask: u32) -> &'static str {
    let m = mask & !0x00100000;
    for &(k, _, desc) in RIGHTS_TABLE {
        if m == (k & !0x00100000) {
            return desc;
        }
    }
    "组合权限标志位"
}

// ── Protected path names ──────────────────────────────────────────────────────

/// Leaf names that should never be modified by bulk operations.
pub const PROTECTED_NAMES: &[&str] = &[
    "$RECYCLE.BIN",
    "System Volume Information",
    "pagefile.sys",
    "swapfile.sys",
    "hiberfil.sys",
    "DumpStack.log.tmp",
];

// ── Enumerations ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AceType {
    Allow,
    Deny,
}

impl std::fmt::Display for AceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AceType::Allow => write!(f, "Allow"),
            AceType::Deny => write!(f, "Deny"),
        }
    }
}

/// Whether a specific permission is allowed, denied, or has no matching rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriState {
    Allow,
    Deny,
    NoRule,
}

impl std::fmt::Display for TriState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TriState::Allow => write!(f, "允许"),
            TriState::Deny => write!(f, "拒绝"),
            TriState::NoRule => write!(f, "无规则"),
        }
    }
}

// ── Flags (thin wrappers for readability) ─────────────────────────────────────

/// Inheritance flags bit-mask (mirrors `OBJECT_INHERIT_ACE` etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct InheritanceFlags(pub u32);

impl InheritanceFlags {
    pub const NONE: Self = Self(0);
    pub const OBJECT_INHERIT: Self = Self(0x1);
    pub const CONTAINER_INHERIT: Self = Self(0x2);
    pub const BOTH: Self = Self(0x3);

    #[allow(dead_code)]
    pub fn has_object_inherit(self) -> bool {
        self.0 & 0x1 != 0
    }
    #[allow(dead_code)]
    pub fn has_container_inherit(self) -> bool {
        self.0 & 0x2 != 0
    }
}

impl std::fmt::Display for InheritanceFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            0 => write!(f, "None"),
            1 => write!(f, "ObjectInherit"),
            2 => write!(f, "ContainerInherit"),
            3 => write!(f, "ContainerInherit|ObjectInherit"),
            n => write!(f, "{n:#04x}"),
        }
    }
}

/// Propagation flags bit-mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct PropagationFlags(pub u32);

impl PropagationFlags {
    pub const NONE: Self = Self(0);
    #[allow(dead_code)]
    pub const NO_PROPAGATE: Self = Self(0x1);
    #[allow(dead_code)]
    pub const INHERIT_ONLY: Self = Self(0x2);
}

impl std::fmt::Display for PropagationFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            0 => write!(f, "None"),
            1 => write!(f, "NoPropagateInherit"),
            2 => write!(f, "InheritOnly"),
            3 => write!(f, "NoPropagateInherit|InheritOnly"),
            n => write!(f, "{n:#04x}"),
        }
    }
}

// ── Core structs ──────────────────────────────────────────────────────────────

/// A single Access Control Entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AceEntry {
    /// Resolved account name (`DOMAIN\User`).  Falls back to raw SID string
    /// when the SID cannot be resolved.
    pub principal: String,

    /// Raw SID string (e.g. `"S-1-5-32-544"`).  Always present.
    pub raw_sid: String,

    /// FileSystemRights bit-mask.
    pub rights_mask: u32,

    pub ace_type: AceType,
    pub inheritance: InheritanceFlags,
    pub propagation: PropagationFlags,

    /// `true` when this ACE was inherited from a parent container.
    pub is_inherited: bool,

    /// `true` when `principal` could not be resolved (orphaned SID).
    pub is_orphan: bool,
}

impl AceEntry {
    /// Short display name for the rights mask.
    pub fn rights_display(&self) -> String {
        rights_short(self.rights_mask)
    }

    /// Long description for the rights mask.
    #[allow(dead_code)]
    pub fn rights_description(&self) -> &'static str {
        rights_desc(self.rights_mask)
    }

    /// Returns a stable string key suitable for set-difference operations.
    ///
    /// Format:  `principal|ace_type|rights_mask|inheritance|is_inherited`
    pub fn diff_key(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}",
            self.principal, self.ace_type, self.rights_mask, self.inheritance.0, self.is_inherited,
        )
    }
}

/// Snapshot of a path's complete ACL (owner + DACL).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AclSnapshot {
    pub path: PathBuf,
    /// Resolved owner account name.
    pub owner: String,
    /// `true` when DACL inheritance from the parent is **disabled**.
    pub is_protected: bool,
    pub entries: Vec<AceEntry>,
}

impl AclSnapshot {
    pub fn allow_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.ace_type == AceType::Allow)
            .count()
    }

    pub fn deny_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.ace_type == AceType::Deny)
            .count()
    }

    pub fn orphan_count(&self) -> usize {
        self.entries.iter().filter(|e| e.is_orphan).count()
    }

    pub fn explicit_count(&self) -> usize {
        self.entries.iter().filter(|e| !e.is_inherited).count()
    }

    pub fn inherited_count(&self) -> usize {
        self.entries.iter().filter(|e| e.is_inherited).count()
    }
}

/// Result of comparing two [`AclSnapshot`]s.
#[derive(Debug, Clone)]
pub struct DiffResult {
    /// ACEs present in A but absent in B.
    pub only_in_a: Vec<AceEntry>,
    /// ACEs present in B but absent in A.
    pub only_in_b: Vec<AceEntry>,
    /// Count of ACEs identical in both.
    pub common_count: usize,
    /// `Some((a_owner, b_owner))` when owners differ.
    pub owner_diff: Option<(String, String)>,
    /// `Some((a_protected, b_protected))` when inheritance state differs.
    pub inherit_diff: Option<(bool, bool)>,
}

impl DiffResult {
    #[allow(dead_code)]
    pub fn has_diff(&self) -> bool {
        !self.only_in_a.is_empty()
            || !self.only_in_b.is_empty()
            || self.owner_diff.is_some()
            || self.inherit_diff.is_some()
    }
}

/// Per-path statistics collected during [`force_repair`].
#[derive(Debug, Default)]
pub struct RepairStats {
    pub total: usize,
    /// Paths where `set_owner` succeeded.
    pub owner_ok: usize,
    /// Paths where `set_owner` failed: `(path, error_message)`.
    pub owner_fail: Vec<(PathBuf, String)>,
    /// Paths where ACL write succeeded.
    pub acl_ok: usize,
    /// Paths where ACL write failed: `(path, error_message)`.
    pub acl_fail: Vec<(PathBuf, String)>,
}

impl RepairStats {
    pub fn total_fail(&self) -> usize {
        self.owner_fail.len() + self.acl_fail.len()
    }

    pub fn summary(&self) -> String {
        format!(
            "共 {} 个对象 | 夺权: {} 成功 / {} 失败 | 赋权: {} 成功 / {} 失败",
            self.total,
            self.owner_ok,
            self.owner_fail.len(),
            self.acl_ok,
            self.acl_fail.len(),
        )
    }
}

/// Effective access result for a single user on a single path.
#[derive(Debug, Clone)]
pub struct EffectiveAccess {
    pub read: TriState,
    pub write: TriState,
    pub execute: TriState,
    pub delete: TriState,
    pub change_perms: TriState,
    pub take_ownership: TriState,
    /// Raw allow mask after Deny has been removed.
    pub effective_mask: u32,
    pub allow_mask: u32,
    pub deny_mask: u32,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
