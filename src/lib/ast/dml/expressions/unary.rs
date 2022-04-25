use crate::lib::ast::dml::expressions::UnaryOperator;
use crate::lib::ast::enums::SQLExpression;

#[derive(Clone, Debug)]
pub struct UnaryOperatorExpression {
    pub operator: UnaryOperator,
    pub operand: SQLExpression,
}
