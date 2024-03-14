use std::ops::Not;

use crate::ast::predule::SQLExpression;
use serde::{Deserialize, Serialize};

use super::not_between::NotBetweenExpression;

// a BETWEEN x AND y
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct BetweenExpression {
    pub a: SQLExpression,
    pub x: SQLExpression,
    pub y: SQLExpression,
}

impl From<BetweenExpression> for SQLExpression {
    fn from(value: BetweenExpression) -> SQLExpression {
        SQLExpression::Between(Box::new(value))
    }
}

impl From<Box<BetweenExpression>> for SQLExpression {
    fn from(value: Box<BetweenExpression>) -> SQLExpression {
        SQLExpression::Between(value)
    }
}

impl From<BetweenExpression> for Option<Box<SQLExpression>> {
    fn from(value: BetweenExpression) -> Option<Box<SQLExpression>> {
        Some(Box::new(SQLExpression::Between(Box::new(value))))
    }
}

impl Not for BetweenExpression {
    type Output = NotBetweenExpression;

    fn not(self) -> Self::Output {
        NotBetweenExpression {
            a: self.a,
            x: self.x,
            y: self.y,
        }
    }
}
