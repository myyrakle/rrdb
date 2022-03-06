use crate::lib::ast::dml::expressions::IExpression;

#[derive(Clone, Debug)]
pub struct FloatExpression {
    pub value: f64,
}

impl IExpression for FloatExpression {}
