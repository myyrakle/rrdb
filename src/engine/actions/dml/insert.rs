use std::collections::HashSet;
use std::io::ErrorKind as IOErrorKind;

use crate::engine::DBEngine;
use crate::engine::ast::dml::insert::{InsertData, InsertQuery};
use crate::engine::ast::types::SQLExpression;
use crate::engine::encoder::schema_encoder::StorageEncoder;
use crate::engine::schema::row::{TableDataField, TableDataRow};
use crate::engine::schema::table::TableSchema;
use crate::engine::storage::TableHeap;
use crate::engine::types::{
    ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow,
};
use crate::errors;
use crate::errors::execute_error::ExecuteError;

impl DBEngine {
    pub async fn insert(&self, query: InsertQuery) -> errors::Result<ExecuteResult> {
        let encoder = StorageEncoder::new();

        let into_table = query.into_table.as_ref().unwrap();

        let database_name = into_table.clone().database_name.unwrap();
        let table_name = into_table.clone().table_name;

        let base_path = self.get_data_directory();

        let database_path = base_path.clone().join(&database_name);

        // 설정파일 경로
        let table_path = database_path.clone().join("tables").join(&table_name);
        let config_path = table_path.join("table.config");

        let table_config = match tokio::fs::read(&config_path).await {
            Ok(data) => {
                let table_config: Option<TableSchema> = encoder.decode(data.as_slice());

                match table_config {
                    Some(table_config) => table_config,
                    None => {
                        return Err(ExecuteError::wrap("invalid config data".to_string()));
                    }
                }
            }
            Err(error) => match error.kind() {
                IOErrorKind::NotFound => {
                    return Err(ExecuteError::wrap("table not found".to_string()));
                }
                _ => {
                    return Err(ExecuteError::wrap(format!("{:?}", error)));
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
                return Err(ExecuteError::wrap(format!(
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
                            None => SQLExpression::Null,
                        };

                        let value = value.list[i].clone().unwrap_or(default_value);

                        let data = self.reduce_expression(value, Default::default()).await?;

                        match columns_map.get(column_name) {
                            Some(column) => {
                                if column.not_null && data.type_code() == 0 {
                                    return Err(ExecuteError::wrap(format!(
                                        "column '{}' is not null column",
                                        column_name
                                    )));
                                }

                                if column.data_type.type_code() != data.type_code()
                                    && data.type_code() != 0
                                {
                                    return Err(ExecuteError::wrap(format!(
                                        "column '{}' type mismatch",
                                        column_name
                                    )));
                                }
                            }
                            None => {
                                return Err(ExecuteError::wrap(format!(
                                    "column '{}' not exists",
                                    column_name
                                )));
                            }
                        }

                        let column_name = column_name.to_owned();

                        fields.push(TableDataField {
                            column_name,
                            data,
                            table_name: into_table.clone(),
                        });
                    }

                    for column_name in remain_columns.clone() {
                        let column_config_info = columns_map.get(column_name).unwrap();

                        let default_value = match &column_config_info.default {
                            Some(default) => default.to_owned(),
                            None => {
                                if column_config_info.not_null {
                                    return Err(ExecuteError::wrap(format!(
                                        "column '{}' is not null column",
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
                                    return Err(ExecuteError::wrap(format!(
                                        "column '{}' type mismatch",
                                        column_name
                                    )));
                                }
                            }
                            None => {
                                return Err(ExecuteError::wrap(format!(
                                    "column '{}' not exists",
                                    column_name
                                )));
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

                let mut heaps = self.table_heaps.write().await;
                let heap = heaps
                    .entry(into_table.clone())
                    .or_insert_with(TableHeap::new);
                for row in rows {
                    let row_bytes = encoder.encode(row);
                    heap.insert(&row_bytes)
                        .map_err(|error| ExecuteError::wrap(format!("{:?}", error)))?;
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
