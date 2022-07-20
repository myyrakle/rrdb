use crate::lib::ast::dml::expressions::UnaryOperator;
use crate::lib::ast::enums::SQLExpression;

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
