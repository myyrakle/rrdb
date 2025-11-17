use crate::engine::ast::types::SQLExpression;

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

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::NotBetweenExpression;
    use crate::engine::ast::types::SQLExpression;

    #[test]
    fn test_From_Box_NotBetweenExpression_for_SQLExpression() {
        let not_between = NotBetweenExpression {
            a: SQLExpression::String("a".into()),
            x: SQLExpression::String("x".into()),
            y: SQLExpression::String("y".into()),
        };
        let sql_expression: SQLExpression = Box::new(not_between).into();
        assert_eq!(
            sql_expression,
            SQLExpression::NotBetween(Box::new(NotBetweenExpression {
                a: SQLExpression::String("a".into()),
                x: SQLExpression::String("x".into()),
                y: SQLExpression::String("y".into()),
            }))
        );
    }
}
