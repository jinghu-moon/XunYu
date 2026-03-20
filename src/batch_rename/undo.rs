// batch_rename/undo.rs
//
// NDJSON append-only 双文件 undo/redo 历史管理。
//
// 文件格式：每个 UndoBatch 序列化为一行紧凑 JSON，最后一行 = 最新 batch。
//   .xun-brn-undo.log  — undo 栈
//   .xun-brn-redo.log  — redo 栈
// 历史上限：各自最多 MAX_HISTORY 条，超出时丢弃最旧行（仅第 101 次触发重写）。

use std::fs::{self, OpenOptions};
use std::io::Write as IoWrite;
use std::path::Path;

use crate::output::{CliError, CliResult};
use serde::{Deserialize, Serialize};

// ─── 内部计时工具（XUN_BRN_TIMING=1 启用）────────────────────────────────────

struct BrnTimer {
    label: &'static str,
    t0: std::time::Instant,
    enabled: bool,
}

impl BrnTimer {
    fn new(label: &'static str) -> Self {
        let enabled = std::env::var_os("XUN_BRN_TIMING").is_some();
        Self { label, t0: std::time::Instant::now(), enabled }
    }
    fn lap(&self, tag: &str) {
        if self.enabled {
            eprintln!("[brn-timing] {}::{} = {:?}", self.label, tag, self.t0.elapsed());
        }
    }
}

// ─── 常量 ──────────────────────────────────────────────────────────────────────

/// 新 NDJSON undo 栈文件。
pub const UNDO_LOG: &str = ".xun-brn-undo.log";
/// 新 NDJSON redo 栈文件。
pub const REDO_LOG: &str = ".xun-brn-redo.log";
/// 旧 JSON 格式（兼容迁移）。
pub const UNDO_FILE_LEGACY: &str = ".xun-brn-undo.json";
/// 对外暴露的旧常量名（向后兼容）。
pub const UNDO_FILE: &str = UNDO_LOG;
/// undo/redo 栈各自最多保留的批次数。
pub const MAX_HISTORY: usize = 100;

// ─── 数据结构 ──────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct UndoRecord {
    pub from: String,
    pub to: String,
}

/// 一次批量重命名操作的历史批次。
#[derive(Serialize, Deserialize, Clone)]
pub struct UndoBatch {
    /// Unix 时间戳（秒）。
    pub ts: u64,
    pub ops: Vec<UndoRecord>,
}

/// 持久化历史的顶层结构（仅供 `read_history` 返回，不再落盘为整体 JSON）。
///
/// - `undo` 栈：最新操作在末尾，undo 时从末尾弹出。
/// - `redo` 栈：undo 后将反转 batch 压入；新操作执行时清空。
#[derive(Serialize, Deserialize, Default)]
pub struct BrnHistory {
    pub undo: Vec<UndoBatch>,
    pub redo: Vec<UndoBatch>,
}

// ─── 私有：NDJSON 底层 I/O ────────────────────────────────────────────────────

/// 将单个 batch 以紧凑 JSON 追加到文件末尾（O(1) I/O）。
/// 返回写入后文件的字节大小，用于粗判是否需要 trim。
fn append_line(path: &Path, batch: &UndoBatch) -> CliResult<u64> {
    use std::io::Seek;
    let line = serde_json::to_string(batch)
        .map_err(|e| CliError::new(1, format!("Failed to serialize undo batch: {}", e)))?;
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .map_err(|e| CliError::new(1, format!("Cannot open undo log '{}': {}", path.display(), e)))?;
    writeln!(file, "{}", line)
        .map_err(|e| CliError::new(1, format!("Cannot write undo log: {}", e)))?;
    // seek(End) 获取写入后总大小，无需额外 stat 系统调用
    let total_size = file.seek(std::io::SeekFrom::End(0)).unwrap_or(0);
    Ok(total_size)
}

