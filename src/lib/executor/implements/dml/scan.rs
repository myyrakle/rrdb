use std::error::Error;
use std::io::ErrorKind;
use std::path::PathBuf;

use futures::future::join_all;

use crate::lib::ast::predule::TableName;
use crate::lib::errors::predule::ExecuteError;
use crate::lib::executor::predule::{Executor, StorageEncoder, TableDataRow};

impl Executor {
    pub async fn full_scan(
        &self,
        table_name: TableName,
    ) -> Result<Vec<(PathBuf, TableDataRow)>, Box<dyn Error + Send>> {
        let encoder = StorageEncoder::new();

        let database_name = table_name.database_name.unwrap();
        let table_name = table_name.table_name;

        let base_path = self.get_base_path();

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
                                                None => Err(ExecuteError::boxed(format!(
                                                    "full scan failed {:?}",
                                                    path
                                                ))),
                                            }
                                        }
                                        Err(error) => Err(ExecuteError::boxed(format!(
                                            "full scan failed {}",
                                            error
                                        ))),
                                    }
                                } else {
                                    Err(ExecuteError::boxed("full scan failed"))
                                }
                            }
                            Err(error) => {
                                Err(ExecuteError::boxed(format!("full scan failed {}", error)))
                            }
                        },
                        Err(error) => {
                            Err(ExecuteError::boxed(format!("full scan failed {}", error)))
                        }
                    }
                });

                let rows = join_all(futures)
                    .await
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>();

                match rows {
                    Ok(rows) => Ok(rows),
                    Err(error) => Err(ExecuteError::boxed(error.to_string())),
                }
            }
            Err(error) => match error.kind() {
                ErrorKind::NotFound => Err(ExecuteError::boxed("base path not exists")),
                _ => Err(ExecuteError::boxed("full scan failed")),
            },
        }
    }

    pub async fn index_scan(&self, _table_name: TableName) {}
}
