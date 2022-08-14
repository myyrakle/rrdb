use crate::lib::ast::predule::{SQLExpression, SelectQuery};

#[derive(Clone, Debug, PartialEq)]
pub enum SubqueryExpression {
    Select(SelectQuery),
}

impl From<SubqueryExpression> for SQLExpression {
    fn from(value: SubqueryExpression) -> SQLExpression {
        SQLExpression::Subquery(value)
    }
}
