use crate::ast::predule::SQLExpression;

#[derive(Clone, Debug, PartialEq)]
pub struct FilterPlan {
    pub expression: SQLExpression,
}
