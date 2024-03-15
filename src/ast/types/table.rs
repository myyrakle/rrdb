use serde::{Deserialize, Serialize};

use crate::ast::dml::parts::from::{FromClause, FromTarget};

// [database_name.]table_name
// 테이블명을 가리키는 값입니다.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct TableName {
    pub database_name: Option<String>,
    pub table_name: String,
}

impl TableName {
    pub fn new(database_name: Option<String>, table_name: String) -> Self {
        TableName {
            database_name,
            table_name,
        }
    }
}

impl From<TableName> for FromClause {
    fn from(value: TableName) -> FromClause {
        FromClause {
            from: FromTarget::Table(value),
            alias: None,
        }
    }
}
