use crate::lib::ast::predule::{
    BetweenExpression, BinaryOperatorExpression, CallExpression, ColumnName, ParenthesesExpression,
    SelectColumn, UnaryOperatorExpression,
};

#[derive(Clone, Debug, PartialEq)]
pub enum SQLExpression {
    // 복합 표현식
    Unary(Box<UnaryOperatorExpression>),     // 단항 연산식
    Binary(Box<BinaryOperatorExpression>),   // 2항 연산식
    Between(Box<BetweenExpression>),         // BETWEEN 식
    Parentheses(Box<ParenthesesExpression>), // 소괄호 표현식
    FunctionCall(CallExpression),            // 함수호출 표현식

    // 끝단 Primitive 값
    ColumnName(ColumnName),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(String),
    SelectColumn(SelectColumn),
    Null,
}
