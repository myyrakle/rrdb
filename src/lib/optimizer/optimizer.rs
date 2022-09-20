use std::error::Error;

use crate::lib::ast::{
    dml::{FromTarget, SelectFromPlan},
    predule::{SelectPlan, SelectQuery},
};

pub struct Optimizer {}

impl Optimizer {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn optimize(&self, query: SelectQuery) -> Result<SelectPlan, Box<dyn Error>> {
        let mut plan = SelectPlan { list: vec![] };

        // FROM 절 분석
        if let Some(from_clause) = query.from_table {
            let alias = from_clause.alias;

            match from_clause.from {
                FromTarget::Table(table_name) => plan.list.push(SelectFromPlan {
                    table_name,
                    alias,
                    index: None,
                    select_columns: query.select_items.iter().filter(|e|e.).cloned().collect()
                }.into()),
                FromTarget::Subquery(_subquery) => {}
            }

            // WHERE 절 필터링 구성
            if let Some(where_clause) = query.where_clause {
                // TODO
            }
        }

        // JOIN 절 구성
        if !query.join_clause.is_empty() {
            // TODO
        }

        // GROUP BY 절 구성
        if let Some(group_by_clause)= query.group_by_clause {
            // TODO

            // HAVING 절 구성
            if let Some(having_clause)= query.having_clause {
                // TODO
            }
        }

        // ORDER BY 절 구성
        if let Some(order_by_clause)= query.order_by_clause {
            // TODO
        }

        // LIMIT OFFSET 절 구성
        if query.limit.is_some() || query.offset.is_some() {
            // TODO
        }

        Ok(plan)
    }
}

impl Default for Optimizer {
    fn default() -> Self {
        Self::new()
    }
}
