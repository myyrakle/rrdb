use std::collections::HashSet;

use crate::engine::actions::index::row_index_key;
use crate::engine::ast::dml::insert::{InsertData, InsertQuery};
use crate::engine::ast::types::SQLExpression;
use crate::engine::schema::row::{TableDataField, TableDataRow};
use crate::engine::types::{
    ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow,
};
use crate::engine::wal::types::EntryType;
use crate::engine::{DBEngine, SharedWALManager};
use crate::errors;
use crate::errors::execute_error::ExecuteError;

impl DBEngine {
    pub async fn insert(
        &self,
        query: InsertQuery,
        wal_manager: SharedWALManager,
    ) -> errors::Result<ExecuteResult> {
        self.insert_internal(query, Some(wal_manager)).await
    }

    /// Re-applies a previously WAL-logged INSERT during crash recovery
    /// replay. Identical to `insert()` but skips the WAL append (the
    /// operation is already durably recorded in the WAL being replayed).
    pub(crate) async fn insert_replay(&self, query: InsertQuery) -> errors::Result<ExecuteResult> {
        self.insert_internal(query, None).await
    }

    async fn insert_internal(
        &self,
        query: InsertQuery,
        wal_manager: Option<SharedWALManager>,
    ) -> errors::Result<ExecuteResult> {
        let into_table = query.into_table.as_ref().unwrap();

        let table_name = into_table.clone().table_name;

        let table_config = self.get_table_config_cached(into_table.clone()).await?;

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
                                        "column '{}' is not null column
                                        ",
                                        column_name
                                    )));
                                }

                                if column.data_type.type_code() != data.type_code()
                                    && data.type_code() != 0
                                {
                                    return Err(ExecuteError::wrap(format!(
                                        "column '{}' type mismatch
                                        ",
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

                    // 명시되지 않은 컬럼 리스트 처리
                    for column_name in remain_columns.clone() {
                        let column_config_info = columns_map.get(column_name).unwrap();

                        let default_value = match &column_config_info.default {
                            Some(default) => default.to_owned(),
                            None => {
                                if column_config_info.not_null {
                                    return Err(ExecuteError::wrap(format!(
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
                                    return Err(ExecuteError::wrap(format!(
                                        "column '{}' type mismatch
                                        ",
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

                // 인덱스 유지보수 준비 (#217)
                self.ensure_indices_loaded().await?;
                let index_metas = self.table_index_metas(into_table).await;

                // 고유 인덱스 사전 검증 (기존 데이터 + 배치 내 중복)
                for meta in index_metas.iter().filter(|meta| meta.is_unique) {
                    let mut batch_keys = HashSet::new();

                    for row in &rows {
                        if let Some(key) = row_index_key(row, &meta.column_name) {
                            let duplicated = !self
                                .index_manager
                                .get(&meta.index_name, &key)
                                .await?
                                .is_empty()
                                || !batch_keys.insert(key);

                            if duplicated {
                                return Err(ExecuteError::wrap(format!(
                                    "duplicate key value violates unique index on column '{}'",
                                    meta.column_name
                                )));
                            }
                        }
                    }
                }

                if let Some(wal_manager) = &wal_manager {
                    let wal_payload = bincode::serialize(&query)
                        .map_err(|error| ExecuteError::wrap(error.to_string()))?;
                    wal_manager
                        .lock()
                        .await
                        .append_record(EntryType::Insert, Some(wal_payload), None)
                        .await?;
                }

                let affected_rows = rows.len();
                let start_index = self.append_table_rows(into_table, &rows).await?;

                // 인덱스 반영 (#217)
                // 안전성: append_table_rows가 row_storage_lock으로 직렬화되므로,
                // start_index는 이 INSERT에 배타적인 범위를 가리킵니다.
                // index_manager.insert는 자체 내부 동기화로 덮어쓰기를 방지합니다.
                for (offset, row) in rows.iter().enumerate() {
                    let row_path = (start_index + offset).to_string();

                    for meta in &index_metas {
                        if let Some(key) = row_index_key(row, &meta.column_name) {
                            self.index_manager
                                .insert(&meta.index_name, key, row_path.clone())
                                .await?;
                        }
                    }
                }

                self.statistics_manager
                    .record_insert(into_table, affected_rows)
                    .await;

                return Ok(ExecuteResult::with_affected_rows(
                    vec![ExecuteColumn {
                        name: "desc".into(),
                        data_type: ExecuteColumnType::String,
                    }],
                    vec![ExecuteRow {
                        fields: vec![ExecuteField::String(format!(
                            "inserted into {}",
                            table_name
                        ))],
                    }],
                    affected_rows,
                ));
            }
            InsertData::Select(_select) => {
                todo!("아직 미구현")
            }
            InsertData::None => {}
        }

        Ok(ExecuteResult::with_affected_rows(
            vec![ExecuteColumn {
                name: "desc".into(),
                data_type: ExecuteColumnType::String,
            }],
            vec![ExecuteRow {
                fields: vec![ExecuteField::String(format!(
                    "inserted into {}",
                    table_name
                ))],
            }],
            0,
        ))
    }
}
