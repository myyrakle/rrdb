use crate::lib::ast::dml::expressions::IExpresstion;

#[derive(Clone, Debug)]
pub struct BinaryExpresstion {
    pub value: i64,
}

impl IExpresstion for BinaryExpresstion {}
