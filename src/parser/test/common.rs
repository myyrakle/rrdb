#![cfg(test)]
use crate::ast::dml::expressions::subquery::SubqueryExpression;
use crate::ast::dml::parts::join::JoinType;
use crate::ast::dml::parts::select_item::SelectItem;
use crate::ast::dml::select::SelectQuery;
use crate::ast::types::{Column, DataType, SQLExpression, SelectColumn, TableName};
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
            name: "IF NULL".into(),
            input: vec![Token::If, Token::Null],
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

#[test]
fn test_next_token_is_right_parentheses() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "토큰 없음".into(),
            input: vec![],
            expected: false,
        },
        TestCase {
            name: ")".into(),
            input: vec![Token::RightParentheses],
            expected: true,
        },
        TestCase {
            name: "(".into(),
            input: vec![Token::LeftParentheses],
            expected: false,
        },
        TestCase {
            name: "DELETE".into(),
            input: vec![Token::Delete],
            expected: false,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got: bool = parser.next_token_is_right_parentheses();

        assert_eq!(got, t.expected, "TC: {}", t.name);
    }
}

#[test]
fn test_next_token_is_comma() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "토큰 없음".into(),
            input: vec![],
            expected: false,
        },
        TestCase {
            name: ",".into(),
            input: vec![Token::Comma],
            expected: true,
        },
        TestCase {
            name: "(".into(),
            input: vec![Token::LeftParentheses],
            expected: false,
        },
        TestCase {
            name: "DELETE".into(),
            input: vec![Token::Delete],
            expected: false,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got: bool = parser.next_token_is_comma();

        assert_eq!(got, t.expected, "TC: {}", t.name);
    }
}

#[test]
fn test_next_token_is_between() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "토큰 없음".into(),
            input: vec![],
            expected: false,
        },
        TestCase {
            name: "BETWEEN".into(),
            input: vec![Token::Between],
            expected: true,
        },
        TestCase {
            name: "NOT BETWEEN".into(),
            input: vec![Token::Not, Token::Between],
            expected: true,
        },
        TestCase {
            name: "NOT NULL".into(),
            input: vec![Token::Not, Token::Null],
            expected: false,
        },
        TestCase {
            name: "NOT".into(),
            input: vec![Token::Not],
            expected: false,
        },
        TestCase {
            name: "(".into(),
            input: vec![Token::LeftParentheses],
            expected: false,
        },
        TestCase {
            name: "DELETE".into(),
            input: vec![Token::Delete],
            expected: false,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got: bool = parser.next_token_is_between();

        assert_eq!(got, t.expected, "TC: {}", t.name);
    }
}

#[test]
fn test_next_token_is_table_alias() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "토큰 없음".into(),
            input: vec![],
            expected: false,
        },
        TestCase {
            name: "AS".into(),
            input: vec![Token::As],
            expected: true,
        },
        TestCase {
            name: "DELETE".into(),
            input: vec![Token::Delete],
            expected: false,
        },
        TestCase {
            name: "foo".into(),
            input: vec![Token::Identifier("foo".into())],
            expected: true,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got: bool = parser.next_token_is_table_alias();

        assert_eq!(got, t.expected, "TC: {}", t.name);
    }
}

#[test]
fn test_next_token_is_order_by() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "토큰 없음".into(),
            input: vec![],
            expected: false,
        },
        TestCase {
            name: "ORDER".into(),
            input: vec![Token::Order],
            expected: false,
        },
        TestCase {
            name: "ORDER BY".into(),
            input: vec![Token::Order, Token::By],
            expected: true,
        },
        TestCase {
            name: "ORDER DELETE".into(),
            input: vec![Token::Order, Token::Delete],
            expected: false,
        },
        TestCase {
            name: "DELETE".into(),
            input: vec![Token::Delete],
            expected: false,
        },
        TestCase {
            name: "foo".into(),
            input: vec![Token::Identifier("foo".into())],
            expected: false,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got: bool = parser.next_token_is_order_by();

        assert_eq!(got, t.expected, "TC: {}", t.name);
    }
}

