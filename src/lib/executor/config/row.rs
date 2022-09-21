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

impl ToString for TableDataFieldType {
    fn to_string(&self) -> String {
        match self {
            TableDataFieldType::Integer(value) => value.to_string(),
            TableDataFieldType::Float(value) => value.to_string(),
            TableDataFieldType::Boolean(value) => value.to_string(),
            TableDataFieldType::String(value) => value.to_owned(),
            TableDataFieldType::Null => "NULL".into(),
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
