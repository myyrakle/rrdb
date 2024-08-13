#![cfg(test)]
use crate::ast::ddl::alter_database::{
    AlterDatabaseAction, AlterDatabaseQuery, AlterDatabaseRenameTo,
};
use crate::ast::SQLStatement;
use crate::lexer::tokens::Token;
use crate::parser::context::ParserContext;
use crate::parser::predule::Parser;

#[test]
fn test_handle_alter_database_query() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: SQLStatement,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "ALTER DATABASE foo RENAME TO bar;".into(),
            input: vec![
                Token::Identifier("foo".to_owned()),
                Token::Rename,
                Token::To,
                Token::Identifier("bar".to_owned()),
                Token::SemiColon,
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
            name: "ALTER DATABASE foo".into(),
            input: vec![Token::Identifier("foo".to_owned())],
            expected: AlterDatabaseQuery::builder()
                .set_name("foo".to_owned())
                .build()
                .into(),
            want_error: false,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got = parser.handle_alter_database_query();

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
