use serde::{Deserialize, Serialize};

use crate::lib::ast::{
    ddl::{Column, CreateTableQuery},
    predule::{ForeignKey, TableName, UniqueKey},
};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TableConfig {
    pub table: TableName,
    pub columns: Vec<Column>,
    pub primary_key: Vec<String>,
    pub foreign_keys: Vec<ForeignKey>,
    pub unique_keys: Vec<UniqueKey>,
}

impl From<CreateTableQuery> for TableConfig {
    fn from(query: CreateTableQuery) -> Self {
        Self {
            table: query.table.unwrap(),
            columns: query.columns,
            primary_key: query.primary_key,
            foreign_keys: query.foreign_keys,
            unique_keys: query.unique_keys,
        }
    }
}
