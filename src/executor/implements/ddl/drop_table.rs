use std::error::Error;
use std::io::ErrorKind;

use crate::lib::ast::ddl::DropTableQuery;
use crate::lib::ast::predule::TableName;
use crate::lib::errors::predule::ExecuteError;
use crate::lib::executor::predule::{ExecuteResult, Executor};
use crate::lib::executor::result::{ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteRow};

impl Executor {
    pub async fn drop_table(
        &self,
        query: DropTableQuery,
    ) -> Result<ExecuteResult, Box<dyn Error + Send>> {
        let base_path = self.get_base_path();

        let TableName {
            database_name,
            table_name,
        } = query.table.unwrap();

        let table_path = base_path
            .clone()
            .join(&database_name.unwrap())
            .join("tables")
            .join(&table_name);

        if let Err(error) = tokio::fs::remove_dir_all(table_path).await {
            match error.kind() {
                ErrorKind::NotFound => return Err(ExecuteError::boxed("table not found")),
                _ => {
                    return Err(ExecuteError::boxed("table drop failed"));
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
