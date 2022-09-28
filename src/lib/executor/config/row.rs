use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::lib::ast::predule::TableName;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, PartialOrd, Hash)]
pub enum TableDataFieldType {
    // 끝단 Primitive 값
    Integer(i64),
    Float(String),
    Boolean(bool),
    String(String),
    Array(Vec<TableDataFieldType>),
    Null,
}

impl TableDataFieldType {
    pub fn type_code(&self) -> isize {
        match self {
            TableDataFieldType::Integer(_) => 1,
            TableDataFieldType::Float(_) => 2,
            TableDataFieldType::Boolean(_) => 3,
            TableDataFieldType::String(_) => 4,
            TableDataFieldType::Array(_) => 5,
            TableDataFieldType::Null => 0,
        }
    }

    pub fn to_array(self) -> Self {
        Self::Array(vec![self])
    }

    pub fn push(&mut self, value: Self) {
        match self {
            TableDataFieldType::Array(array) => array.push(value),
            _ => {}
        }
    }

    pub fn is_null(&self) -> bool {
        self.type_code() == 0
    }

    pub fn is_array(&self) -> bool {
        self.type_code() == 5
    }
}

impl ToString for TableDataFieldType {
    fn to_string(&self) -> String {
        match self {
            TableDataFieldType::Integer(value) => value.to_string(),
            TableDataFieldType::Float(value) => value.to_string(),
            TableDataFieldType::Boolean(value) => value.to_string(),
            TableDataFieldType::String(value) => value.to_owned(),
            TableDataFieldType::Array(value) => value
                .iter()
                .map(|e| e.to_string())
                .intersperse(", ".to_owned())
                .collect(),
            TableDataFieldType::Null => "NULL".into(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TableDataField {
    pub table_name: TableName,
    pub column_name: String,
    pub data: TableDataFieldType,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TableDataRow {
    pub fields: Vec<TableDataField>,
}
