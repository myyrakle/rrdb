use std::error::Error;
use std::io::ErrorKind;

use futures::future::join_all;

use crate::ast::other::show_tables::ShowTablesQuery;
use crate::errors::predule::ExecuteError;
use crate::executor::config::table::TableConfig;
use crate::executor::encoder::storage::StorageEncoder;
use crate::executor::predule::{
    ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow, Executor,
};

impl Executor {
    pub async fn show_tables(
        &self,
        query: ShowTablesQuery,
    ) -> Result<ExecuteResult, Box<dyn Error + Send>> {
        let encoder = StorageEncoder::new();

        let base_path = self.get_base_path();
        let database_path = base_path.clone().join(query.database);
        let tables_path = database_path.join("tables");

        match std::fs::read_dir(&tables_path) {
            Ok(read_dir_result) => {
                let futures = read_dir_result.map(|e| async {
                    match e {
                        Ok(entry) => match entry.file_type() {
                            Ok(file_type) => {
                                if file_type.is_dir() {
                                    let mut path = entry.path();
                                    path.push("table.config");

                                    match tokio::fs::read(path).await {
                                        Ok(result) => {
                                            let table_config: TableConfig =
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
                        },
                        Err(_) => None,
                    }
                });

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
                ErrorKind::NotFound => Err(ExecuteError::boxed("base path not exists")),
                _ => Err(ExecuteError::boxed("table listup failed")),
            },
        }
    }
}
