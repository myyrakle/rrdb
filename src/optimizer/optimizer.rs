use std::error::Error;

use crate::ast::dml::{
    delete::DeleteQuery,
    parts::from::FromTarget,
    plan::{
        delete::{delete_plan::DeletePlan, from::DeleteFromPlan},
        select::{
            filter::FilterPlan,
            from::SelectFromPlan,
            limit_offset::LimitOffsetPlan,
            scan::ScanType,
            select_plan::{SelectPlan, SelectPlanItem},
        },
        update::{from::UpdateFromPlan, update_plan::UpdatePlan},
    },
    select::SelectQuery,
    update::UpdateQuery,
};

pub struct Optimizer {}

impl Optimizer {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn optimize_select(
        &self,
        query: SelectQuery,
    ) -> Result<SelectPlan, Box<dyn Error + Send>> {
        let mut has_from = false;
        let mut plan = SelectPlan { list: vec![] };

        // FROM 절 분석
        if let Some(from_clause) = query.from_table {
            has_from = true;
            let alias = from_clause.alias;

            match from_clause.from {
                FromTarget::Table(table_name) => plan.list.push(
                    SelectFromPlan {
                        table_name,
                        alias,
                        scan: ScanType::FullScan, // TODO: 인덱스 스캔 처리
                    }
                    .into(),
                ),
                FromTarget::Subquery(_subquery) => {}
            }
        }

        if has_from {
            // JOIN 절 구성
            if !query.join_clause.is_empty() {
                // TODO
            }

            // WHERE 절 필터링 구성
            if let Some(where_clause) = query.where_clause {
                let expression = where_clause.expression;

                plan.list.push(FilterPlan { expression }.into());
            }

            // GROUP BY 절 구성
            if let Some(group_by_clause) = query.group_by_clause {
                plan.list.push(group_by_clause.into());

                // HAVING 절 구성
                if let Some(having_clause) = query.having_clause {
                    plan.list.push(
                        FilterPlan {
                            expression: *having_clause.expression,
                        }
                        .into(),
                    );
                }
            } else if query.has_aggregate {
                plan.list.push(SelectPlanItem::GroupAll);
            }

            // ORDER BY 절 구성
            if let Some(order_by_clause) = query.order_by_clause {
                plan.list.push(order_by_clause.into());
            }

            // LIMIT OFFSET 절 구성
            if query.limit.is_some() || query.offset.is_some() {
                plan.list.push(
                    LimitOffsetPlan {
                        limit: query.limit,
                        offset: query.offset,
                    }
                    .into(),
                );
            }
        }

        Ok(plan)
    }

    pub async fn optimize_update(
        &self,
        query: UpdateQuery,
    ) -> Result<UpdatePlan, Box<dyn Error + Send>> {
        let mut plan = UpdatePlan { list: vec![] };

        let target_table = query.target_table.clone().unwrap();

        plan.list.push(
            UpdateFromPlan {
                table_name: target_table.table.clone(),
                alias: target_table.alias,
                scan: ScanType::FullScan, // TODO: 인덱스 스캔 처리
            }
            .into(),
        );

        // WHERE 절 분석
        if let Some(where_clause) = query.where_clause {
            // WHERE 절 필터링 구성

            let expression = where_clause.expression;

            plan.list.push(FilterPlan { expression }.into());
        }

        Ok(plan)
    }

    pub async fn optimize_delete(
        &self,
        query: DeleteQuery,
    ) -> Result<DeletePlan, Box<dyn Error + Send>> {
        let mut plan = DeletePlan { list: vec![] };

        let target_table = query.from_table.clone().unwrap();

        plan.list.push(
            DeleteFromPlan {
                table_name: target_table.table.clone(),
                alias: target_table.alias,
                scan: ScanType::FullScan, // TODO: 인덱스 스캔 처리
            }
            .into(),
        );

        // WHERE 절 분석
        if let Some(where_clause) = query.where_clause {
            // WHERE 절 필터링 구성

            let expression = where_clause.expression;

            plan.list.push(FilterPlan { expression }.into());
        }

        Ok(plan)
    }
}

impl Default for Optimizer {
    fn default() -> Self {
        Self::new()
    }
}
