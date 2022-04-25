use crate::lib::ast::dml::expressions::{
    BetweenExpression, BinaryOperatorExpression, UnaryOperatorExpression,
};
use crate::lib::ast::types::ColumnName;

#[derive(Clone, Debug, PartialEq)]
pub enum SQLExpression {
    // 복합 표현식
    Unary(Box<UnaryOperatorExpression>),   // 단항 연산식
    Binary(Box<BinaryOperatorExpression>), // 2항 연산식
    Between(Box<BetweenExpression>),       // BETWEEN 식

    // 끝단 Primitive 값
    ColumnName(ColumnName),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(String),
    Null,
}
