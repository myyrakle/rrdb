use std::collections::HashMap;
use std::error::Error;

use futures::future::join_all;

use crate::lib::ast::dml::{DeletePlanItem, DeleteQuery};
use crate::lib::ast::predule::ScanType;
use crate::lib::errors::predule::ExecuteError;
use crate::lib::errors::type_error::TypeError;
use crate::lib::executor::config::TableDataFieldType;
use crate::lib::executor::predule::{
    ExecuteColumn, ExecuteField, ExecuteResult, ExecuteRow, Executor, ReduceContext,
};
use crate::lib::executor::result::ExecuteColumnType;
use crate::lib::optimizer::predule::Optimizer;

impl Executor {
    pub async fn delete(&self, query: DeleteQuery) -> Result<ExecuteResult, Box<dyn Error + Send>> {
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
                DeletePlanItem::Filter(filter) => {
                    let futures = rows.iter().cloned().map(|(path, row)| {
                        let table_alias_map = table_alias_map.clone();
                        let filter = filter.clone();
                        async move {
                            let reduce_context = ReduceContext {
                                row: Some(row.to_owned()),
                                table_alias_map,
                                config_columns: vec![],
                            };

                            let condition = self
                                .reduce_expression(filter.expression.clone(), reduce_context)
                                .await?;

                            match condition {
                                TableDataFieldType::Boolean(boolean) => Ok((path, row, boolean)),
                                TableDataFieldType::Null => Ok((path, row, false)),
                                _ => Err(TypeError::dyn_boxed(
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

        // 삭제 작업
        for (path, _) in rows.into_iter() {
            if let Err(error) = tokio::fs::remove_file(&path).await {
                return Err(ExecuteError::boxed(format!(
                    "file {:?} remove failed: {}",
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
                    "deleted from {:?}",
                    table.table_name
                ))],
            }]),
        })
    }
}
