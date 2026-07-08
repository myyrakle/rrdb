use std::collections::{HashMap, HashSet};

use futures::future::join_all;

use crate::engine::actions::index::row_index_key;
use crate::engine::ast::dml::delete::DeleteQuery;
use crate::engine::ast::dml::plan::delete::delete_plan::DeletePlanItem;
use crate::engine::ast::dml::plan::select::scan::ScanType;
use crate::engine::expression::ReduceContext;
use crate::engine::optimizer::predule::Optimizer;
use crate::engine::schema::row::TableDataFieldType;
use crate::engine::types::{
    ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow,
};
use crate::engine::wal::types::EntryType;
use crate::engine::{DBEngine, SharedWALManager};
use crate::errors;
use crate::errors::execute_error::ExecuteError;
use crate::errors::type_error::TypeError;

impl DBEngine {
    pub async fn delete(
        &self,
        query: DeleteQuery,
        wal_manager: SharedWALManager,
    ) -> errors::Result<ExecuteResult> {
        self.delete_internal(query, Some(wal_manager)).await
    }

    /// Re-applies a previously WAL-logged DELETE during crash recovery
    /// replay. Identical to `delete()` but skips the WAL append (the
    /// operation is already durably recorded in the WAL being replayed).
    pub(crate) async fn delete_replay(&self, query: DeleteQuery) -> errors::Result<ExecuteResult> {
        self.delete_internal(query, None).await
    }

    async fn delete_internal(
        &self,
        query: DeleteQuery,
        wal_manager: Option<SharedWALManager>,
    ) -> errors::Result<ExecuteResult> {
        let table = query.from_table.as_ref().unwrap().table.clone();

        // WAL-first: 쿼리를 실행/소비하기 전에 페이로드를 미리 직렬화합니다.
        let wal_payload = match &wal_manager {
            Some(_) => Some(
                bincode::serialize(&query).map_err(|error| ExecuteError::wrap(error.to_string()))?,
            ),
            None => None,
        };

        // 최적화 작업 (대상 테이블의 인덱스/통계로 컨텍스트 구성)
        let optimizer = Optimizer::with_context(self.build_optimizer_context(&table).await);

        let plan = optimizer.optimize_delete(query).await?;

        let mut table_alias_map = HashMap::new();
        let mut table_infos = vec![];

        let mut rows = vec![];

        for each_plan in plan.list {
            match each_plan {
                // From 처리
                DeletePlanItem::DeleteFrom(from) => {
                    let table_name = from.table_name.clone();

                    let table_config = self.get_table_config_cached(table_name.clone()).await?;

                    table_infos.push(table_config);

                    if let Some(alias) = from.alias {
                        table_alias_map.insert(alias, table_name.clone());
                    }

                    match from.scan {
                        ScanType::FullScan => {
                            let mut result =
                                self.full_scan(table_name).await?.into_iter().collect();

                            rows.append(&mut result);
                        }
                        ScanType::IndexScan(index_scan_plan) => {
                            let mut result = self.index_scan(table_name, &index_scan_plan).await?;

                            rows.append(&mut result);
                        }
                    }
                }
                // 필터링 처리
                DeletePlanItem::Filter(filter) => {
                    let total_count = rows.len();
                    let futures = rows.iter().cloned().map(|(path, row)| {
                        let table_alias_map = table_alias_map.clone();
                        let filter = filter.clone();
                        async move {
                            let reduce_context = ReduceContext {
                                row: Some(row.to_owned()),
                                table_alias_map,
                                config_columns: vec![],
                                total_count,
                            };

                            let condition = self
                                .reduce_expression(filter.expression.clone(), reduce_context)
                                .await?;

                            match condition {
                                TableDataFieldType::Boolean(boolean) => Ok((path, row, boolean)),
                                TableDataFieldType::Null => Ok((path, row, false)),
                                _ => Err(TypeError::wrap(
                                    "condition expression is valid only for boolean and null types",
                                )),
                            }
                        }
                    });

                    let result = join_all(futures)
                        .await
                        .into_iter()
                        .collect::<Result<Vec<_>, _>>()?;

                    rows = result
                        .into_iter()
                        .filter(|(_, _, boolean)| *boolean)
                        .map(|(path, row, _)| (path, row))
                        .collect();
                }
            }
        }

        self.ensure_indices_loaded().await?;
        let index_metas = self.table_index_metas(&table).await;

        let mut row_indexes = HashSet::new();
        // 삭제할 인덱스 항목: (index_name, key, row_path)
        let mut index_removals: Vec<(String, String, String)> = vec![];

        for (location, row) in &rows {
            row_indexes.insert(location.row_index);

            for meta in &index_metas {
                if let Some(key) = row_index_key(row, &meta.column_name) {
                    index_removals.push((
                        meta.index_name.clone(),
                        key,
                        location.row_index.to_string(),
                    ));
                }
            }
        }

        let affected_rows = row_indexes.len();

        if !row_indexes.is_empty() {
            // WAL-first: 먼저 durable하게 기록한 뒤 실제 데이터/인덱스를 변경합니다.
            if let Some(wal_manager) = &wal_manager {
                wal_manager
                    .lock()
                    .await
                    .append_record(EntryType::Delete, wal_payload, None)
                    .await?;
            }

            // 소프트 삭제: row index를 유지한 채 tombstone만 표시합니다 (세그먼트 재작성/압축 없음, #217)
            self.delete_table_rows(&table, row_indexes).await?;

            for (index_name, key, row_path) in &index_removals {
                self.index_manager.remove(index_name, key, row_path).await?;
            }

            self.statistics_manager
                .record_delete(&table, affected_rows)
                .await;
        }

        Ok(ExecuteResult::with_affected_rows(
            vec![ExecuteColumn {
                name: "desc".into(),
                data_type: ExecuteColumnType::String,
            }],
            vec![ExecuteRow {
                fields: vec![ExecuteField::String(format!(
                    "deleted from {:?}",
                    table.table_name
                ))],
            }],
            affected_rows,
        ))
    }
}
