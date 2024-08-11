#![cfg(test)]
use crate::ast::types::{Column, DataType};
use crate::lexer::tokens::Token;
use crate::parser::predule::Parser;

#[test]
fn test_parse_table_column() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: Column,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "id INT PRIMARY KEY".into(),
            input: vec![
                Token::Identifier("id".into()),
                Token::Identifier("INT".into()),
                Token::Primary,
                Token::Key,
            ],
            expected: Column {
                name: "id".into(),
                data_type: DataType::Int,
                primary_key: true,
                comment: "".into(),
                not_null: true,
                default: None,
            },
            want_error: false,
        },
        TestCase {
            name: "오류: id INT PRIMARY".into(),
            input: vec![
                Token::Identifier("id".into()),
                Token::Identifier("INT".into()),
                Token::Primary,
            ],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: id INT PRIMARY NULL".into(),
            input: vec![
                Token::Identifier("id".into()),
                Token::Identifier("INT".into()),
                Token::Primary,
                Token::Null,
            ],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "id INT NOT NULL".into(),
            input: vec![
                Token::Identifier("id".into()),
                Token::Identifier("INT".into()),
                Token::Not,
                Token::Null,
            ],
            expected: Column {
                name: "id".into(),
                data_type: DataType::Int,
                primary_key: false,
                comment: "".into(),
                not_null: true,
                default: None,
            },
            want_error: false,
        },
        TestCase {
            name: "id INT NULL".into(),
            input: vec![
                Token::Identifier("id".into()),
                Token::Identifier("INT".into()),
                Token::Null,
            ],
            expected: Column {
                name: "id".into(),
                data_type: DataType::Int,
                primary_key: false,
                comment: "".into(),
                not_null: false,
                default: None,
            },
            want_error: false,
        },
        TestCase {
            name: "오류: id INT NOT".into(),
            input: vec![
                Token::Identifier("id".into()),
                Token::Identifier("INT".into()),
                Token::Not,
            ],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: id INT NOT TABLE".into(),
            input: vec![
                Token::Identifier("id".into()),
                Token::Identifier("INT".into()),
                Token::Not,
                Token::Table,
            ],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "id INT COMMENT 'foo'".into(),
            input: vec![
                Token::Identifier("id".into()),
                Token::Identifier("INT".into()),
                Token::Comment,
                Token::String("foo".into()),
            ],
            expected: Column {
                name: "id".into(),
                data_type: DataType::Int,
                primary_key: false,
                comment: "foo".into(),
                not_null: false,
                default: None,
            },
            want_error: false,
        },
        TestCase {
            name: "오류: id INT COMMENT".into(),
            input: vec![
                Token::Identifier("id".into()),
                Token::Identifier("INT".into()),
                Token::Comment,
            ],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: id INT COMMENT DELETE".into(),
            input: vec![
                Token::Identifier("id".into()),
                Token::Identifier("INT".into()),
                Token::Comment,
                Token::Delete,
            ],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: id INT DEFAULT (아직 미구현)".into(),
            input: vec![
                Token::Identifier("id".into()),
                Token::Identifier("INT".into()),
                Token::Default,
            ],
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
            name: "오류: DELETE".into(),
            input: vec![Token::Delete],
            expected: Default::default(),
            want_error: true,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got: Result<_, crate::errors::RRDBError> = parser.parse_table_column();

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
