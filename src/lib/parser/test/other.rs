#![cfg(test)]

use crate::lib::ast::other::UseDatabaseQuery;
use crate::lib::ast::predule::ShowDatabasesQuery;
use crate::lib::parser::context::ParserContext;
use crate::lib::parser::predule::Parser;

#[test]
pub fn show_databases_1() {
    let text = r#"
        Show Databases
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = ShowDatabasesQuery {};

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn show_databases_2() {
    let text = r#"
        \l
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = ShowDatabasesQuery {};

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn use_databases_1() {
    let text = r#"
        Use asdf;
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = UseDatabaseQuery {
        database_name: "asdf".into(),
    };

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}
