use std::{collections::HashMap, iter::FromIterator};

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

impl TableConfig {
    pub fn get_columns_map(&self) -> HashMap<String, Column> {
        HashMap::from_iter(self.columns.iter().cloned().map(|e| (e.name.clone(), e)))
    }

    pub fn get_required_columns_map(&self) -> HashMap<String, Column> {
        HashMap::from_iter(
            self.columns
                .iter()
                .cloned()
                .filter(|e| e.not_null && e.default.is_none())
                .map(|e| (e.name.clone(), e)),
        )
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
