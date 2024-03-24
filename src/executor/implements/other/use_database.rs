use std::error::Error;

use crate::{
    ast::other::use_database::UseDatabaseQuery,
    executor::predule::{
        ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow, Executor,
    },
};

impl Executor {
    pub async fn use_databases(&self, query: UseDatabaseQuery) -> Result<ExecuteResult, RRDBError> {
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
