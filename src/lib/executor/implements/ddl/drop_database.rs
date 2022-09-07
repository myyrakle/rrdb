use std::error::Error;
use std::io::ErrorKind;

use crate::lib::ast::ddl::DropDatabaseQuery;
use crate::lib::errors::predule::ExecuteError;
use crate::lib::executor::predule::{ExecuteResult, Executor};
use crate::lib::executor::result::{ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteRow};

impl Executor {
    pub async fn drop_database(
        &self,
        query: DropDatabaseQuery,
    ) -> Result<ExecuteResult, Box<dyn Error>> {
        let base_path = self.get_base_path();
        let mut database_path = base_path.clone();

        let database_name = query
            .database_name
            .clone()
            .ok_or_else(|| ExecuteError::boxed("no database name"))?;

        database_path.push(&database_name);

        #[allow(clippy::single_match)]
        match tokio::fs::remove_dir_all(database_path.clone()).await {
            Ok(()) => {
                // 성공
            }
            Err(error) => match error.kind() {
                ErrorKind::NotFound => return Err(ExecuteError::boxed("database not found")),
                _ => {
                    return Err(ExecuteError::boxed("database drop failed"));
                }
            },
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
