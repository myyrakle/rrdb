use crate::lib::ast::dml::expressions::IExpression;

#[derive(Clone, Debug, PartialEq)]
pub struct StringExpression {
    pub value: String,
}

impl IExpression for StringExpression {}
