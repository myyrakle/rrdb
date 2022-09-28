use std::cmp::Ordering;
use std::collections::HashMap;
use std::error::Error;
use std::io::ErrorKind;
use std::path::PathBuf;

use futures::future::join_all;

use crate::lib::ast::dml::{OrderByNulls, OrderByType, SelectItem, SelectKind};
use crate::lib::ast::predule::{
    SQLExpression, ScanType, SelectColumn, SelectPlanItem, SelectQuery, TableName,
};
use crate::lib::errors::predule::ExecuteError;
use crate::lib::errors::type_error::TypeError;
use crate::lib::executor::config::{TableDataField, TableDataFieldType};
use crate::lib::executor::predule::{
    ExecuteColumn, ExecuteField, ExecuteResult, ExecuteRow, Executor, ReduceContext,
    StorageEncoder, TableDataRow,
};
use crate::lib::optimizer::predule::Optimizer;

impl Executor {
    pub async fn select(&self, query: SelectQuery) -> Result<ExecuteResult, Box<dyn Error + Send>> {
        // 최적화 작업
        let optimizer = Optimizer::new();

        let select_items = query.select_items.clone();

        let plan = optimizer.optimize_select(query).await?;

        let mut table_alias_map = HashMap::new();
        let mut table_infos = vec![];

        let mut rows = vec![];

        for each_plan in plan.list {
            match each_plan {
                // Select From 처리
                SelectPlanItem::From(from) => {
                    let table_name = from.table_name.clone();

                    let table_config = self.get_table_config(table_name.clone()).await?;

                    table_infos.push(table_config);

                    if let Some(alias) = from.alias {
                        table_alias_map.insert(alias, table_name.clone());
                    }

                    match from.scan {
                        ScanType::FullScan => {
                            let mut result = self
                                .full_scan(table_name)
                                .await?
                                .into_iter()
                                .map(|(_, e)| e)
                                .collect();

                            rows.append(&mut result);
                        }
                        ScanType::IndexScan(_index) => {
                            unimplemented!()
                        }
                    }
                }
                SelectPlanItem::Filter(filter) => {
                    let futures = rows.iter().cloned().map(|e| {
                        let table_alias_map = table_alias_map.clone();
                        let filter = filter.clone();
                        async move {
                            let reduce_context = ReduceContext {
                                row: Some(e.to_owned()),
                                table_alias_map,
                                config_columns: vec![],
                            };

                            let condition = self
                                .reduce_expression(filter.expression.clone(), reduce_context)
                                .await?;

                            match condition {
                                TableDataFieldType::Boolean(boolean) => Ok((e, boolean)),
                                TableDataFieldType::Null => Ok((e, false)),
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
                        .filter(|(_, boolean)| *boolean)
                        .map(|(e, _)| e)
                        .collect();
                }
                SelectPlanItem::Group(ref group_by_clause) => {
                    let mut grouped_map =
                        HashMap::<Vec<TableDataField>, Vec<TableDataField>>::new();

                    for row in rows {
                        let mut group_key = vec![];
                        let mut group_value = vec![];

                        for field in row.fields {
                            // group by 절에 포함된 컬럼일 경우 키값으로 사용
                            if let Some(_) = group_by_clause.group_by_items.iter().find(|e| {
                                e.item.column_name == field.column_name
                                    && e.item.table_name
                                        == Some(field.table_name.table_name.clone())
                            }) {
                                group_key.push(field);
                            }
                            // 미포함된 컬럼일 경우 배열 형태로 값 중첩
                            else {
                                group_value.push(field);
                            }
                        }

                        match grouped_map.get_mut(&group_key) {
                            Some(value) => {
                                for i in 0..value.len() {
                                    value[i].push(group_value[i].data.clone());
                                }
                            }
                            None => {
                                grouped_map.insert(
                                    group_key,
                                    group_value
                                        .into_iter()
                                        .map(|e| e.to_array())
                                        .collect::<Vec<_>>(),
                                );

                                ()
                            }
                        }
                    }

                    rows = grouped_map
                        .into_iter()
                        .map(|(mut key, mut value)| {
                            key.append(&mut value);
                            TableDataRow { fields: key }
                        })
                        .collect();
                }
                SelectPlanItem::LimitOffset(limit_offset) => {
                    let offset = limit_offset.offset.unwrap_or(0) as usize;

                    match limit_offset.limit {
                        Some(limit) => {
                            rows = rows.drain(offset..(offset + limit as usize)).collect()
                        }
                        None => rows = rows.drain(offset..).collect(),
                    }
                }
                SelectPlanItem::Order(ref order_by_clause) => {
                    let futures = rows.into_iter().map(|e| {
                        let table_alias_map = table_alias_map.clone();

                        async move {
                            let mut order_by_values = vec![];

                            let reduce_context = ReduceContext {
                                row: Some(e.to_owned()),
                                table_alias_map,
                                config_columns: vec![],
                            };

                            for order_by_item in &order_by_clause.order_by_items {
                                let expression = &order_by_item.item;

                                let value = match self
                                    .reduce_expression(expression.clone(), reduce_context.clone())
                                    .await
                                {
                                    Ok(value) => value,
                                    Err(error) => return Err(error),
                                };

                                order_by_values.push(value);
                            }

                            Ok((e, order_by_values))
                        }
                    });

                    let mut order_by_rows = join_all(futures)
                        .await
                        .into_iter()
                        .collect::<Result<Vec<_>, _>>()?;

                    let order_by_items = &order_by_clause.order_by_items;

                    order_by_rows.sort_by(|(_, l), (_, r)| {
                        for (i, order_by_item) in order_by_items.iter().enumerate() {
                            let lhs = &l[i];
                            let rhs = &r[i];

                            if lhs.is_null() && rhs.is_null() {
                                continue;
                            }

                            if lhs.is_null() {
                                match order_by_item.nulls {
                                    OrderByNulls::First => {
                                        return Ordering::Less;
                                    }
                                    OrderByNulls::Last => {
                                        return Ordering::Greater;
                                    }
                                }
                            }

                            if rhs.is_null() {
                                match order_by_item.nulls {
                                    OrderByNulls::First => {
                                        return Ordering::Greater;
                                    }
                                    OrderByNulls::Last => {
                                        return Ordering::Less;
                                    }
                                }
                            }

                            match order_by_item.order_type {
                                OrderByType::Asc => {
                                    if lhs < rhs {
                                        return Ordering::Less;
                                    } else if lhs > rhs {
                                        return Ordering::Greater;
                                    } else {
                                        continue;
                                    }
                                }
                                OrderByType::Desc => {
                                    if lhs < rhs {
                                        return Ordering::Greater;
                                    } else if lhs > rhs {
                                        return Ordering::Less;
                                    } else {
                                        continue;
                                    }
                                }
                            }
                        }

                        Ordering::Equal
                    });

                    rows = order_by_rows.into_iter().map(|(e, _)| e).collect();
                }
                _ => unimplemented!("미구현"),
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

        // *까지 전부 유효한 항목으로 펼침
        let select_items = select_items
            .into_iter()
            .flat_map(|e| match e {
                SelectKind::SelectItem(item) => vec![item],
                SelectKind::WildCard(wildcard) => match wildcard.alias {
                    // a.* 와 같은 형태일 경우
                    Some(alias) => match table_alias_map.get(&alias) {
                        Some(found_table_name) => config_columns
                            .iter()
                            .filter(|(table_name, _)| {
                                found_table_name.table_name == table_name.table_name
                            })
                            .map(|(table_name, column_name)| {
                                SelectItem::builder()
                                    .set_item(
                                        SelectColumn::new(
                                            Some(table_name.table_name.clone()),
                                            column_name.name.clone(),
                                        )
                                        .into(),
                                    )
                                    .build()
                            })
                            .collect(),
                        None => config_columns
                            .iter()
                            .filter(|(table_name, _)| alias == table_name.table_name)
                            .map(|(table_name, column_name)| {
                                SelectItem::builder()
                                    .set_item(
                                        SelectColumn::new(
                                            Some(table_name.table_name.clone()),
                                            column_name.name.clone(),
                                        )
                                        .into(),
                                    )
                                    .build()
                            })
                            .collect(),
                    },
                    None => config_columns
                        .iter()
                        .map(|(table_name, column_name)| {
                            SelectItem::builder()
                                .set_item(
                                    SelectColumn::new(
                                        Some(table_name.table_name.clone()),
                                        column_name.name.clone(),
                                    )
                                    .into(),
                                )
                                .build()
                        })
                        .collect(),
                },
            })
            .collect::<Vec<_>>();

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
                                                Some(decoded) => Ok((path.to_path_buf(), decoded)),
                                                None => Err(ExecuteError::boxed(format!(
                                                    "full scan failed {:?}",
                                                    path
                                                ))),
                                            }
                                        }
                                        Err(error) => Err(ExecuteError::boxed(format!(
                                            "full scan failed {}",
                                            error
                                        ))),
                                    }
                                } else {
                                    Err(ExecuteError::boxed("full scan failed"))
                                }
                            }
                            Err(error) => {
                                Err(ExecuteError::boxed(format!("full scan failed {}", error)))
                            }
                        },
                        Err(error) => {
                            Err(ExecuteError::boxed(format!("full scan failed {}", error)))
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
