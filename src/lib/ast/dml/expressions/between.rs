use crate::lib::ast::enums::SQLExpression;

// a BETWEEN x AND y
#[derive(Clone, Debug)]
pub struct BetweenExpression {
    pub a: SQLExpression,
    pub x: SQLExpression,
    pub y: SQLExpression,
}
