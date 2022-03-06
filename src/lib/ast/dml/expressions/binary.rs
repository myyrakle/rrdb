use crate::lib::ast::dml::expressions::{BinaryOperator, IExpression};

#[derive(Clone, Debug)]
pub struct BinaryExpression {
    pub operator: BinaryOperator,
    pub lhs: Box<dyn IExpression>,
    pub rhs: Box<dyn IExpression>,
}

impl IExpression for BinaryExpression {}
