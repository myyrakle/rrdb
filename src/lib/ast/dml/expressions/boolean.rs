use crate::lib::ast::dml::expressions::IExpression;

#[derive(Clone, Debug, PartialEq)]
pub struct BooleanExpression {
    pub value: bool,
}

impl IExpression for BooleanExpression {}
