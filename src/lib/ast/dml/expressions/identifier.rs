use crate::lib::ast::dml::expressions::IExpression;

#[derive(Clone, Debug)]
pub struct IdentifierExpression {
    pub idendifier: String,
}

impl IExpression for IdentifierExpression {}
