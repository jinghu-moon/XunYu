//! Bookmark 业务逻辑服务
//!
//! 封装书签管理操作，支持 CommandSpec 和 Operation 实现。
//! 27 个子命令 + 5 个 tag 子命令。

use std::time::{SystemTime, UNIX_EPOCH};

use crate::bookmark::state::Store;
use crate::bookmark::storage::db_path;
use crate::xun_core::error::XunError;
use crate::xun_core::operation::{Change, Operation, OperationResult, Preview, RiskLevel};
use crate::xun_core::value::Value;

/// 将 CliError 转换为 XunError。
fn cli_err(msg: &str) -> impl Fn(crate::output::CliError) -> XunError + '_ {
    move |e| XunError::user(format!("{msg}: {}", e.message))
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn load_store() -> Result<Store, XunError> {
    let path = db_path();
    Store::load_or_default(&path)
        .map_err(|e| XunError::Internal(anyhow::anyhow!("failed to load bookmark store: {e}")))
}

fn save_store(store: &mut Store) -> Result<(), XunError> {
    let path = db_path();
    let now = now_unix();
    store
        .save(&path, now)
        .map_err(|e| XunError::Internal(anyhow::anyhow!("failed to save bookmark store: {e}")))
}

/// 将单个 bookmark 转为 Value::Record。
fn bookmark_to_value(b: &crate::bookmark::state::Bookmark) -> Value {
    let mut rec = crate::xun_core::value::Record::new();
    rec.insert(
        "name".into(),
        Value::String(b.name.clone().unwrap_or_default()),
    );
    rec.insert("path".into(), Value::String(b.path.clone()));
    rec.insert(
        "tags".into(),
        Value::List(b.tags.iter().map(|t| Value::String(t.clone())).collect()),
    );
    rec.insert("visits".into(), Value::Int(b.visit_count.unwrap_or(0) as i64));
    rec.insert("pinned".into(), Value::Bool(b.pinned));
    if let Some(ref ws) = b.workspace {
        rec.insert("workspace".into(), Value::String(ws.clone()));
    }
    rec.insert("source".into(), Value::String(format!("{:?}", b.source)));
    Value::Record(rec)
}

// ============================================================
// BookmarkDeleteOp — Operation trait 实现
// ============================================================

/// 书签删除操作（实现 Operation trait）。
pub struct BookmarkDeleteOp {
    name: String,
    preview: Preview,
}

impl BookmarkDeleteOp {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let preview = Preview::new(format!("Delete bookmark '{}'", name))
            .add_change(Change::new("delete", &name))
            .with_risk_level(RiskLevel::Medium);
        Self { name, preview }
    }
}

impl Operation for BookmarkDeleteOp {
    fn preview(&self) -> &Preview {
        &self.preview
    }

    fn execute(&self, _ctx: &mut crate::xun_core::context::CmdContext) -> Result<OperationResult, XunError> {
        let mut store = load_store()?;
        store.delete_explicit(&self.name).map_err(|e| {
            XunError::user(format!("failed to delete bookmark '{}': {e}", self.name))
        })?;
        save_store(&mut store)?;
        Ok(OperationResult::new().with_changes_applied(1))
    }

    fn rollback(&self, _ctx: &mut crate::xun_core::context::CmdContext) -> Result<(), XunError> {
        Err(XunError::user("use 'xun bookmark undo' to restore deleted bookmarks"))
    }
}

// ── BookmarkUndoOp / BookmarkRedoOp ──────────────────────────────

/// 书签撤销操作（实现 Operation trait）。
pub struct BookmarkUndoOp {
    steps: usize,
    preview: Preview,
}

impl BookmarkUndoOp {
    pub fn new(steps: usize) -> Self {
        let preview = Preview::new(format!("Undo last {steps} bookmark operation(s)"))
            .add_change(Change::new("undo_steps", &steps.to_string()))
            .with_risk_level(RiskLevel::Low);
        Self { steps, preview }
    }
}

impl Operation for BookmarkUndoOp {
    fn preview(&self) -> &Preview {
        &self.preview
    }

