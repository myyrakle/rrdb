use std::error::Error;
use std::fs::FileType;
use std::io::ErrorKind;

use futures::future::join_all;

use crate::lib::ast::predule::ShowDatabasesQuery;
use crate::lib::errors::predule::ExecuteError;
use crate::lib::executor::predule::{
    DatabaseConfig, ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow,
    Executor, StorageEncoder,
};

impl Executor {
    pub async fn show_databses(
        &self,
        query: ShowDatabasesQuery,
    ) -> Result<ExecuteResult, Box<dyn Error>> {
        let encoder = StorageEncoder::new();

        let base_path = self.get_base_path();

        match std::fs::read_dir(&base_path) {
            Ok(read_dir_result) => {
                let futures = read_dir_result.map(|e| async {
                    match e {
                        Ok(entry) => match entry.file_type() {
                            Ok(file_type) => {
                                if file_type.is_dir() {
                                    let mut path = entry.path();
                                    path.push("database.config");

                                    match tokio::fs::read(path).await {
                                        Ok(result) => {
                                            let databaseConfig: DatabaseConfig =
                                                encoder.decode(result.as_slice()).unwrap();

                                            Some(databaseConfig.database_name)
                                        }
                                        Err(_) => None,
                                    }
                                } else {
                                    None
                                }
                            }
                            Err(_) => None,
                        },
                        Err(_) => None,
                    }
                });

                let database_list: Vec<_> = join_all(futures)
                    .await
                    .into_iter()
                    .filter_map(|e| e)
                    .collect();

                Ok(ExecuteResult {
                    columns: (vec![ExecuteColumn {
                        name: "database name".into(),
                        data_type: ExecuteColumnType::String,
                    }]),
                    rows: database_list
                        .into_iter()
                        .map(|e| ExecuteRow {
                            fields: vec![ExecuteField::String(e)],
                        })
                        .collect(),
                })
            }
            Err(error) => match error.kind() {
                ErrorKind::NotFound => return Err(ExecuteError::boxed("base path not exists")),
                _ => {
                    return Err(ExecuteError::boxed("database listup failed"));
                }
            },
        }
    }
}
