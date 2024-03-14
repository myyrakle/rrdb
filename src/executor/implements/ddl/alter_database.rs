use std::error::Error;
use std::io::ErrorKind;

use crate::ast::ddl::{AlterDatabaseAction, AlterDatabaseQuery};
use crate::errors::predule::ExecuteError;
use crate::executor::encoder::StorageEncoder;
use crate::executor::predule::{DatabaseConfig, ExecuteResult, Executor};
use crate::executor::result::{ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteRow};

impl Executor {
    pub async fn alter_database(
        &self,
        query: AlterDatabaseQuery,
    ) -> Result<ExecuteResult, Box<dyn Error + Send>> {
        let encoder = StorageEncoder::new();

        let base_path = self.get_base_path();

        #[allow(clippy::single_match)]
        match query.action {
            Some(action) => match action {
                AlterDatabaseAction::RenameTo(rename) => {
                    // 기존 데이터베이스명
                    let from_database_name = query
                        .database_name
                        .clone()
                        .ok_or_else(|| ExecuteError::dyn_boxed("no database name"))?;

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
                            ErrorKind::NotFound => {
                                return Err(ExecuteError::boxed("database not found"))
                            }
                            _ => {
                                return Err(ExecuteError::boxed("database alter failed"));
                            }
                        }
                    }

                    // config data 파일 내용 변경
                    let mut config_path = to_path.clone();
                    config_path.push("database.config");

                    match tokio::fs::read(&config_path).await {
                        Ok(data) => {
                            let database_config: Option<DatabaseConfig> =
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
                                        return Err(ExecuteError::dyn_boxed("no database name"));
                                    }
                                }
                                None => {
                                    return Err(ExecuteError::boxed("invalid config data"));
                                }
                            }
                        }
                        Err(error) => match error.kind() {
                            ErrorKind::NotFound => {
                                return Err(ExecuteError::boxed("database not found"));
                            }
                            _ => {
                                return Err(ExecuteError::boxed(format!("{:?}", error)));
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
