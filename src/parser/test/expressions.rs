#![cfg(test)]

use crate::ast::dml::expressions::between::BetweenExpression;
use crate::ast::dml::expressions::binary::BinaryOperatorExpression;
use crate::ast::dml::expressions::call::CallExpression;
use crate::ast::dml::expressions::list::ListExpression;
use crate::ast::dml::expressions::not_between::NotBetweenExpression;
use crate::ast::dml::expressions::operators::{BinaryOperator, UnaryOperator};
use crate::ast::dml::expressions::parentheses::ParenthesesExpression;
use crate::ast::dml::expressions::unary::UnaryOperatorExpression;
use crate::ast::types::{ConditionalFunction, SQLExpression, UserDefinedFunction};
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
