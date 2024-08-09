#![cfg(test)]

use crate::ast::dml::expressions::between::BetweenExpression;
use crate::ast::dml::expressions::binary::BinaryOperatorExpression;
use crate::ast::dml::expressions::call::CallExpression;
use crate::ast::dml::expressions::list::ListExpression;
use crate::ast::dml::expressions::not_between::NotBetweenExpression;
use crate::ast::dml::expressions::operators::{BinaryOperator, UnaryOperator};
use crate::ast::dml::expressions::parentheses::ParenthesesExpression;
use crate::ast::dml::expressions::unary::UnaryOperatorExpression;
use crate::ast::dml::parts::select_item::SelectItem;
use crate::ast::dml::select::SelectQuery;
use crate::ast::types::{ConditionalFunction, SQLExpression, SelectColumn, UserDefinedFunction};
use crate::lexer::predule::OperatorToken;
use crate::lexer::tokens::Token;
use crate::parser::predule::Parser;

#[test]
fn test_parse_expression() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: SQLExpression,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "integer".into(),
            input: vec![Token::Integer(42)],
            expected: SQLExpression::Integer(42),
            want_error: false,
        },
        TestCase {
            name: "float".into(),
            input: vec![Token::Float(42.42)],
            expected: SQLExpression::Float(42.42),
            want_error: false,
        },
        TestCase {
            name: "string".into(),
            input: vec![Token::String("hello".to_owned())],
            expected: SQLExpression::String("hello".to_owned()),
            want_error: false,
        },
        TestCase {
            name: "boolean".into(),
            input: vec![Token::Boolean(true)],
            expected: SQLExpression::Boolean(true),
            want_error: false,
        },
        TestCase {
            name: "null".into(),
            input: vec![Token::Null],
            expected: SQLExpression::Null,
            want_error: false,
        },
        TestCase {
            name: "Not TRUE".into(),
            input: vec![Token::Not, Token::Boolean(true)],
            expected: UnaryOperatorExpression {
                operator: UnaryOperator::Not,
                operand: SQLExpression::Boolean(true),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "!TRUE".into(),
            input: vec![Token::Not, Token::Boolean(true)],
            expected: UnaryOperatorExpression {
                operator: UnaryOperator::Not,
                operand: SQLExpression::Boolean(true),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "3 + 5".into(),
            input: vec![
                Token::Integer(3),
                Token::Operator(OperatorToken::Plus),
                Token::Integer(5),
            ],
            expected: BinaryOperatorExpression {
                operator: BinaryOperator::Add,
                lhs: SQLExpression::Integer(3),
                rhs: SQLExpression::Integer(5),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "1 + 2 + 3".into(),
            input: vec![
                Token::Integer(1),
                Token::Operator(OperatorToken::Plus),
                Token::Integer(2),
                Token::Operator(OperatorToken::Plus),
                Token::Integer(3),
            ],
            expected: BinaryOperatorExpression {
                operator: BinaryOperator::Add,
                lhs: BinaryOperatorExpression {
                    operator: BinaryOperator::Add,
                    lhs: SQLExpression::Integer(1),
                    rhs: SQLExpression::Integer(2),
                }
                .into(),
                rhs: SQLExpression::Integer(3),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "1 + 2 * 3".into(),
            input: vec![
                Token::Integer(1),
                Token::Operator(OperatorToken::Plus),
                Token::Integer(2),
                Token::Operator(OperatorToken::Asterisk),
                Token::Integer(3),
            ],
            expected: BinaryOperatorExpression {
                operator: BinaryOperator::Add,
                lhs: SQLExpression::Integer(1),
                rhs: BinaryOperatorExpression {
                    operator: BinaryOperator::Mul,
                    lhs: SQLExpression::Integer(2),
                    rhs: SQLExpression::Integer(3),
                }
                .into(),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "1 + 2 * 3 + 4".into(),
            input: vec![
                Token::Integer(1),
                Token::Operator(OperatorToken::Plus),
                Token::Integer(2),
                Token::Operator(OperatorToken::Asterisk),
                Token::Integer(3),
                Token::Operator(OperatorToken::Plus),
                Token::Integer(4),
            ],
            expected: BinaryOperatorExpression {
                operator: BinaryOperator::Add,
                lhs: BinaryOperatorExpression {
                    operator: BinaryOperator::Add,
                    lhs: SQLExpression::Integer(1),
                    rhs: BinaryOperatorExpression {
                        operator: BinaryOperator::Mul,
                        lhs: SQLExpression::Integer(2),
                        rhs: SQLExpression::Integer(3),
                    }
                    .into(),
                }
                .into(),
                rhs: SQLExpression::Integer(4),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "2 * (3 + 5)".into(),
            input: vec![
                Token::Integer(2),
                Token::Operator(OperatorToken::Asterisk),
                Token::LeftParentheses,
                Token::Integer(3),
                Token::Operator(OperatorToken::Plus),
                Token::Integer(5),
                Token::RightParentheses,
            ],
            expected: BinaryOperatorExpression {
                operator: BinaryOperator::Mul,
                lhs: SQLExpression::Integer(2),
                rhs: SQLExpression::Binary(
                    BinaryOperatorExpression {
                        operator: BinaryOperator::Add,
                        lhs: SQLExpression::Integer(3),
                        rhs: SQLExpression::Integer(5),
                    }
                    .into(),
                ),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "-2 * 5".into(),
            input: vec![
                Token::Operator(OperatorToken::Minus),
                Token::Integer(2),
                Token::Operator(OperatorToken::Asterisk),
                Token::Integer(5),
            ],
            expected: BinaryOperatorExpression {
                operator: BinaryOperator::Mul,
                lhs: UnaryOperatorExpression {
                    operator: UnaryOperator::Neg,
                    operand: SQLExpression::Integer(2),
                }
                .into(),
                rhs: SQLExpression::Integer(5),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "foobar(null, 1)".into(),
            input: vec![
                Token::Identifier("foobar".to_owned()),
                Token::LeftParentheses,
                Token::Null,
                Token::Comma,
                Token::Integer(1),
                Token::RightParentheses,
            ],
            expected: CallExpression {
                function: UserDefinedFunction {
                    database_name: None,
                    function_name: "foobar".into(),
                }
                .into(),
                arguments: vec![SQLExpression::Null, SQLExpression::Integer(1)],
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "coalesce(null, 1)".into(),
            input: vec![
                Token::Identifier("coalesce".to_owned()),
                Token::LeftParentheses,
                Token::Null,
                Token::Comma,
                Token::Integer(1),
                Token::RightParentheses,
            ],
            expected: CallExpression {
                function: ConditionalFunction::Coalesce.into(),
                arguments: vec![SQLExpression::Null, SQLExpression::Integer(1)],
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "3 between 1 and 5".into(),
            input: vec![
                Token::Integer(3),
                Token::Between,
                Token::Integer(1),
                Token::And,
                Token::Integer(5),
            ],
            expected: BetweenExpression {
                a: SQLExpression::Integer(3),
                x: SQLExpression::Integer(1),
                y: SQLExpression::Integer(5),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "(3 between 1 and 5)".into(),
            input: vec![
                Token::LeftParentheses,
                Token::Integer(3),
                Token::Between,
                Token::Integer(1),
                Token::And,
                Token::Integer(5),
                Token::RightParentheses,
            ],
            expected: ParenthesesExpression {
                expression: BetweenExpression {
                    a: SQLExpression::Integer(3),
                    x: SQLExpression::Integer(1),
                    y: SQLExpression::Integer(5),
                }
                .into(),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "(3 + 1) BETWEEN 2 AND 44".into(),
            input: vec![
                Token::LeftParentheses,
                Token::Integer(3),
                Token::Operator(OperatorToken::Plus),
                Token::Integer(1),
                Token::RightParentheses,
                Token::Between,
                Token::Integer(2),
                Token::And,
                Token::Integer(44),
            ],
            expected: BetweenExpression {
                a: ParenthesesExpression {
                    expression: BinaryOperatorExpression {
                        operator: BinaryOperator::Add,
                        lhs: SQLExpression::Integer(3),
                        rhs: SQLExpression::Integer(1),
                    }
                    .into(),
                }
                .into(),
                x: SQLExpression::Integer(2),
                y: SQLExpression::Integer(44),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "3.0 between 1.2 and 5.3".into(),
            input: vec![
                Token::Float(3.0),
                Token::Between,
                Token::Float(1.2),
                Token::And,
                Token::Float(5.3),
            ],
            expected: BetweenExpression {
                a: SQLExpression::Float(3.0),
                x: SQLExpression::Float(1.2),
                y: SQLExpression::Float(5.3),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: r#""foo" between "3" and "4""#.into(),
            input: vec![
                Token::String("foo".to_owned()),
                Token::Between,
                Token::String("3".to_owned()),
                Token::And,
                Token::String("4".to_owned()),
            ],
            expected: BetweenExpression {
                a: SQLExpression::String("foo".to_owned()),
                x: SQLExpression::String("3".to_owned()),
                y: SQLExpression::String("4".to_owned()),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: r#"true between false and false"#.into(),
            input: vec![
                Token::Boolean(true),
                Token::Between,
                Token::Boolean(false),
                Token::And,
                Token::Boolean(false),
            ],
            expected: BetweenExpression {
                a: SQLExpression::Boolean(true),
                x: SQLExpression::Boolean(false),
                y: SQLExpression::Boolean(false),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: r#"null between null and null"#.into(),
            input: vec![
                Token::Null,
                Token::Between,
                Token::Null,
                Token::And,
                Token::Null,
            ],
            expected: BetweenExpression {
                a: SQLExpression::Null,
                x: SQLExpression::Null,
                y: SQLExpression::Null,
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "3 between 1 and 5 + 1".into(),
            input: vec![
                Token::Integer(3),
                Token::Between,
                Token::Integer(1),
                Token::And,
                Token::Integer(5),
                Token::Operator(OperatorToken::Plus),
                Token::Integer(1),
            ],
            expected: BetweenExpression {
                a: SQLExpression::Integer(3),
                x: SQLExpression::Integer(1),
                y: BinaryOperatorExpression {
                    operator: BinaryOperator::Add,
                    lhs: SQLExpression::Integer(5),
                    rhs: SQLExpression::Integer(1),
                }
                .into(),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "3 between 1 + 1 and 99".into(),
            input: vec![
                Token::Integer(3),
                Token::Between,
                Token::Integer(1),
                Token::Operator(OperatorToken::Plus),
                Token::Integer(1),
                Token::And,
                Token::Integer(99),
            ],
            expected: BetweenExpression {
                a: SQLExpression::Integer(3),
                x: BinaryOperatorExpression {
                    operator: BinaryOperator::Add,
                    lhs: SQLExpression::Integer(1),
                    rhs: SQLExpression::Integer(1),
                }
                .into(),
                y: SQLExpression::Integer(99),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "3 not between 1 and 5".into(),
            input: vec![
                Token::Integer(3),
                Token::Not,
                Token::Between,
                Token::Integer(1),
                Token::And,
                Token::Integer(5),
            ],
            expected: NotBetweenExpression {
                a: SQLExpression::Integer(3),
                x: SQLExpression::Integer(1),
                y: SQLExpression::Integer(5),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "(1,2,3)".into(),
            input: vec![
                Token::LeftParentheses,
                Token::Integer(1),
                Token::Comma,
                Token::Integer(2),
                Token::Comma,
                Token::Integer(3),
                Token::RightParentheses,
            ],
            expected: ListExpression {
                value: vec![
                    SQLExpression::Integer(1),
                    SQLExpression::Integer(2),
                    SQLExpression::Integer(3),
                ],
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "1 in (1,2,3)".into(),
            input: vec![
                Token::Integer(1),
                Token::In,
                Token::LeftParentheses,
                Token::Integer(1),
                Token::Comma,
                Token::Integer(2),
                Token::Comma,
                Token::Integer(3),
                Token::RightParentheses,
            ],
            expected: BinaryOperatorExpression {
                operator: BinaryOperator::In,
                lhs: SQLExpression::Integer(1),
                rhs: ListExpression {
                    value: vec![
                        SQLExpression::Integer(1),
                        SQLExpression::Integer(2),
                        SQLExpression::Integer(3),
                    ],
                }
                .into(),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "1 not in (1,2,3)".into(),
            input: vec![
                Token::Integer(1),
                Token::Not,
                Token::In,
                Token::LeftParentheses,
                Token::Integer(1),
                Token::Comma,
                Token::Integer(2),
                Token::Comma,
                Token::Integer(3),
                Token::RightParentheses,
            ],
            expected: BinaryOperatorExpression {
                operator: BinaryOperator::NotIn,
                lhs: SQLExpression::Integer(1),
                rhs: ListExpression {
                    value: vec![
                        SQLExpression::Integer(1),
                        SQLExpression::Integer(2),
                        SQLExpression::Integer(3),
                    ],
                }
                .into(),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "3+(10*2+44)-11".into(),
            input: vec![
                Token::Integer(3),
                Token::Operator(OperatorToken::Plus),
                Token::LeftParentheses,
                Token::Integer(10),
                Token::Operator(OperatorToken::Asterisk),
                Token::Integer(2),
                Token::Operator(OperatorToken::Plus),
                Token::Integer(44),
                Token::RightParentheses,
                Token::Operator(OperatorToken::Minus),
                Token::Integer(11),
            ],
            expected: BinaryOperatorExpression {
                operator: BinaryOperator::Sub,
                lhs: SQLExpression::Binary(
                    BinaryOperatorExpression {
                        operator: BinaryOperator::Add,
                        lhs: SQLExpression::Integer(3),
                        rhs: ParenthesesExpression {
                            expression: BinaryOperatorExpression {
                                operator: BinaryOperator::Add,
                                lhs: BinaryOperatorExpression {
                                    operator: BinaryOperator::Mul,
                                    lhs: SQLExpression::Integer(10),
                                    rhs: SQLExpression::Integer(2),
                                }
                                .into(),
                                rhs: SQLExpression::Integer(44),
                            }
                            .into(),
                        }
                        .into(),
                    }
                    .into(),
                ),
                rhs: SQLExpression::Integer(11),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "3.14 + 4.4".into(),
            input: vec![
                Token::Float(3.14),
                Token::Operator(OperatorToken::Plus),
                Token::Float(4.4),
            ],
            expected: BinaryOperatorExpression {
                operator: BinaryOperator::Add,
                lhs: SQLExpression::Float(3.14),
                rhs: SQLExpression::Float(4.4),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: r#""3.14" + "4.4""#.into(),
            input: vec![
                Token::String("3.14".to_owned()),
                Token::Operator(OperatorToken::Plus),
                Token::String("4.4".to_owned()),
            ],
            expected: BinaryOperatorExpression {
                operator: BinaryOperator::Add,
                lhs: SQLExpression::String("3.14".to_owned()),
                rhs: SQLExpression::String("4.4".to_owned()),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: r#"true + false"#.into(),
            input: vec![
                Token::Boolean(true),
                Token::Operator(OperatorToken::Plus),
                Token::Boolean(false),
            ],
            expected: BinaryOperatorExpression {
                operator: BinaryOperator::Add,
                lhs: SQLExpression::Boolean(true),
                rhs: SQLExpression::Boolean(false),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: r#"null + null"#.into(),
            input: vec![
                Token::Null,
                Token::Operator(OperatorToken::Plus),
                Token::Null,
            ],
            expected: BinaryOperatorExpression {
                operator: BinaryOperator::Add,
                lhs: SQLExpression::Null,
                rhs: SQLExpression::Null,
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: r#"(SELECT 1) + 10"#.into(),
            input: vec![
                Token::LeftParentheses,
                Token::Select,
                Token::Integer(1),
                Token::RightParentheses,
                Token::Operator(OperatorToken::Plus),
                Token::Integer(10),
            ],
            expected: BinaryOperatorExpression {
                operator: BinaryOperator::Add,
                lhs: SQLExpression::Subquery(
                    SelectQuery::builder()
                        .add_select_item(
                            SelectItem::builder()
                                .set_item(SQLExpression::Integer(1))
                                .build(),
                        )
                        .build()
                        .into(),
                ),
                rhs: SQLExpression::Integer(10),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: r#"(SELECT 1) BETWEEN 0 AND 5"#.into(),
            input: vec![
                Token::LeftParentheses,
                Token::Select,
                Token::Integer(1),
                Token::RightParentheses,
                Token::Between,
                Token::Integer(0),
                Token::And,
                Token::Integer(5),
            ],
            expected: BetweenExpression {
                a: SQLExpression::Subquery(
                    SelectQuery::builder()
                        .add_select_item(
                            SelectItem::builder()
                                .set_item(SQLExpression::Integer(1))
                                .build(),
                        )
                        .build()
                        .into(),
                ),
                x: SQLExpression::Integer(0),
                y: SQLExpression::Integer(5),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: r#"foo between 1 and 5"#.into(),
            input: vec![
                Token::Identifier("foo".to_owned()),
                Token::Between,
                Token::Integer(1),
                Token::And,
                Token::Integer(5),
            ],
            expected: BetweenExpression {
                a: SQLExpression::SelectColumn(SelectColumn::new(None, "foo".to_owned())),
                x: SQLExpression::Integer(1),
                y: SQLExpression::Integer(5),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: r#"foo(1) + 10"#.into(),
            input: vec![
                Token::Identifier("foo".to_owned()),
                Token::LeftParentheses,
                Token::Integer(1),
                Token::RightParentheses,
                Token::Operator(OperatorToken::Plus),
                Token::Integer(10),
            ],
            expected: BinaryOperatorExpression {
                operator: BinaryOperator::Add,
                lhs: CallExpression {
                    function: UserDefinedFunction {
                        database_name: None,
                        function_name: "foo".into(),
                    }
                    .into(),
                    arguments: vec![SQLExpression::Integer(1)],
                }
                .into(),
                rhs: SQLExpression::Integer(10),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: r#"foo(1) BETWEEN 1 and 5"#.into(),
            input: vec![
                Token::Identifier("foo".to_owned()),
                Token::LeftParentheses,
                Token::Integer(1),
                Token::RightParentheses,
                Token::Between,
                Token::Integer(1),
                Token::And,
                Token::Integer(5),
            ],
            expected: BetweenExpression {
                a: CallExpression {
                    function: UserDefinedFunction {
                        database_name: None,
                        function_name: "foo".into(),
                    }
                    .into(),
                    arguments: vec![SQLExpression::Integer(1)],
                }
                .into(),
                x: SQLExpression::Integer(1),
                y: SQLExpression::Integer(5),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "오류: * 5".into(),
            input: vec![Token::Operator(OperatorToken::Asterisk), Token::Integer(5)],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: (".into(),
            input: vec![Token::LeftParentheses],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: )".into(),
            input: vec![Token::RightParentheses],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: DELETE".into(),
            input: vec![Token::Delete],
            expected: Default::default(),
            want_error: true,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got = parser.parse_expression(Default::default());

        assert_eq!(
            got.is_err(),
            t.want_error,
            "{}: want_error: {}, error: {:?}",
            t.name,
            t.want_error,
            got.err()
        );

        if let Ok(statements) = got {
            assert_eq!(statements, t.expected, "TC: {}", t.name);
        }
    }
}

#[test]
fn test_parse_unary_expression() {
    struct TestCase {
        name: String,
        operator: UnaryOperator,
        input: Vec<Token>,
        expected: SQLExpression,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "! 3 BETWEEN 2 AND 5".into(),
            operator: UnaryOperator::Not,
            input: vec![
                Token::Integer(3),
                Token::Between,
                Token::Integer(2),
                Token::And,
                Token::Integer(5),
            ],
            expected: BetweenExpression {
                a: UnaryOperatorExpression {
                    operator: UnaryOperator::Not,
                    operand: SQLExpression::Integer(3),
                }
                .into(),
                x: SQLExpression::Integer(2),
                y: SQLExpression::Integer(5),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "실패: 빈 토큰".into(),
            operator: UnaryOperator::Not,
            input: vec![],
            expected: Default::default(),
            want_error: true,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got: Result<SQLExpression, crate::errors::RRDBError> =
            parser.parse_unary_expression(t.operator, Default::default());

        assert_eq!(
            got.is_err(),
            t.want_error,
            "{}: want_error: {}, error: {:?}",
            t.name,
            t.want_error,
            got.err()
        );

        if let Ok(statements) = got {
            assert_eq!(statements, t.expected, "TC: {}", t.name);
        }
    }
}

#[test]
fn test_parse_parentheses_expression() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: SQLExpression,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "(3)".into(),
            input: vec![
                Token::LeftParentheses,
                Token::Integer(3),
                Token::RightParentheses,
            ],
            expected: ParenthesesExpression {
                expression: SQLExpression::Integer(3),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "(3, 4, 5)".into(),
            input: vec![
                Token::LeftParentheses,
                Token::Integer(3),
                Token::Comma,
                Token::Integer(4),
                Token::Comma,
                Token::Integer(5),
                Token::RightParentheses,
            ],
            expected: ListExpression {
                value: vec![
                    SQLExpression::Integer(3),
                    SQLExpression::Integer(4),
                    SQLExpression::Integer(5),
                ],
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "오류 (3,".into(),
            input: vec![Token::LeftParentheses, Token::Integer(3), Token::Comma],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: 빈 토큰".into(),
            input: vec![],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: )".into(),
            input: vec![Token::RightParentheses],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: (".into(),
            input: vec![Token::LeftParentheses],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "(3".into(),
            input: vec![Token::LeftParentheses, Token::Integer(3)],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "(3;;".into(),
            input: vec![
                Token::LeftParentheses,
                Token::Integer(3),
                Token::SemiColon,
                Token::SemiColon,
            ],
            expected: Default::default(),
            want_error: true,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got: Result<SQLExpression, crate::errors::RRDBError> =
            parser.parse_parentheses_expression(Default::default());

        assert_eq!(
            got.is_err(),
            t.want_error,
            "{}: want_error: {}, error: {:?}",
            t.name,
            t.want_error,
            got.err()
        );

        if let Ok(statements) = got {
            assert_eq!(statements, t.expected, "TC: {}", t.name);
        }
    }
}

#[test]
fn test_parse_binary_expression() {
    struct TestCase {
        name: String,
        lhs: SQLExpression,
        input: Vec<Token>,
        expected: SQLExpression,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "3 + 5".into(),
            lhs: SQLExpression::Integer(3),
            input: vec![Token::Operator(OperatorToken::Plus), Token::Integer(5)],
            expected: BinaryOperatorExpression {
                operator: BinaryOperator::Add,
                lhs: SQLExpression::Integer(3),
                rhs: SQLExpression::Integer(5),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "-3 * 5 + 2".into(),
            lhs: UnaryOperatorExpression {
                operator: UnaryOperator::Neg,
                operand: SQLExpression::Integer(3),
            }
            .into(),
            input: vec![
                Token::Operator(OperatorToken::Asterisk),
                Token::LeftParentheses,
                Token::Integer(5),
                Token::Operator(OperatorToken::Plus),
                Token::Integer(2),
                Token::RightParentheses,
            ],
            expected: BinaryOperatorExpression {
                operator: BinaryOperator::Add,
                lhs: UnaryOperatorExpression {
                    operator: UnaryOperator::Neg,
                    operand: SQLExpression::Integer(3),
                }
                .into(),
                rhs: BinaryOperatorExpression {
                    operator: BinaryOperator::Add,
                    lhs: SQLExpression::Integer(5),
                    rhs: SQLExpression::Integer(2),
                }
                .into(),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "실패: 빈 토큰".into(),
            lhs: SQLExpression::Integer(3),
            input: vec![],
            expected: Default::default(),
            want_error: true,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got: Result<SQLExpression, crate::errors::RRDBError> =
            parser.parse_binary_expression(t.lhs, Default::default());

        assert_eq!(
            got.is_err(),
            t.want_error,
            "{}: want_error: {}, error: {:?}",
            t.name,
            t.want_error,
            got.err()
        );

        if let Ok(statements) = got {
            assert_eq!(statements, t.expected, "TC: {}", t.name);
        }
    }
}
