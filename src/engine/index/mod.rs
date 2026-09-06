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

/// Metadata describing an index on one or more table columns (#220).
///
/// Single-column indexes keep `columns` with exactly one element; composite
/// indexes list the columns in key order. `column_name()` preserves the old
/// single-column accessor for callers that only deal with single indexes.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct IndexMeta {
    pub index_name: String,
    pub table_name: TableName,
    pub column_name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
}

impl IndexMeta {
    /// Create metadata for a single-column index (#217, existing behavior).
    pub fn new(
        index_name: String,
        table_name: TableName,
        column_name: String,
        is_unique: bool,
    ) -> Self {
        Self {
            index_name,
            table_name,
            column_name: column_name.clone(),
            columns: vec![column_name],
            is_unique,
        }
    }

    /// Create metadata for a composite (multi-column) index (#220).
    ///
    /// The `columns` order defines the key order. Panics on an empty list:
    /// an index over zero columns is meaningless and callers validate before
    /// reaching here.
    pub fn new_composite(
        index_name: String,
        table_name: TableName,
        columns: Vec<String>,
        is_unique: bool,
    ) -> Self {
        assert!(
            !columns.is_empty(),
            "composite index must have at least one column"
        );

        Self {
            index_name,
            table_name,
            // 하위 호환: 기존 디스크 메타/호출자는 column_name을 사용.
            // 복합 인덱스에서는 첫 번째 컬럼을 유지 (표시/통계 용도).
            column_name: columns[0].clone(),
            columns,
            is_unique,
        }
    }

    /// The indexed columns in key order.
    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    /// The first indexed column (legacy single-column accessor).
    pub fn column_name(&self) -> &str {
        &self.column_name
    }

    /// True when this index covers more than one column.
    pub fn is_composite(&self) -> bool {
        self.columns.len() > 1
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

/// Length-prefixed encoding of one key component so that the concatenation of
/// multiple components is unambiguous (#220).
///
/// Plain concatenation is not injective: ("S:ab", "S:c") and ("S:a", "S:bc")
/// both merge to "S:abS:c". Prefixing each component with its byte length
/// ("5:S:ab" style) makes ("S:ab","S:c") -> "5:S:ab3:S:c" differ from
/// ("S:a","S:bc") -> "3:S:a5:S:bc". Length prefixes also preserve
/// lexicographic comparability within equal-prefix groups, and every single
/// component key maps to exactly one encoded form, so composite keys remain
/// injective and order-preserving on the first differing column.
pub fn encode_composite_key_component(key: &str) -> String {
    format!("{:06}:{}", key.len(), key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composite_component_encoding_is_injective_for_splits() {
        let a = (
            encode_composite_key_component("S:ab"),
            encode_composite_key_component("S:c"),
        );
        let b = (
            encode_composite_key_component("S:a"),
            encode_composite_key_component("S:bc"),
        );

        assert_ne!(format!("{}{}", a.0, a.1), format!("{}{}", b.0, b.1));
    }

    #[test]
    fn composite_component_encoding_round_trips() {
        for key in ["S:hello", "I:00000000000000FF", "", "N:"] {
            let encoded = encode_composite_key_component(key);
            let expected = format!("{:06}:{}", key.len(), key);
            assert_eq!(encoded, expected);
        }
    }
}
