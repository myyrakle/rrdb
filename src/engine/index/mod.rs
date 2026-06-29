pub mod btree;
pub mod manager;

use serde::{Deserialize, Serialize};

use crate::engine::ast::types::TableName;
use crate::engine::schema::row::TableDataFieldType;

/// A serializable index entry that maps a key value to a row file path.
/// Stored on disk via BSON encoding and loaded into memory on startup.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct IndexEntry {
    /// The indexed column value (stringified for uniform comparison)
    pub key: String,
    /// Path to the row file that contains this key
    pub row_path: String,
}

/// Metadata describing an index on a specific table column.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct IndexMeta {
    pub index_name: String,
    pub table_name: TableName,
    pub column_name: String,
    pub is_unique: bool,
}

impl IndexMeta {
    pub fn new(
        index_name: String,
        table_name: TableName,
        column_name: String,
        is_unique: bool,
    ) -> Self {
        Self {
            index_name,
            table_name,
            column_name,
            is_unique,
        }
    }
}

/// Convert a TableDataFieldType to a comparable string key for indexing.
/// This provides a deterministic ordering for the BTree.
pub fn field_to_key(field: &TableDataFieldType) -> String {
    match field {
        TableDataFieldType::Integer(v) => format!("I:{:020}", v),
        TableDataFieldType::Float(v) => format!("F:{:020}", v.value),
        TableDataFieldType::Boolean(v) => format!("B:{}", if *v { 1 } else { 0 }),
        TableDataFieldType::String(v) => format!("S:{}", v),
        TableDataFieldType::Array(_) => format!("A:{}", field.to_string()),
        TableDataFieldType::Null => "N:".to_string(),
    }
}
