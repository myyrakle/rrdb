use crate::lib::ast::dml::expressions::IExpresstion;

#[derive(Clone, Debug)]
pub struct StringExpresstion {
    pub value: String,
}

impl IExpresstion for StringExpresstion {}
