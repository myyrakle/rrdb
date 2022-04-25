use crate::lib::ast::dml::expressions::BinaryOperator;
use crate::lib::ast::enums::SQLExpression;

#[derive(Clone, Debug)]
pub struct BinaryOperatorExpression {
    pub operator: BinaryOperator,
    pub lhs: SQLExpression,
    pub rhs: SQLExpression,
}
