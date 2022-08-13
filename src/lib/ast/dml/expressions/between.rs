use crate::lib::ast::predule::SQLExpression;

// a BETWEEN x AND y
#[derive(Clone, Debug, PartialEq)]
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
