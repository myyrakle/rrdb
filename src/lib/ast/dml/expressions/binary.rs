use crate::lib::ast::predule::{BinaryOperator, SQLExpression};

#[derive(Clone, Debug, PartialEq)]
pub struct BinaryOperatorExpression {
    pub operator: BinaryOperator,
    pub lhs: SQLExpression,
    pub rhs: SQLExpression,
}

impl Into<SQLExpression> for BinaryOperatorExpression {
    fn into(self) -> SQLExpression {
        SQLExpression::Binary(Box::new(self))
    }
}

impl Into<SQLExpression> for Box<BinaryOperatorExpression> {
    fn into(self) -> SQLExpression {
        SQLExpression::Binary(self)
    }
}
