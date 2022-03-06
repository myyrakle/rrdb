use crate::lib::ast::dml::expressions::{BinaryOperator, IExpresstion};

#[derive(Clone, Debug)]
pub struct BinaryExpresstion {
    pub operator: BinaryOperator,
    pub lhs: Box<dyn IExpresstion>,
    pub rhs: Box<dyn IExpresstion>,
}

impl IExpresstion for BinaryExpresstion {}
