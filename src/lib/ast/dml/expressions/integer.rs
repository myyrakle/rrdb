use crate::lib::ast::dml::expressions::IExpresstion;

#[derive(Clone, Debug)]
pub struct IntegerExpresstion {
    pub value: i64,
}

impl IExpresstion for IntegerExpresstion {}
