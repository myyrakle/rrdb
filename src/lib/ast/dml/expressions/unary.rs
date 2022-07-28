use crate::lib::ast::predule::{SQLExpression, UnaryOperator};

#[derive(Clone, Debug, PartialEq)]
pub struct UnaryOperatorExpression {
    pub operator: UnaryOperator,
    pub operand: SQLExpression,
}

impl Into<SQLExpression> for UnaryOperatorExpression {
    fn into(self) -> SQLExpression {
        SQLExpression::Unary(Box::new(self))
    }
}
