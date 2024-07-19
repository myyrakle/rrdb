use std::ops::Not;

use crate::ast::types::SQLExpression;
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

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use crate::ast::types::SQLExpression;

    #[test]
    fn test_From_Box_BetweenExpression_for_SQLExpression() {
        use super::BetweenExpression;
        let between = BetweenExpression {
            a: SQLExpression::String("a".into()),
            x: SQLExpression::String("x".into()),
            y: SQLExpression::String("y".into()),
        };
        let sql_expression: SQLExpression = Box::new(between).into();
        assert_eq!(
            sql_expression,
            SQLExpression::Between(Box::new(BetweenExpression {
                a: SQLExpression::String("a".into()),
                x: SQLExpression::String("x".into()),
                y: SQLExpression::String("y".into())
            }))
        );
    }

    #[test]
    fn test_From_BetweenExpression_for_Option_Box_SQLExpression() {
        use super::BetweenExpression;
        let between = BetweenExpression {
            a: SQLExpression::String("a".into()),
            x: SQLExpression::String("x".into()),
            y: SQLExpression::String("y".into()),
        };
        let sql_expression: Option<Box<SQLExpression>> = between.into();
        assert_eq!(
            sql_expression,
            Some(Box::new(SQLExpression::Between(Box::new(
                BetweenExpression {
                    a: SQLExpression::String("a".into()),
                    x: SQLExpression::String("x".into()),
                    y: SQLExpression::String("y".into())
                }
            ))))
        );
    }

    #[test]
    fn test_Not_for_BetweenExpression() {
        use super::BetweenExpression;
        let between = BetweenExpression {
            a: SQLExpression::String("a".into()),
            x: SQLExpression::String("x".into()),
            y: SQLExpression::String("y".into()),
        };
        let not_between = !between;
        assert_eq!(
            not_between,
            super::NotBetweenExpression {
                a: SQLExpression::String("a".into()),
                x: SQLExpression::String("x".into()),
                y: SQLExpression::String("y".into())
            }
        );
    }
}
