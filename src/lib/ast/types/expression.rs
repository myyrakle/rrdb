use crate::lib::ast::predule::{
    BetweenExpression, BinaryOperatorExpression, CallExpression, ListExpression,
    NotBetweenExpression, ParenthesesExpression, SelectColumn, SubqueryExpression,
    UnaryOperatorExpression, WhereClause,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub enum SQLExpression {
    // 복합 표현식
    Unary(Box<UnaryOperatorExpression>),     // 단항 연산식
    Binary(Box<BinaryOperatorExpression>),   // 2항 연산식
    Between(Box<BetweenExpression>),         // BETWEEN 식
    NotBetween(Box<NotBetweenExpression>),   // NOT BETWEEN 식
    Parentheses(Box<ParenthesesExpression>), // 소괄호 표현식
    FunctionCall(CallExpression),            // 함수호출 표현식
    Subquery(SubqueryExpression),            // SQL 서브쿼리 (미구현)

    // 끝단 Primitive 값
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(String),
    List(ListExpression),
    SelectColumn(SelectColumn),
    Null,
}

impl SQLExpression {
    pub fn is_unary(&self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match self.clone() {
            Self::Unary(_) => true,
            _ => false,
        }
    }
}

impl From<SQLExpression> for WhereClause {
    fn from(value: SQLExpression) -> WhereClause {
        WhereClause {
            expression: Some(Box::new(value)),
        }
    }
}

impl From<SQLExpression> for Option<Box<SQLExpression>> {
    fn from(value: SQLExpression) -> Option<Box<SQLExpression>> {
        Some(Box::new(value))
    }
}
