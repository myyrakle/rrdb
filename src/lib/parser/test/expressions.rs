#![cfg(test)]

use crate::lib::ast::predule::{
    BetweenExpression, BinaryOperator, BinaryOperatorExpression, CallExpression, FunctionName,
    SQLExpression, SelectItem, SelectQuery,
};
use crate::lib::dml::{UnaryOperator, UnaryOperatorExpression};
use crate::lib::parser::predule::Parser;

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
                .set_item(SQLExpression::Binary(
                    BinaryOperatorExpression {
                        operator: BinaryOperator::Add,
                        lhs: SQLExpression::Integer(3),
                        rhs: SQLExpression::Integer(5),
                    }
                    .into(),
                ))
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(parser.parse().unwrap(), vec![expected],);
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
                .set_item(SQLExpression::Binary(
                    BinaryOperatorExpression {
                        operator: BinaryOperator::Add,
                        lhs: SQLExpression::Binary(
                            BinaryOperatorExpression {
                                operator: BinaryOperator::Add,
                                lhs: SQLExpression::Integer(1),
                                rhs: SQLExpression::Integer(2),
                            }
                            .into(),
                        ),
                        rhs: SQLExpression::Integer(3),
                    }
                    .into(),
                ))
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(parser.parse().unwrap(), vec![expected],);
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
                .set_item(SQLExpression::Binary(
                    BinaryOperatorExpression {
                        operator: BinaryOperator::Add,
                        lhs: SQLExpression::Integer(1),
                        rhs: SQLExpression::Binary(
                            BinaryOperatorExpression {
                                operator: BinaryOperator::Mul,
                                lhs: SQLExpression::Integer(2),
                                rhs: SQLExpression::Integer(3),
                            }
                            .into(),
                        ),
                    }
                    .into(),
                ))
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(parser.parse().unwrap(), vec![expected],);
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
                .set_item(SQLExpression::Binary(
                    BinaryOperatorExpression {
                        operator: BinaryOperator::Add,
                        lhs: SQLExpression::Binary(
                            BinaryOperatorExpression {
                                operator: BinaryOperator::Add,
                                lhs: SQLExpression::Integer(1),
                                rhs: SQLExpression::Binary(
                                    BinaryOperatorExpression {
                                        operator: BinaryOperator::Mul,
                                        lhs: SQLExpression::Integer(2),
                                        rhs: SQLExpression::Integer(3),
                                    }
                                    .into(),
                                ),
                            }
                            .into(),
                        ),
                        rhs: SQLExpression::Integer(4),
                    }
                    .into(),
                ))
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(parser.parse().unwrap(), vec![expected],);
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
                .set_item(SQLExpression::Binary(
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
                ))
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(parser.parse().unwrap(), vec![expected],);
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
                .set_item(SQLExpression::Binary(
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
                ))
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(parser.parse().unwrap(), vec![expected],);
}

#[test]
pub fn function_call_expression_1() {
    let text = r#"
        SELECT coalesce(null, 1) as foo
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
        .add_select_item(
            SelectItem::builder()
                .set_item(SQLExpression::FunctionCall(CallExpression {
                    function_name: FunctionName {
                        database_name: None,
                        function_name: "coalesce".into(),
                    },
                    arguments: vec![SQLExpression::Null, SQLExpression::Integer(1)],
                }))
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(parser.parse().unwrap(), vec![expected],);
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
                .set_item(SQLExpression::Between(
                    BetweenExpression {
                        a: SQLExpression::Integer(3),
                        x: SQLExpression::Integer(1),
                        y: SQLExpression::Integer(5),
                    }
                    .into(),
                ))
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(parser.parse().unwrap(), vec![expected],);
}
