use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub enum TableDataFieldType {
    // 끝단 Primitive 값
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(String),
    Null,
}

impl TableDataFieldType {
    pub fn type_code(&self) -> isize {
        match self {
            TableDataFieldType::Integer(_) => 1,
            TableDataFieldType::Float(_) => 2,
            TableDataFieldType::Boolean(_) => 3,
            TableDataFieldType::String(_) => 4,
            TableDataFieldType::Null => 0,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TableDataField {
    pub column_name: String,
    pub data: TableDataFieldType,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TableDataRow {
    pub fields: Vec<TableDataField>,
}
