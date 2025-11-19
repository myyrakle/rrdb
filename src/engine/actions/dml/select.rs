use std::cmp::Ordering;
use std::collections::HashMap;

use futures::future::join_all;

use crate::engine::DBEngine;
use crate::engine::ast::dml::parts::order_by::{OrderByNulls, OrderByType};
use crate::engine::ast::dml::parts::select_item::{SelectItem, SelectKind};
use crate::engine::ast::dml::plan::select::scan::ScanType;
use crate::engine::ast::dml::plan::select::select_plan::SelectPlanItem;
use crate::engine::ast::dml::select::SelectQuery;
use crate::engine::ast::types::{SQLExpression, SelectColumn, TableName};
use crate::engine::expression::ReduceContext;
use crate::engine::optimizer::predule::Optimizer;
use crate::engine::schema::row::{TableDataField, TableDataFieldType, TableDataRow};
use crate::engine::types::{ExecuteColumn, ExecuteField, ExecuteResult, ExecuteRow};
use crate::errors::RRDBError;
use crate::errors::type_error::TypeError;

impl DBEngine {
    pub async fn select(&self, query: SelectQuery) -> Result<ExecuteResult, RRDBError> {
        // 최적화 작업
        let optimizer = Optimizer::new();

        let select_items = query.select_items.clone();

        let plan = optimizer.optimize_select(query).await?;

        let mut table_alias_map = HashMap::new();
        let mut table_alias_reverse_map = HashMap::new();
        let mut table_infos = vec![];

        let mut rows = vec![];

        let mut no_from_clause = true;

        for each_plan in plan.list {
            match each_plan {
                // Select From 처리
                SelectPlanItem::From(from) => {
                    no_from_clause = false;

                    let table_name = from.table_name.clone();

                    let table_config = self.get_table_config(table_name.clone()).await?;

                    table_infos.push(table_config);

                    if let Some(alias) = from.alias {
                        table_alias_map.insert(alias.clone(), table_name.clone());
                        table_alias_reverse_map.insert(table_name.clone().table_name, alias);
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
                                total_count: 0,
                            };

                            let condition = self
                                .reduce_expression(filter.expression.clone(), reduce_context)
                                .await?;

                            match condition {
                                TableDataFieldType::Boolean(boolean) => Ok((e, boolean)),
                                TableDataFieldType::Null => Ok((e, false)),
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
                            if let Some(_found) = group_by_clause.group_by_items.iter().find(|e| {
                                let mut table_name_matched = false;

                                if let Some(table_name) = &e.item.table_name {
                                    if table_name == &field.table_name.table_name {
                                        table_name_matched = true;
                                    } else if let Some(table_name) = table_alias_map.get(table_name)
                                    {
                                        if table_name.table_name == field.table_name.table_name {
                                            table_name_matched = true;
                                        }
                                    } else if let Some(table_name) =
                                        table_alias_reverse_map.get(table_name)
                                        && table_name == &field.table_name.table_name
                                    {
                                        table_name_matched = true;
                                    }
                                } else {
                                    table_name_matched = true;
                                }

                                e.item.column_name == field.column_name && table_name_matched
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
                SelectPlanItem::GroupAll => {
                    let mut fields = vec![];

                    for row in rows {
                        if fields.is_empty() {
                            for field in row.fields {
                                fields.push(field.to_array());
                            }
                        } else {
                            for (i, field) in row.fields.into_iter().enumerate() {
                                fields[i].push(field.data)
                            }
                        }
                    }

                    rows = vec![TableDataRow { fields }];
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
                                total_count: 0,
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

        if no_from_clause {
            rows.push(TableDataRow {
                fields: vec![TableDataField {
                    table_name: TableName::new(None, "no_table".into()),
                    column_name: "result".into(),
                    data: TableDataFieldType::Null,
                }],
            });
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
        let total_count = rows.len();
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
                            total_count,
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
            total_count: 0,
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

    pub async fn filter(&self) {}

    pub async fn order_by(&self) {}

    pub async fn inner_join(&self) {}
}