#[test]
fn test_next_token_is_group_by() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "토큰 없음".into(),
            input: vec![],
            expected: false,
        },
        TestCase {
            name: "GROUP".into(),
            input: vec![Token::Group],
            expected: false,
        },
        TestCase {
            name: "GROUP BY".into(),
            input: vec![Token::Group, Token::By],
            expected: true,
        },
        TestCase {
            name: "GROUP DELETE".into(),
            input: vec![Token::Group, Token::Delete],
            expected: false,
        },
        TestCase {
            name: "DELETE".into(),
            input: vec![Token::Delete],
            expected: false,
        },
        TestCase {
            name: "foo".into(),
            input: vec![Token::Identifier("foo".into())],
            expected: false,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got: bool = parser.next_token_is_group_by();

        assert_eq!(got, t.expected, "TC: {}", t.name);
    }
}

#[test]
fn test_next_token_is_column() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "토큰 없음".into(),
            input: vec![],
            expected: false,
        },
        TestCase {
            name: "column".into(),
            input: vec![Token::Column],
            expected: true,
        },
        TestCase {
            name: "DELETE".into(),
            input: vec![Token::Delete],
            expected: false,
        },
        TestCase {
            name: "AS".into(),
            input: vec![Token::As],
            expected: false,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got: bool = parser.next_token_is_column();

        assert_eq!(got, t.expected, "TC: {}", t.name);
    }
}

#[test]
fn test_next_token_is_not_null() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "토큰 없음".into(),
            input: vec![],
            expected: false,
        },
        TestCase {
            name: "NOT".into(),
            input: vec![Token::Not],
            expected: false,
        },
        TestCase {
            name: "NOT NULL".into(),
            input: vec![Token::Not, Token::Null],
            expected: true,
        },
        TestCase {
            name: "NOT DELETE".into(),
            input: vec![Token::Not, Token::Delete],
            expected: false,
        },
        TestCase {
            name: "DELETE".into(),
            input: vec![Token::Delete],
            expected: false,
        },
        TestCase {
            name: "AS".into(),
            input: vec![Token::As],
            expected: false,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got: bool = parser.next_token_is_not_null();

        assert_eq!(got, t.expected, "TC: {}", t.name);
    }
}

#[test]
fn test_next_token_is_data_type() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "토큰 없음".into(),
            input: vec![],
            expected: false,
        },
        TestCase {
            name: "DATA TYPE".into(),
            input: vec![Token::Data, Token::Type],
            expected: true,
        },
        TestCase {
            name: "DATA".into(),
            input: vec![Token::Data],
            expected: false,
        },
        TestCase {
            name: "DATA DELETE".into(),
            input: vec![Token::Data, Token::Delete],
            expected: false,
        },
        TestCase {
            name: "AS".into(),
            input: vec![Token::As],
            expected: false,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got: bool = parser.next_token_is_data_type();

        assert_eq!(got, t.expected, "TC: {}", t.name);
    }
}

#[test]
fn test_next_token_is_default() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "토큰 없음".into(),
            input: vec![],
            expected: false,
        },
        TestCase {
            name: "DEFAULT".into(),
            input: vec![Token::Default],
            expected: true,
        },
        TestCase {
            name: "AS".into(),
            input: vec![Token::As],
            expected: false,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got: bool = parser.next_token_is_default();

        assert_eq!(got, t.expected, "TC: {}", t.name);
    }
}

