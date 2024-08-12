#![cfg(test)]
use crate::ast::types::{Column, DataType, SelectColumn, TableName};
use crate::lexer::tokens::Token;
use crate::parser::context::ParserContext;
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

#[test]
fn test_parse_data_type() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: DataType,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "INT".into(),
            input: vec![Token::Identifier("INT".into())],
            expected: DataType::Int,
            want_error: false,
        },
        TestCase {
            name: "오류: 빈 토큰".into(),
            input: vec![],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: VARCHAR".into(),
            input: vec![Token::Identifier("VARCHAR".into())],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: VARCHAR)".into(),
            input: vec![Token::Identifier("VARCHAR".into()), Token::RightParentheses],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: VARCHAR(".into(),
            input: vec![Token::Identifier("VARCHAR".into()), Token::LeftParentheses],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: VARCHAR(TRUE".into(),
            input: vec![
                Token::Identifier("VARCHAR".into()),
                Token::LeftParentheses,
                Token::Boolean(true),
            ],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: VARCHAR(444".into(),
            input: vec![
                Token::Identifier("VARCHAR".into()),
                Token::LeftParentheses,
                Token::Integer(444),
            ],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: VARCHAR(444(".into(),
            input: vec![
                Token::Identifier("VARCHAR".into()),
                Token::LeftParentheses,
                Token::Integer(444),
                Token::LeftParentheses,
            ],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: asdf".into(),
            input: vec![Token::Identifier("asdf".into())],
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

        let got: Result<_, crate::errors::RRDBError> = parser.parse_data_type();

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

#[test]
fn test_parse_table_name() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: TableName,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "table_name".into(),
            input: vec![Token::Identifier("table_name".into())],
            expected: TableName::new(None, "table_name".into()),
            want_error: false,
        },
        TestCase {
            name: "dd.table_name".into(),
            input: vec![
                Token::Identifier("dd".into()),
                Token::Period,
                Token::Identifier("table_name".into()),
            ],
            expected: TableName::new(Some("dd".into()), "table_name".into()),
            want_error: false,
        },
        TestCase {
            name: "오류: dd.".into(),
            input: vec![Token::Identifier("dd".into()), Token::Period],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: dd.DELETE".into(),
            input: vec![Token::Identifier("dd".into()), Token::Period, Token::Delete],
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

        let got: Result<_, crate::errors::RRDBError> = parser.parse_table_name(Default::default());

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

#[test]
fn test_has_if_not_exists() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: bool,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "IF NOT EXISTS".into(),
            input: vec![Token::If, Token::Not, Token::Exists],
            expected: true,
            want_error: false,
        },
        TestCase {
            name: "IF NOT DELETE".into(),
            input: vec![Token::If, Token::Not, Token::Delete],
            expected: false,
            want_error: true,
        },
        TestCase {
            name: "IF NOT".into(),
            input: vec![Token::If, Token::Not],
            expected: false,
            want_error: true,
        },
        TestCase {
            name: "IF".into(),
            input: vec![Token::If],
            expected: false,
            want_error: true,
        },
        TestCase {
            name: "오류: 빈 토큰".into(),
            input: vec![],
            expected: false,
            want_error: true,
        },
        TestCase {
            name: "DELETE".into(),
            input: vec![Token::Delete],
            expected: false,
            want_error: false,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got: Result<_, crate::errors::RRDBError> = parser.has_if_not_exists();

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

#[test]
fn test_has_if_exists() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: bool,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "IF EXISTS".into(),
            input: vec![Token::If, Token::Exists],
            expected: true,
            want_error: false,
        },
        TestCase {
            name: "IF DELETE".into(),
            input: vec![Token::If, Token::Delete],
            expected: false,
            want_error: true,
        },
        TestCase {
            name: "IF".into(),
            input: vec![Token::If],
            expected: false,
            want_error: true,
        },
        TestCase {
            name: "오류: 빈 토큰".into(),
            input: vec![],
            expected: false,
            want_error: true,
        },
        TestCase {
            name: "DELETE".into(),
            input: vec![Token::Delete],
            expected: false,
            want_error: false,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got: Result<_, crate::errors::RRDBError> = parser.has_if_exists();

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

#[test]
fn test_parse_select_column() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: SelectColumn,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "id".into(),
            input: vec![Token::Identifier("id".into())],
            expected: SelectColumn::new(None, "id".into()),
            want_error: false,
        },
        TestCase {
            name: "foo.id".into(),
            input: vec![
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("id".into()),
            ],
            expected: SelectColumn::new(Some("foo".into()), "id".into()),
            want_error: false,
        },
        TestCase {
            name: "오류: foo.DELETE".into(),
            input: vec![
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Delete,
            ],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "DELETE".into(),
            input: vec![Token::Delete],
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

        let got: Result<_, crate::errors::RRDBError> = parser.parse_select_column();

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

#[test]
fn test_next_token_is_binary_operator() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: bool,
        context: ParserContext,
    }

    let test_cases = vec![
        TestCase {
            name: "토큰 없음".into(),
            input: vec![],
            expected: false,
            context: Default::default(),
        },
        TestCase {
            name: "AND".into(),
            input: vec![Token::And],
            expected: true,
            context: Default::default(),
        },
        TestCase {
            name: "IS".into(),
            input: vec![Token::Is],
            expected: true,
            context: Default::default(),
        },
        TestCase {
            name: "NOT".into(),
            input: vec![Token::Not],
            expected: false,
            context: Default::default(),
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got: bool = parser.next_token_is_binary_operator(t.context);

        assert_eq!(got, t.expected, "TC: {}", t.name);
    }
}
