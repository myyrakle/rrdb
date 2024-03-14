use crate::ast::predule::{SQLExpression, SelectQuery};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub enum SubqueryExpression {
    Select(Box<SelectQuery>),
}

impl From<SubqueryExpression> for SQLExpression {
    fn from(value: SubqueryExpression) -> SQLExpression {
        SQLExpression::Subquery(value)
    }
}