#[test]
fn test_get_next_join_type() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: Option<JoinType>,
    }

    let test_cases = vec![
        TestCase {
            name: "토큰 없음".into(),
            input: vec![],
            expected: None,
        },
        TestCase {
            name: "Inner".into(),
            input: vec![Token::Inner],
            expected: None,
        },
        TestCase {
            name: "Inner DELETE".into(),
            input: vec![Token::Inner, Token::Delete],
            expected: None,
        },
        TestCase {
            name: "Inner Join".into(),
            input: vec![Token::Inner, Token::Join],
            expected: Some(JoinType::InnerJoin),
        },
        TestCase {
            name: "Left".into(),
            input: vec![Token::Left],
            expected: None,
        },
        TestCase {
            name: "Left Join".into(),
            input: vec![Token::Left, Token::Join],
            expected: Some(JoinType::LeftOuterJoin),
        },
        TestCase {
            name: "Left Outer Join".into(),
            input: vec![Token::Left, Token::Outer, Token::Join],
            expected: Some(JoinType::LeftOuterJoin),
        },
        TestCase {
            name: "Left Outer Delete".into(),
            input: vec![Token::Left, Token::Outer, Token::Delete],
            expected: None,
        },
        TestCase {
            name: "Left Outer".into(),
            input: vec![Token::Left, Token::Outer],
            expected: None,
        },
        TestCase {
            name: "Left Delete".into(),
            input: vec![Token::Left, Token::Delete],
            expected: None,
        },
        TestCase {
            name: "Right".into(),
            input: vec![Token::Right],
            expected: None,
        },
        TestCase {
            name: "Right Join".into(),
            input: vec![Token::Right, Token::Join],
            expected: Some(JoinType::RightOuterJoin),
        },
        TestCase {
            name: "Right Outer Join".into(),
            input: vec![Token::Right, Token::Outer, Token::Join],
            expected: Some(JoinType::RightOuterJoin),
        },
        TestCase {
            name: "Right Outer".into(),
            input: vec![Token::Right, Token::Outer],
            expected: None,
        },
        TestCase {
            name: "Right Outer Delete".into(),
            input: vec![Token::Right, Token::Outer, Token::Delete],
            expected: None,
        },
        TestCase {
            name: "Right Delete".into(),
            input: vec![Token::Right, Token::Delete],
            expected: None,
        },
        TestCase {
            name: "Full".into(),
            input: vec![Token::Full],
            expected: None,
        },
        TestCase {
            name: "Full Join".into(),
            input: vec![Token::Full, Token::Join],
            expected: Some(JoinType::FullOuterJoin),
        },
        TestCase {
            name: "Full Outer Join".into(),
            input: vec![Token::Full, Token::Outer, Token::Join],
            expected: Some(JoinType::FullOuterJoin),
        },
        TestCase {
            name: "Full Outer".into(),
            input: vec![Token::Full, Token::Outer],
            expected: None,
        },
        TestCase {
            name: "Full Outer Select".into(),
            input: vec![Token::Full, Token::Outer, Token::Select],
            expected: None,
        },
        TestCase {
            name: "Full Delete".into(),
            input: vec![Token::Full, Token::Delete],
            expected: None,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got = parser.get_next_join_type();

        assert_eq!(got, t.expected, "TC: {}", t.name);
    }
}

#[test]
fn test_parse_table_alias() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: String,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "토큰 없음".into(),
            input: vec![],
            expected: "".into(),
            want_error: true,
        },
        TestCase {
            name: "오류: Delete".into(),
            input: vec![Token::Delete],
            expected: "".into(),
            want_error: true,
        },
        TestCase {
            name: "오류: As".into(),
            input: vec![Token::As],
            expected: "".into(),
            want_error: true,
        },
        TestCase {
            name: "오류: As Delete".into(),
            input: vec![Token::As, Token::Delete],
            expected: "".into(),
            want_error: true,
        },
        TestCase {
            name: "As foo".into(),
            input: vec![Token::As, Token::Identifier("foo".into())],
            expected: "foo".into(),
            want_error: false,
        },
        TestCase {
            name: "foo".into(),
            input: vec![Token::Identifier("foo".into())],
            expected: "foo".into(),
            want_error: false,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got = parser.parse_table_alias();

        assert_eq!(got.is_err(), t.want_error, "TC: {}", t.name);

        if let Ok(alias) = got {
            assert_eq!(alias, t.expected, "TC: {}", t.name);
        }
    }
}

#[test]
fn test_parse_subquery() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: SubqueryExpression,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "(SELECT 1)".into(),
            input: vec![
                Token::LeftParentheses,
                Token::Select,
                Token::Integer(1),
                Token::RightParentheses,
            ],
            expected: SubqueryExpression::Select(Box::new(
                SelectQuery::builder()
                    .add_select_item(
                        SelectItem::builder()
                            .set_item(SQLExpression::Integer(1))
                            .build(),
                    )
                    .build(),
            )),
            want_error: false,
        },
        TestCase {
            name: "토큰 없음".into(),
            input: vec![],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: )".into(),
            input: vec![Token::RightParentheses],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: (".into(),
            input: vec![Token::LeftParentheses],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: (SELECT 1".into(),
            input: vec![Token::LeftParentheses, Token::Select, Token::Integer(1)],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: (SELECT 1;;".into(),
            input: vec![
                Token::LeftParentheses,
                Token::Select,
                Token::Integer(1),
                Token::SemiColon,
                Token::SemiColon,
            ],
            expected: Default::default(),
            want_error: true,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got = parser.parse_subquery(Default::default());

        assert_eq!(got.is_err(), t.want_error, "TC: {}", t.name);

        if let Ok(subquery) = got {
            assert_eq!(subquery, t.expected, "TC: {}", t.name);
        }
    }
}
