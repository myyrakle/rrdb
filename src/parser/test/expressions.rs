#![cfg(test)]

use crate::ast::dml::ParenthesesExpression;
use crate::ast::predule::{
    BetweenExpression, BinaryOperator, BinaryOperatorExpression, CallExpression,
    ConditionalFunction, ListExpression, NotBetweenExpression, SQLExpression, SelectItem,
    SelectQuery, UnaryOperator, UnaryOperatorExpression, UserDefinedFunction,
};
use crate::parser::context::ParserContext;
use crate::parser::predule::Parser;

#[test]
pub fn unary_expression_1() {
    let text = r#"
        SELECT Not TRUE AS foo
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
        .add_select_item(
            SelectItem::builder()
                .set_item(
                    UnaryOperatorExpression {
                        operator: UnaryOperator::Not,
                        operand: SQLExpression::Boolean(true),
                    }
                    .into(),
                )
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn unary_expression_2() {
    let text = r#"
        SELECT !TRUE AS foo
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
        .add_select_item(
            SelectItem::builder()
                .set_item(
                    UnaryOperatorExpression {
                        operator: UnaryOperator::Not,
                        operand: SQLExpression::Boolean(true),
                    }
                    .into(),
                )
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn arithmetic_expression_1() {
    let text = r#"
        SELECT 3 + 5 AS foo
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
        .add_select_item(
            SelectItem::builder()
                .set_item(
                    BinaryOperatorExpression {
                        operator: BinaryOperator::Add,
                        lhs: SQLExpression::Integer(3),
                        rhs: SQLExpression::Integer(5),
                    }
                    .into(),
                )
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn arithmetic_expression_2() {
    let text = r#"
        SELECT 1 + 2 + 3 AS foo
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
        .add_select_item(
            SelectItem::builder()
                .set_item(
                    BinaryOperatorExpression {
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
                )
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn arithmetic_expression_3() {
    let text = r#"
        SELECT 1 + 2 * 3 AS foo
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
        .add_select_item(
            SelectItem::builder()
                .set_item(
                    BinaryOperatorExpression {
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
                )
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn arithmetic_expression_4() {
    let text = r#"
        SELECT 1 + 2 * 3 + 4 AS foo
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
        .add_select_item(
            SelectItem::builder()
                .set_item(
                    BinaryOperatorExpression {
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
                )
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn arithmetic_expression_5() {
    let text = r#"
        SELECT 2 * (3 + 5) AS foo
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
        .add_select_item(
            SelectItem::builder()
                .set_item(
                    BinaryOperatorExpression {
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
                )
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn arithmetic_expression_6() {
    let text = r#"
        SELECT -2 * 5 AS foo
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
        .add_select_item(
            SelectItem::builder()
                .set_item(
                    BinaryOperatorExpression {
                        operator: BinaryOperator::Mul,
                        lhs: UnaryOperatorExpression {
                            operator: UnaryOperator::Neg,
                            operand: SQLExpression::Integer(2),
                        }
                        .into(),
                        rhs: SQLExpression::Integer(5),
                    }
                    .into(),
                )
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn function_call_expression_1() {
    let text = r#"
        SELECT foobar(null, 1) as foo
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
        .add_select_item(
            SelectItem::builder()
                .set_item(
                    CallExpression {
                        function: UserDefinedFunction {
                            database_name: None,
                            function_name: "foobar".into(),
                        }
                        .into(),
                        arguments: vec![SQLExpression::Null, SQLExpression::Integer(1)],
                    }
                    .into(),
                )
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn function_call_expression_2() {
    let text = r#"
        SELECT coalesce(null, 1) as foo
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
        .add_select_item(
            SelectItem::builder()
                .set_item(
                    CallExpression {
                        function: ConditionalFunction::Coalesce.into(),
                        arguments: vec![SQLExpression::Null, SQLExpression::Integer(1)],
                    }
                    .into(),
                )
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn between_expression_1() {
    let text = r#"
        SELECT 3 between 1 and 5 as foo
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
        .add_select_item(
            SelectItem::builder()
                .set_item(
                    BetweenExpression {
                        a: SQLExpression::Integer(3),
                        x: SQLExpression::Integer(1),
                        y: SQLExpression::Integer(5),
                    }
                    .into(),
                )
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn between_expression_2() {
    let text = r#"
        SELECT 3 between 1 and 5 + 1 as foo
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
        .add_select_item(
            SelectItem::builder()
                .set_item(
                    BetweenExpression {
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
                )
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn between_expression_3() {
    let text = r#"
        SELECT 3 between 1 + 1 and 99 as foo
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
        .add_select_item(
            SelectItem::builder()
                .set_item(
                    BetweenExpression {
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
                )
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn not_between_expression_1() {
    let text = r#"
        SELECT 3 not between 1 and 5 as foo
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
        .add_select_item(
            SelectItem::builder()
                .set_item(
                    NotBetweenExpression {
                        a: SQLExpression::Integer(3),
                        x: SQLExpression::Integer(1),
                        y: SQLExpression::Integer(5),
                    }
                    .into(),
                )
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn list_expression_1() {
    let text = r#"
        SELECT (1,2,3) as foo
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
        .add_select_item(
            SelectItem::builder()
                .set_item(
                    ListExpression {
                        value: vec![
                            SQLExpression::Integer(1),
                            SQLExpression::Integer(2),
                            SQLExpression::Integer(3),
                        ],
                    }
                    .into(),
                )
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn in_expression_1() {
    let text = r#"
        SELECT 1 in (1,2,3) as foo
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
        .add_select_item(
            SelectItem::builder()
                .set_item(
                    BinaryOperatorExpression {
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
                )
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn not_in_expression_1() {
    let text = r#"
        SELECT 1 not in (1,2,3) as foo
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
        .add_select_item(
            SelectItem::builder()
                .set_item(
                    BinaryOperatorExpression {
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
                )
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn complex_expression_1() {
    let text = r#"
        SELECT 3+(10*2+44)-11 AS foo
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
        .add_select_item(
            SelectItem::builder()
                .set_item(
                    BinaryOperatorExpression {
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
                )
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}
