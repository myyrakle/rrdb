use crate::lib::pgwire::protocol::DataTypeOid;

use super::config::TableDataFieldType;

#[derive(Debug, Clone)]
pub struct ExecuteResult {
    pub rows: Vec<ExecuteRow>,       // 데이터 행 -> 실 데이터
    pub columns: Vec<ExecuteColumn>, // 데이터 열에 대한 메타데이터
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecuteColumn {
    pub data_type: ExecuteColumnType,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExecuteRow {
    pub fields: Vec<ExecuteField>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecuteColumnType {
    Bool,
    Integer,
    Float,
    String,
}

impl From<ExecuteColumnType> for DataTypeOid {
    fn from(value: ExecuteColumnType) -> DataTypeOid {
        match value {
            ExecuteColumnType::Bool => DataTypeOid::Bool,
            ExecuteColumnType::Integer => DataTypeOid::Int8,
            ExecuteColumnType::Float => DataTypeOid::Float8,
            ExecuteColumnType::String => DataTypeOid::Text,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExecuteField {
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Null,
}

impl From<TableDataFieldType> for ExecuteField {
    fn from(value: TableDataFieldType) -> ExecuteField {
        match value {
            TableDataFieldType::Boolean(value) => ExecuteField::Bool(value),
            TableDataFieldType::Integer(value) => ExecuteField::Integer(value),
            TableDataFieldType::Float(value) => ExecuteField::Float(value),
            TableDataFieldType::String(value) => ExecuteField::String(value),
            TableDataFieldType::Null => ExecuteField::Null,
        }
    }
}
