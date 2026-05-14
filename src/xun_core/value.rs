//! StructuredValue — 统一数据模型
//!
//! CLI / Dashboard / 未来 AI 的共享数据层。
//! 所有命令产出 StructuredValue，各 Renderer 独立消费。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use specta::Type;

/// XunYu 的统一结构化值 — 类似 Nushell 的 Value。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Type)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    /// 毫秒级时长
    Duration(u64),
    /// 字节级文件大小
    Filesize(u64),
    /// ISO 8601 日期字符串
    Date(String),
    /// 异构列表
    List(Vec<Value>),
    /// 有序键值对
    Record(BTreeMap<String, Value>),
}

/// 有序键值对（BTreeMap 保证 key 排序）。
pub type Record = BTreeMap<String, Value>;

/// 带 schema 的表格 — 列表类命令的标准输出。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Type)]
pub struct Table {
    pub columns: Vec<ColumnDef>,
    pub rows: Vec<Record>,
}

/// 列定义。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Type)]
pub struct ColumnDef {
    pub name: String,
    pub kind: ValueKind,
    pub sortable: bool,
}

/// 列的语义类型。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "lowercase")]
pub enum ValueKind {
    String,
    Int,
    Float,
    Bool,
    Date,
    Duration,
    Filesize,
    Path,
}

// --- 便捷构造 ---

impl ColumnDef {
    pub fn new(name: impl Into<String>, kind: ValueKind) -> Self {
        Self {
            name: name.into(),
            kind,
            sortable: false,
        }
    }

    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }
}

impl Table {
    pub fn new(columns: Vec<ColumnDef>) -> Self {
        Self {
            columns,
            rows: Vec::new(),
        }
    }

    pub fn push_row(&mut self, row: Record) {
        self.rows.push(row);
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }
}

impl Value {
    /// 是否为 Null。
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
}

// --- From 转换 ---

impl From<String> for Value {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Self::Int(n)
    }
}

impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Self::Int(n as i64)
    }
}

impl From<u64> for Value {
    fn from(n: u64) -> Self {
        Self::Int(n as i64)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Self::Float(f)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}
