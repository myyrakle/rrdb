#![cfg(test)]

use crate::{
    ast::{
        ddl::{
            alter_database::{AlterDatabaseAction, AlterDatabaseQuery, AlterDatabaseRenameTo},
            alter_table::{AlterTableAction, AlterTableQuery, AlterTableRenameColumn},
            create_database::CreateDatabaseQuery,
            create_table::CreateTableQuery,
            drop_database::DropDatabaseQuery,
            drop_table::DropTableQuery,
        },
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

#[test]
fn test_handle_alter_query() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: SQLStatement,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "ALTER DATABASE foo RENAME TO bar".into(),
            input: vec![
                Token::Database,
                Token::Identifier("foo".to_owned()),
                Token::Rename,
                Token::To,
                Token::Identifier("bar".to_owned()),
            ],
            expected: AlterDatabaseQuery::builder()
                .set_name("foo".to_owned())
                .set_action(AlterDatabaseAction::RenameTo(AlterDatabaseRenameTo {
                    name: "bar".into(),
                }))
                .build()
                .into(),
            want_error: false,
        },
        TestCase {
            name: "ALTER TABLE foo RENAME a to b".into(),
            input: vec![
                Token::Table,
                Token::Identifier("foo".to_owned()),
                Token::Rename,
                Token::Identifier("a".to_owned()),
                Token::To,
                Token::Identifier("b".to_owned()),
            ],
            expected: AlterTableQuery::builder()
                .set_table(TableName::new(None, "foo".to_owned()))
                .set_action(AlterTableAction::RenameColumn(AlterTableRenameColumn {
                    from_name: "a".into(),
                    to_name: "b".into(),
                }))
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
            name: "오류: NULL".into(),
            input: vec![
                Token::Null,
                Token::Null,
                Token::Null,
                Token::Null,
                Token::Null,
            ],
            expected: Default::default(),
            want_error: true,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got = parser.handle_alter_query(Default::default());

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
fn test_handle_drop_query() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: SQLStatement,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "DROP DATABASE foo".into(),
            input: vec![Token::Database, Token::Identifier("foo".to_owned())],
            expected: DropDatabaseQuery::builder()
                .set_name("foo".to_owned())
                .build()
                .into(),
            want_error: false,
        },
        TestCase {
            name: "DROP TABLE foo".into(),
            input: vec![Token::Table, Token::Identifier("foo".to_owned())],
            expected: DropTableQuery::builder()
                .set_table(TableName::new(None, "foo".to_owned()))
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
            name: "오류: NULL".into(),
            input: vec![Token::Null, Token::Null],
            expected: Default::default(),
            want_error: true,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got = parser.handle_drop_query(Default::default());

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
fn test_handle_create_table_query() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: SQLStatement,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "CREATE TABLE foo(id INT PRIMARY KEY)".into(),
            input: vec![
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
            name: "오류: 빈 토큰".into(),
            input: vec![],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: CREATE TABLE foo".into(),
            input: vec![Token::Identifier("foo".to_owned())],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: CREATE TABLE foo)".into(),
            input: vec![Token::Identifier("foo".to_owned()), Token::RightParentheses],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: CREATE TABLE foo(".into(),
            input: vec![Token::Identifier("foo".to_owned()), Token::LeftParentheses],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: CREATE TABLE foo(id INT PRIMARY KEY".into(),
            input: vec![
                Token::Identifier("foo".to_owned()),
                Token::LeftParentheses,
                Token::Identifier("id".to_owned()),
                Token::Identifier("INT".to_owned()),
                Token::Primary,
                Token::Key,
            ],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: CREATE TABLE foo(id INT PRIMARY KEY(".into(),
            input: vec![
                Token::Identifier("foo".to_owned()),
                Token::LeftParentheses,
                Token::Identifier("id".to_owned()),
                Token::Identifier("INT".to_owned()),
                Token::Primary,
                Token::Key,
                Token::LeftParentheses,
            ],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "오류: CREATE TABLE foo(id INT PRIMARY KEY))".into(),
            input: vec![
                Token::Identifier("foo".to_owned()),
                Token::LeftParentheses,
                Token::Identifier("id".to_owned()),
                Token::Identifier("INT".to_owned()),
                Token::Primary,
                Token::Key,
                Token::RightParentheses,
                Token::RightParentheses,
            ],
            expected: Default::default(),
            want_error: true,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got = parser.handle_create_table_query(Default::default());

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
