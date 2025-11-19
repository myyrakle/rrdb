use crate::engine::ast::dml::expressions::binary::BinaryOperatorExpression;
use crate::engine::ast::dml::expressions::call::CallExpression;
use crate::engine::ast::dml::expressions::list::ListExpression;
use crate::engine::ast::dml::expressions::not_between::NotBetweenExpression;
use crate::engine::ast::dml::expressions::parentheses::ParenthesesExpression;
use crate::engine::ast::dml::expressions::subquery::SubqueryExpression;
use crate::engine::ast::dml::expressions::unary::UnaryOperatorExpression;
use crate::engine::ast::dml::parts::_where::WhereClause;
use crate::engine::{
    ast::dml::expressions::between::BetweenExpression, schema::row::TableDataFieldType,
};
use crate::utils::collection::join_vec;

use serde::{Deserialize, Serialize};

use super::select_column::SelectColumn;

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

impl Default for SQLExpression {
    fn default() -> Self {
        Self::Null
    }
}

impl SQLExpression {
    pub fn is_unary(&self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match self.clone() {
            Self::Unary(_) => true,
            _ => false,
        }
    }

    // Select 절의 표현식 목록에서 실제로 DB에서 가져와야하는 대상 컬럼 목록을 추출합니다.
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
                    vec![]
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

    pub fn find_aggregate_columns(&self) -> Vec<SelectColumn> {
        Self::find_aggregate_columns_recursion(self, Default::default())
    }

