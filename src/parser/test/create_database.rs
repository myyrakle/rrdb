#![cfg(test)]
use crate::ast::ddl::create_database::CreateDatabaseQuery;
use crate::ast::SQLStatement;
use crate::lexer::tokens::Token;
use crate::parser::predule::Parser;

#[test]
fn test_handle_create_database_query() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: SQLStatement,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "CREATE DATABASE test_db;".into(),
            input: vec![Token::Identifier("test_db".to_owned()), Token::SemiColon],
            expected: CreateDatabaseQuery::builder()
                .set_name("test_db".to_owned())
                .build()
                .into(),
            want_error: false,
        },
        TestCase {
            name: "CREATE DATABASE test_db".into(),
            input: vec![Token::Identifier("test_db".to_owned())],
            expected: CreateDatabaseQuery::builder()
                .set_name("test_db".to_owned())
                .build()
                .into(),
            want_error: false,
        },
        TestCase {
            name: "CREATE DATABASE IF NOT EXISTS test_db;".into(),
            input: vec![
                Token::If,
                Token::Not,
                Token::Exists,
                Token::Identifier("test_db".to_owned()),
                Token::SemiColon,
            ],
            expected: CreateDatabaseQuery::builder()
                .set_name("test_db".to_owned())
                .set_if_not_exists(true)
                .build()
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
            name: "오류: CREATE DATABASE IF NOT EXISTS".into(),
            input: vec![Token::If, Token::Not, Token::Exists],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "CREATE DATABASE IF NOT EXISTS DELETE;".into(),
            input: vec![
                Token::If,
                Token::Not,
                Token::Exists,
                Token::Delete,
                Token::SemiColon,
            ],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "CREATE DATABASE test_db DELETE".into(),
            input: vec![Token::Identifier("test_db".to_owned()), Token::Delete],
            expected: Default::default(),
            want_error: true,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got = parser.handle_create_database_query();

        assert_eq!(
            got.is_err(),
            t.want_error,
            "TC: {} Error: {:?}",
            t.name,
            got.err()
        );

        if let Ok(alias) = got {
            assert_eq!(alias, t.expected, "TC: {}", t.name);
        }
    }
}