/// 将多个 batch 批量追加到文件（一次 open → 多次 write → close）。
/// 比循环调用 append_line 少 N-1 次 CreateFile 系统调用。
/// 返回写入后文件的字节大小。
fn append_lines(path: &Path, batches: &[UndoBatch]) -> CliResult<u64> {
    use std::io::Seek;
    if batches.is_empty() {
        return Ok(path.metadata().map(|m| m.len()).unwrap_or(0));
    }
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .map_err(|e| CliError::new(1, format!("Cannot open undo log '{}': {}", path.display(), e)))?;
    for batch in batches {
        let line = serde_json::to_string(batch)
            .map_err(|e| CliError::new(1, format!("Failed to serialize undo batch: {}", e)))?;
        writeln!(file, "{}", line)
            .map_err(|e| CliError::new(1, format!("Cannot write undo log: {}", e)))?;
    }
    let total_size = file.seek(std::io::SeekFrom::End(0)).unwrap_or(0);
    Ok(total_size)
}

/// 读取 NDJSON 日志文件，按行解析为 Vec<UndoBatch>，忽略空行。
fn read_log(path: &Path) -> CliResult<Vec<UndoBatch>> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let data = fs::read_to_string(path)
        .map_err(|e| CliError::new(1, format!("Cannot read undo log '{}': {}", path.display(), e)))?;
    let mut batches = Vec::new();
    for line in data.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let batch: UndoBatch = serde_json::from_str(trimmed)
            .map_err(|e| CliError::new(1, format!("Invalid undo log line: {}", e)))?;
        batches.push(batch);
    }
    Ok(batches)
}

/// 将 batches 整体重写为 NDJSON 文件（每行一条）。
fn write_log(path: &Path, batches: &[UndoBatch]) -> CliResult {
    if batches.is_empty() {
        let _ = fs::remove_file(path); // 忽略 not-found
        return Ok(());
    }
    let mut out = String::with_capacity(batches.len() * 128);
    for batch in batches {
        let line = serde_json::to_string(batch)
            .map_err(|e| CliError::new(1, format!("Failed to serialize undo batch: {}", e)))?;
        out.push_str(&line);
        out.push('\n');
    }
    fs::write(path, out.as_bytes())
        .map_err(|e| CliError::new(1, format!("Cannot write undo log '{}': {}", path.display(), e)))
}

/// 若文件行数超过 `max`，丢弃最旧的行，重写文件。
fn trim_log(path: &Path, max: usize) -> CliResult {
    if !path.exists() {
        return Ok(());
    }
    let batches = read_log(path)?;
    if batches.len() > max {
        let trimmed = &batches[batches.len() - max..];
        write_log(path, trimmed)?;
    }
    Ok(())
}

// ─── 私有：旧格式迁移 ─────────────────────────────────────────────────────────

/// 检测并迁移旧 `.xun-brn-undo.json`（BrnHistory / Vec<UndoBatch> / flat）。
///
/// 迁移后删除旧文件；函数幂等，不存在旧文件时直接返回。
fn migrate_legacy(dir: &Path) -> CliResult {
    let legacy = dir.join(UNDO_FILE_LEGACY);
    if !legacy.exists() {
        return Ok(());
    }
    let data = fs::read_to_string(&legacy)
        .map_err(|e| CliError::new(1, format!("Cannot read legacy undo file: {}", e)))?;

    // 尝试 BrnHistory（新旧 JSON 顶层结构）
    if let Ok(h) = serde_json::from_str::<BrnHistoryJson>(&data) {
        write_log(&dir.join(UNDO_LOG), &h.undo)?;
        write_log(&dir.join(REDO_LOG), &h.redo)?;
        let _ = fs::remove_file(&legacy);
        return Ok(());
    }
    // 旧 batched Vec<UndoBatch>
    if let Ok(batches) = serde_json::from_str::<Vec<UndoBatch>>(&data) {
        write_log(&dir.join(UNDO_LOG), &batches)?;
        let _ = fs::remove_file(&legacy);
        return Ok(());
    }
    // legacy flat Vec<UndoRecord>
    if let Ok(records) = serde_json::from_str::<Vec<UndoRecord>>(&data) {
        let batch = UndoBatch { ts: 0, ops: records };
        write_log(&dir.join(UNDO_LOG), &[batch])?;
        let _ = fs::remove_file(&legacy);
        return Ok(());
    }
    // 无法识别 → 忽略（不阻塞正常流程）
    Ok(())
}

