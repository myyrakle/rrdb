use crate::lib::ast::predule::{
    BetweenExpression, BinaryOperatorExpression, CallExpression, ListExpression,
    NotBetweenExpression, ParenthesesExpression, SelectColumn, SubqueryExpression,
    UnaryOperatorExpression, WhereClause,
};
use crate::lib::utils::collection::join_vec;

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

    pub fn get_select_column_list(&self) -> Vec<SelectColumn> {
        Self::get_select_column_list_recursion(self)
    }

    fn get_select_column_list_recursion(expression: &Self) -> Vec<SelectColumn> {
        match expression {
            SQLExpression::Integer(_)
            | SQLExpression::Float(_)
            | SQLExpression::Boolean(_)
            | SQLExpression::String(_)
            | SQLExpression::Null => {
                vec![]
            }
            SQLExpression::List(list) => list
                .value
                .iter()
                .map(|e| Self::get_select_column_list_recursion(e))
                .flatten()
                .collect(),
            SQLExpression::SelectColumn(column) => {
                vec![column.to_owned()]
            }
            SQLExpression::Unary(unary) => Self::get_select_column_list(&unary.operand),
            SQLExpression::Binary(binary) => join_vec!(
                Self::get_select_column_list(&binary.lhs),
                Self::get_select_column_list(&binary.rhs)
            ),
            SQLExpression::Between(between) => join_vec!(
                Self::get_select_column_list(&between.a),
                Self::get_select_column_list(&between.x),
                Self::get_select_column_list(&between.y)
            ),
            SQLExpression::NotBetween(between) => join_vec!(
                Self::get_select_column_list(&between.a),
                Self::get_select_column_list(&between.x),
                Self::get_select_column_list(&between.y)
            ),
            SQLExpression::SelectColumn(column) => {
                vec![column.to_owned()]
            }
            SQLExpression::Parentheses(paren) => Self::get_select_column_list(&paren.expression),
            SQLExpression::FunctionCall(function_call) => function_call
                .arguments
                .iter()
                .map(|e| Self::get_select_column_list_recursion(e))
                .flatten()
                .collect(),
            SQLExpression::Subquery(_subquery) => unimplemented!(),
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
