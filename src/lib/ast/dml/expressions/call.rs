use crate::lib::ast::dml::expressions::{BinaryOperator, IExpression};

#[derive(Clone, Debug)]
pub struct CallExpression {
    pub function_name: String,
    pub arguments: Vec<Box<dyn IExpression>>,
}

impl IExpression for CallExpression {}
