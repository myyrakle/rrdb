use crate::lib::ast::predule::SQLExpression;

// a NOT BETWEEN x AND y
#[derive(Clone, Debug, PartialEq)]
pub struct NotBetweenExpression {
    pub a: SQLExpression,
    pub x: SQLExpression,
    pub y: SQLExpression,
}

impl Into<SQLExpression> for NotBetweenExpression {
    fn into(self) -> SQLExpression {
        SQLExpression::NotBetween(Box::new(self))
    }
}

impl Into<SQLExpression> for Box<NotBetweenExpression> {
    fn into(self) -> SQLExpression {
        SQLExpression::NotBetween(self)
    }
}
