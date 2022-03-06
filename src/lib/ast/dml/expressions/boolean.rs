use crate::lib::ast::dml::expressions::IExpression;

#[derive(Clone, Debug)]
pub struct BooleanExpression {
    pub value: bool,
}

impl IExpression for BooleanExpression {}
