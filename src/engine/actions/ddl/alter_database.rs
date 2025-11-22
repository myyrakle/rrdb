use std::io::ErrorKind as IOErrorKind;

use crate::engine::DBEngine;
use crate::engine::ast::ddl::alter_database::{AlterDatabaseAction, AlterDatabaseQuery};

use crate::engine::encoder::schema_encoder::StorageEncoder;
use crate::engine::schema::database::DatabaseSchema;
use crate::engine::types::{
    ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow,
};
use crate::errors::execute_error::ExecuteError;
use crate::errors::{self, ErrorKind, Errors};

impl DBEngine {
    pub async fn alter_database(&self, query: AlterDatabaseQuery) -> errors::Result<ExecuteResult> {
        let encoder = StorageEncoder::new();

        let base_path = self.get_data_directory();

        #[allow(clippy::single_match)]
        match query.action {
            Some(action) => match action {
                AlterDatabaseAction::RenameTo(rename) => {
                    // 기존 데이터베이스명
                    let from_database_name = query.database_name.clone().ok_or_else(|| {
                        Errors::new(ErrorKind::ExecuteError("no database name".to_string()))
                    })?;

                    // 변경할 데이터베이스명
                    let to_database_name = rename.name;

                    // 실제 데이터베이스 디렉터리 경로
                    let mut from_path = base_path.clone();
                    let mut to_path = base_path.clone();

                    from_path.push(from_database_name);
                    to_path.push(to_database_name.clone());

                    // 디렉터리명 변경
                    let result = tokio::fs::rename(&from_path, &to_path).await;

                    if let Err(error) = result {
                        match error.kind() {
                            IOErrorKind::NotFound => {
                                return Err(Errors::new(ErrorKind::ExecuteError(
                                    "database not found".to_string(),
                                )));
                            }
                            _ => {
                                return Err(Errors::new(ErrorKind::ExecuteError(
                                    "database alter failed".to_string(),
                                )));
                            }
                        }
                    }

                    // config data 파일 내용 변경
                    let mut config_path = to_path.clone();
                    config_path.push("database.config");

                    match tokio::fs::read(&config_path).await {
                        Ok(data) => {
                            let database_config: Option<DatabaseSchema> =
                                encoder.decode(data.as_slice());

                            match database_config {
                                Some(mut database_config) => {
                                    database_config.database_name = to_database_name;
                                    if let Err(_error) = tokio::fs::write(
                                        config_path,
                                        encoder.encode(database_config),
                                    )
                                    .await
                                    {
                                        return Err(Errors::new(ErrorKind::ExecuteError(
                                            "no database name".to_string(),
                                        )));
                                    }
                                }
                                None => {
                                    return Err(Errors::new(ErrorKind::ExecuteError(
                                        "invalid config data".to_string(),
                                    )));
                                }
                            }
                        }
                        Err(error) => match error.kind() {
                            IOErrorKind::NotFound => {
                                return Err(Errors::new(ErrorKind::ExecuteError(
                                    "database not found".to_string(),
                                )));
                            }
                            _ => {
                                return Err(ExecuteError::wrap(format!("{:?}", error)));
                            }
                        },
                    }
                }
            },
            None => {}
        }

        Ok(ExecuteResult {
            columns: (vec![ExecuteColumn {
                name: "desc".into(),
                data_type: ExecuteColumnType::String,
            }]),
            rows: (vec![ExecuteRow {
                fields: vec![ExecuteField::String("alter database".into())],
            }]),
        })
    }
}
