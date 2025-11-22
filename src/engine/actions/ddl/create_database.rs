use std::io::ErrorKind as IOErrorKind;

use crate::engine::DBEngine;
use crate::engine::ast::ddl::create_database::CreateDatabaseQuery;
use crate::engine::encoder::schema_encoder::StorageEncoder;
use crate::engine::schema::database::DatabaseSchema;

use crate::engine::types::{
    ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow,
};
use crate::errors::execute_error::ExecuteError;
use crate::errors::{self, ErrorKind, Errors};

impl DBEngine {
    pub async fn create_database(
        &self,
        query: CreateDatabaseQuery,
    ) -> errors::Result<ExecuteResult> {
        let encoder = StorageEncoder::new();

        let base_path = self.get_data_directory();

        let database_name = query
            .database_name
            .clone()
            .ok_or_else(|| Errors::new(ErrorKind::ExecuteError("no database name".to_string())))?;

        let database_path = base_path.clone().join(&database_name);

        if let Err(error) = tokio::fs::create_dir(database_path.clone()).await {
            match error.kind() {
                IOErrorKind::AlreadyExists => {
                    if query.if_not_exists {
                        return Ok(ExecuteResult {
                            columns: (vec![ExecuteColumn {
                                name: "desc".into(),
                                data_type: ExecuteColumnType::String,
                            }]),
                            rows: (vec![ExecuteRow {
                                fields: vec![ExecuteField::String(
                                    "database already exists".into(),
                                )],
                            }]),
                        });
                    } else {
                        return Err(Errors::new(ErrorKind::ExecuteError(
                            "already exists database".to_string(),
                        )));
                    }
                }
                _ => {
                    return Err(Errors::new(ErrorKind::ExecuteError(
                        "database create failed".to_string(),
                    )));
                }
            }
        }

        // tables 경로 추가
        let tables_path = database_path.clone().join("tables");

        if let Err(error) = tokio::fs::create_dir(&tables_path).await {
            match error.kind() {
                IOErrorKind::AlreadyExists => {
                    return Err(Errors::new(ErrorKind::ExecuteError(
                        "already exists tables".to_string(),
                    )));
                }
                _ => {
                    return Err(Errors::new(ErrorKind::ExecuteError(
                        "tables create failed".to_string(),
                    )));
                }
            }
        }

        // 각 데이터베이스 단위 설정파일 생성
        let config_path = database_path.clone().join("database.config");
        let database_info = DatabaseSchema {
            database_name: database_name.clone(),
        };

        if let Err(error) = tokio::fs::write(config_path, encoder.encode(database_info)).await {
            return Err(ExecuteError::wrap(error.to_string()));
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
