use std::io::ErrorKind as IOErrorKind;

use crate::engine::DBEngine;

use crate::engine::ast::ddl::drop_database::DropDatabaseQuery;
use crate::engine::types::{
    ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow,
};
use crate::errors::execute_error::ExecuteError;
use crate::errors;

impl DBEngine {
    pub async fn drop_database(&self, query: DropDatabaseQuery) -> errors::Result<ExecuteResult> {
        let base_path = self.get_data_directory();
        let mut database_path = base_path.clone();

        let database_name = query
            .database_name
            .clone()
            .ok_or_else(|| ExecuteError::wrap("no database name".to_string()))?;

        database_path.push(&database_name);

        if let Err(error) = tokio::fs::remove_dir_all(database_path.clone()).await {
            match error.kind() {
                IOErrorKind::NotFound => {
                    return Err(ExecuteError::wrap(
                        "database not found".to_string(),
                    ));
                }
                _ => {
                    return Err(ExecuteError::wrap(
                        "database drop failed".to_string(),
                    ));
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
                    "database dropped: {}",
                    database_name
                ))],
            }]),
        })
    }
}
