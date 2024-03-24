use std::error::Error;
use std::io::ErrorKind;

use crate::ast::other::desc_table::DescTableQuery;
use crate::errors::predule::ExecuteError;
use crate::executor::config::table::TableConfig;
use crate::executor::encoder::storage::StorageEncoder;
use crate::executor::predule::{
    ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow, Executor,
};

impl Executor {
    pub async fn desc_table(&self, query: DescTableQuery) -> Result<ExecuteResult, RRDBError> {
        let encoder = StorageEncoder::new();

        let database_name = query.table_name.database_name.unwrap();
        let table_name = query.table_name.table_name;

        let base_path = self.get_base_path();
        let table_path = base_path
            .join(&database_name)
            .join("tables")
            .join(&table_name);
        let config_path = table_path.join("table.config");

        match std::fs::read(&config_path) {
            Ok(read_result) => {
                let table_info: TableConfig = encoder
                    .decode(read_result.as_slice())
                    .ok_or_else(|| ExecuteError::new("config decode error"))?;

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
                ErrorKind::NotFound => Err(ExecuteError::new(format!(
                    "table '{}' not exists",
                    table_name
                ))),
                _ => Err(ExecuteError::new("database listup failed")),
            },
        }
    }
}
