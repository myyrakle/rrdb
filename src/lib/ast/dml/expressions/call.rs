use crate::lib::ast::predule::{FunctionName, SQLExpression};

#[derive(Clone, Debug, PartialEq)]
pub struct CallExpression {
    pub function_name: FunctionName,
    pub arguments: Vec<SQLExpression>,
}

impl Into<SQLExpression> for CallExpression {
    fn into(self) -> SQLExpression {
        SQLExpression::FunctionCall(self)
    }
}
