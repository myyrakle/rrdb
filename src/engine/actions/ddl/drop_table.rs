use std::io::ErrorKind as IOErrorKind;

use crate::engine::DBEngine;
use crate::engine::ast::ddl::drop_table::DropTableQuery;
use crate::engine::ast::types::TableName;
use crate::engine::types::{
    ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow,
};
use crate::errors::{self, ErrorKind, Errors};

impl DBEngine {
    pub async fn drop_table(&self, query: DropTableQuery) -> errors::Result<ExecuteResult> {
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
                IOErrorKind::NotFound => {
                    return Err(Errors::new(ErrorKind::ExecuteError(
                        "table not found".to_string(),
                    )));
                }
                _ => {
                    return Err(Errors::new(ErrorKind::ExecuteError(
                        "table drop failed".to_string(),
                    )));
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
