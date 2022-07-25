use crate::lib::ast::predule::{FunctionName, SQLExpression};

#[derive(Clone, Debug, PartialEq)]
pub struct CallExpression {
    pub function_name: FunctionName,
    pub arguments: Vec<SQLExpression>,
}
