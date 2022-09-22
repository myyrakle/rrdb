use std::error::Error;
use std::io::ErrorKind;

use super::config::TableConfig;
use super::encoder::StorageEncoder;
use super::predule::Executor;
use crate::lib::ast::predule::TableName;
use crate::lib::errors::execute_error::ExecuteError;

impl Executor {
    pub async fn get_table_config(
        &self,
        table_name: TableName,
    ) -> Result<TableConfig, Box<dyn Error + Send>> {
        let encoder = StorageEncoder::new();

        let base_path = self.get_base_path();

        let TableName {
            database_name,
            table_name,
        } = table_name;

        let database_name = database_name.unwrap();

        let database_path = base_path.clone().join(&database_name);
        let table_path = database_path.clone().join(&table_name);

        // config data 파일 내용 변경
        let config_path = table_path.clone().join("table.config");

        match tokio::fs::read(&config_path).await {
            Ok(data) => {
                let table_config: Option<TableConfig> = encoder.decode(data.as_slice());

                match table_config {
                    Some(table_config) => {
                        return Ok(table_config);
                    }
                    None => {
                        return Err(ExecuteError::boxed("invalid config data"));
                    }
                }
            }
            Err(error) => match error.kind() {
                ErrorKind::NotFound => {
                    return Err(ExecuteError::boxed("table not found"));
                }
                _ => {
                    return Err(ExecuteError::boxed(format!("{:?}", error)));
                }
            },
        }
    }
}
