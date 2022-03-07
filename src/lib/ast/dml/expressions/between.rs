use crate::lib::ast::dml::expressions::{BinaryOperator, IExpression};

// a BETWEEN x AND y
#[derive(Clone, Debug)]
pub struct BetweenExpression {
    pub a: Box<dyn IExpression>,
    pub x: Box<dyn IExpression>,
    pub y: Box<dyn IExpression>,
}

impl IExpression for BetweenExpression {}
