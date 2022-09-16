use std::error::Error;
use std::io::ErrorKind;

use futures::future::join_all;

use crate::lib::ast::other::UseDatabaseQuery;
use crate::lib::ast::predule::ShowDatabasesQuery;
use crate::lib::errors::predule::ExecuteError;
use crate::lib::executor::predule::{
    DatabaseConfig, ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow,
    Executor, StorageEncoder,
};

impl Executor {
    pub async fn use_databases(
        &self,
        query: UseDatabaseQuery,
    ) -> Result<ExecuteResult, Box<dyn Error>> {
        Ok(ExecuteResult {
            columns: (vec![ExecuteColumn {
                name: "desc".into(),
                data_type: ExecuteColumnType::String,
            }]),
            rows: (vec![ExecuteRow {
                fields: vec![ExecuteField::String(format!(
                    "database changed: {}",
                    query.database_name
                ))],
            }]),
        })
    }
}
