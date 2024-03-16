use crate::ast::types::SQLExpression;
use serde::{Deserialize, Serialize};

use super::operators::UnaryOperator;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct UnaryOperatorExpression {
    pub operator: UnaryOperator,
    pub operand: SQLExpression,
}

impl From<UnaryOperatorExpression> for SQLExpression {
    fn from(value: UnaryOperatorExpression) -> SQLExpression {
        SQLExpression::Unary(Box::new(value))
    }
}

impl From<UnaryOperatorExpression> for Option<Box<SQLExpression>> {
    fn from(value: UnaryOperatorExpression) -> Option<Box<SQLExpression>> {
        Some(Box::new(SQLExpression::Unary(Box::new(value))))
    }
}
