use crate::lib::ast::predule::SQLExpression;

// a NOT BETWEEN x AND y
#[derive(Clone, Debug, PartialEq)]
pub struct NotBetweenExpression {
    pub a: SQLExpression,
    pub x: SQLExpression,
    pub y: SQLExpression,
}
