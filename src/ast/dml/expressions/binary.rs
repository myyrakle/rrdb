use crate::ast::predule::{BinaryOperator, SQLExpression};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct BinaryOperatorExpression {
    pub operator: BinaryOperator,
    pub lhs: SQLExpression,
    pub rhs: SQLExpression,
}

impl From<BinaryOperatorExpression> for SQLExpression {
    fn from(value: BinaryOperatorExpression) -> SQLExpression {
        SQLExpression::Binary(Box::new(value))
    }
}

impl From<Box<BinaryOperatorExpression>> for SQLExpression {
    fn from(value: Box<BinaryOperatorExpression>) -> SQLExpression {
        SQLExpression::Binary(value)
    }
}

impl From<BinaryOperatorExpression> for Option<SQLExpression> {
    fn from(value: BinaryOperatorExpression) -> Option<SQLExpression> {
        Some(SQLExpression::Binary(Box::new(value)))
    }
}

impl From<BinaryOperatorExpression> for Box<SQLExpression> {
    fn from(value: BinaryOperatorExpression) -> Box<SQLExpression> {
        Box::new(SQLExpression::Binary(Box::new(value)))
    }
}

impl From<BinaryOperatorExpression> for Option<Box<SQLExpression>> {
    fn from(value: BinaryOperatorExpression) -> Option<Box<SQLExpression>> {
        Some(Box::new(SQLExpression::Binary(Box::new(value))))
    }
}
