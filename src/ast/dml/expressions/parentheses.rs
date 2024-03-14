use crate::ast::predule::SQLExpression;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct ParenthesesExpression {
    pub expression: SQLExpression,
}

impl From<ParenthesesExpression> for SQLExpression {
    fn from(value: ParenthesesExpression) -> SQLExpression {
        SQLExpression::Parentheses(Box::new(value))
    }
}
