#![cfg(test)]
use crate::ast::ddl::DropDatabaseQuery;
use crate::parser::context::ParserContext;
use crate::parser::predule::Parser;

#[test]
pub fn drop_database() {
    let text = r#"
        DROP DATABASE IF exists test_db;
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = DropDatabaseQuery::builder()
        .set_name("test_db".to_owned())
        .set_if_exists(true)
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected],
    );
}
