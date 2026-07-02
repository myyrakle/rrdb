use std::collections::{HashMap, HashSet};

use futures::future::join_all;

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
        let wal_payload =
            bson::to_vec(&query).map_err(|error| ExecuteError::wrap(error.to_string()))?;

        let table = query.from_table.as_ref().unwrap().table.clone();

        // 최적화 작업
        let optimizer = Optimizer::new();

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
                        ScanType::IndexScan(_index) => {
                            unimplemented!()
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

        let row_indexes = rows
            .into_iter()
            .map(|(location, _)| location.row_index)
            .collect::<HashSet<_>>();

        let affected_rows = row_indexes.len();

        if !row_indexes.is_empty() {
            wal_manager
                .lock()
                .await
                .append_record(EntryType::Delete, Some(wal_payload), None)
                .await?;
            self.delete_table_rows(&table, row_indexes).await?;
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
