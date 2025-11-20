use std::io::ErrorKind as IOErrorKind;
use std::path::PathBuf;

use futures::future::join_all;

use crate::engine::DBEngine;
use crate::engine::ast::types::TableName;
use crate::engine::encoder::schema_encoder::StorageEncoder;
use crate::engine::schema::row::TableDataRow;
use crate::errors::{Errors, ErrorKind};
use crate::errors::execute_error::ExecuteError;

impl DBEngine {
    pub async fn full_scan(
        &self,
        table_name: TableName,
    ) -> Result<Vec<(PathBuf, TableDataRow)>, Errors> {
        let encoder = StorageEncoder::new();

        let database_name = table_name.database_name.unwrap();
        let table_name = table_name.table_name;

        let base_path = self.get_data_directory();

        let database_path = base_path.clone().join(&database_name);

        let table_path = database_path.clone().join("tables").join(&table_name);

        // 데이터 행 파일 경로
        let rows_path = table_path.clone().join("rows");

        match std::fs::read_dir(&rows_path) {
            Ok(read_dir_result) => {
                let futures = read_dir_result.map(|e| async {
                    match e {
                        Ok(entry) => match entry.file_type() {
                            Ok(file_type) => {
                                if file_type.is_file() {
                                    let path = entry.path();

                                    match tokio::fs::read(&path).await {
                                        Ok(result) => {
                                            match encoder.decode::<TableDataRow>(result.as_slice())
                                            {
                                                Some(decoded) => Ok((path.to_path_buf(), decoded)),
                                                None => Err(ExecuteError::wrap(format!(
                                                    "full scan failed {:?}",
                                                    path
                                                ))),
                                            }
                                        }
                                        Err(error) => Err(ExecuteError::wrap(format!(
                                            "full scan failed {}",
                                            error
                                        ))),
                                    }
                                } else {
                                    Err(Errors::new(ErrorKind::ExecuteError(
                                        "full scan failed".to_string(),
                                    )))
                                }
                            }
                            Err(error) => {
                                Err(ExecuteError::wrap(format!("full scan failed {}", error)))
                            }
                        },
                        Err(error) => {
                            Err(ExecuteError::wrap(format!("full scan failed {}", error)))
                        }
                    }
                });

                let rows = join_all(futures)
                    .await
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>();

                match rows {
                    Ok(rows) => Ok(rows),
                    Err(error) => Err(ExecuteError::wrap(error.to_string())),
                }
            }
            Err(error) => match error.kind() {
                IOErrorKind::NotFound => Err(Errors::new(ErrorKind::ExecuteError(
                    "base path not exists (3)".to_string(),
                ))),
                _ => Err(Errors::new(ErrorKind::ExecuteError(
                    "full scan failed".to_string(),
                ))),
            },
        }
    }

    pub async fn index_scan(&self, _table_name: TableName) {}
}
