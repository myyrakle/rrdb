use crate::lib::ast::dml::expressions::IExpression;

#[derive(Clone, Debug)]
pub struct IntegerExpression {
    pub value: i64,
}

impl IExpression for IntegerExpression {}
