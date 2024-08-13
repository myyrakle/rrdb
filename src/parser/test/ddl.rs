#![cfg(test)]

use crate::{
    ast::{
        ddl::{create_database::CreateDatabaseQuery, create_table::CreateTableQuery},
        types::{Column, DataType, TableName},
        SQLStatement,
    },
    lexer::tokens::Token,
    parser::parser::Parser,
};

#[test]
fn test_handle_create_query() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: SQLStatement,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "CREATE DATABASE foo;".into(),
            input: vec![
                Token::Database,
                Token::Identifier("foo".to_owned()),
                Token::SemiColon,
            ],
            expected: CreateDatabaseQuery::builder()
                .set_name("foo".to_owned())
                .build()
                .into(),
            want_error: false,
        },
        TestCase {
            name: "CREATE TABLE foo(id INT PRIMARY KEY)".into(),
            input: vec![
                Token::Table,
                Token::Identifier("foo".to_owned()),
                Token::LeftParentheses,
                Token::Identifier("id".to_owned()),
                Token::Identifier("INT".to_owned()),
                Token::Primary,
                Token::Key,
                Token::RightParentheses,
            ],
            expected: CreateTableQuery::builder()
                .set_table(TableName::new(None, "foo".to_owned()))
                .add_column(
                    Column::builder()
                        .set_name("id".to_owned())
                        .set_data_type(DataType::Int)
                        .set_primary_key(true)
                        .build(),
                )
                .build()
                .into(),
            want_error: false,
        },
        TestCase {
            name: "오류: CREATE UPDATE".into(),
            input: vec![Token::Update],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: 빈 토큰".into(),
            input: vec![],
            expected: Default::default(),
            want_error: true,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);
        let got = parser.handle_create_query(Default::default());

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