/// 供 `migrate_legacy` 内部反序列化旧 JSON 顶层结构。
#[derive(Deserialize)]
struct BrnHistoryJson {
    #[serde(default)]
    undo: Vec<UndoBatch>,
    #[serde(default)]
    redo: Vec<UndoBatch>,
}

// ─── 公开 API ─────────────────────────────────────────────────────────────────

/// 将一批 rename 记录追加到 undo 栈（O(1) I/O），同时清空 redo 栈。
pub fn push_undo(dir: &Path, records: &[UndoRecord]) -> CliResult {
    let t = BrnTimer::new("push_undo");

    migrate_legacy(dir)?;
    t.lap("migrate_legacy");

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let batch = UndoBatch { ts, ops: records.to_vec() };
    t.lap("build_batch");

    let undo_path = dir.join(UNDO_LOG);
    let file_size = append_line(&undo_path, &batch)?;
    t.lap("append_line");

    // 清空 redo 栈（ignore not-found）
    let redo_path = dir.join(REDO_LOG);
    let _ = fs::remove_file(&redo_path);
    t.lap("clear_redo");

    // 两阶段 trim：先用文件大小粗判（下界 50B/行），可能超出时才精确读文件计行
    // 50B 是单条 ops 最短可能行长（保证不漏判），正常行 ~70-200B
    const MIN_LINE: u64 = 50;
    if file_size > MAX_HISTORY as u64 * MIN_LINE {
        // 精确计行：读全文统计非空行数
        let n = fs::read_to_string(&undo_path)
            .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count())
            .unwrap_or(0);
        if cfg!(debug_assertions) && std::env::var_os("XUN_BRN_TIMING").is_some() {
            eprintln!("[dbg] file_size={} lines={}", file_size, n);
        }
        if n > MAX_HISTORY {
            trim_log(&undo_path, MAX_HISTORY)?;
        }
    }
    t.lap("trim_check");

    Ok(())
}

/// 执行 n 步 undo：从 undo 栈末尾弹出，反转后压入 redo 栈，执行 rename。
pub fn run_undo_steps(dir: &str, steps: usize) -> CliResult {
    let t = BrnTimer::new("run_undo_steps");
    let undo_dir = Path::new(dir);
    migrate_legacy(undo_dir)?;
    t.lap("migrate_legacy");

    let undo_path = undo_dir.join(UNDO_LOG);
    let redo_path = undo_dir.join(REDO_LOG);

    let mut undo_batches = read_log(&undo_path)?;
    t.lap("read_undo_log");
    if undo_batches.is_empty() {
        return Ok(());
    }

    let n = steps.min(undo_batches.len());
    let to_process = undo_batches.split_off(undo_batches.len() - n); // 末尾 n 个

    // 执行 undo rename：record 语义为 from=重命名后文件，to=原始文件
    // undo 时将当前文件（from）改回原始名（to）
    for batch in &to_process {
        for op in &batch.ops {
            let src = Path::new(&op.from); // 当前存在的文件
            let dst = Path::new(&op.to);   // 还原目标
            if let Err(e) = fs::rename(src, dst)
                && e.kind() != std::io::ErrorKind::NotFound
            {
                return Err(CliError::new(
                    1,
                    format!("Undo rename failed '{}' -> '{}': {}", src.display(), dst.display(), e),
                ));
            }
        }
    }
    // 批量压入 redo 栈（一次 open → 多次 write，消除循环内重复 CreateFile 开销）
    append_lines(&redo_path, &to_process)?;
    t.lap("rename+append_redo");

    // 重写 undo 栈（去掉已弹出的末尾 n 行）
    write_log(&undo_path, &undo_batches)?;
    t.lap("write_undo_log");

    // trim redo 栈：用文件大小启发式判断，避免读全文
    const AVG_LINE: u64 = 150;
    let redo_size = redo_path.metadata().map(|m| m.len()).unwrap_or(0);
    if redo_size > MAX_HISTORY as u64 * AVG_LINE {
        trim_log(&redo_path, MAX_HISTORY)?;
    }
    t.lap("trim_redo");

    Ok(())
}

