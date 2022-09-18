use std::error::Error;
use std::io::ErrorKind;

use crate::lib::ast::other::DescTableQuery;
use crate::lib::errors::predule::ExecuteError;
use crate::lib::executor::predule::{
    ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow, Executor,
    StorageEncoder, TableConfig,
};

impl Executor {
    pub async fn desc_table(&self, query: DescTableQuery) -> Result<ExecuteResult, Box<dyn Error>> {
        let encoder = StorageEncoder::new();

        let base_path = self.get_base_path();
        let mut table_path = base_path;
        table_path.push(query.table_name.database_name.unwrap());
        table_path.push(query.table_name.table_name);
        table_path.push("table.config");

        match std::fs::read(&table_path) {
            Ok(read_result) => {
                let table_info: TableConfig = encoder
                    .decode(read_result.as_slice())
                    .ok_or_else(|| ExecuteError::boxed("config decode error"))?;

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
                ErrorKind::NotFound => Err(ExecuteError::boxed("base path not exists")),
                _ => Err(ExecuteError::boxed("database listup failed")),
            },
        }
    }
}
