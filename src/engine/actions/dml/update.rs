use std::collections::HashMap;

use futures::future::join_all;

use crate::engine::DBEngine;
use crate::engine::ast::dml::plan::select::scan::ScanType;
use crate::engine::ast::dml::plan::update::update_plan::UpdatePlanItem;
use crate::engine::ast::dml::update::UpdateQuery;
use crate::engine::encoder::schema_encoder::StorageEncoder;
use crate::engine::expression::ReduceContext;
use crate::engine::optimizer::predule::Optimizer;
use crate::engine::schema::row::TableDataFieldType;
use crate::engine::types::{
    ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow,
};
use crate::errors;
use crate::errors::type_error::TypeError;
use crate::errors::execute_error::ExecuteError;

impl DBEngine {
    pub async fn update(&self, query: UpdateQuery) -> errors::Result<ExecuteResult> {
        let encoder = StorageEncoder::new();

        let table = query.target_table.clone().unwrap().table;
        let update_items = query.update_items.clone();

        // 최적화 작업
        let optimizer = Optimizer::new();

        let plan = optimizer.optimize_update(query).await?;

        let mut table_alias_map = HashMap::new();
        let mut table_infos = vec![];

        let mut rows = vec![];

        for each_plan in plan.list {
            match each_plan {
                // Select From 처리
                UpdatePlanItem::UpdateFrom(from) => {
                    let table_name = from.table_name.clone();

                    let table_config = self.get_table_config(table_name.clone()).await?;

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
        for (path, mut row) in rows.into_iter() {
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

            if let Err(error) = tokio::fs::write(&path, encoder.encode(row)).await {
                return Err(ExecuteError::wrap(format!(
                    "path '{:?}' write failed: {}",
                    path, error
                )));
            }
        }

        Ok(ExecuteResult {
            columns: (vec![ExecuteColumn {
                name: "desc".into(),
                data_type: ExecuteColumnType::String,
            }]),
            rows: (vec![ExecuteRow {
                fields: vec![ExecuteField::String(format!(
                    "updated from {:?}",
                    table.table_name
                ))],
            }]),
        })
    }
}
