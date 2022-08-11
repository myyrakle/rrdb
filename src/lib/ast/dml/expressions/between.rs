use crate::lib::ast::predule::SQLExpression;

// a BETWEEN x AND y
#[derive(Clone, Debug, PartialEq)]
pub struct BetweenExpression {
    pub a: SQLExpression,
    pub x: SQLExpression,
    pub y: SQLExpression,
}

impl Into<SQLExpression> for BetweenExpression {
    fn into(self) -> SQLExpression {
        SQLExpression::Between(Box::new(self))
    }
}

impl Into<SQLExpression> for Box<BetweenExpression> {
    fn into(self) -> SQLExpression {
        SQLExpression::Between(self)
    }
}

impl Into<Option<Box<SQLExpression>>> for BetweenExpression {
    fn into(self) -> Option<Box<SQLExpression>> {
        Some(Box::new(SQLExpression::Between(Box::new(self))))
    }
}
