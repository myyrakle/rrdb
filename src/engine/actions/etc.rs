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
use crate::errors;
use crate::errors::execute_error::ExecuteError;

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
                let table_info: TableSchema = encoder
                    .decode(read_result.as_slice())
                    .ok_or_else(|| ExecuteError::wrap("config decode error".to_string()))?;

                Ok(ExecuteResult::new(
                    vec![
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
                    ],
                    table_info
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
                ))
            }
            Err(error) => match error.kind() {
                IOErrorKind::NotFound => Err(ExecuteError::wrap(format!(
                    "table '{}' not exists",
                    table_name
                ))),
                _ => Err(ExecuteError::wrap("database listup failed".to_string())),
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

                Ok(ExecuteResult::new(
                    vec![ExecuteColumn {
                        name: "database name".into(),
                        data_type: ExecuteColumnType::String,
                    }],
                    database_list
                        .map(|e| ExecuteRow {
                            fields: vec![ExecuteField::String(e)],
                        })
                        .collect(),
                ))
            }
            Err(error) => match error.kind() {
                IOErrorKind::NotFound => {
                    Err(ExecuteError::wrap("base path not exists".to_string()))
                }
                _ => Err(ExecuteError::wrap("database listup failed".to_string())),
            },
        }
    }

    pub async fn find_database(&self, database_name: String) -> errors::Result<bool> {
        let database_path = self.get_data_directory().join(database_name);

        match tokio::fs::metadata(database_path).await {
            Ok(metadata) => Ok(metadata.is_dir()),
            Err(error) => match error.kind() {
                IOErrorKind::NotFound => Ok(false),
                _ => Err(ExecuteError::wrap(error.to_string())),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::config::launch_config::LaunchConfig;
    use crate::engine::DBEngine;
    use crate::engine::ast::ddl::create_database::CreateDatabaseQuery;

    #[tokio::test]
    async fn find_database_checks_database_directory_directly() {
        let base_path = PathBuf::from("target/test_find_database/direct_directory_lookup");
        if base_path.exists() {
            tokio::fs::remove_dir_all(&base_path).await.unwrap();
        }

        let config = LaunchConfig::default_for_base_path(&base_path);
        tokio::fs::create_dir_all(&config.data_directory)
            .await
            .unwrap();

        let engine = DBEngine::new(config);
        engine
            .create_database(
                CreateDatabaseQuery::builder()
                    .set_name("lookup_db".to_string())
                    .set_if_not_exists(false),
            )
            .await
            .unwrap();

        assert!(engine.find_database("lookup_db".to_string()).await.unwrap());
        assert!(
            !engine
                .find_database("missing_db".to_string())
                .await
                .unwrap()
        );
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

                Ok(ExecuteResult::new(
                    vec![ExecuteColumn {
                        name: "table name".into(),
                        data_type: ExecuteColumnType::String,
                    }],
                    table_list
                        .map(|e| ExecuteRow {
                            fields: vec![ExecuteField::String(e)],
                        })
                        .collect(),
                ))
            }
            Err(error) => match error.kind() {
                IOErrorKind::NotFound => {
                    Err(ExecuteError::wrap("base path not exists".to_string()))
                }
                _ => Err(ExecuteError::wrap("table listup failed".to_string())),
            },
        }
    }
}

impl DBEngine {
    pub async fn use_databases(&self, query: UseDatabaseQuery) -> errors::Result<ExecuteResult> {
        Ok(ExecuteResult::new(
            vec![ExecuteColumn {
                name: "desc".into(),
                data_type: ExecuteColumnType::String,
            }],
            vec![ExecuteRow {
                fields: vec![ExecuteField::String(format!(
                    "database changed: {}",
                    query.database_name
                ))],
            }],
        ))
    }
}
