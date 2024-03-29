use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ast::{
    ddl::create_table::CreateTableQuery,
    types::{Column, ForeignKey, TableName, UniqueKey},
};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TableConfig {
    pub table: TableName,
    pub columns: Vec<Column>,
    pub primary_key: Vec<String>,
    pub foreign_keys: Vec<ForeignKey>,
    pub unique_keys: Vec<UniqueKey>,
}

impl TableConfig {
    pub fn get_columns_map(&self) -> HashMap<String, Column> {
        HashMap::from_iter(self.columns.iter().cloned().map(|e| (e.name.clone(), e)))
    }

    pub fn get_required_columns(&self) -> Vec<Column> {
        self.columns
            .iter()
            .filter(|e| e.not_null && e.default.is_none())
            .cloned()
            .collect()
    }
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
