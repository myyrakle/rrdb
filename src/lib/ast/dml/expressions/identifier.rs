use crate::lib::ast::dml::expressions::{BinaryOperator, IExpression};

#[derive(Clone, Debug)]
pub struct IdentifierExpression {
    pub idendifier: String,
}

impl IExpression for IdentifierExpression {}
