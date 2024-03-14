use crate::ast::predule::SQLExpression;

use serde::{Deserialize, Serialize};

// a NOT BETWEEN x AND y
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct NotBetweenExpression {
    pub a: SQLExpression,
    pub x: SQLExpression,
    pub y: SQLExpression,
}

impl From<NotBetweenExpression> for SQLExpression {
    fn from(value: NotBetweenExpression) -> SQLExpression {
        SQLExpression::NotBetween(Box::new(value))
    }
}

impl From<Box<NotBetweenExpression>> for SQLExpression {
    fn from(value: Box<NotBetweenExpression>) -> SQLExpression {
        SQLExpression::NotBetween(value)
    }
}
