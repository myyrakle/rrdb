use std::io::ErrorKind;

use crate::ast::ddl::drop_database::DropDatabaseQuery;
use crate::errors::predule::ExecuteError;
use crate::errors::RRDBError;
use crate::executor::predule::{ExecuteResult, Executor};
use crate::executor::result::{ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteRow};

impl Executor {
    pub async fn drop_database(
        &self,
        query: DropDatabaseQuery,
    ) -> Result<ExecuteResult, RRDBError> {
        let base_path = self.get_base_path();
        let mut database_path = base_path.clone();

        let database_name = query
            .database_name
            .clone()
            .ok_or_else(|| ExecuteError::new("no database name"))?;

        database_path.push(&database_name);

        if let Err(error) = tokio::fs::remove_dir_all(database_path.clone()).await {
            match error.kind() {
                ErrorKind::NotFound => return Err(ExecuteError::new("database not found")),
                _ => {
                    return Err(ExecuteError::new("database drop failed"));
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
