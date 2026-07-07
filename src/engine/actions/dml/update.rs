use std::collections::HashMap;

use futures::future::join_all;

use crate::engine::actions::index::row_index_key;
use crate::engine::ast::dml::plan::select::scan::ScanType;
use crate::engine::ast::dml::plan::update::update_plan::UpdatePlanItem;
use crate::engine::ast::dml::update::UpdateQuery;
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
    pub async fn update(
        &self,
        query: UpdateQuery,
        wal_manager: SharedWALManager,
    ) -> errors::Result<ExecuteResult> {
        let wal_payload =
            bincode::serialize(&query).map_err(|error| ExecuteError::wrap(error.to_string()))?;

        let table = query.target_table.clone().unwrap().table;
        let update_items = query.update_items.clone();

        // 최적화 작업 (대상 테이블의 인덱스/통계로 컨텍스트 구성)
        let optimizer = Optimizer::with_context(self.build_optimizer_context(&table).await);

        let plan = optimizer.optimize_update(query).await?;

        let mut table_alias_map = HashMap::new();
        let mut table_infos = vec![];

        let mut rows = vec![];

        for each_plan in plan.list {
            match each_plan {
                // Select From 처리
                UpdatePlanItem::UpdateFrom(from) => {
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
                UpdatePlanItem::Filter(filter) => {
                    let futures = rows.iter().cloned().map(|(path, row)| {
                        let table_alias_map = table_alias_map.clone();
                        let filter = filter.clone();
                        async move {
                            let reduce_context = ReduceContext {
                                row: Some(row.to_owned()),
                                table_alias_map,
                                config_columns: vec![],
                                total_count: 0,
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

        let config_columns = table_infos
            .into_iter()
            .flat_map(|table_info| {
                table_info
                    .columns
                    .iter()
                    .cloned()
                    .map(|column| (table_info.table.to_owned(), column))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        // 수정 작업
        self.ensure_indices_loaded().await?;
        let index_metas = self.table_index_metas(&table).await;

        let mut replacements = HashMap::new();
        // 인덱스 반영 목록: (index_name, old_key, new_key, row_path)
        let mut index_operations: Vec<(String, Option<String>, Option<String>, String)> = vec![];

        for (location, mut row) in rows.into_iter() {
            let old_row = row.clone();
            let reduce_context = ReduceContext {
                row: None,
                table_alias_map: table_alias_map.clone(),
                config_columns: config_columns.clone(),
                total_count: 0,
            };

            for update_item in &update_items {
                let column_name = update_item.column.clone();
                let set_value = update_item.value.clone();

                let set_value = self
                    .reduce_expression(set_value, reduce_context.clone())
                    .await?;

                let found = row.fields.iter_mut().find(|e| e.column_name == column_name);

                match found {
                    Some(found) => found.data = set_value,
                    None => {
                        return Err(ExecuteError::wrap(format!(
                            "column '{}' not found in data row",
                            column_name
                        )));
                    }
                }
            }

            // 인덱스 컬럼 값 변경 감지 (#217)
            for meta in &index_metas {
                let old_key = row_index_key(&old_row, &meta.column_name);
                let new_key = row_index_key(&row, &meta.column_name);

                if old_key != new_key {
                    index_operations.push((
                        meta.index_name.clone(),
                        old_key,
                        new_key,
                        location.row_index.to_string(),
                    ));
                }
            }

            replacements.insert(location.row_index, row);
        }

        let affected_rows = replacements.len();

        if !replacements.is_empty() {
            // 인덱스 선반영: 고유 제약 위반은 여기서 검출되며, 실패 시 적용분을 되돌립니다
            let mut applied = 0;

            for (index_name, old_key, new_key, row_path) in &index_operations {
                if let Err(error) = self
                    .apply_index_operation(index_name, old_key, new_key, row_path)
                    .await
                {
                    for (index_name, old_key, new_key, row_path) in
                        index_operations[..applied].iter().rev()
                    {
                        let _ = self
                            .apply_index_operation(index_name, new_key, old_key, row_path)
                            .await;
                    }

                    return Err(error);
                }

                applied += 1;
            }

            // WAL 기록: 실패 시 선반영한 인덱스 변경을 되돌립니다
            if let Err(error) = wal_manager
                .lock()
                .await
                .append_record(EntryType::Set, Some(wal_payload), None)
                .await
            {
                for (index_name, old_key, new_key, row_path) in index_operations.iter().rev() {
                    let _ = self
                        .apply_index_operation(index_name, new_key, old_key, row_path)
                        .await;
                }

                return Err(error);
            }

            if let Err(error) = self.update_table_rows(&table, replacements).await {
                // 저장 실패 시 인덱스 되돌리기
                for (index_name, old_key, new_key, row_path) in index_operations.iter().rev() {
                    let _ = self
                        .apply_index_operation(index_name, new_key, old_key, row_path)
                        .await;
                }

                return Err(error);
            }
        }

        Ok(ExecuteResult::with_affected_rows(
            vec![ExecuteColumn {
                name: "desc".into(),
                data_type: ExecuteColumnType::String,
            }],
            vec![ExecuteRow {
                fields: vec![ExecuteField::String(format!(
                    "updated from {:?}",
                    table.table_name
                ))],
            }],
            affected_rows,
        ))
    }
}
