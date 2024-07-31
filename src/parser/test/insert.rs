#![cfg(test)]

use crate::ast::dml::insert::InsertQuery;
use crate::ast::dml::parts::insert_values::InsertValue;
use crate::ast::dml::parts::select_item::SelectItem;
use crate::ast::dml::select::SelectQuery;
use crate::ast::types::{SQLExpression, SelectColumn, TableName};
use crate::lexer::tokens::Token;
use crate::parser::predule::Parser;

#[test]
fn test_insert_query() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: InsertQuery,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "INSERT INTO foo.bar(a, b, c) Values(1, 2, 3)".into(),
            input: vec![
                Token::Insert,
                Token::Into,
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("bar".into()),
                Token::LeftParentheses,
                Token::Identifier("a".into()),
                Token::Comma,
                Token::Identifier("b".into()),
                Token::Comma,
                Token::Identifier("c".into()),
                Token::RightParentheses,
                Token::Values,
                Token::LeftParentheses,
                Token::Integer(1),
                Token::Comma,
                Token::Integer(2),
                Token::Comma,
                Token::Integer(3),
                Token::RightParentheses,
            ],
            expected: InsertQuery::builder()
                .set_into_table(TableName {
                    database_name: Some("foo".into()),
                    table_name: "bar".into(),
                })
                .set_columns(vec!["a".into(), "b".into(), "c".into()])
                .set_values(vec![InsertValue {
                    list: vec![
                        Some(SQLExpression::Integer(1)),
                        Some(SQLExpression::Integer(2)),
                        Some(SQLExpression::Integer(3)),
                    ],
                }])
                .build(),
            want_error: false,
        },
        TestCase {
            name: "INSERT INTO foo.bar(a, b, c) Values(1, 2, 3), (4, 5, 6)".into(),
            input: vec![
                Token::Insert,
                Token::Into,
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("bar".into()),
                Token::LeftParentheses,
                Token::Identifier("a".into()),
                Token::Comma,
                Token::Identifier("b".into()),
                Token::Comma,
                Token::Identifier("c".into()),
                Token::RightParentheses,
                Token::Values,
                Token::LeftParentheses,
                Token::Integer(1),
                Token::Comma,
                Token::Integer(2),
                Token::Comma,
                Token::Integer(3),
                Token::RightParentheses,
                Token::Comma,
                Token::LeftParentheses,
                Token::Integer(4),
                Token::Comma,
                Token::Integer(5),
                Token::Comma,
                Token::Integer(6),
                Token::RightParentheses,
            ],
            expected: InsertQuery::builder()
                .set_into_table(TableName {
                    database_name: Some("foo".into()),
                    table_name: "bar".into(),
                })
                .set_columns(vec!["a".into(), "b".into(), "c".into()])
                .set_values(vec![
                    InsertValue {
                        list: vec![
                            Some(SQLExpression::Integer(1)),
                            Some(SQLExpression::Integer(2)),
                            Some(SQLExpression::Integer(3)),
                        ],
                    },
                    InsertValue {
                        list: vec![
                            Some(SQLExpression::Integer(4)),
                            Some(SQLExpression::Integer(5)),
                            Some(SQLExpression::Integer(6)),
                        ],
                    },
                ])
                .build(),
            want_error: false,
        },
        TestCase {
            name: "INSERT INTO foo.bar(a, b, c) Values(1, 2, 3), (4, 5, DEFAULT)".into(),
            input: vec![
                Token::Insert,
                Token::Into,
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("bar".into()),
                Token::LeftParentheses,
                Token::Identifier("a".into()),
                Token::Comma,
                Token::Identifier("b".into()),
                Token::Comma,
                Token::Identifier("c".into()),
                Token::RightParentheses,
                Token::Values,
                Token::LeftParentheses,
                Token::Integer(1),
                Token::Comma,
                Token::Integer(2),
                Token::Comma,
                Token::Integer(3),
                Token::RightParentheses,
                Token::Comma,
                Token::LeftParentheses,
                Token::Integer(4),
                Token::Comma,
                Token::Integer(5),
                Token::Comma,
                Token::Default,
                Token::RightParentheses,
            ],
            expected: InsertQuery::builder()
                .set_into_table(TableName {
                    database_name: Some("foo".into()),
                    table_name: "bar".into(),
                })
                .set_columns(vec!["a".into(), "b".into(), "c".into()])
                .set_values(vec![
                    InsertValue {
                        list: vec![
                            Some(SQLExpression::Integer(1)),
                            Some(SQLExpression::Integer(2)),
                            Some(SQLExpression::Integer(3)),
                        ],
                    },
                    InsertValue {
                        list: vec![
                            Some(SQLExpression::Integer(4)),
                            Some(SQLExpression::Integer(5)),
                            None,
                        ],
                    },
                ])
                .build(),
            want_error: false,
        },
        TestCase {
            name: "INSERT INTO foo.bar(a, b, c) Select s.a, s.b, s.c from boom.some as s".into(),
            input: vec![
                Token::Insert,
                Token::Into,
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("bar".into()),
                Token::LeftParentheses,
                Token::Identifier("a".into()),
                Token::Comma,
                Token::Identifier("b".into()),
                Token::Comma,
                Token::Identifier("c".into()),
                Token::RightParentheses,
                Token::Select,
                Token::Identifier("s".into()),
                Token::Period,
                Token::Identifier("a".into()),
                Token::Comma,
                Token::Identifier("s".into()),
                Token::Period,
                Token::Identifier("b".into()),
                Token::Comma,
                Token::Identifier("s".into()),
                Token::Period,
                Token::Identifier("c".into()),
                Token::From,
                Token::Identifier("boom".into()),
                Token::Period,
                Token::Identifier("some".into()),
                Token::As,
                Token::Identifier("s".into()),
            ],
            expected: InsertQuery::builder()
                .set_into_table(TableName {
                    database_name: Some("foo".into()),
                    table_name: "bar".into(),
                })
                .set_columns(vec!["a".into(), "b".into(), "c".into()])
                .set_select(
                    SelectQuery::builder()
                        .add_select_item(
                            SelectItem::builder()
                                .set_item(SelectColumn::new(Some("s".into()), "a".into()).into())
                                .build(),
                        )
                        .add_select_item(
                            SelectItem::builder()
                                .set_item(SelectColumn::new(Some("s".into()), "b".into()).into())
                                .build(),
                        )
                        .add_select_item(
                            SelectItem::builder()
                                .set_item(SelectColumn::new(Some("s".into()), "c".into()).into())
                                .build(),
                        )
                        .set_from_table(TableName {
                            database_name: Some("boom".into()),
                            table_name: "some".into(),
                        })
                        .set_from_alias("s".into())
                        .build(),
                )
                .build(),
            want_error: false,
        },
        TestCase {
            name: "실패: 빈 토큰".into(),
            input: vec![],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "실패: INSERT".into(),
            input: vec![Token::Insert],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "실패: SELECT".into(),
            input: vec![Token::Select],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "실패: INSERT INTO foo.bar".into(),
            input: vec![
                Token::Insert,
                Token::Into,
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("bar".into()),
            ],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "실패: INSERT SELECT".into(),
            input: vec![Token::Insert, Token::Select],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "실패: INSERT INTO foo.bar(".into(),
            input: vec![
                Token::Insert,
                Token::Into,
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("bar".into()),
                Token::LeftParentheses,
            ],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "실패: INSERT INTO foo.bar)".into(),
            input: vec![
                Token::Insert,
                Token::Into,
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("bar".into()),
                Token::RightParentheses,
            ],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "실패: INSERT INTO foo.bar(a,b".into(),
            input: vec![
                Token::Insert,
                Token::Into,
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("bar".into()),
                Token::LeftParentheses,
                Token::Identifier("a".into()),
                Token::Comma,
                Token::Identifier("b".into()),
            ],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "실패: INSERT INTO foo.bar(a,b) INSERT".into(),
            input: vec![
                Token::Insert,
                Token::Into,
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("bar".into()),
                Token::LeftParentheses,
                Token::Identifier("a".into()),
                Token::Comma,
                Token::Identifier("b".into()),
                Token::RightParentheses,
                Token::Insert,
            ],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "실패: INSERT INTO foo.bar(a,b)".into(),
            input: vec![
                Token::Insert,
                Token::Into,
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("bar".into()),
                Token::LeftParentheses,
                Token::Identifier("a".into()),
                Token::Comma,
                Token::Identifier("b".into()),
                Token::RightParentheses,
            ],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "실패: INSERT INTO foo.bar(a, b, c) Values(1, 2), (4, 5)".into(),
            input: vec![
                Token::Insert,
                Token::Into,
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("bar".into()),
                Token::LeftParentheses,
                Token::Identifier("a".into()),
                Token::Comma,
                Token::Identifier("b".into()),
                Token::Comma,
                Token::Identifier("c".into()),
                Token::RightParentheses,
                Token::Values,
                Token::LeftParentheses,
                Token::Integer(1),
                Token::Comma,
                Token::Integer(2),
                Token::RightParentheses,
                Token::Comma,
                Token::LeftParentheses,
                Token::Integer(4),
                Token::Comma,
                Token::Integer(5),
                Token::RightParentheses,
            ],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "실패: INSERT INTO foo.bar(a, b, c) Select s.a, s.b from boom.some as s".into(),
            input: vec![
                Token::Insert,
                Token::Into,
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("bar".into()),
                Token::LeftParentheses,
                Token::Identifier("a".into()),
                Token::Comma,
                Token::Identifier("b".into()),
                Token::Comma,
                Token::Identifier("c".into()),
                Token::RightParentheses,
                Token::Select,
                Token::Identifier("s".into()),
                Token::Period,
                Token::Identifier("a".into()),
                Token::Comma,
                Token::Identifier("s".into()),
                Token::Period,
                Token::Identifier("b".into()),
                Token::From,
                Token::Identifier("boom".into()),
                Token::Period,
                Token::Identifier("some".into()),
                Token::As,
                Token::Identifier("s".into()),
            ],
            expected: Default::default(),
            want_error: true,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got = parser.handle_insert_query(Default::default());

        assert_eq!(
            got.is_err(),
            t.want_error,
            "{}: want_error: {}, error: {:?}",
            t.name,
            t.want_error,
            got.err()
        );

        if let Ok(statements) = got {
            assert_eq!(statements, t.expected.into(), "TC: {}", t.name);
        }
    }
}
