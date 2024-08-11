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
            name: "id INT PRIMARY".into(),
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
            name: "오류: 빈 토큰".into(),
            input: vec![],
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
