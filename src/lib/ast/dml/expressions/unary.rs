use crate::lib::ast::predule::{SQLExpression, UnaryOperator};

#[derive(Clone, Debug, PartialEq)]
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
