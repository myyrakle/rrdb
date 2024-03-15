use std::error::Error;
use std::io::ErrorKind;

use crate::ast::ddl::create_database::CreateDatabaseQuery;
use crate::errors::predule::ExecuteError;
use crate::executor::config::database::DatabaseConfig;
use crate::executor::encoder::storage::StorageEncoder;
use crate::executor::predule::{ExecuteResult, Executor};
use crate::executor::result::{ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteRow};

impl Executor {
    pub async fn create_database(
        &self,
        query: CreateDatabaseQuery,
    ) -> Result<ExecuteResult, Box<dyn Error + Send>> {
        let encoder = StorageEncoder::new();

        let base_path = self.get_base_path();

        let database_name = query
            .database_name
            .clone()
            .ok_or_else(|| ExecuteError::dyn_boxed("no database name"))?;

        let database_path = base_path.clone().join(&database_name);

        if let Err(error) = tokio::fs::create_dir(database_path.clone()).await {
            match error.kind() {
                ErrorKind::AlreadyExists => {
                    return Err(ExecuteError::boxed("already exists database"))
                }
                _ => {
                    return Err(ExecuteError::boxed("database create failed"));
                }
            }
        }

        // tables 경로 추가
        let tables_path = database_path.clone().join("tables");

        if let Err(error) = tokio::fs::create_dir(&tables_path).await {
            match error.kind() {
                ErrorKind::AlreadyExists => {
                    return Err(ExecuteError::boxed("already exists tables"))
                }
                _ => {
                    return Err(ExecuteError::boxed("tables create failed"));
                }
            }
        }

        // 각 데이터베이스 단위 설정파일 생성
        let config_path = database_path.clone().join("database.config");
        let database_info = DatabaseConfig {
            database_name: database_name.clone(),
        };

        if let Err(error) = tokio::fs::write(config_path, encoder.encode(database_info)).await {
            return Err(ExecuteError::boxed(error.to_string()));
        }

        Ok(ExecuteResult {
            columns: (vec![ExecuteColumn {
                name: "desc".into(),
                data_type: ExecuteColumnType::String,
            }]),
            rows: (vec![ExecuteRow {
                fields: vec![ExecuteField::String(format!(
                    "database created: {}",
                    database_name
                ))],
            }]),
        })
    }
}
