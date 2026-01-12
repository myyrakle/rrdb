use std::collections::HashMap;

use futures::future::join_all;

use crate::engine::DBEngine;
use crate::engine::ast::dml::delete::DeleteQuery;
use crate::engine::ast::dml::plan::delete::delete_plan::DeletePlanItem;
use crate::engine::ast::dml::plan::select::scan::ScanType;
use crate::engine::expression::ReduceContext;
use crate::engine::optimizer::predule::Optimizer;
use crate::engine::schema::row::TableDataFieldType;
use crate::engine::storage::TableHeap;
use crate::engine::types::{
    ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow,
};
use crate::errors;
use crate::errors::execute_error::ExecuteError;
use crate::errors::type_error::TypeError;

impl DBEngine {
    pub async fn delete(&self, query: DeleteQuery) -> errors::Result<ExecuteResult> {
        let table = query.from_table.as_ref().unwrap().table.clone();

        // 최적화 작업
        let optimizer = Optimizer::new();
        let plan = optimizer.optimize_delete(query).await?;

        let mut table_alias_map = HashMap::new();

        let mut rows = vec![];

        for each_plan in plan.list {
            match each_plan {
                // From 처리
                DeletePlanItem::DeleteFrom(from) => {
                    let table_name = from.table_name.clone();

                    let _table_config = self.get_table_config(table_name.clone()).await?;

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
                    let futures = rows.iter().cloned().map(|(row_id, row)| {
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
                                TableDataFieldType::Boolean(boolean) => Ok((row_id, row, boolean)),
                                TableDataFieldType::Null => Ok((row_id, row, false)),
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
                        .map(|(row_id, row, _)| (row_id, row))
                        .collect();
                }
            }
        }
        let mut heaps = self.table_heaps.write().await;
        let heap = heaps.entry(table.clone()).or_insert_with(TableHeap::new);
        for (row_id, _) in rows.into_iter() {
            heap.delete(row_id)
                .map_err(|error| ExecuteError::wrap(format!("{:?}", error)))?;
        }

        Ok(ExecuteResult {
            columns: (vec![ExecuteColumn {
                name: "desc".into(),
                data_type: ExecuteColumnType::String,
            }]),
            rows: (vec![ExecuteRow {
                fields: vec![ExecuteField::String(format!(
                    "deleted from {:?}",
                    table.table_name
                ))],
            }]),
        })
    }
}
