//! TableRow — 表格行渲染 trait
//!
//! 结构体实现 TableRow 后可自动渲染为 Table、JSON、TSV 等格式。

use crate::xun_core::value::{ColumnDef, Record, Table, Value};

/// 表格行 trait。
///
/// 结构体实现此 trait 后，可自动转换为 Table 行数据。
pub trait TableRow {
    /// 列定义。
    fn columns() -> Vec<ColumnDef>;

    /// 单元格值（顺序与 columns 对应）。
    fn cells(&self) -> Vec<Value>;

    /// 转换为 Record（BTreeMap）。
    fn to_record(&self) -> Record {
        let cols = Self::columns();
        let cells = self.cells();
        let mut rec = Record::new();
        for (col, val) in cols.iter().zip(cells) {
            rec.insert(col.name.clone(), val);
        }
        rec
    }

    /// 转换为 Table（单行）。
    fn to_table(&self) -> Table {
        let mut table = Table::new(Self::columns());
        table.push_row(self.to_record());
        table
    }

    /// 批量转换为 Table（多行）。
    fn vec_to_table(items: &[Self]) -> Table
    where
        Self: Sized,
    {
        let mut table = Table::new(Self::columns());
        for item in items {
            table.push_row(item.to_record());
        }
        table
    }
}
