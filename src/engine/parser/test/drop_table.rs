#![cfg(test)]

use crate::engine::ast::ddl::drop_table::DropTableQuery;
use crate::engine::ast::types::TableName;
use crate::engine::parser::context::ParserContext;
use crate::engine::parser::predule::Parser;

#[test]
pub fn drop_table() {
    let text = r#"
        drop table if exists "foo_db".foo;
    "#
    .to_owned();

    let mut parser = Parser::with_string(text).unwrap();

    let expected = DropTableQuery::builder()
        .set_table(TableName::new(Some("foo_db".to_owned()), "foo".to_owned()))
        .set_if_exists(true)
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected],
    );
}