    fn execute(&self, _ctx: &mut crate::xun_core::context::CmdContext) -> Result<OperationResult, XunError> {
        let path = db_path();
        let mut store = load_store()?;
        let applied = crate::bookmark::undo::run_undo_steps(&path, &mut store, self.steps)
            .map_err(|e| XunError::user(format!("undo failed: {}", e.message)))?;
        save_store(&mut store)?;
        Ok(OperationResult::new().with_changes_applied(applied as u32))
    }

    fn rollback(&self, ctx: &mut crate::xun_core::context::CmdContext) -> Result<(), XunError> {
        let op = BookmarkRedoOp::new(self.steps);
        op.execute(ctx)?;
        Ok(())
    }
}

/// 书签重做操作（实现 Operation trait）。
pub struct BookmarkRedoOp {
    steps: usize,
    preview: Preview,
}

impl BookmarkRedoOp {
    pub fn new(steps: usize) -> Self {
        let preview = Preview::new(format!("Redo last {steps} bookmark operation(s)"))
            .add_change(Change::new("redo_steps", &steps.to_string()))
            .with_risk_level(RiskLevel::Low);
        Self { steps, preview }
    }
}

impl Operation for BookmarkRedoOp {
    fn preview(&self) -> &Preview {
        &self.preview
    }

    fn execute(&self, _ctx: &mut crate::xun_core::context::CmdContext) -> Result<OperationResult, XunError> {
        let path = db_path();
        let mut store = load_store()?;
        let applied = crate::bookmark::undo::run_redo_steps(&path, &mut store, self.steps)
            .map_err(|e| XunError::user(format!("redo failed: {}", e.message)))?;
        save_store(&mut store)?;
        Ok(OperationResult::new().with_changes_applied(applied as u32))
    }

    fn rollback(&self, ctx: &mut crate::xun_core::context::CmdContext) -> Result<(), XunError> {
        let op = BookmarkUndoOp::new(self.steps);
        op.execute(ctx)?;
        Ok(())
    }
}

// ============================================================
// CRUD 服务
// ============================================================

