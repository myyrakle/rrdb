use std::io::ErrorKind as IOErrorKind;

use futures::future::join_all;

use crate::engine::DBEngine;
use crate::engine::ast::other::desc_table::DescTableQuery;
use crate::engine::ast::other::show_databases::ShowDatabasesQuery;
use crate::engine::ast::other::show_tables::ShowTablesQuery;
use crate::engine::ast::other::use_database::UseDatabaseQuery;

use crate::engine::encoder::schema_encoder::StorageEncoder;
use crate::engine::schema::database::DatabaseSchema;
use crate::engine::schema::table::TableSchema;
use crate::engine::types::{
    ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow,
};
use crate::errors::execute_error::ExecuteError;
use crate::errors;

impl DBEngine {
    pub async fn desc_table(&self, query: DescTableQuery) -> errors::Result<ExecuteResult> {
        let encoder = StorageEncoder::new();

        let database_name = query.table_name.database_name.unwrap();
        let table_name = query.table_name.table_name;

        let base_path = self.get_data_directory();
        let table_path = base_path
            .join(database_name)
            .join("tables")
            .join(&table_name);
        let config_path = table_path.join("table.config");

        match tokio::fs::read(config_path).await {
            Ok(read_result) => {
                let table_info: TableSchema =
                    encoder.decode(read_result.as_slice()).ok_or_else(|| {
                        ExecuteError::wrap("config decode error".to_string())
                    })?;

                Ok(ExecuteResult {
                    columns: (vec![
                        ExecuteColumn {
                            name: "Field".into(),
                            data_type: ExecuteColumnType::String,
                        },
                        ExecuteColumn {
                            name: "Type".into(),
                            data_type: ExecuteColumnType::String,
                        },
                        ExecuteColumn {
                            name: "Null".into(),
                            data_type: ExecuteColumnType::String,
                        },
                        ExecuteColumn {
                            name: "Default".into(),
                            data_type: ExecuteColumnType::String,
                        },
                        ExecuteColumn {
                            name: "Comment".into(),
                            data_type: ExecuteColumnType::String,
                        },
                    ]),
                    rows: table_info
                        .columns
                        .iter()
                        .map(|e| ExecuteRow {
                            fields: vec![
                                ExecuteField::String(e.name.to_owned()),
                                ExecuteField::String(e.data_type.to_owned().into()),
                                ExecuteField::String(if e.not_null { "NO" } else { "YES" }.into()),
                                ExecuteField::String(format!("{:?}", e.default)), // TODO: 표현식 역 parsing 구현
                                ExecuteField::String(e.comment.to_owned()),
                            ],
                        })
                        .collect(),
                })
            }
            Err(error) => match error.kind() {
                IOErrorKind::NotFound => Err(ExecuteError::wrap(format!(
                    "table '{}' not exists",
                    table_name
                ))),
                _ => Err(ExecuteError::wrap(
                    "database listup failed".to_string(),
                )),
            },
        }
    }
}

impl DBEngine {
    pub async fn show_databases(
        &self,
        _query: ShowDatabasesQuery,
    ) -> errors::Result<ExecuteResult> {


        let base_path = self.get_data_directory();

        match tokio::fs::read_dir(&base_path).await {
            Ok(mut read_dir_result) => {
                let mut futures = Vec::new();

                while let Ok(Some(entry)) = read_dir_result.next_entry().await {
                    futures.push(async move {
                        match entry.file_type().await {
                            Ok(file_type) => {
                                if file_type.is_dir() {
                                    let mut path = entry.path();
                                    path.push("database.config");

                                    match tokio::fs::read(path).await {
                                        Ok(result) => {
                                            let encoder = StorageEncoder::new();
                                            let database_config: DatabaseSchema =
                                                encoder.decode(result.as_slice()).unwrap();

                                            Some(database_config.database_name)
                                        }
                                        Err(_) => None,
                                    }
                                } else {
                                    None
                                }
                            }
                            Err(_) => None,
                        }
                    });
                }

                let database_list = join_all(futures).await.into_iter().flatten();

                Ok(ExecuteResult {
                    columns: (vec![ExecuteColumn {
                        name: "database name".into(),
                        data_type: ExecuteColumnType::String,
                    }]),
                    rows: database_list
                        .map(|e| ExecuteRow {
                            fields: vec![ExecuteField::String(e)],
                        })
                        .collect(),
                })
            }
            Err(error) => match error.kind() {
                IOErrorKind::NotFound => Err(ExecuteError::wrap(
                    "base path not exists".to_string(),
                )),
                _ => Err(ExecuteError::wrap(
                    "database listup failed".to_string(),
                )),
            },
        }
    }

    pub async fn find_database(&self, database_name: String) -> errors::Result<bool> {
        let result = self.show_databases(ShowDatabasesQuery {}).await?;

        Ok(result.rows.iter().any(|e| {
            if let ExecuteField::String(name) = &e.fields[0] {
                name == &database_name
            } else {
                false
            }
        }))
    }
}

impl DBEngine {
    pub async fn show_tables(&self, query: ShowTablesQuery) -> errors::Result<ExecuteResult> {


        let base_path = self.get_data_directory();
        let database_path = base_path.clone().join(query.database);
        let tables_path = database_path.join("tables");

        match tokio::fs::read_dir(&tables_path).await {
            Ok(mut read_dir_result) => {
                let mut futures = Vec::new();

                while let Ok(Some(entry)) = read_dir_result.next_entry().await {
                    futures.push(async move {
                        match entry.file_type().await {
                            Ok(file_type) => {
                                if file_type.is_dir() {
                                    let mut path = entry.path();
                                    path.push("table.config");

                                    match tokio::fs::read(path).await {
                                        Ok(result) => {
                                            let encoder = StorageEncoder::new();
                                            let table_config: TableSchema =
                                                match encoder.decode(result.as_slice()) {
                                                    Some(decoded) => decoded,
                                                    None => return None,
                                                };

                                            Some(table_config.table.table_name)
                                        }
                                        Err(_) => None,
                                    }
                                } else {
                                    None
                                }
                            }
                            Err(_) => None,
                        }
                    });
                }

                let table_list = join_all(futures).await.into_iter().flatten();

                Ok(ExecuteResult {
                    columns: (vec![ExecuteColumn {
                        name: "table name".into(),
                        data_type: ExecuteColumnType::String,
                    }]),
                    rows: table_list
                        .map(|e| ExecuteRow {
                            fields: vec![ExecuteField::String(e)],
                        })
                        .collect(),
                })
            }
            Err(error) => match error.kind() {
                IOErrorKind::NotFound => Err(ExecuteError::wrap(
                    "base path not exists".to_string(),
                )),
                _ => Err(ExecuteError::wrap(
                    "table listup failed".to_string(),
                )),
            },
        }
    }
}

impl DBEngine {
    pub async fn use_databases(&self, query: UseDatabaseQuery) -> errors::Result<ExecuteResult> {
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
