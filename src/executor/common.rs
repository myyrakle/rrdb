use std::io::ErrorKind;

use super::config::table::TableConfig;
use super::encoder::storage::StorageEncoder;
use super::predule::Executor;
use crate::ast::types::TableName;
use crate::errors::execute_error::ExecuteError;
use crate::errors::RRDBError;

impl Executor {
    pub async fn get_table_config(&self, table_name: TableName) -> Result<TableConfig, RRDBError> {
        let encoder = StorageEncoder::new();

        let base_path = self.get_base_path();

        let TableName {
            database_name,
            table_name,
        } = table_name;

        let database_name = database_name.unwrap();

        let database_path = base_path.clone().join(&database_name);
        let table_path = database_path.clone().join("tables").join(&table_name);

        // config data 파일 내용 변경
        let config_path = table_path.clone().join("table.config");

        match tokio::fs::read(&config_path).await {
            Ok(data) => {
                let table_config: Option<TableConfig> = encoder.decode(data.as_slice());

                match table_config {
                    Some(table_config) => Ok(table_config),
                    None => Err(ExecuteError::new("invalid config data")),
                }
            }
            Err(error) => match error.kind() {
                ErrorKind::NotFound => Err(ExecuteError::new("table not found")),
                _ => Err(ExecuteError::new(format!("{:?}", error))),
            },
        }
    }
}
