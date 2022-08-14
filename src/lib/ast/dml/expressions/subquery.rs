use crate::lib::ast::predule::{SQLExpression, SelectQuery};

#[derive(Clone, Debug, PartialEq)]
pub enum SubqueryExpression {
    Select(Box<SelectQuery>),
}

impl From<SubqueryExpression> for SQLExpression {
    fn from(value: SubqueryExpression) -> SQLExpression {
        SQLExpression::Subquery(value)
    }
}