/// 保存当前目录为书签。
pub fn save_bookmark(
    name: Option<&str>,
    tag: Option<&str>,
    desc: Option<&str>,
    workspace: Option<&str>,
) -> Result<Value, XunError> {
    let mut store = load_store()?;
    let now = now_unix();
    let cwd = std::env::current_dir()
        .map_err(|e| XunError::user(format!("cannot get current dir: {e}")))?;
    let home: Option<std::path::PathBuf> = std::env::var("USERPROFILE")
        .ok()
        .or_else(|| std::env::var("HOME").ok())
        .map(std::path::PathBuf::from);
    let resolved_name = name.unwrap_or_else(|| {
        cwd.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("bookmark")
    });
    store
        .set(resolved_name, &cwd.to_string_lossy(), &cwd, home.as_deref(), now)
        .map_err(|e| XunError::user(format!("failed to save bookmark: {e}")))?;
    // 设置 metadata
    let tags: Vec<String> = tag
        .map(|t| t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        .unwrap_or_default();
    if !tags.is_empty() || desc.is_some() {
        let _ = store.set_explicit_metadata(resolved_name, tags, desc.unwrap_or("").to_string());
    }
    if let Some(ws) = workspace {
        let _ = store.set_explicit_workspace(resolved_name, Some(ws.to_string()));
    }
    save_store(&mut store)?;
    Ok(Value::String(resolved_name.to_string()))
}

/// 设置书签（指定路径）。
pub fn set_bookmark(
    name: &str,
    path: Option<&str>,
    tag: Option<&str>,
    desc: Option<&str>,
    workspace: Option<&str>,
) -> Result<Value, XunError> {
    let mut store = load_store()?;
    let now = now_unix();
    let cwd = std::env::current_dir()
        .map_err(|e| XunError::user(format!("cannot get current dir: {e}")))?;
    let home: Option<std::path::PathBuf> = std::env::var("USERPROFILE")
        .ok()
        .or_else(|| std::env::var("HOME").ok())
        .map(std::path::PathBuf::from);
    let raw_path = path.unwrap_or(".");
    store
        .set(name, raw_path, &cwd, home.as_deref(), now)
        .map_err(|e| XunError::user(format!("failed to set bookmark: {e}")))?;
    let tags: Vec<String> = tag
        .map(|t| t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        .unwrap_or_default();
    if !tags.is_empty() || desc.is_some() {
        let _ = store.set_explicit_metadata(name, tags, desc.unwrap_or("").to_string());
    }
    if let Some(ws) = workspace {
        let _ = store.set_explicit_workspace(name, Some(ws.to_string()));
    }
    save_store(&mut store)?;
    Ok(Value::String(name.to_string()))
}

/// rename 书签。
pub fn rename_bookmark(old: &str, new: &str) -> Result<(), XunError> {
    let mut store = load_store()?;
    store
        .rename(old, new)
        .map_err(|e| XunError::user(format!("failed to rename bookmark: {e}")))?;
    save_store(&mut store)
}

// ============================================================
// Pin / Unpin / Touch
// ============================================================

/// pin 书签。
pub fn pin_bookmark(name: &str) -> Result<(), XunError> {
    let mut store = load_store()?;
    store
        .pin(name)
        .map_err(|e| XunError::user(format!("failed to pin bookmark: {e}")))?;
    save_store(&mut store)
}

/// unpin 书签。
pub fn unpin_bookmark(name: &str) -> Result<(), XunError> {
    let mut store = load_store()?;
    store
        .unpin(name)
        .map_err(|e| XunError::user(format!("failed to unpin bookmark: {e}")))?;
    save_store(&mut store)
}

/// touch 书签（更新访问计数）。
pub fn touch_bookmark(name: &str) -> Result<(), XunError> {
    let mut store = load_store()?;
    let now = now_unix();
    store
        .touch_explicit(name, now)
        .map_err(|e| XunError::user(format!("failed to touch bookmark: {e}")))?;
    save_store(&mut store)
}

// ============================================================
// Tag 操作
// ============================================================

/// 添加标签。
pub fn tag_add(name: &str, tags: &str) -> Result<Value, XunError> {
    let mut store = load_store()?;
    let tag_list: Vec<String> = tags.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    let added = store
        .add_tags(name, &tag_list)
        .map_err(|e| XunError::user(format!("failed to add tags: {e}")))?;
    save_store(&mut store)?;
    Ok(Value::Int(added as i64))
}

/// 批量添加标签。
pub fn tag_add_batch(names: &[String], tags: &str) -> Result<Value, XunError> {
    let mut store = load_store()?;
    let tag_list: Vec<String> = tags.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    let mut total = 0usize;
    for name in names {
        match store.add_tags(name, &tag_list) {
            Ok(n) => total += n,
            Err(_) => continue,
        }
    }
    save_store(&mut store)?;
    Ok(Value::Int(total as i64))
}

/// 移除标签。
pub fn tag_remove(name: &str, tags: &str) -> Result<Value, XunError> {
    let mut store = load_store()?;
    let tag_list: Vec<String> = tags.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    let removed = store
        .remove_tags(name, &tag_list)
        .map_err(|e| XunError::user(format!("failed to remove tags: {e}")))?;
    save_store(&mut store)?;
    Ok(Value::Int(removed as i64))
}

/// 列出所有标签及计数。
pub fn tag_list() -> Result<Value, XunError> {
    let store = load_store()?;
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for b in &store.bookmarks {
        for t in &b.tags {
            *counts.entry(t.clone()).or_insert(0) += 1;
        }
    }
    let mut sorted: Vec<(String, usize)> = counts.into_iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let items: Vec<Value> = sorted
        .into_iter()
        .map(|(tag, count)| {
            let mut rec = crate::xun_core::value::Record::new();
            rec.insert("tag".into(), Value::String(tag));
            rec.insert("count".into(), Value::Int(count as i64));
            Value::Record(rec)
        })
        .collect();
    Ok(Value::List(items))
}

/// 全局重命名标签。
pub fn tag_rename(old: &str, new: &str) -> Result<Value, XunError> {
    let mut store = load_store()?;
    let renamed = store.rename_tag_globally(old, new);
    save_store(&mut store)?;
    Ok(Value::Int(renamed as i64))
}

// ============================================================
// Undo / Redo
// ============================================================

/// 撤销操作。
pub fn undo_bookmark(steps: usize) -> Result<Value, XunError> {
    let path = db_path();
    let mut store = load_store()?;
    let applied = crate::bookmark::undo::run_undo_steps(&path, &mut store, steps)
        .map_err(|e| XunError::user(format!("undo failed: {}", e.message)))?;
    save_store(&mut store)?;
    Ok(Value::Int(applied as i64))
}

/// 重做操作。
pub fn redo_bookmark(steps: usize) -> Result<Value, XunError> {
    let path = db_path();
    let mut store = load_store()?;
    let applied = crate::bookmark::undo::run_redo_steps(&path, &mut store, steps)
        .map_err(|e| XunError::user(format!("redo failed: {}", e.message)))?;
    save_store(&mut store)?;
    Ok(Value::Int(applied as i64))
}

// ============================================================
// 查询服务
// ============================================================

/// 列出书签。
pub fn list_bookmarks(
    tag: Option<&str>,
    workspace: Option<&str>,
) -> Result<Value, XunError> {
    let store = load_store()?;
    let items: Vec<Value> = store
        .bookmarks
        .iter()
        .filter(|b| {
            if let Some(t) = tag {
                b.tags.iter().any(|bt| bt == t)
            } else {
                true
            }
        })
        .filter(|b| {
            if let Some(w) = workspace {
                b.workspace.as_deref() == Some(w)
            } else {
                true
            }
        })
        .map(bookmark_to_value)
        .collect();
    Ok(Value::List(items))
}

/// 最近访问的书签。
pub fn recent_bookmarks(
    limit: usize,
    tag: Option<&str>,
    workspace: Option<&str>,
) -> Result<Value, XunError> {
    let store = load_store()?;
    let mut filtered: Vec<&crate::bookmark::state::Bookmark> = store
        .bookmarks
        .iter()
        .filter(|b| {
            if let Some(t) = tag {
                b.tags.iter().any(|bt| bt == t)
            } else {
                true
            }
        })
        .filter(|b| {
            if let Some(w) = workspace {
                b.workspace.as_deref() == Some(w)
            } else {
                true
            }
        })
        .collect();
    // 按 visit_count 降序排序
    filtered.sort_by(|a, b| {
        let la = a.visit_count.unwrap_or(0);
        let lb = b.visit_count.unwrap_or(0);
        lb.cmp(&la)
    });
    filtered.truncate(limit);
    let items: Vec<Value> = filtered.into_iter().map(bookmark_to_value).collect();
    Ok(Value::List(items))
}

/// 书签统计。
pub fn stats_bookmarks() -> Result<Value, XunError> {
    use crate::bookmark::core::BookmarkSource;
    let store = load_store()?;
    let total = store.bookmarks.len();
    let pinned = store.bookmarks.iter().filter(|b| b.pinned).count();
    let explicit = store.bookmarks.iter().filter(|b| matches!(b.source, BookmarkSource::Explicit)).count();
    let learned = store.bookmarks.iter().filter(|b| matches!(b.source, BookmarkSource::Learned)).count();
    let imported = store.bookmarks.iter().filter(|b| matches!(b.source, BookmarkSource::Imported)).count();
    let total_visits: u64 = store.bookmarks.iter().map(|b| b.visit_count.unwrap_or(0) as u64).sum();

    let mut rec = crate::xun_core::value::Record::new();
    rec.insert("total".into(), Value::Int(total as i64));
    rec.insert("pinned".into(), Value::Int(pinned as i64));
    rec.insert("explicit".into(), Value::Int(explicit as i64));
    rec.insert("learned".into(), Value::Int(learned as i64));
    rec.insert("imported".into(), Value::Int(imported as i64));
    rec.insert("total_visits".into(), Value::Int(total_visits as i64));
    Ok(Value::Record(rec))
}

/// 列出所有书签名称（用于 tab completion）。
pub fn keys_bookmarks() -> Result<Value, XunError> {
    let store = load_store()?;
    let keys: Vec<Value> = store
        .bookmarks
        .iter()
        .filter_map(|b| b.name.as_ref().map(|n| Value::String(n.clone())))
        .collect();
    Ok(Value::List(keys))
}

/// 所有书签（机器输出）。
pub fn all_bookmarks(tag: Option<&str>) -> Result<Value, XunError> {
    list_bookmarks(tag, None)
}

// ============================================================
// 导航命令（委托给现有 cmd_*）
// ============================================================

/// 跳转到书签（模糊匹配）。
pub fn z_bookmark(
    patterns: &[String],
    tag: Option<&str>,
    list: bool,
    score: bool,
    why: bool,
    preview: bool,
    limit: Option<usize>,
    global: bool,
    child: bool,
    base: Option<&str>,
    workspace: Option<&str>,
    preset: Option<&str>,
) -> Result<Value, XunError> {
    use crate::bookmark::commands::cmd_z;
    use crate::cli::ZCmd;
    cmd_z(ZCmd {
        patterns: patterns.to_vec(),
        tag: tag.map(|s| s.to_string()),
        list,
        score,
        why,
        preview,
        limit,
        json: false,
        tsv: false,
        global,
        child,
        base: base.map(|s| s.to_string()),
        workspace: workspace.map(|s| s.to_string()),
        preset: preset.map(|s| s.to_string()),
    })
    .map_err(cli_err("bookmark z failed"))?;
    Ok(Value::Null)
}

/// 交互式选择书签。
pub fn zi_bookmark(patterns: &[String], tag: Option<&str>, global: bool) -> Result<Value, XunError> {
    use crate::bookmark::commands::cmd_zi;
    use crate::cli::ZiCmd;
    cmd_zi(ZiCmd {
        patterns: patterns.to_vec(),
        tag: tag.map(|s| s.to_string()),
        list: false,
        score: false,
        why: false,
        preview: false,
        limit: None,
        json: false,
        tsv: false,
        global,
        child: false,
        base: None,
        workspace: None,
        preset: None,
    })
    .map_err(cli_err("bookmark zi failed"))?;
    Ok(Value::Null)
}

/// 在 Explorer 中打开书签。
pub fn o_bookmark(patterns: &[String], tag: Option<&str>, global: bool) -> Result<Value, XunError> {
    use crate::bookmark::commands::cmd_open;
    use crate::cli::OpenCmd;
    cmd_open(OpenCmd {
        patterns: patterns.to_vec(),
        tag: tag.map(|s| s.to_string()),
        list: false,
        score: false,
        why: false,
        preview: false,
        limit: None,
        json: false,
        tsv: false,
        global,
        child: false,
        base: None,
        workspace: None,
        preset: None,
    })
    .map_err(cli_err("bookmark o failed"))?;
    Ok(Value::Null)
}

/// 交互式选择后在 Explorer 打开。
pub fn oi_bookmark(patterns: &[String], tag: Option<&str>, global: bool) -> Result<Value, XunError> {
    use crate::bookmark::commands::cmd_oi;
    use crate::cli::OiCmd;
    cmd_oi(OiCmd {
        patterns: patterns.to_vec(),
        tag: tag.map(|s| s.to_string()),
        list: false,
        score: false,
        why: false,
        preview: false,
        limit: None,
        json: false,
        tsv: false,
        global,
        child: false,
        base: None,
        workspace: None,
        preset: None,
    })
    .map_err(cli_err("bookmark oi failed"))?;
    Ok(Value::Null)
}

/// 在文件管理器中打开书签。
pub fn open_bookmark(patterns: &[String], tag: Option<&str>, global: bool) -> Result<Value, XunError> {
    use crate::bookmark::commands::cmd_open;
    use crate::cli::OpenCmd;
    cmd_open(OpenCmd {
        patterns: patterns.to_vec(),
        tag: tag.map(|s| s.to_string()),
        list: false,
        score: false,
        why: false,
        preview: false,
        limit: None,
        json: false,
        tsv: false,
        global,
        child: false,
        base: None,
        workspace: None,
        preset: None,
    })
    .map_err(cli_err("bookmark open failed"))?;
    Ok(Value::Null)
}

// ============================================================
// 维护命令
// ============================================================

/// 健康检查。
pub fn check_bookmarks(days: u64) -> Result<Value, XunError> {
    use crate::bookmark::commands::cmd_check;
    use crate::cli::CheckCmd;
    cmd_check(CheckCmd { days, format: "json".into() })
        .map_err(cli_err("bookmark check failed"))?;
    Ok(Value::Null)
}

/// 清理死链。
pub fn gc_bookmarks(purge: bool, dry_run: bool, learned: bool) -> Result<Value, XunError> {
    use crate::bookmark::commands::cmd_gc;
    use crate::cli::GcCmd;
    cmd_gc(GcCmd { purge, dry_run, learned, format: "json".into() })
        .map_err(cli_err("bookmark gc failed"))?;
    Ok(Value::Null)
}

/// 去重。
pub fn dedup_bookmarks(mode: &str, yes: bool) -> Result<Value, XunError> {
    use crate::bookmark::commands::cmd_dedup;
    use crate::cli::DedupCmd;
    cmd_dedup(DedupCmd { mode: mode.to_string(), format: "json".into(), yes })
        .map_err(cli_err("bookmark dedup failed"))?;
    Ok(Value::Null)
}

// ============================================================
// I/O 命令
// ============================================================

/// 导出书签。
pub fn export_bookmarks(format: &str, out: Option<&str>) -> Result<Value, XunError> {
    use crate::bookmark::commands::cmd_export;
    use crate::cli::ExportCmd;
    cmd_export(ExportCmd {
        format: format.to_string(),
        out: out.map(|s| s.to_string()),
    })
    .map_err(cli_err("bookmark export failed"))?;
    Ok(Value::Null)
}

/// 导入书签。
pub fn import_bookmarks(
    format: &str,
    from: Option<&str>,
    input: Option<&str>,
    mode: &str,
    yes: bool,
) -> Result<Value, XunError> {
    use crate::bookmark::commands::cmd_bookmark_import;
    use crate::cli::ImportCmd;
    cmd_bookmark_import(ImportCmd {
        format: format.to_string(),
        from: from.map(|s| s.to_string()),
        input: input.map(|s| s.to_string()),
        mode: mode.to_string(),
        yes,
    })
    .map_err(cli_err("bookmark import failed"))?;
    Ok(Value::Null)
}

// ============================================================
// 集成命令
// ============================================================

/// 生成 shell 集成脚本。
pub fn init_bookmark(shell: &str, cmd: Option<&str>) -> Result<Value, XunError> {
    use crate::bookmark::commands::cmd_bookmark_init;
    use crate::cli::BookmarkInitCmd;
    cmd_bookmark_init(BookmarkInitCmd {
        shell: shell.to_string(),
        cmd: cmd.map(|s| s.to_string()),
    })
    .map_err(cli_err("bookmark init failed"))?;
    Ok(Value::Null)
}

/// 学习路径。
pub fn learn_bookmark(path: &str) -> Result<(), XunError> {
    let mut store = load_store()?;
    let now = now_unix();
    let cwd = std::env::current_dir()
        .map_err(|e| XunError::user(format!("cannot get current dir: {e}")))?;
    let home: Option<std::path::PathBuf> = std::env::var("USERPROFILE")
        .ok()
        .or_else(|| std::env::var("HOME").ok())
        .map(std::path::PathBuf::from);
    store
        .learn(path, &cwd, home.as_deref(), now)
        .map_err(|e| XunError::user(format!("failed to learn path: {e}")))?;
    save_store(&mut store)
}
