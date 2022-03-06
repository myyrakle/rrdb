use crate::lib::ast::dml::expressions::IExpresstion;

#[derive(Clone, Debug)]
pub struct FloatExpresstion {
    pub value: f64,
}

impl IExpresstion for FloatExpresstion {}
