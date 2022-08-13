use crate::lib::ast::predule::{FunctionName, SQLExpression};

#[derive(Clone, Debug, PartialEq)]
pub struct CallExpression {
    pub function_name: FunctionName,
    pub arguments: Vec<SQLExpression>,
}

impl From<CallExpression> for SQLExpression {
    fn from(value: CallExpression) -> SQLExpression {
        SQLExpression::FunctionCall(value)
    }
}
