#![cfg(test)]

use crate::ast::other::desc_table::DescTableQuery;
use crate::ast::other::show_databases::ShowDatabasesQuery;
use crate::ast::other::show_tables::ShowTablesQuery;
use crate::ast::other::use_database::UseDatabaseQuery;
use crate::ast::types::TableName;
use crate::parser::context::ParserContext;
use crate::parser::predule::Parser;

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

#[test]
pub fn desc_table_1() {
    let text = r#"
        desc asdf;
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = DescTableQuery {
        table_name: TableName {
            database_name: None,
            table_name: "asdf".into(),
        },
    };

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn show_tables_1() {
    let text = r#"
        show tables;
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = ShowTablesQuery {
        database: "rrdb".into(),
    };

    assert_eq!(
        parser
            .parse(ParserContext::default().set_default_database("rrdb".into()))
            .unwrap(),
        vec![expected.into()],
    );
}
