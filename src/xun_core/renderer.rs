//! Renderer — 多端输出抽象
//!
//! 将 StructuredValue 渲染到不同目标：Terminal / JSON / TSV / Dashboard。

use std::io;

use crate::xun_core::value::{Record, Table, Value};

/// 输出格式枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// TTY → Table，Pipe → Json
    Auto,
    Table,
    Json,
    Tsv,
    Csv,
}

impl OutputFormat {
    /// 根据 TTY 状态解析 Auto。
    pub fn resolve(self, is_tty: bool) -> Self {
        match self {
            Self::Auto => {
                if is_tty {
                    Self::Table
                } else {
                    Self::Json
                }
            }
            other => other,
        }
    }
}

/// Renderer trait — 将 StructuredValue 渲染到不同目标。
pub trait Renderer {
    fn render_value(&mut self, value: &Value) -> io::Result<()>;
    fn render_table(&mut self, table: &Table) -> io::Result<()>;
    fn render_info(&mut self, msg: &str);
    fn render_warning(&mut self, msg: &str);
}

// ============================================================
// TerminalRenderer — comfy_table 表格渲染
// ============================================================

pub struct TerminalRenderer<'a> {
    no_color: bool,
    writer: &'a mut dyn io::Write,
}

impl<'a> TerminalRenderer<'a> {
    pub fn new(no_color: bool, writer: &'a mut dyn io::Write) -> Self {
        Self { no_color, writer }
    }
}

impl<'a> Renderer for TerminalRenderer<'a> {
    fn render_value(&mut self, value: &Value) -> io::Result<()> {
        match value {
            Value::Record(rec) => {
                // 单条记录渲染为 key: value 列表
                let table = record_to_table(rec);
                self.render_table(&table)
            }
            _ => {
                writeln!(self.writer, "{value:?}")
            }
        }
    }

    fn render_table(&mut self, table: &Table) -> io::Result<()> {
        use comfy_table::presets::UTF8_FULL;
        use comfy_table::modifiers::UTF8_ROUND_CORNERS;
        use comfy_table::ContentArrangement;

        let mut ct = comfy_table::Table::new();
        ct.load_preset(UTF8_FULL);
        ct.apply_modifier(UTF8_ROUND_CORNERS);
        ct.set_content_arrangement(ContentArrangement::Dynamic);

        // Header
        let headers: Vec<String> = table.columns.iter().map(|c| c.name.clone()).collect();
        ct.set_header(&headers);

        // Rows
        for row in &table.rows {
            let cells: Vec<String> = table
                .columns
                .iter()
                .map(|col| {
                    row.get(&col.name)
                        .map(value_to_string)
                        .unwrap_or_default()
                })
                .collect();
            ct.add_row(cells);
        }

        let output = ct.to_string();
        if self.no_color {
            writeln!(self.writer, "{}", console::strip_ansi_codes(&output))
        } else {
            write!(self.writer, "{output}")
        }
    }

    fn render_info(&mut self, msg: &str) {
        let _ = writeln!(self.writer, "{msg}");
    }

    fn render_warning(&mut self, msg: &str) {
        let _ = writeln!(self.writer, "Warning: {msg}");
    }
}

// ============================================================
// JsonRenderer — serde_json 序列化
// ============================================================

pub struct JsonRenderer<'a> {
    pretty: bool,
    writer: &'a mut dyn io::Write,
}

impl<'a> JsonRenderer<'a> {
    pub fn new(pretty: bool, writer: &'a mut dyn io::Write) -> Self {
        Self { pretty, writer }
    }
}

impl<'a> Renderer for JsonRenderer<'a> {
    fn render_value(&mut self, value: &Value) -> io::Result<()> {
        if self.pretty {
            serde_json::to_writer_pretty(&mut self.writer, value)
        } else {
            serde_json::to_writer(&mut self.writer, value)
        }
        .map_err(io::Error::other)?;
        writeln!(self.writer)
    }

