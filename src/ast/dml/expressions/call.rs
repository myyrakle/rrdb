use crate::ast::predule::{Function, SQLExpression};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct CallExpression {
    pub function: Function,
    pub arguments: Vec<SQLExpression>,
}

impl From<CallExpression> for SQLExpression {
    fn from(value: CallExpression) -> SQLExpression {
        SQLExpression::FunctionCall(value)
    }
}
