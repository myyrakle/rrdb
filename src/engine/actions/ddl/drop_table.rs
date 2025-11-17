use std::io::ErrorKind;

use crate::engine::DBEngine;
use crate::engine::ast::ddl::drop_table::DropTableQuery;
use crate::engine::ast::types::TableName;
use crate::engine::types::{
    ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow,
};
use crate::errors::RRDBError;
use crate::errors::predule::ExecuteError;

impl DBEngine {
    pub async fn drop_table(&self, query: DropTableQuery) -> Result<ExecuteResult, RRDBError> {
        let base_path = self.get_data_directory();

        let TableName {
            database_name,
            table_name,
        } = query.table.unwrap();

        let table_path = base_path
            .clone()
            .join(database_name.unwrap())
            .join("tables")
            .join(&table_name);

        if let Err(error) = tokio::fs::remove_dir_all(table_path).await {
            match error.kind() {
                ErrorKind::NotFound => return Err(ExecuteError::wrap("table not found")),
                _ => {
                    return Err(ExecuteError::wrap("table drop failed"));
                }
            }
        }

        Ok(ExecuteResult {
            columns: (vec![ExecuteColumn {
                name: "desc".into(),
                data_type: ExecuteColumnType::String,
            }]),
            rows: (vec![ExecuteRow {
                fields: vec![ExecuteField::String(format!(
                    "table dropped: {}",
                    table_name
                ))],
            }]),
        })
    }
}
