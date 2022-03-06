use crate::lib::{ast::dml::expressions::IExpresstion, UnaryOperator};

#[derive(Clone, Debug)]
pub struct UnaryExpresstion {
    pub operator: UnaryOperator,
    pub operand: Box<dyn IExpresstion>,
}

impl IExpresstion for UnaryExpresstion {}
