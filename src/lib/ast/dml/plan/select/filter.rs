use crate::lib::ast::predule::SQLExpression;

#[derive(Clone, Debug, PartialEq)]
pub struct FilterPlan {
    expression: SQLExpression,
}
