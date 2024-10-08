#![cfg(test)]

use crate::ast::other::desc_table::DescTableQuery;
use crate::ast::other::show_databases::ShowDatabasesQuery;
use crate::ast::other::show_tables::ShowTablesQuery;
use crate::ast::other::use_database::UseDatabaseQuery;
use crate::ast::types::TableName;
use crate::ast::SQLStatement;
use crate::lexer::tokens::Token;
use crate::parser::context::ParserContext;
use crate::parser::predule::Parser;

#[test]
fn test_parse_show_query() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        context: ParserContext,
        expected: SQLStatement,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "Show Databases".into(),
            input: vec![Token::Databases],
            context: Default::default(),
            expected: ShowDatabasesQuery {}.into(),
            want_error: false,
        },
        TestCase {
            name: "Show Tables".into(),
            input: vec![Token::Tables],
            context: ParserContext::default().set_default_database("rrdb".into()),
            expected: ShowTablesQuery {
                database: "rrdb".into(),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "오류: 빈 토큰".into(),
            input: vec![],
            context: Default::default(),
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: DELETe".into(),
            input: vec![Token::Delete],
            context: Default::default(),
            expected: Default::default(),
            want_error: true,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got = parser.parse_show_query(t.context);

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

#[test]
fn test_parse_desc_query() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        context: ParserContext,
        expected: SQLStatement,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "Desc Table".into(),
            input: vec![Token::Identifier("asdf".into())],
            context: Default::default(),
            expected: DescTableQuery {
                table_name: TableName {
                    database_name: None,
                    table_name: "asdf".into(),
                },
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "Desc Table with Database".into(),
            input: vec![
                Token::Identifier("rrdb".into()),
                Token::Period,
                Token::Identifier("asdf".into()),
            ],
            context: Default::default(),
            expected: DescTableQuery {
                table_name: TableName {
                    database_name: Some("rrdb".into()),
                    table_name: "asdf".into(),
                },
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "오류: 빈 토큰".into(),
            input: vec![],
            context: Default::default(),
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: DELETe".into(),
            input: vec![Token::Delete],
            context: Default::default(),
            expected: Default::default(),
            want_error: true,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got = parser.parse_desc_query(t.context);

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

#[test]
fn test_parse_backslash_query() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        context: ParserContext,
        expected: SQLStatement,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "\\l".into(),
            input: vec![Token::Identifier("l".into())],
            context: Default::default(),
            expected: ShowDatabasesQuery {}.into(),
            want_error: false,
        },
        TestCase {
            name: "알 수없는 식별자: \\r".into(),
            input: vec![Token::Identifier("r".into())],
            context: Default::default(),
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: 빈 토큰".into(),
            input: vec![],
            context: Default::default(),
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: DELETE".into(),
            input: vec![Token::Delete],
            context: Default::default(),
            expected: Default::default(),
            want_error: true,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got = parser.parse_backslash_query(t.context);

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

#[test]
fn test_parse_use_query() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: SQLStatement,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "Use Database".into(),
            input: vec![Token::Identifier("asdf".into())],
            expected: UseDatabaseQuery {
                database_name: "asdf".into(),
            }
            .into(),
            want_error: false,
        },
        TestCase {
            name: "오류: 빈 토큰".into(),
            input: vec![],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: DELETe".into(),
            input: vec![Token::Delete],
            expected: Default::default(),
            want_error: true,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got = parser.parse_use_query(Default::default());

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
