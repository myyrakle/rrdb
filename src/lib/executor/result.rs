use itertools::Itertools;

use crate::lib::{ast::predule::DataType, pgwire::protocol::DataTypeOid};

use crate::lib::executor::predule::TableDataFieldType;

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
    Null,
}

impl From<ExecuteColumnType> for DataTypeOid {
    fn from(value: ExecuteColumnType) -> DataTypeOid {
        match value {
            ExecuteColumnType::Bool => DataTypeOid::Bool,
            ExecuteColumnType::Integer => DataTypeOid::Int8,
            ExecuteColumnType::Float => DataTypeOid::Float8,
            ExecuteColumnType::String => DataTypeOid::Text,
            ExecuteColumnType::Null => DataTypeOid::Unspecified,
        }
    }
}

impl From<DataType> for ExecuteColumnType {
    fn from(value: DataType) -> ExecuteColumnType {
        match value {
            DataType::Boolean => ExecuteColumnType::Bool,
            DataType::Int => ExecuteColumnType::Integer,
            DataType::Float => ExecuteColumnType::Float,
            DataType::Varchar(_) => ExecuteColumnType::String,
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
            TableDataFieldType::Float(value) => ExecuteField::Float(value.into()),
            TableDataFieldType::String(value) => ExecuteField::String(value),
            TableDataFieldType::Array(value) => ExecuteField::String(
                value
                    .iter()
                    .map(|e| e.to_string())
                    .intersperse(", ".to_owned())
                    .collect(),
            ),
            TableDataFieldType::Null => ExecuteField::Null,
        }
    }
}
