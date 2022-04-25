use crate::lib::ast::enums::SQLExpression;

#[derive(Clone, Debug, PartialEq)]
pub struct CallExpression {
    pub function_name: String,
    pub arguments: Vec<SQLExpression>,
}
