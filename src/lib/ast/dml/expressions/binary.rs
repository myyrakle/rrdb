use crate::lib::ast::dml::expressions::BinaryOperator;
use crate::lib::ast::enums::SQLExpression;

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
