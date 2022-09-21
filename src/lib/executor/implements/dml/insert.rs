use std::collections::HashSet;
use std::error::Error;
use std::io::ErrorKind;
use std::iter::FromIterator;

use crate::lib::ast::dml::InsertData;
use crate::lib::ast::predule::InsertQuery;
use crate::lib::errors::predule::ExecuteError;
use crate::lib::executor::config::{TableDataField, TableDataRow};
use crate::lib::executor::predule::{
    ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow, Executor,
    StorageEncoder, TableConfig,
};

impl Executor {
    pub async fn insert(&self, query: InsertQuery) -> Result<ExecuteResult, Box<dyn Error>> {
        let encoder = StorageEncoder::new();

        let into_table = query.into_table.unwrap();

        let database_name = into_table.database_name.clone().unwrap();
        let table_name = into_table.table_name;

        let base_path = self.get_base_path();

        let database_path = base_path.clone().join(&database_name);

        let table_path = database_path.clone().join(&table_name);

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

        match query.data {
            InsertData::Values(values) => {
                let mut rows = vec![];

                for value in &values {
                    let mut fields = vec![];

                    for (i, column_name) in query.columns.iter().enumerate() {
                        let value = value.list[i].clone();

                        let data = self.reduce_expression(value).await?;

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

                        fields.push(TableDataField { column_name, data });
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
