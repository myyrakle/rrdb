use std::collections::HashSet;
use std::error::Error;
use std::io::ErrorKind;
use std::iter::FromIterator;

use crate::ast::dml::InsertData;
use crate::ast::predule::{InsertQuery, SQLExpression};
use crate::errors::predule::ExecuteError;
use crate::executor::predule::{
    ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow, Executor,
    StorageEncoder, TableConfig, TableDataField, TableDataRow,
};

impl Executor {
    pub async fn insert(&self, query: InsertQuery) -> Result<ExecuteResult, Box<dyn Error + Send>> {
        let encoder = StorageEncoder::new();

        let into_table = query.into_table.as_ref().unwrap();

        let database_name = into_table.clone().database_name.unwrap();
        let table_name = into_table.clone().table_name;

        let base_path = self.get_base_path();

        let database_path = base_path.clone().join(&database_name);

        let table_path = database_path.clone().join("tables").join(&table_name);

        // 데이터 행 파일 경로
        let rows_path = table_path.clone().join("rows");

        // 설정파일 경로
        let config_path = table_path.join("table.config");

        let table_config = match tokio::fs::read(&config_path).await {
            Ok(data) => {
                let table_config: Option<TableConfig> = encoder.decode(data.as_slice());

                match table_config {
                    Some(table_config) => table_config,
                    None => {
                        return Err(ExecuteError::boxed("invalid config data"));
                    }
                }
            }
            Err(error) => match error.kind() {
                ErrorKind::NotFound => {
                    return Err(ExecuteError::boxed("table not found"));
                }
                _ => {
                    return Err(ExecuteError::boxed(format!("{:?}", error)));
                }
            },
        };

        // 입력된 컬럼
        let input_columns_set: HashSet<String> = HashSet::from_iter(query.columns.iter().cloned());

        // 필수 컬럼
        let required_columns = table_config.get_required_columns();

        // 테이블 컬럼 맵
        let columns_map = table_config.get_columns_map();

        // 필수 입력 컬럼값 검증
        for required_column in required_columns {
            if !input_columns_set.contains(&required_column.name) {
                return Err(ExecuteError::boxed(format!(
                    "column '{}' is required, but it was not provided",
                    &required_column.name
                )));
            }
        }

        let remain_columns = table_config
            .columns
            .iter()
            .filter(|e| !query.columns.contains(&(*e).clone().name))
            .map(|e| &e.name);

        match &query.data {
            InsertData::Values(values) => {
                let mut rows = vec![];

                for value in values {
                    let mut fields = vec![];

                    // 명시적으로 전달된 컬럼값 리스트 처리
                    for (i, column_name) in query.columns.iter().enumerate() {
                        let column_config_info = columns_map.get(column_name).unwrap();

                        let default_value = match &column_config_info.default {
                            Some(default) => default.to_owned(),
                            None => {
                                if column_config_info.not_null {
                                    return Err(ExecuteError::boxed(format!(
                                        "column '{}' is not null column
                                        ",
                                        column_name
                                    )));
                                }

                                SQLExpression::Null
                            }
                        };

                        let value = value.list[i].clone().unwrap_or(default_value);

                        let data = self.reduce_expression(value, Default::default()).await?;

                        match columns_map.get(column_name) {
                            Some(column) => {
                                if column.data_type.type_code() != data.type_code()
                                    && data.type_code() != 0
                                {
                                    return Err(ExecuteError::boxed(format!(
                                        "column '{}' type mismatch
                                        ",
                                        column_name
                                    )));
                                }
                            }
                            None => {
                                return Err(ExecuteError::boxed(format!(
                                    "column '{}' not exists",
                                    column_name
                                )))
                            }
                        }

                        let column_name = column_name.to_owned();

                        fields.push(TableDataField {
                            column_name,
                            data,
                            table_name: into_table.clone(),
                        });
                    }

                    // 명시되지 않은 컬럼 리스트 처리
                    for column_name in remain_columns.clone() {
                        let column_config_info = columns_map.get(column_name).unwrap();

                        let default_value = match &column_config_info.default {
                            Some(default) => default.to_owned(),
                            None => {
                                if column_config_info.not_null {
                                    return Err(ExecuteError::boxed(format!(
                                        "column '{}' is not null column
                                        ",
                                        column_name
                                    )));
                                }

                                SQLExpression::Null
                            }
                        };

                        let data = self
                            .reduce_expression(default_value, Default::default())
                            .await?;

                        match columns_map.get(column_name) {
                            Some(column) => {
                                if column.data_type.type_code() != data.type_code()
                                    && data.type_code() != 0
                                {
                                    return Err(ExecuteError::boxed(format!(
                                        "column '{}' type mismatch
                                        ",
                                        column_name
                                    )));
                                }
                            }
                            None => {
                                return Err(ExecuteError::boxed(format!(
                                    "column '{}' not exists",
                                    column_name
                                )))
                            }
                        }

                        let column_name = column_name.to_owned();

                        fields.push(TableDataField {
                            column_name,
                            data,
                            table_name: into_table.clone(),
                        });
                    }

                    let row = TableDataRow { fields };
                    rows.push(row);
                }

                for row in rows {
                    let file_name = uuid::Uuid::new_v4().to_string();

                    let row_file_path = rows_path.join(file_name);

                    if let Err(error) = tokio::fs::write(row_file_path, encoder.encode(row)).await {
                        return Err(ExecuteError::boxed(error.to_string()));
                    }
                }
            }
            InsertData::Select(_select) => {
                todo!("아직 미구현")
            }
            InsertData::None => {}
        }

        Ok(ExecuteResult {
            columns: (vec![ExecuteColumn {
                name: "desc".into(),
                data_type: ExecuteColumnType::String,
            }]),
            rows: (vec![ExecuteRow {
                fields: vec![ExecuteField::String(format!(
                    "inserted into {}",
                    table_name
                ))],
            }]),
        })
    }
}