    fn find_aggregate_columns_recursion(
        this: &Self,
        mut context: RecursionContext,
    ) -> Vec<SelectColumn> {
        match this {
            Self::Unary(unary) => Self::find_aggregate_columns_recursion(&unary.operand, context),
            Self::Binary(binary) => join_vec!(
                Self::find_aggregate_columns_recursion(&binary.lhs, context),
                Self::find_aggregate_columns_recursion(&binary.rhs, context)
            ),
            Self::Between(between) => join_vec!(
                Self::find_aggregate_columns_recursion(&between.a, context),
                Self::find_aggregate_columns_recursion(&between.x, context),
                Self::find_aggregate_columns_recursion(&between.y, context)
            ),
            Self::NotBetween(not_between) => join_vec!(
                Self::find_aggregate_columns_recursion(&not_between.a, context),
                Self::find_aggregate_columns_recursion(&not_between.x, context),
                Self::find_aggregate_columns_recursion(&not_between.y, context)
            ),
            Self::Parentheses(paren) => {
                Self::find_aggregate_columns_recursion(&paren.expression, context)
            }
            Self::FunctionCall(call) => {
                if call.function.is_aggregate() {
                    context.in_aggregate = true;
                    call.arguments
                        .iter()
                        .cloned()
                        .flat_map(|e| Self::find_aggregate_columns_recursion(&e, context))
                        .collect()
                } else {
                    vec![]
                }
            }
            Self::SelectColumn(column) => {
                if context.in_aggregate {
                    vec![column.to_owned()]
                } else {
                    vec![]
                }
            }
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

impl From<TableDataFieldType> for SQLExpression {
    fn from(value: TableDataFieldType) -> SQLExpression {
        match value {
            TableDataFieldType::Integer(value) => SQLExpression::Integer(value),
            TableDataFieldType::Float(value) => SQLExpression::Float(value.into()),
            TableDataFieldType::Boolean(value) => SQLExpression::Boolean(value),
            TableDataFieldType::String(value) => SQLExpression::String(value),
            TableDataFieldType::Null => SQLExpression::Null,
            TableDataFieldType::Array(value) => SQLExpression::List(
                value
                    .into_iter()
                    .map(|e| e.into())
                    .collect::<Vec<_>>()
                    .into(),
            ),
        }
    }
}

#[derive(Debug, Clone, Default, Copy)]
struct RecursionContext {
    pub in_aggregate: bool,
}

#[allow(non_snake_case)]
#[cfg(test)]
mod tests {
    use crate::engine::{
        ast::{
            dml::expressions::{
                between::BetweenExpression,
                binary::BinaryOperatorExpression,
                call::CallExpression,
                not_between::NotBetweenExpression,
                operators::{BinaryOperator, UnaryOperator},
                parentheses::ParenthesesExpression,
                unary::UnaryOperatorExpression,
            },
            types::{
                AggregateFunction, ConditionalFunction, Function, SQLExpression, SelectColumn,
            },
        },
        schema::row::TableDataFieldType,
    };

    #[test]
    fn test_SQLExpression_get_select_column_list() {
        struct TestCase {
            name: String,
            expression: SQLExpression,
            expected: Vec<SelectColumn>,
        }

        let test_cases = vec![
            TestCase {
                name: "단일 값 필드".into(),
                expression: SQLExpression::Integer(4444),
                expected: vec![],
            },
            TestCase {
                name: "List 필드".into(),
                expression: SQLExpression::List(
                    vec![
                        SQLExpression::Integer(1),
                        SQLExpression::Integer(2),
                        SQLExpression::Integer(3),
                    ]
                    .into(),
                ),
                expected: vec![],
            },
            TestCase {
                name: "유효한 Select 필드".into(),
                expression: SQLExpression::SelectColumn(SelectColumn::new(None, "id".into())),
                expected: vec![SelectColumn::new(None, "id".into())],
            },
            TestCase {
                name: "유효한 단항연산 필드".into(),
                expression: SQLExpression::Unary(Box::new(UnaryOperatorExpression {
                    operator: UnaryOperator::Neg,
                    operand: SQLExpression::SelectColumn(SelectColumn::new(None, "id".into())),
                })),
                expected: vec![SelectColumn::new(None, "id".into())],
            },
            TestCase {
                name: "유효한 이항연산 필드".into(),
                expression: SQLExpression::Binary(Box::new(BinaryOperatorExpression {
                    lhs: SQLExpression::SelectColumn(SelectColumn::new(None, "id".into())),
                    rhs: SQLExpression::SelectColumn(SelectColumn::new(None, "name".into())),
                    operator: BinaryOperator::Add,
                })),
                expected: vec![
                    SelectColumn::new(None, "id".into()),
                    SelectColumn::new(None, "name".into()),
                ],
            },
            TestCase {
                name: "유효한 BETWEEN 필드".into(),
                expression: SQLExpression::Between(Box::new(
                    crate::engine::ast::dml::expressions::between::BetweenExpression {
                        a: SQLExpression::SelectColumn(SelectColumn::new(None, "id".into())),
                        x: SQLExpression::Integer(1),
                        y: SQLExpression::Integer(10),
                    },
                )),
                expected: vec![SelectColumn::new(None, "id".into())],
            },
            TestCase {
                name: "유효한 NOT BETWEEN 필드".into(),
                expression: SQLExpression::NotBetween(Box::new(
                    crate::engine::ast::dml::expressions::not_between::NotBetweenExpression {
                        a: SQLExpression::SelectColumn(SelectColumn::new(None, "id".into())),
                        x: SQLExpression::Integer(1),
                        y: SQLExpression::Integer(10),
                    },
                )),
                expected: vec![SelectColumn::new(None, "id".into())],
            },
            TestCase {
                name: "유효한 소괄호 필드".into(),
                expression: SQLExpression::Parentheses(Box::new(
                    crate::engine::ast::dml::expressions::parentheses::ParenthesesExpression {
                        expression: SQLExpression::SelectColumn(SelectColumn::new(
                            None,
                            "id".into(),
                        )),
                    },
                )),
                expected: vec![SelectColumn::new(None, "id".into())],
            },
            TestCase {
                name: "유효한 함수호출 필드".into(),
                expression: SQLExpression::FunctionCall(CallExpression {
                    function: Function::BuiltIn(AggregateFunction::Count.into()),
                    arguments: vec![SQLExpression::SelectColumn(SelectColumn::new(
                        None,
                        "id".into(),
                    ))],
                }),
                expected: vec![SelectColumn::new(None, "id".into())],
            },
        ];

        for test_case in test_cases {
            assert_eq!(
                test_case.expression.get_select_column_list(),
                test_case.expected,
                "{}",
                test_case.name
            );
        }
    }

    #[test]
    fn test_SQLExpression_has_aggregate() {
        struct TestCase {
            name: String,
            expression: SQLExpression,
            expected: bool,
        }

        let test_cases = vec![
            TestCase {
                name: "단일 값 필드".into(),
                expression: SQLExpression::Integer(4444),
                expected: false,
            },
            TestCase {
                name: "단항 연산 필드 (aggregate 없음)".into(),
                expression: SQLExpression::Unary(Box::new(UnaryOperatorExpression {
                    operator: UnaryOperator::Neg,
                    operand: SQLExpression::Integer(4444),
                })),
                expected: false,
            },
            TestCase {
                name: "단항 연산 필드 (aggregate 있음)".into(),
                expression: SQLExpression::Unary(Box::new(UnaryOperatorExpression {
                    operator: UnaryOperator::Neg,
                    operand: SQLExpression::FunctionCall(CallExpression {
                        function: Function::BuiltIn(AggregateFunction::Count.into()),
                        arguments: vec![SQLExpression::Integer(1)],
                    }),
                })),
                expected: true,
            },
            TestCase {
                name: "이항 연산 필드 (aggregate 없음)".into(),
                expression: SQLExpression::Binary(Box::new(BinaryOperatorExpression {
                    lhs: SQLExpression::Integer(1),
                    rhs: SQLExpression::Integer(2),
                    operator: BinaryOperator::Add,
                })),
                expected: false,
            },
            TestCase {
                name: "이항 연산 필드 (aggregate 있음)".into(),
                expression: SQLExpression::Binary(Box::new(BinaryOperatorExpression {
                    lhs: SQLExpression::FunctionCall(CallExpression {
                        function: Function::BuiltIn(AggregateFunction::Count.into()),
                        arguments: vec![SQLExpression::Integer(1)],
                    }),
                    rhs: SQLExpression::Integer(2),
                    operator: BinaryOperator::Add,
                })),
                expected: true,
            },
            TestCase {
                name: "BETWEEN 필드 (aggregate 없음)".into(),
                expression: SQLExpression::Between(Box::new(BetweenExpression {
                    a: SQLExpression::Integer(1),
                    x: SQLExpression::Integer(2),
                    y: SQLExpression::Integer(3),
                })),
                expected: false,
            },
            TestCase {
                name: "BETWEEN 필드 (aggregate 있음)".into(),
                expression: SQLExpression::Between(Box::new(BetweenExpression {
                    a: SQLExpression::FunctionCall(CallExpression {
                        function: Function::BuiltIn(AggregateFunction::Count.into()),
                        arguments: vec![SQLExpression::Integer(1)],
                    }),
                    x: SQLExpression::Integer(2),
                    y: SQLExpression::Integer(3),
                })),
                expected: true,
            },
            TestCase {
                name: "NOT BETWEEN 필드 (aggregate 없음)".into(),
                expression: SQLExpression::NotBetween(Box::new(NotBetweenExpression {
                    a: SQLExpression::Integer(1),
                    x: SQLExpression::Integer(2),
                    y: SQLExpression::Integer(3),
                })),
                expected: false,
            },
            TestCase {
                name: "NOT BETWEEN 필드 (aggregate 있음)".into(),
                expression: SQLExpression::NotBetween(Box::new(NotBetweenExpression {
                    a: SQLExpression::FunctionCall(CallExpression {
                        function: Function::BuiltIn(AggregateFunction::Count.into()),
                        arguments: vec![SQLExpression::Integer(1)],
                    }),
                    x: SQLExpression::Integer(2),
                    y: SQLExpression::Integer(3),
                })),
                expected: true,
            },
            TestCase {
                name: "소괄호 필드 (aggregate 없음)".into(),
                expression: SQLExpression::Parentheses(Box::new(ParenthesesExpression {
                    expression: SQLExpression::Integer(1),
                })),
                expected: false,
            },
            TestCase {
                name: "소괄호 필드 (aggregate 있음)".into(),
                expression: SQLExpression::Parentheses(Box::new(ParenthesesExpression {
                    expression: SQLExpression::FunctionCall(CallExpression {
                        function: Function::BuiltIn(AggregateFunction::Count.into()),
                        arguments: vec![SQLExpression::Integer(1)],
                    }),
                })),
                expected: true,
            },
        ];

        for test_case in test_cases {
            assert_eq!(
                test_case.expression.has_aggregate(),
                test_case.expected,
                "{}",
                test_case.name
            );
        }
    }

    #[test]
    fn test_SQLExperssion_find_aggregate_columns() {
        struct TestCase {
            name: String,
            expression: SQLExpression,
            expected: Vec<SelectColumn>,
        }

        let test_cases = vec![
            TestCase {
                name: "단일 값 필드".into(),
                expression: SQLExpression::Integer(4444),
                expected: vec![],
            },
            TestCase {
                name: "단항 연산 필드 (aggregate 없음)".into(),
                expression: SQLExpression::Unary(Box::new(UnaryOperatorExpression {
                    operator: UnaryOperator::Neg,
                    operand: SQLExpression::SelectColumn(SelectColumn::new(None, "id".into())),
                })),
                expected: vec![],
            },
            TestCase {
                name: "단항 연산 필드 (aggregate 있음)".into(),
                expression: SQLExpression::Unary(Box::new(UnaryOperatorExpression {
                    operator: UnaryOperator::Neg,
                    operand: SQLExpression::FunctionCall(CallExpression {
                        function: Function::BuiltIn(AggregateFunction::Count.into()),
                        arguments: vec![SQLExpression::SelectColumn(SelectColumn::new(
                            None,
                            "id".into(),
                        ))],
                    }),
                })),
                expected: vec![SelectColumn::new(None, "id".into())],
            },
            TestCase {
                name: "이항 연산 필드 (aggregate 없음)".into(),
                expression: SQLExpression::Binary(Box::new(BinaryOperatorExpression {
                    lhs: SQLExpression::SelectColumn(SelectColumn::new(None, "id".into())),
                    rhs: SQLExpression::SelectColumn(SelectColumn::new(None, "name".into())),
                    operator: BinaryOperator::Add,
                })),
                expected: vec![],
            },
            TestCase {
                name: "이항 중첩 연산 필드 (aggregate 있음)".into(),
                expression: SQLExpression::Binary(Box::new(BinaryOperatorExpression {
                    lhs: SQLExpression::FunctionCall(CallExpression {
                        function: Function::BuiltIn(AggregateFunction::Count.into()),
                        arguments: vec![SQLExpression::SelectColumn(SelectColumn::new(
                            None,
                            "id".into(),
                        ))],
                    }),
                    rhs: SQLExpression::SelectColumn(SelectColumn::new(None, "name".into())),
                    operator: BinaryOperator::Add,
                })),
                expected: vec![SelectColumn::new(None, "id".into())],
            },
            TestCase {
                name: "BETWEEN 필드 (aggregate 없음)".into(),
                expression: SQLExpression::Between(Box::new(BetweenExpression {
                    a: SQLExpression::SelectColumn(SelectColumn::new(None, "id".into())),
                    x: SQLExpression::Integer(1),
                    y: SQLExpression::Integer(10),
                })),
                expected: vec![],
            },
            TestCase {
                name: "BETWEEN 필드 (aggregate 있음)".into(),
                expression: SQLExpression::Between(Box::new(BetweenExpression {
                    a: SQLExpression::FunctionCall(CallExpression {
                        function: Function::BuiltIn(AggregateFunction::Count.into()),
                        arguments: vec![SQLExpression::SelectColumn(SelectColumn::new(
                            None,
                            "id".into(),
                        ))],
                    }),
                    x: SQLExpression::Integer(1),
                    y: SQLExpression::Integer(10),
                })),
                expected: vec![SelectColumn::new(None, "id".into())],
            },
            TestCase {
                name: "NOT BETWEEN 필드 (aggregate 없음)".into(),
                expression: SQLExpression::NotBetween(Box::new(NotBetweenExpression {
                    a: SQLExpression::SelectColumn(SelectColumn::new(None, "id".into())),
                    x: SQLExpression::Integer(1),
                    y: SQLExpression::Integer(10),
                })),
                expected: vec![],
            },
            TestCase {
                name: "NOT BETWEEN 필드 (aggregate 있음)".into(),
                expression: SQLExpression::NotBetween(Box::new(NotBetweenExpression {
                    a: SQLExpression::FunctionCall(CallExpression {
                        function: Function::BuiltIn(AggregateFunction::Count.into()),
                        arguments: vec![SQLExpression::SelectColumn(SelectColumn::new(
                            None,
                            "id".into(),
                        ))],
                    }),
                    x: SQLExpression::Integer(1),
                    y: SQLExpression::Integer(10),
                })),
                expected: vec![SelectColumn::new(None, "id".into())],
            },
            TestCase {
                name: "소괄호 필드 (aggregate 없음)".into(),
                expression: SQLExpression::Parentheses(Box::new(ParenthesesExpression {
                    expression: SQLExpression::SelectColumn(SelectColumn::new(None, "id".into())),
                })),
                expected: vec![],
            },
            TestCase {
                name: "소괄호 필드 (aggregate 있음)".into(),
                expression: SQLExpression::Parentheses(Box::new(ParenthesesExpression {
                    expression: SQLExpression::FunctionCall(CallExpression {
                        function: Function::BuiltIn(AggregateFunction::Count.into()),
                        arguments: vec![SQLExpression::SelectColumn(SelectColumn::new(
                            None,
                            "id".into(),
                        ))],
                    }),
                })),
                expected: vec![SelectColumn::new(None, "id".into())],
            },
            TestCase {
                name: "함수호출 필드 (aggregate 없음)".into(),
                expression: SQLExpression::FunctionCall(CallExpression {
                    function: Function::BuiltIn(ConditionalFunction::NullIf.into()),
                    arguments: vec![SQLExpression::SelectColumn(SelectColumn::new(
                        None,
                        "id".into(),
                    ))],
                }),
                expected: vec![],
            },
        ];

        for test_case in test_cases {
            assert_eq!(
                test_case.expression.find_aggregate_columns(),
                test_case.expected,
                "{}",
                test_case.name
            );
        }
    }

    #[test]
    fn test_From_SQLExpression_for_Option_Box_SQLExpression() {
        let expression = SQLExpression::Integer(1);
        let expected = Some(Box::new(expression.clone()));

        assert_eq!(Option::<Box<SQLExpression>>::from(expression), expected);
    }

    #[test]
    fn test_From_TableDataFieldType_for_SQLExpression() {
        struct TestCase {
            name: String,
            value: TableDataFieldType,
            expected: SQLExpression,
        }

        let test_cases = vec![
            TestCase {
                name: "정수형".into(),
                value: TableDataFieldType::Integer(1),
                expected: SQLExpression::Integer(1),
            },
            TestCase {
                name: "실수형".into(),
                value: TableDataFieldType::Float(1.0.into()),
                expected: SQLExpression::Float(1.0),
            },
            TestCase {
                name: "부울형".into(),
                value: TableDataFieldType::Boolean(true),
                expected: SQLExpression::Boolean(true),
            },
            TestCase {
                name: "문자열형".into(),
                value: TableDataFieldType::String("hello".into()),
                expected: SQLExpression::String("hello".into()),
            },
            TestCase {
                name: "배열형".into(),
                value: TableDataFieldType::Array(vec![
                    TableDataFieldType::Integer(1),
                    TableDataFieldType::Integer(2),
                    TableDataFieldType::Integer(3),
                ]),
                expected: SQLExpression::List(
                    vec![
                        SQLExpression::Integer(1),
                        SQLExpression::Integer(2),
                        SQLExpression::Integer(3),
                    ]
                    .into(),
                ),
            },
            TestCase {
                name: "NULL 값".into(),
                value: TableDataFieldType::Null,
                expected: SQLExpression::Null,
            },
        ];

        for test_case in test_cases {
            assert_eq!(
                SQLExpression::from(test_case.value),
                test_case.expected,
                "{}",
                test_case.name
            );
        }
    }

    #[test]
    fn test_SQLExpression_find_non_aggregate_columns() {
        struct TestCase {
            name: String,
            expression: SQLExpression,
            expected: Vec<SelectColumn>,
        }

        let test_cases = vec![
            TestCase {
                name: "단일 값 필드".into(),
                expression: SQLExpression::Integer(4444),
                expected: vec![],
            },
            TestCase {
                name: "단일 Select 필드".into(),
                expression: SQLExpression::SelectColumn(SelectColumn::new(None, "id".into())),
                expected: vec![SelectColumn::new(None, "id".into())],
            },
            TestCase {
                name: "단항 연산 필드 (aggregate 없음)".into(),
                expression: SQLExpression::Unary(Box::new(UnaryOperatorExpression {
                    operator: UnaryOperator::Neg,
                    operand: SQLExpression::SelectColumn(SelectColumn::new(None, "id".into())),
                })),
                expected: vec![SelectColumn::new(None, "id".into())],
            },
            TestCase {
                name: "단항 연산 필드 (aggregate 있음)".into(),
                expression: SQLExpression::Unary(Box::new(UnaryOperatorExpression {
                    operator: UnaryOperator::Neg,
                    operand: SQLExpression::FunctionCall(CallExpression {
                        function: Function::BuiltIn(AggregateFunction::Count.into()),
                        arguments: vec![SQLExpression::SelectColumn(SelectColumn::new(
                            None,
                            "id".into(),
                        ))],
                    }),
                })),
                expected: vec![],
            },
            TestCase {
                name: "이항 연산 필드 (aggregate 없음)".into(),
                expression: SQLExpression::Binary(Box::new(BinaryOperatorExpression {
                    lhs: SQLExpression::SelectColumn(SelectColumn::new(None, "id".into())),
                    rhs: SQLExpression::SelectColumn(SelectColumn::new(None, "name".into())),
                    operator: BinaryOperator::Add,
                })),
                expected: vec![
                    SelectColumn::new(None, "id".into()),
                    SelectColumn::new(None, "name".into()),
                ],
            },
            TestCase {
                name: "이항 중첩 연산 필드 (aggregate 있음)".into(),
                expression: SQLExpression::Binary(Box::new(BinaryOperatorExpression {
                    lhs: SQLExpression::FunctionCall(CallExpression {
                        function: Function::BuiltIn(AggregateFunction::Count.into()),
                        arguments: vec![SQLExpression::SelectColumn(SelectColumn::new(
                            None,
                            "id".into(),
                        ))],
                    }),
                    rhs: SQLExpression::SelectColumn(SelectColumn::new(None, "name".into())),
                    operator: BinaryOperator::Add,
                })),
                expected: vec![SelectColumn::new(None, "name".into())],
            },
            TestCase {
                name: "BETWEEN 필드 (aggregate 없음)".into(),
                expression: SQLExpression::Between(Box::new(BetweenExpression {
                    a: SQLExpression::SelectColumn(SelectColumn::new(None, "id".into())),
                    x: SQLExpression::Integer(1),
                    y: SQLExpression::Integer(10),
                })),
                expected: vec![SelectColumn::new(None, "id".into())],
            },
            TestCase {
                name: "BETWEEN 필드 (aggregate 있음)".into(),
                expression: SQLExpression::Between(Box::new(BetweenExpression {
                    a: SQLExpression::FunctionCall(CallExpression {
                        function: Function::BuiltIn(AggregateFunction::Count.into()),
                        arguments: vec![SQLExpression::SelectColumn(SelectColumn::new(
                            None,
                            "id".into(),
                        ))],
                    }),
                    x: SQLExpression::Integer(1),
                    y: SQLExpression::Integer(10),
                })),
                expected: vec![],
            },
            TestCase {
                name: "NOT BETWEEN 필드 (aggregate 없음)".into(),
                expression: SQLExpression::NotBetween(Box::new(NotBetweenExpression {
                    a: SQLExpression::SelectColumn(SelectColumn::new(None, "id".into())),
                    x: SQLExpression::Integer(1),
                    y: SQLExpression::Integer(10),
                })),
                expected: vec![SelectColumn::new(None, "id".into())],
            },
            TestCase {
                name: "NOT BETWEEN 필드 (aggregate 있음)".into(),
                expression: SQLExpression::NotBetween(Box::new(NotBetweenExpression {
                    a: SQLExpression::FunctionCall(CallExpression {
                        function: Function::BuiltIn(AggregateFunction::Count.into()),
                        arguments: vec![SQLExpression::SelectColumn(SelectColumn::new(
                            None,
                            "id".into(),
                        ))],
                    }),
                    x: SQLExpression::Integer(1),
                    y: SQLExpression::Integer(10),
                })),
                expected: vec![],
            },
            TestCase {
                name: "소괄호 필드 (aggregate 없음)".into(),
                expression: SQLExpression::Parentheses(Box::new(ParenthesesExpression {
                    expression: SQLExpression::SelectColumn(SelectColumn::new(None, "id".into())),
                })),
                expected: vec![SelectColumn::new(None, "id".into())],
            },
            TestCase {
                name: "소괄호 필드 (aggregate 있음)".into(),
                expression: SQLExpression::Parentheses(Box::new(ParenthesesExpression {
                    expression: SQLExpression::FunctionCall(CallExpression {
                        function: Function::BuiltIn(AggregateFunction::Count.into()),
                        arguments: vec![SQLExpression::SelectColumn(SelectColumn::new(
                            None,
                            "id".into(),
                        ))],
                    }),
                })),
                expected: vec![],
            },
            TestCase {
                name: "함수호출 필드 (aggregate 없음)".into(),
                expression: SQLExpression::FunctionCall(CallExpression {
                    function: Function::BuiltIn(ConditionalFunction::Coalesce.into()),
                    arguments: vec![SQLExpression::SelectColumn(SelectColumn::new(
                        None,
                        "id".into(),
                    ))],
                }),
                expected: vec![SelectColumn::new(None, "id".into())],
            },
        ];

        for test_case in test_cases {
            assert_eq!(
                test_case.expression.find_non_aggregate_columns(),
                test_case.expected,
                "{}",
                test_case.name
            );
        }
    }
}
