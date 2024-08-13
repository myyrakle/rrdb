#![cfg(test)]
use crate::ast::ddl::drop_database::DropDatabaseQuery;
use crate::ast::SQLStatement;
use crate::lexer::tokens::Token;
use crate::parser::predule::Parser;

#[test]
fn test_handle_drop_database_query() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: SQLStatement,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "DROP DATABASE test_db;".into(),
            input: vec![Token::Identifier("test_db".to_owned()), Token::SemiColon],
            expected: DropDatabaseQuery::builder()
                .set_name("test_db".to_owned())
                .build()
                .into(),
            want_error: false,
        },
        TestCase {
            name: "DROP DATABASE test_db".into(),
            input: vec![Token::Identifier("test_db".to_owned())],
            expected: DropDatabaseQuery::builder()
                .set_name("test_db".to_owned())
                .build()
                .into(),
            want_error: false,
        },
        TestCase {
            name: "DROP DATABASE IF EXISTS test_db;".into(),
            input: vec![
                Token::If,
                Token::Exists,
                Token::Identifier("test_db".to_owned()),
                Token::SemiColon,
            ],
            expected: DropDatabaseQuery::builder()
                .set_name("test_db".to_owned())
                .set_if_exists(true)
                .build()
                .into(),
            want_error: false,
        },
        TestCase {
            name: "오류: DROP DATABASE IF EXISTS".into(),
            input: vec![Token::If, Token::Exists],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: DROP DATABASE IF EXISTS DELETE".into(),
            input: vec![Token::If, Token::Exists, Token::Delete],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: DROP DATABASE test_db&&".into(),
            input: vec![Token::Identifier("test_db".to_owned()), Token::And],
            expected: Default::default(),
            want_error: true,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got = parser.handle_drop_database_query();

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
