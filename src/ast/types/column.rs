use crate::ast::predule::DataType;
use serde::{Deserialize, Serialize};

use super::expression::SQLExpression;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct Column {
    pub name: String,
    pub data_type: DataType,
    pub comment: String,
    pub default: Option<SQLExpression>,
    pub not_null: bool,
    pub primary_key: bool,
}

impl Column {
    pub fn builder() -> ColumnBuilder {
        ColumnBuilder::default()
    }
}

#[derive(Default)]
pub struct ColumnBuilder {
    name: Option<String>,
    data_type: Option<DataType>,
    comment: Option<String>,
    default: Option<SQLExpression>,
    not_null: Option<bool>,
    primary_key: Option<bool>,
}

impl ColumnBuilder {
    pub fn set_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn set_data_type(mut self, data_type: DataType) -> Self {
        self.data_type = Some(data_type);
        self
    }

    pub fn set_comment(mut self, comment: String) -> Self {
        self.comment = Some(comment);
        self
    }

    pub fn set_default(mut self, default: SQLExpression) -> Self {
        self.default = Some(default);
        self
    }

    pub fn set_not_null(mut self, not_null: bool) -> Self {
        self.not_null = Some(not_null);
        self
    }

    pub fn set_primary_key(mut self, primary_key: bool) -> Self {
        self.primary_key = Some(primary_key);
        if primary_key {
            self.not_null = Some(true);
        }
        self
    }

    pub fn build(self) -> Column {
        Column {
            name: self.name.unwrap(),
            data_type: self.data_type.unwrap(),
            comment: self.comment.unwrap_or_else(|| "".into()),
            default: self.default,
            not_null: self.not_null.unwrap_or(false),
            primary_key: self.primary_key.unwrap_or(false),
        }
    }
}

// [column_name.]table_name
// 컬럼명을 가리키는 값입니다.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnName {
    pub table_name: Option<String>,
    pub column_name: String,
}

impl ColumnName {
    pub fn new(table_name: Option<String>, column_name: String) -> Self {
        ColumnName {
            table_name,
            column_name,
        }
    }
}
