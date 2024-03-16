use crate::ast::types::SQLExpression;

#[derive(Clone, Debug, PartialEq)]
pub struct FilterPlan {
    pub expression: SQLExpression,
}
