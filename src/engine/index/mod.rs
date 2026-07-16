pub mod btree;
pub mod manager;
pub mod page;
pub mod page_btree;
pub mod page_store;

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

/// Convert a TableDataFieldType to a lexicographically sortable string key.
///
/// Integer encoding: flips the sign bit so that negative values sort before
/// positive values in lexicographic order. The result is zero-padded to a
/// fixed width so string comparison matches numeric comparison.
///
/// Float encoding: uses the IEEE 754 bit-pattern trick -- flip the sign bit
/// for non-negative floats, flip all bits for negative floats -- producing
/// a uint64 whose big-endian byte order matches total float ordering.
/// The resulting u64 is then encoded as a fixed-width hex string.
///
/// Boolean and String use natural ordering.
/// Null sorts before everything (prefix "N:").
pub fn field_to_key(field: &TableDataFieldType) -> String {
    match field {
        TableDataFieldType::Integer(v) => {
            // Flip the sign bit so negative < positive in unsigned comparison
            let bits = (*v as i64 as u64) ^ (1u64 << 63);
            format!("I:{:016X}", bits)
        }
        TableDataFieldType::Float(v) => {
            let raw = v.value.to_bits();
            // For positive floats (sign bit = 0): flip sign bit -> sorts after negatives
            // For negative floats (sign bit = 1): flip all bits -> reverses order so
            // more-negative values (larger magnitude) sort first
            let sortable = if raw & (1u64 << 63) != 0 {
                !raw
            } else {
                raw ^ (1u64 << 63)
            };
            format!("F:{:016X}", sortable)
        }
        TableDataFieldType::Boolean(v) => format!("B:{}", if *v { 1 } else { 0 }),
        TableDataFieldType::String(v) => format!("S:{}", v),
        TableDataFieldType::Array(_) => format!("A:{}", field.to_string()),
        TableDataFieldType::Null => "N:".to_string(),
    }
}