    fn render_table(&mut self, table: &Table) -> io::Result<()> {
        // Table → Vec<Record> (rows 作为 JSON 数组)
        let rows_as_values: Vec<&Record> = table.rows.iter().collect();
        if self.pretty {
            serde_json::to_writer_pretty(&mut self.writer, &rows_as_values)
        } else {
            serde_json::to_writer(&mut self.writer, &rows_as_values)
        }
        .map_err(io::Error::other)?;
        writeln!(self.writer)
    }

    fn render_info(&mut self, _msg: &str) {
        // JsonRenderer 不输出 UI 信息
    }

    fn render_warning(&mut self, _msg: &str) {
        // JsonRenderer 不输出 UI 信息
    }
}

// ============================================================
// TsvRenderer — Tab 分隔
// ============================================================

pub struct TsvRenderer<'a> {
    writer: &'a mut dyn io::Write,
}

impl<'a> TsvRenderer<'a> {
    pub fn new(writer: &'a mut dyn io::Write) -> Self {
        Self { writer }
    }
}

impl<'a> Renderer for TsvRenderer<'a> {
    fn render_value(&mut self, value: &Value) -> io::Result<()> {
        match value {
            Value::Record(rec) => {
                let table = record_to_table(rec);
                self.render_table(&table)
            }
            _ => writeln!(self.writer, "{value:?}"),
        }
    }

    fn render_table(&mut self, table: &Table) -> io::Result<()> {
        // Header
        let headers: Vec<&str> = table.columns.iter().map(|c| c.name.as_str()).collect();
        writeln!(self.writer, "{}", headers.join("\t"))?;

        // Rows
        for row in &table.rows {
            let cells: Vec<String> = table
                .columns
                .iter()
                .map(|col| {
                    row.get(&col.name)
                        .map(|v| tsv_escape(&value_to_string(v)))
                        .unwrap_or_default()
                })
                .collect();
            writeln!(self.writer, "{}", cells.join("\t"))?;
        }
        Ok(())
    }

    fn render_info(&mut self, msg: &str) {
        let _ = writeln!(self.writer, "# {msg}");
    }

    fn render_warning(&mut self, msg: &str) {
        let _ = writeln!(self.writer, "# Warning: {msg}");
    }
}

// ============================================================
// Helpers
// ============================================================

fn value_to_string(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => format!("{f:.2}"),
        Value::String(s) => s.clone(),
        Value::Duration(ms) => format_duration(*ms),
        Value::Filesize(bytes) => format_filesize(*bytes),
        Value::Date(s) => s.clone(),
        Value::List(items) => {
            let strs: Vec<String> = items.iter().map(value_to_string).collect();
            strs.join(", ")
        }
        Value::Record(rec) => {
            let pairs: Vec<String> = rec
                .iter()
                .map(|(k, v)| format!("{k}={}", value_to_string(v)))
                .collect();
            pairs.join(", ")
        }
    }
}

fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{ms}ms")
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        format!("{}m{}s", ms / 60_000, (ms % 60_000) / 1000)
    }
}

fn format_filesize(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes < KB {
        format!("{bytes} B")
    } else if bytes < MB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    }
}

fn tsv_escape(s: &str) -> String {
    if s.contains('\t') || s.contains('\n') || s.contains('\r') {
        // 将 tab 替换为空格，换行替换为逗号
        s.replace('\t', " ")
            .replace('\n', ", ")
            .replace('\r', "")
    } else {
        s.to_string()
    }
}

fn record_to_table(rec: &Record) -> Table {
    use crate::xun_core::value::{ColumnDef, ValueKind};
    let mut table = Table::new(vec![
        ColumnDef::new("key", ValueKind::String),
        ColumnDef::new("value", ValueKind::String),
    ]);
    for (k, v) in rec {
        let mut row = std::collections::BTreeMap::new();
        row.insert("key".into(), Value::String(k.clone()));
        row.insert("value".into(), Value::String(value_to_string(v)));
        table.push_row(row);
    }
    table
}
