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
                .flat_map(Self::get_select_column_list_recursion)
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
            SQLExpression::Parentheses(paren) => Self::get_select_column_list(&paren.expression),
            SQLExpression::FunctionCall(function_call) => function_call
                .arguments
                .iter()
                .flat_map(Self::get_select_column_list_recursion)
                .collect(),
            SQLExpression::Subquery(_subquery) => unimplemented!(),
        }
    }

    pub fn has_aggregate(&self) -> bool {
        Self::has_aggregate_recursion(self)
    }

    fn has_aggregate_recursion(this: &Self) -> bool {
        match this {
            Self::Unary(unary) => Self::has_aggregate_recursion(&unary.operand),
            Self::Binary(binary) => {
                Self::has_aggregate_recursion(&binary.lhs)
                    | Self::has_aggregate_recursion(&binary.rhs)
            }
            Self::Between(between) => {
                Self::has_aggregate_recursion(&between.a)
                    | Self::has_aggregate_recursion(&between.x)
                    | Self::has_aggregate_recursion(&between.y)
            }
            Self::NotBetween(not_between) => {
                Self::has_aggregate_recursion(&not_between.a)
                    | Self::has_aggregate_recursion(&not_between.x)
                    | Self::has_aggregate_recursion(&not_between.y)
            }
            Self::Parentheses(paren) => Self::has_aggregate_recursion(&paren.expression),
            Self::FunctionCall(call) => call.function.is_aggregate(),
            _ => false,
        }
    }

    pub fn find_non_aggregate_columns(&self) -> Vec<SelectColumn> {
        Self::find_non_aggregate_columns_recursion(self)
    }

    fn find_non_aggregate_columns_recursion(this: &Self) -> Vec<SelectColumn> {
        match this {
            Self::Unary(unary) => Self::find_non_aggregate_columns_recursion(&unary.operand),
            Self::Binary(binary) => join_vec!(
                Self::find_non_aggregate_columns_recursion(&binary.lhs),
                Self::find_non_aggregate_columns_recursion(&binary.rhs)
            ),
            Self::Between(between) => join_vec!(
                Self::find_non_aggregate_columns_recursion(&between.a),
                Self::find_non_aggregate_columns_recursion(&between.x),
                Self::find_non_aggregate_columns_recursion(&between.y)
            ),
            Self::NotBetween(not_between) => join_vec!(
                Self::find_non_aggregate_columns_recursion(&not_between.a),
                Self::find_non_aggregate_columns_recursion(&not_between.x),
                Self::find_non_aggregate_columns_recursion(&not_between.y)
            ),
            Self::Parentheses(paren) => {
                Self::find_non_aggregate_columns_recursion(&paren.expression)
            }
            Self::FunctionCall(call) => {
                if call.function.is_aggregate() {
                    return vec![];
                } else {
                    call.arguments
                        .iter()
                        .cloned()
                        .flat_map(|e| Self::find_non_aggregate_columns_recursion(&e))
                        .collect()
                }
            }
            Self::SelectColumn(column) => vec![column.to_owned()],
            _ => vec![],
        }
    }
}

impl From<SQLExpression> for WhereClause {
    fn from(value: SQLExpression) -> WhereClause {
        WhereClause { expression: value }
    }
}

impl From<SQLExpression> for Option<Box<SQLExpression>> {
    fn from(value: SQLExpression) -> Option<Box<SQLExpression>> {
        Some(Box::new(value))
    }
}
