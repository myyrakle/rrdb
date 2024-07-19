use crate::ast::types::SQLExpression;
use serde::{Deserialize, Serialize};

use super::operators::BinaryOperator;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct BinaryOperatorExpression {
    pub operator: BinaryOperator,
    pub lhs: SQLExpression,
    pub rhs: SQLExpression,
}

impl From<BinaryOperatorExpression> for SQLExpression {
    fn from(value: BinaryOperatorExpression) -> SQLExpression {
        SQLExpression::Binary(Box::new(value))
    }
}

impl From<Box<BinaryOperatorExpression>> for SQLExpression {
    fn from(value: Box<BinaryOperatorExpression>) -> SQLExpression {
        SQLExpression::Binary(value)
    }
}

impl From<BinaryOperatorExpression> for Option<SQLExpression> {
    fn from(value: BinaryOperatorExpression) -> Option<SQLExpression> {
        Some(SQLExpression::Binary(Box::new(value)))
    }
}

impl From<BinaryOperatorExpression> for Box<SQLExpression> {
    fn from(value: BinaryOperatorExpression) -> Box<SQLExpression> {
        Box::new(SQLExpression::Binary(Box::new(value)))
    }
}

impl From<BinaryOperatorExpression> for Option<Box<SQLExpression>> {
    fn from(value: BinaryOperatorExpression) -> Option<Box<SQLExpression>> {
        Some(Box::new(SQLExpression::Binary(Box::new(value))))
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use crate::ast::dml::expressions::operators::BinaryOperator;

    #[test]
    fn test_From_BinaryOperatorExpression_for_Option_Box_SQLExpression() {
        use crate::ast::dml::expressions::binary::BinaryOperatorExpression;
        use crate::ast::types::SQLExpression;
        use std::convert::From;

        let binary_operator_expression = BinaryOperatorExpression {
            operator: BinaryOperator::Add,
            lhs: SQLExpression::Integer(1),
            rhs: SQLExpression::Integer(2),
        };
        let res: Option<Box<SQLExpression>> = From::from(binary_operator_expression.clone());

        assert_eq!(
            res,
            Some(Box::new(SQLExpression::Binary(Box::new(
                binary_operator_expression
            ))))
        );
    }
}
