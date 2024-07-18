use crate::ast::types::SQLExpression;
use serde::{Deserialize, Serialize};

use super::operators::UnaryOperator;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct UnaryOperatorExpression {
    pub operator: UnaryOperator,
    pub operand: SQLExpression,
}

impl From<UnaryOperatorExpression> for SQLExpression {
    fn from(value: UnaryOperatorExpression) -> SQLExpression {
        SQLExpression::Unary(Box::new(value))
    }
}

impl From<UnaryOperatorExpression> for Option<Box<SQLExpression>> {
    fn from(value: UnaryOperatorExpression) -> Option<Box<SQLExpression>> {
        Some(Box::new(SQLExpression::Unary(Box::new(value))))
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::UnaryOperatorExpression;
    use crate::ast::dml::expressions::operators::UnaryOperator;
    use crate::ast::types::SQLExpression;

    #[test]
    fn test_From_UnaryOperatorExpression_for_SQLExpression() {
        let unary = UnaryOperatorExpression {
            operator: UnaryOperator::Neg,
            operand: SQLExpression::Integer(1),
        };
        let sql_expression: SQLExpression = unary.into();
        assert_eq!(
            sql_expression,
            SQLExpression::Unary(Box::new(UnaryOperatorExpression {
                operator: UnaryOperator::Neg,
                operand: SQLExpression::Integer(1),
            }))
        );
    }

    #[test]
    fn test_From_UnaryOperatorExpression_for_Option_Box_SQLExpression() {
        let unary = UnaryOperatorExpression {
            operator: UnaryOperator::Neg,
            operand: SQLExpression::Integer(1),
        };
        let sql_expression: Option<Box<SQLExpression>> = unary.into();

        assert_eq!(
            sql_expression,
            Some(Box::new(SQLExpression::Unary(Box::new(
                UnaryOperatorExpression {
                    operator: UnaryOperator::Neg,
                    operand: SQLExpression::Integer(1),
                }
            ))))
        );
    }
}
