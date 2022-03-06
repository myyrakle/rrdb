use crate::lib::ast::dml::expressions::IExpresstion;

#[derive(Clone, Debug)]
pub struct BooleanExpresstion {
    pub value: bool,
}

impl IExpresstion for BooleanExpresstion {}