/// 执行 n 步 redo：从 redo 栈末尾弹出，反转后压入 undo 栈，执行 rename。
pub fn run_redo_steps(dir: &str, steps: usize) -> CliResult {
    let t = BrnTimer::new("run_redo_steps");
    let undo_dir = Path::new(dir);
    migrate_legacy(undo_dir)?;
    t.lap("migrate_legacy");

    let undo_path = undo_dir.join(UNDO_LOG);
    let redo_path = undo_dir.join(REDO_LOG);

    let mut redo_batches = read_log(&redo_path)?;
    t.lap("read_redo_log");
    if redo_batches.is_empty() {
        return Ok(());
    }

    let n = steps.min(redo_batches.len());
    let to_process = redo_batches.split_off(redo_batches.len() - n);

    // 执行 redo rename：record 语义同 undo record（from=重命名后，to=原始）
    // redo 时将原始文件（to）重新改名为（from）
    for batch in &to_process {
        for op in &batch.ops {
            let src = Path::new(&op.to);  // 当前存在的（原始名）
            let dst = Path::new(&op.from); // 重新变成重命名后的名
            if let Err(e) = fs::rename(src, dst)
                && e.kind() != std::io::ErrorKind::NotFound
            {
                return Err(CliError::new(
                    1,
                    format!("Redo rename failed '{}' -> '{}': {}", src.display(), dst.display(), e),
                ));
            }
        }
    }
    // 批量压回 undo 栈（一次 open → 多次 write，消除循环内重复 CreateFile 开销）
    append_lines(&undo_path, &to_process)?;
    t.lap("rename+append_undo");

    // 重写 redo 栈
    write_log(&redo_path, &redo_batches)?;
    t.lap("write_redo_log");

    // trim undo 栈：用文件大小启发式判断，避免读全文
    const AVG_LINE: u64 = 150;
    let undo_size = undo_path.metadata().map(|m| m.len()).unwrap_or(0);
    if undo_size > MAX_HISTORY as u64 * AVG_LINE {
        trim_log(&undo_path, MAX_HISTORY)?;
    }
    t.lap("trim_undo");

    Ok(())
}

/// 单步 undo（向后兼容包装）。
pub fn run_undo(dir: &str) -> CliResult {
    let undo_dir = Path::new(dir);
    migrate_legacy(undo_dir)?;

    let undo_path = undo_dir.join(UNDO_LOG);
    if !undo_path.exists() {
        return Err(CliError::new(
            1,
            format!("Undo file '{}' not found. Nothing to undo.", undo_path.display()),
        ));
    }
    run_undo_steps(dir, 1)
}

/// 统一读取历史文件，兼容旧格式（内部先 migrate_legacy）。
pub fn read_history(dir: &Path) -> CliResult<BrnHistory> {
    migrate_legacy(dir)?;
    let undo = read_log(&dir.join(UNDO_LOG))?;
    let redo = read_log(&dir.join(REDO_LOG))?;
    Ok(BrnHistory { undo, redo })
}

// ─── 兼容 re-export（供旧测试使用）────────────────────────────────────────────

/// 读取 undo 历史（返回 undo 栈）。
pub fn read_undo_history(dir: &Path) -> CliResult<Vec<UndoBatch>> {
    read_history(dir).map(|h| h.undo)
}

/// 追加 undo batch（兼容旧测试）。
pub fn append_undo(dir: &Path, records: &[UndoRecord]) -> CliResult {
    push_undo(dir, records)
}
