use crate::lib::{ast::dml::expressions::IExpression, UnaryOperator};

#[derive(Clone, Debug)]
pub struct UnaryExpression {
    pub operator: UnaryOperator,
    pub operand: Box<dyn IExpression>,
}

impl IExpression for UnaryExpression {}
