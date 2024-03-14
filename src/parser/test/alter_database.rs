#![cfg(test)]
use crate::ast::ddl::{AlterDatabaseAction, AlterDatabaseQuery, AlterDatabaseRenameTo};
use crate::parser::context::ParserContext;
use crate::parser::predule::Parser;

#[test]
pub fn alter_database_1() {
    let text = r#"
        ALTER DATABASE foo RENAME TO bar;
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = AlterDatabaseQuery::builder()
        .set_name("foo".to_owned())
        .set_action(AlterDatabaseAction::RenameTo(AlterDatabaseRenameTo {
            name: "bar".into(),
        }))
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected],
    );
}

#[test]
pub fn alter_database_2() {
    let text = r#"
        ALTER DATABASE foo;
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = AlterDatabaseQuery::builder()
        .set_name("foo".to_owned())
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected],
    );
}
