use crate::engine::ast::{dml::select::SelectQuery, types::SQLExpression};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub enum SubqueryExpression {
    Select(Box<SelectQuery>),
}

impl Default for SubqueryExpression {
    fn default() -> Self {
        SubqueryExpression::Select(Box::new(SelectQuery::builder()))
    }
}

impl From<SubqueryExpression> for SQLExpression {
    fn from(value: SubqueryExpression) -> SQLExpression {
        SQLExpression::Subquery(value)
    }
}
