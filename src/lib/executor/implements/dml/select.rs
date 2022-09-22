use std::collections::HashMap;
use std::error::Error;
use std::io::ErrorKind;
use std::path::PathBuf;

use futures::future::join_all;

use crate::lib::ast::dml::{SelectPlanItem, SelectScanType};
use crate::lib::ast::predule::{SQLExpression, SelectQuery, TableName};
use crate::lib::errors::execute_error::ExecuteError;
use crate::lib::executor::config::TableDataRow;
use crate::lib::executor::encoder::StorageEncoder;
use crate::lib::executor::predule::{ExecuteResult, Executor, ReduceContext};
use crate::lib::executor::result::{ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteRow};
use crate::lib::optimizer::predule::Optimizer;

impl Executor {
    pub async fn select(&self, query: SelectQuery) -> Result<ExecuteResult, Box<dyn Error + Send>> {
        // 최적화 작업
        let optimizer = Optimizer::new();

        let select_items = query.select_items.clone();

        let plan = optimizer.optimize(query).await?;

        let mut table_alias_map = HashMap::new();
        let mut table_infos = vec![];

        let mut rows = vec![];

        for each_plan in plan.list {
            match each_plan {
                SelectPlanItem::From(from) => {
                    let table_name = from.table_name.clone();

                    let table_config = self.get_table_config(table_name.clone()).await?;

                    table_infos.push(table_config);

                    if let Some(alias) = from.alias {
                        table_alias_map.insert(alias, table_name.clone());
                    }

                    match from.scan {
                        SelectScanType::FullScan => {
                            let mut result = self
                                .full_scan(table_name)
                                .await?
                                .into_iter()
                                .map(|(_, e)| e)
                                .collect();

                            rows.append(&mut result);
                        }
                        SelectScanType::IndexScan(_index) => {
                            unimplemented!()
                        }
                    }
                }
                _ => unimplemented!("미구현"),
            }
        }

        // 필요한 SELECT Item만 최종 계산
        let rows = rows.into_iter().map(|row| {
            let table_alias_map = table_alias_map.clone();
            let select_items = select_items.clone();
            async move {
                let fields = select_items.iter().map(|select_item| {
                    let table_alias_map = table_alias_map.clone();
                    let row = row.clone();
                    async move {
                        let reduce_context = ReduceContext {
                            row: Some(row.clone()),
                            table_alias_map: table_alias_map.clone(),
                            config_columns: vec![],
                        };

                        let value = self
                            .reduce_expression(
                                select_item.item.as_ref().unwrap().clone(),
                                reduce_context.clone(),
                            )
                            .await;

                        match value {
                            Ok(value) => Ok(ExecuteField::from(value)),
                            Err(error) => Err(error),
                        }
                    }
                });

                let fields = join_all(fields)
                    .await
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>();

                match fields {
                    Ok(fields) => Ok(ExecuteRow { fields }),
                    Err(error) => Err(error),
                }
            }
        });

        let rows = join_all(rows)
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>();

        let config_columns = table_infos
            .into_iter()
            .map(|table_info| {
                table_info
                    .columns
                    .iter()
                    .cloned()
                    .map(|column| (table_info.table.to_owned(), column.to_owned()))
                    .collect::<Vec<_>>()
            })
            .flatten()
            .collect::<Vec<_>>();

        let reduce_context = ReduceContext {
            row: None,
            table_alias_map,
            config_columns,
        };

        let columns = select_items
            .into_iter()
            .map(|e| {
                let item = e.item.unwrap();

                let name = match e.alias {
                    Some(alias) => alias,
                    None => match &item {
                        SQLExpression::SelectColumn(column) => column.column_name.to_owned(),
                        _ => "?column?".into(),
                    },
                };
                let data_type = self.reduce_type(item, reduce_context.clone())?;

                Ok(ExecuteColumn { name, data_type })
            })
            .collect::<Result<Vec<_>, _>>()?;

        match rows {
            Ok(rows) => Ok(ExecuteResult { columns, rows }),
            Err(error) => Err(error),
        }
    }

    pub async fn full_scan(
        &self,
        table_name: TableName,
    ) -> Result<Vec<(PathBuf, TableDataRow)>, Box<dyn Error + Send>> {
        let encoder = StorageEncoder::new();

        let database_name = table_name.database_name.unwrap();
        let table_name = table_name.table_name;

        let base_path = self.get_base_path();

        let database_path = base_path.clone().join(&database_name);

        let table_path = database_path.clone().join(&table_name);

        // 데이터 행 파일 경로
        let rows_path = table_path.clone().join("rows");

        match std::fs::read_dir(&rows_path) {
            Ok(read_dir_result) => {
                let futures = read_dir_result.map(|e| async {
                    match e {
                        Ok(entry) => match entry.file_type() {
                            Ok(file_type) => {
                                if file_type.is_file() {
                                    let path = entry.path();

                                    match tokio::fs::read(&path).await {
                                        Ok(result) => {
                                            match encoder.decode::<TableDataRow>(result.as_slice())
                                            {
                                                Some(decoded) => {
                                                    return Ok((path.to_path_buf(), decoded))
                                                }
                                                None => {
                                                    return Err(ExecuteError::boxed(format!(
                                                        "full scan failed {:?}",
                                                        path
                                                    )))
                                                }
                                            }
                                        }
                                        Err(error) => {
                                            return Err(ExecuteError::boxed(format!(
                                                "full scan failed {}",
                                                error.to_string()
                                            )))
                                        }
                                    }
                                } else {
                                    return Err(ExecuteError::boxed(format!("full scan failed")));
                                }
                            }
                            Err(error) => {
                                return Err(ExecuteError::boxed(format!(
                                    "full scan failed {}",
                                    error
                                )))
                            }
                        },
                        Err(error) => {
                            return Err(ExecuteError::boxed(format!("full scan failed {}", error)))
                        }
                    }
                });

                let rows = join_all(futures)
                    .await
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>();

                match rows {
                    Ok(rows) => Ok(rows),
                    Err(error) => Err(ExecuteError::boxed(error.to_string())),
                }
            }
            Err(error) => match error.kind() {
                ErrorKind::NotFound => Err(ExecuteError::boxed("base path not exists")),
                _ => Err(ExecuteError::boxed("full scan failed")),
            },
        }
    }

    pub async fn filter(&self) {}

    pub async fn index_scan(&self, _table_name: TableName) {}

    pub async fn order_by(&self) {}

    pub async fn inner_join(&self) {}
}
